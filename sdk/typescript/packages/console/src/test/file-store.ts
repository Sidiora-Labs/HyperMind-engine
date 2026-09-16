import { mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import type { SessionStore, UploadSession } from "../upload-session.js";

export interface FileSessionStoreOptions {
  maximumBytes?: number;
}

function decode(raw: string): UploadSession | undefined {
  let value: unknown;
  try {
    value = JSON.parse(raw);
  } catch {
    return undefined;
  }
  if (typeof value !== "object" || value === null || Array.isArray(value)) return undefined;
  const candidate = value as UploadSession;
  if (candidate.schema !== "hypermind.console-upload-session.v1") return undefined;
  if (typeof candidate.id !== "string" || candidate.id.length === 0) return undefined;
  return candidate;
}

export function fileSessionStore(directory: string, options: FileSessionStoreOptions = {}): SessionStore {
  const file = (id: string): string => path.join(directory, `${id}.json`);
  return {
    async read(id: string): Promise<UploadSession | undefined> {
      let raw: string;
      try {
        raw = await readFile(file(id), "utf8");
      } catch {
        return undefined;
      }
      return decode(raw);
    },
    async write(session: UploadSession): Promise<void> {
      const raw = JSON.stringify(session);
      const bytes = Buffer.byteLength(raw);
      if (options.maximumBytes !== undefined && bytes > options.maximumBytes) {
        throw new Error(`the store budget of ${options.maximumBytes} bytes cannot hold ${bytes} bytes`);
      }
      await mkdir(directory, { recursive: true });
      await writeFile(file(session.id), raw, "utf8");
    },
    async remove(id: string): Promise<void> {
      await rm(file(id), { force: true });
    },
    async list(): Promise<UploadSession[]> {
      let names: string[];
      try {
        names = await readdir(directory);
      } catch {
        return [];
      }
      const sessions: UploadSession[] = [];
      for (const name of names) {
        if (!name.endsWith(".json")) continue;
        let raw: string;
        try {
          raw = await readFile(path.join(directory, name), "utf8");
        } catch {
          continue;
        }
        const session = decode(raw);
        if (session !== undefined) sessions.push(session);
      }
      return sessions;
    },
  };
}
