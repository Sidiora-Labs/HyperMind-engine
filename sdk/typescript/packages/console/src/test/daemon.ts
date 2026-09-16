import { chmod, mkdtemp, rm, writeFile } from "node:fs/promises";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { spawn } from "node:child_process";

const ROOT = path.resolve(__dirname, "../../../../../..");
const HM = path.join(ROOT, "target/debug/hm");

export interface DaemonFixture {
  directory: string;
  socket: string;
  token: Uint8Array;
  connectionId: Uint8Array;
  stop: () => Promise<void>;
}

export async function daemonFixture(): Promise<DaemonFixture> {
  const directory = await mkdtemp(path.join(os.tmpdir(), "hypermind-console-"));
  const socket = path.join(directory, "hypermind.sock");
  const data = path.join(directory, "data");
  const config = path.join(directory, "hypermind.conf");
  const tokenHex = "44".repeat(32);
  await writeFile(
    config,
    [
      `socket=${socket}`,
      `data=${data}`,
      `user=${"11".repeat(16)}`,
      `kek=${"22".repeat(32)}`,
      `admin_token=${"33".repeat(32)}`,
      `actor=7:${tokenHex}`,
      "projection_map_bytes=67108864",
      "",
    ].join("\n"),
    { mode: 0o600 },
  );
  await chmod(config, 0o600);
  const child = spawn(HM, ["serve", "--config", config], {
    stdio: "ignore",
    env: { ...process.env, HM_RECONSTRUCTION_PROVIDER: "" },
  });
  let ready = false;
  for (let attempt = 0; attempt < 200; attempt += 1) {
    try {
      const probe = net.createConnection(socket);
      await new Promise<void>((resolve, reject) => {
        probe.once("connect", resolve);
        probe.once("error", reject);
      });
      probe.destroy();
      ready = true;
      break;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 10));
    }
  }
  if (!ready) throw new Error("daemon did not become ready");
  return {
    directory,
    socket,
    token: Buffer.from(tokenHex, "hex"),
    connectionId: Uint8Array.from({ length: 16 }, () => 9),
    stop: async () => {
      if (child.exitCode === null) {
        child.kill("SIGINT");
        await new Promise<void>((resolve) => child.once("exit", () => resolve()));
      }
      await rm(directory, { recursive: true, force: true });
    },
  };
}
