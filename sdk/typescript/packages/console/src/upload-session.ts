import { ConsoleDataError } from "./errors.js";
import { escapeText, section } from "./html.js";
import { ConsoleTransport, requireItem } from "./transport.js";

const SURFACE = "upload session";
const SCHEMA = "hypermind.console-upload-session.v1";
const STORAGE_PREFIX = `${SCHEMA}:`;

export const MAXIMUM_SESSION_AGE_MS = 24 * 60 * 60 * 1000;
export const MAXIMUM_STALL_MS = 30 * 60 * 1000;

export interface UploadDocument {
  name: string;
  text: string;
}

export interface UploadBatch {
  firstLsn: number;
  lastLsn: number;
  files: number;
  bytes: number;
}

export interface UploadSession {
  schema: "hypermind.console-upload-session.v1";
  version: number;
  id: string;
  conversation: string;
  filesTotal: number;
  bytesTotal: number;
  acceptedBatches: UploadBatch[];
  pending: UploadDocument[];
  stage: "uploading" | "processing";
  createdAtMs: number;
  updatedAtMs: number;
  lastProgressAtMs: number;
  degraded: boolean;
  persisted: boolean;
}

export type UploadStatus = "in_progress" | "incomplete_degraded" | "complete";

export interface SessionStore {
  read(id: string): Promise<UploadSession | undefined>;
  write(session: UploadSession): Promise<void>;
  remove(id: string): Promise<void>;
  list(): Promise<UploadSession[]>;
}

function parseSession(raw: string): UploadSession | undefined {
  let value: unknown;
  try {
    value = JSON.parse(raw);
  } catch {
    return undefined;
  }
  if (typeof value !== "object" || value === null || Array.isArray(value)) return undefined;
  const candidate = value as UploadSession;
  if (candidate.schema !== SCHEMA || typeof candidate.id !== "string") return undefined;
  if (!Array.isArray(candidate.acceptedBatches) || !Array.isArray(candidate.pending)) return undefined;
  return candidate;
}

export function webStorageSessionStore(storage: Storage): SessionStore {
  const key = (id: string): string => `${STORAGE_PREFIX}${id}`;
  return {
    read(id: string): Promise<UploadSession | undefined> {
      const raw = storage.getItem(key(id));
      return Promise.resolve(raw === null ? undefined : parseSession(raw));
    },
    write(session: UploadSession): Promise<void> {
      storage.setItem(key(session.id), JSON.stringify(session));
      return Promise.resolve();
    },
    remove(id: string): Promise<void> {
      storage.removeItem(key(id));
      return Promise.resolve();
    },
    list(): Promise<UploadSession[]> {
      const sessions: UploadSession[] = [];
      for (let index = 0; index < storage.length; index += 1) {
        const name = storage.key(index);
        if (name === null || !name.startsWith(STORAGE_PREFIX)) continue;
        const raw = storage.getItem(name);
        const session = raw === null ? undefined : parseSession(raw);
        if (session !== undefined) sessions.push(session);
      }
      return Promise.resolve(sessions);
    },
  };
}

export function newSessionId(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

export async function saveSession(store: SessionStore, session: UploadSession): Promise<UploadSession> {
  const whole: UploadSession = { ...session, degraded: false, persisted: true };
  try {
    await store.write(whole);
    return whole;
  } catch {
    const metadata: UploadSession = { ...session, pending: [], degraded: true, persisted: true };
    try {
      await store.write(metadata);
      return metadata;
    } catch {
      return { ...session, pending: [], degraded: true, persisted: false };
    }
  }
}

export async function updateSession(store: SessionStore, session: UploadSession): Promise<UploadSession | null> {
  const existing = await store.read(session.id);
  if (existing === undefined) return null;
  return saveSession(store, { ...session, updatedAtMs: Date.now() });
}

export async function deleteSession(store: SessionStore, id: string): Promise<void> {
  await store.remove(id);
}

export async function listSessions(store: SessionStore): Promise<UploadSession[]> {
  const sessions = await store.list();
  return sessions.sort((left, right) => right.updatedAtMs - left.updatedAtMs);
}

function expired(session: UploadSession, nowMs: number): boolean {
  return nowMs - session.createdAtMs > MAXIMUM_SESSION_AGE_MS
    || nowMs - session.lastProgressAtMs > MAXIMUM_STALL_MS;
}

export async function purgeExpiredSessions(store: SessionStore): Promise<void> {
  const nowMs = Date.now();
  for (const session of await store.list()) {
    if (expired(session, nowMs)) await store.remove(session.id);
  }
}

export async function findResumableSession(
  store: SessionStore,
  conversation?: string,
): Promise<UploadSession | null> {
  const nowMs = Date.now();
  const resumable = (await listSessions(store)).filter((session) =>
    !expired(session, nowMs)
    && uploadStatus(session) !== "complete"
    && (conversation === undefined || session.conversation === conversation));
  return resumable[0] ?? null;
}

function record(value: unknown, field: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new ConsoleDataError(SURFACE, field);
  }
  return value as Record<string, unknown>;
}

