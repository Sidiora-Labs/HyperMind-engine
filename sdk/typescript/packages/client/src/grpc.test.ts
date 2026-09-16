import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { execFileSync, spawn } from "node:child_process";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { Client, render } from "./index";

test("gRPC mTLS preserves real daemon sessions, replay, and capability errors", { timeout: 30_000 }, async (context) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "hm-ts-grpc-"));
  const file = (name: string): string => path.join(directory, name);
  const openssl = (...args: string[]): void => { execFileSync("openssl", args, { stdio: "ignore" }); };
  openssl("req", "-x509", "-newkey", "rsa:2048", "-nodes", "-subj", "/CN=SDK Test CA", "-days", "1", "-keyout", file("ca.key"), "-out", file("ca.pem"));
  openssl("req", "-newkey", "rsa:2048", "-nodes", "-subj", "/CN=localhost", "-keyout", file("client.key"), "-out", file("client.csr"));
  await writeFile(file("extensions"), "subjectAltName=DNS:localhost,IP:127.0.0.1\nextendedKeyUsage=serverAuth,clientAuth\n");
  openssl("x509", "-req", "-in", file("client.csr"), "-CA", file("ca.pem"), "-CAkey", file("ca.key"), "-CAcreateserial", "-days", "1", "-extfile", file("extensions"), "-out", file("client.pem"));
  const listener = net.createServer();
  await new Promise<void>((resolve) => listener.listen(0, "127.0.0.1", resolve));
  const port = (listener.address() as net.AddressInfo).port;
  await new Promise<void>((resolve, reject) => listener.close((error) => error ? reject(error) : resolve()));
  const socket = file("hm.sock");
  await writeFile(file("hm.conf"), [`socket=${socket}`, `data=${file("data")}`, `user=${"11".repeat(16)}`, `kek=${"22".repeat(32)}`, `admin_token=${"33".repeat(32)}`, `actor=7:${"44".repeat(32)}`, "projection_map_bytes=67108864", ""].join("\n"), { mode: 0o600 });
  const child = spawn(path.resolve(__dirname, "../../../../../target/debug/hm"), ["serve", "--config", file("hm.conf"), "--grpc-bind", `127.0.0.1:${port}`, "--tls-cert", file("client.pem"), "--tls-key", file("client.key"), "--tls-client-ca", file("ca.pem")], { stdio: "ignore" });
  context.after(async () => {
    if (child.exitCode === null) { const exited = new Promise<void>((resolve) => child.once("exit", () => resolve())); child.kill("SIGINT"); await exited; }
    await rm(directory, { recursive: true, force: true });
  });
  for (let attempt = 0; ; attempt += 1) {
    try { await new Promise<void>((resolve, reject) => { const probe = net.createConnection(socket); probe.once("connect", () => { probe.destroy(); resolve(); }); probe.once("error", reject); }); break; }
    catch { if (attempt > 200) throw new Error("real daemon did not start"); await new Promise((resolve) => setTimeout(resolve, 10)); }
  }
  const grpc = { target: `localhost:${port}`, ca: await readFile(file("ca.pem")), certificate: await readFile(file("client.pem")), privateKey: await readFile(file("client.key")) };
  const config = { grpc, capabilityToken: Buffer.alloc(32, 0x44), connectionId: Buffer.alloc(16, 0x78), requestTimeoutMs: 3_000 };
  const client = await Client.connect(config);
  context.after(() => client.close());
  const session = client.session("typescript-grpc");
  const first = await session.remember("cobalt lighthouse grpc evidence");
  assert.deepEqual(await session.recall("cobalt", { mode: "lexical" }), [first]);
  await session.believe({ beliefId: "grpc-fact", beliefType: "fact", canonicalIdentity: "grpc-project", value: "cobalt lighthouse", provenance: [{ firstLsn: first, lastLsn: first, byteStart: 0, byteEnd: 6 }] });
  assert.equal((await session.asOf("fact", "grpc-project", { validAtNs: BigInt(Date.now()) * 1_000_000n }))?.value, "cobalt lighthouse");
  const bundle = await session.activate("cobalt", 4096);
  assert.equal(bundle.sections.length, 10);
  assert.ok(render(bundle, { sameTurnLsns: [first] }).sections.every((section) => section.items.every((item) => item.role === "user" && !item.provenance.includes(first))));
  assert.equal(await session.attest({ used: [first] }), 1);
  const abort = new AbortController();
  const stream = client.subscribe({ conversation: "typescript-grpc", sinceLsn: 0n, signal: abort.signal })[Symbol.asyncIterator]();
  assert.equal((await stream.next()).value?.lsn, first);
  abort.abort();
  await stream.return?.();
  const verbs = ["remember", "recall", "activate", "attest", "believe", "retract", "dispute", "intend", "bind", "predict", "outcome", "consolidate", "forget", "inspect"] as const;
  for (const verb of verbs) assert.equal(typeof (await client.callTool(verb, {})).ok, "boolean", `${verb} envelope`);
  client.close();
  const resumed = await Client.connect(config);
  context.after(() => resumed.close());
  assert.equal(resumed.welcome.nextClientSeq, 4n);
  const second = await resumed.session("typescript-grpc").remember("second durable grpc memory");
  assert.ok(second > first);
  const uds = await Client.connect({ socketPath: socket, capabilityToken: config.capabilityToken });
  context.after(() => uds.close());
  assert.deepEqual(await uds.recall("cobalt", { mode: "lexical" }), [first]);
  await assert.rejects(Client.connect({ ...config, capabilityToken: Buffer.alloc(32) }));
  await assert.rejects(Client.connect({ ...config, grpc: { ...grpc, certificate: new Uint8Array() } }), /certificate/);
});