function lsn(source: Record<string, unknown>, field: string): number {
  const value = source[field];
  if (typeof value !== "number" || !Number.isInteger(value) || value <= 0) {
    throw new ConsoleDataError(SURFACE, field);
  }
  return value;
}

function byteLength(text: string): number {
  return new TextEncoder().encode(text).length;
}

export async function resumeUpload(
  transport: ConsoleTransport,
  store: SessionStore,
  session: UploadSession,
): Promise<UploadSession> {
  let current: UploadSession = {
    ...session,
    acceptedBatches: [...session.acceptedBatches],
    pending: [...session.pending],
    stage: "uploading",
  };
  while (current.pending.length > 0) {
    const document = current.pending[0];
    const envelope = await transport.callTool("remember", {
      conversation: current.conversation,
      content: document.text,
      kind: "document",
    });
    const item = record(requireItem<unknown>(envelope, SURFACE), "items[0]");
    const accepted: UploadBatch = {
      firstLsn: lsn(item, "first_lsn"),
      lastLsn: lsn(item, "last_lsn"),
      files: 1,
      bytes: byteLength(document.text),
    };
    const atMs = Date.now();
    current = {
      ...current,
      acceptedBatches: [...current.acceptedBatches, accepted],
      pending: current.pending.slice(1),
      updatedAtMs: atMs,
      lastProgressAtMs: atMs,
    };
    const progress = await saveSession(store, current);
    current = { ...current, degraded: progress.degraded, persisted: progress.persisted };
  }
  const finished: UploadSession = { ...current, stage: "processing" };
  const stored = await saveSession(store, finished);
  return { ...finished, degraded: stored.degraded, persisted: stored.persisted };
}

function acceptedFiles(session: UploadSession): number {
  return session.acceptedBatches.reduce((total, batch) => total + batch.files, 0);
}

export function uploadStatus(session: UploadSession): UploadStatus {
  if (session.degraded || !session.persisted) return "incomplete_degraded";
  if (session.pending.length === 0 && acceptedFiles(session) >= session.filesTotal) return "complete";
  return "in_progress";
}

function sentence(session: UploadSession, status: UploadStatus, accepted: number): string {
  if (!session.persisted) {
    return "Nothing reached storage: this upload session was written nowhere, it will not survive a reload, and it is reported as incomplete.";
  }
  if (session.degraded) {
    return `This upload session is degraded: its record was written but the documents still waiting to be sent were not, so ${accepted} of ${session.filesTotal} documents are accounted for and the upload is incomplete.`;
  }
  if (status === "complete") {
    return `Every one of the ${session.filesTotal} documents is accounted for by an accepted batch.`;
  }
  return `${accepted} of ${session.filesTotal} documents are accounted for by an accepted batch; the upload is still in progress.`;
}

export function renderUploadSession(session: UploadSession): string {
  const status = uploadStatus(session);
  const accepted = acceptedFiles(session);
  const facts = [
    `<dt>session</dt><dd>${escapeText(session.id)}</dd>`,
    `<dt>conversation</dt><dd>${escapeText(session.conversation)}</dd>`,
    `<dt>stage</dt><dd>${escapeText(session.stage)}</dd>`,
    `<dt>status</dt><dd>${escapeText(status)}</dd>`,
    `<dt>documents accepted</dt><dd>${escapeText(`${accepted} of ${session.filesTotal}`)}</dd>`,
    `<dt>documents waiting</dt><dd>${escapeText(String(session.pending.length))}</dd>`,
    `<dt>degraded</dt><dd>${escapeText(String(session.degraded))}</dd>`,
    `<dt>persisted</dt><dd>${escapeText(String(session.persisted))}</dd>`,
  ].join("");
  const rows = session.acceptedBatches
    .map((batch) => `<tr><td>${escapeText(String(batch.firstLsn))}</td><td>${escapeText(String(batch.lastLsn))}</td><td>${escapeText(String(batch.files))}</td><td>${escapeText(String(batch.bytes))}</td></tr>`)
    .join("");
  const body = `<p>${escapeText(sentence(session, status, accepted))}</p><dl>${facts}</dl>`
    + `<table><thead><tr><th>first lsn</th><th>last lsn</th><th>documents</th><th>bytes</th></tr></thead><tbody>${rows}</tbody></table>`;
  return section("Upload session", body);
}
