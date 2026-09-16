import assert from "node:assert/strict";
import { chmod, mkdtemp, rm, writeFile } from "node:fs/promises";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { spawn } from "node:child_process";
import test from "node:test";
import { Client, EngineError, PendingWriteError } from "./index";

const ROOT = path.resolve(__dirname, "../../../../..");
const HM = path.join(ROOT, "target/debug/hm");

async function daemonFixture(): Promise<{
  directory: string;
  socket: string;
  token: Uint8Array;
  stop: () => Promise<void>;
}> {
  const directory = await mkdtemp(path.join(os.tmpdir(), "hypermind-ts-"));
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
  const child = spawn(HM, ["serve", "--config", config], { stdio: "ignore" });
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
    stop: async () => {
      if (child.exitCode === null) {
        child.kill("SIGINT");
        await new Promise<void>((resolve) => child.once("exit", () => resolve()));
      }
      await rm(directory, { recursive: true, force: true });
    },
  };
}

test("client uses the real daemon and recovers stable sequence history", async (context) => {
  const fixture = await daemonFixture();
  context.after(fixture.stop);
  const connectionId = Uint8Array.from({ length: 16 }, () => 9);
  const client = await Client.connect({
    socketPath: fixture.socket,
    capabilityToken: fixture.token,
    connectionId,
  });
  context.after(() => client.close());
  const session = client.session("typescript-continuity");
  await assert.rejects(session.remember("RECONSTRUCTION\nAn inferred deployment"), /cannot be remembered verbatim/);
  const evidence = await session.remember(
    "heliotrope continuity evidence: Alice Smith changed src/main.rs for GH-123",
  );
  assert.equal(evidence, 1n);
  assert.deepEqual(await session.recall("heliotrope", { mode: "lexical" }), [1n]);
  assert.deepEqual(await session.recall("GH-123", { mode: "entity" }), [1n]);
  assert.deepEqual(await session.recall("", { mode: "near", anchor: "src/main.rs" }), [1n]);
  assert.deepEqual(await session.recall("", {
    mode: "temporal",
    temporalFromNs: 0n,
    temporalToNs: 9_223_372_036_854_775_807n,
  }), [1n]);
  await assert.rejects(
    session.recall("heliotrope", { mode: "semantic" }),
    (error) => error instanceof EngineError && error.code === 49,
  );
  const checkpoint = await session.checkpoint("turn-a", Buffer.from("opaque-a"));
  assert.equal(checkpoint, 2n);
  assert.deepEqual(
    Buffer.from((await client.latestCheckpoint("turn-a"))!.blob),
    Buffer.from("opaque-a"),
  );
  await session.intend("open_loop", "ts-task", "finish sequence recovery");
  const binding = await session.bind({
    task: "ts-task",
    canonicalEntity: "repository",
    property: "revision",
    evidenceLsn: evidence,
    revision: "rev-a",
    freshnessRequirementNs: 60_000_000_000n,
  });
  assert.equal(binding.status, "resolved");
  const bundle = await session.activate("heliotrope", 2048);
  assert.equal(bundle.sections.length, 10);
  assert.equal(bundle.sections.some((section) => section.tier === "intent" && section.items.length > 0), true);
  assert.equal(bundle.sections.some((section) => section.tier === "bindings" && section.items.length > 0), true);
  const precomputed = await session.activate("heliotrope", {
    budgetTokens: 2048,
    queryEmbedding: Int8Array.from([1, 2, 3, 4, 5, 6, 7, 8]),
    queryBinaryPrefilter: Uint8Array.from([0xff]),
  });
  assert.equal(precomputed.sections.length, 10);
  client.close();

  const resumed = await Client.connect({
    socketPath: fixture.socket,
    capabilityToken: fixture.token,
    connectionId,
  });
  context.after(() => resumed.close());
  assert.equal(resumed.welcome.nextClientSeq, 5n);
  assert.equal(await resumed.session("typescript-continuity").remember("after reconnect"), 5n);
});

test("known pre-dispatch rejection does not burn a sequence", async (context) => {
  const fixture = await daemonFixture();
  context.after(fixture.stop);
  const client = await Client.connect({
    socketPath: fixture.socket,
    capabilityToken: fixture.token,
    connectionId: Uint8Array.from({ length: 16 }, () => 7),
  });
  context.after(() => client.close());
  await assert.rejects(
    client.checkpoint("turn", new Uint8Array()),
    (error) => error instanceof EngineError && error.effectState === "not_dispatched",
  );
  assert.equal(await client.checkpoint("turn", Buffer.from("valid")), 1n);
});

test("typed belief API distinguishes valid and known time and preserves retracted history", async (context) => {
  const fixture = await daemonFixture();
  context.after(fixture.stop);
  const client = await Client.connect({
    socketPath: fixture.socket,
    capabilityToken: fixture.token,
    connectionId: Uint8Array.from({ length: 16 }, () => 5),
  });
  context.after(() => client.close());
  const session = client.session("typescript-beliefs");
  const evidence = await session.remember("observed deployment region");
  const source = [{
    firstLsn: evidence,
    lastLsn: evidence,
    byteStart: 0,
    byteEnd: 26,
  }];
  assert.equal(await session.believe({
    beliefId: "region-v1",
    beliefType: "fact",
    canonicalIdentity: "deployment:active",
    value: "Europe",
    validFromNs: 1n,
    validToNs: 100n,
    provenance: source,
    conflictDomain: "deployment:region",
  }), 2n);
  assert.equal(await session.believe({
    beliefId: "region-v2",
    beliefType: "fact",
    canonicalIdentity: "deployment:active",
    value: "America",
    validFromNs: 101n,
    provenance: source,
    conflictDomain: "deployment:region",
  }), 3n);
  const valid = await session.asOf("fact", "deployment:active", { validAtNs: 50n });
  const known = await session.asOf("fact", "deployment:active", { knownAtLsn: 3n });
  assert.equal(valid?.value, "Europe");
  assert.equal(valid?.version, 1n);
  assert.equal(known?.value, "America");
  assert.equal(known?.supersedesVersion, 1n);
  assert.deepEqual(known?.provenance, source);
  assert.equal(await session.retract("region-v2", source), 4n);
  assert.equal(await session.asOf("fact", "deployment:active", { knownAtLsn: 4n }), undefined);
  assert.equal(
    (await session.asOf("fact", "deployment:active", { knownAtLsn: 3n }))?.value,
    "America",
  );
});

test("concurrent clients with one identity recover occupied sequence slots", async (context) => {
  const fixture = await daemonFixture();
  context.after(fixture.stop);
  const connectionId = Uint8Array.from({ length: 16 }, () => 6);
  const config = {
    socketPath: fixture.socket,
    capabilityToken: fixture.token,
    connectionId,
  };
  const first = await Client.connect(config);
  const second = await Client.connect(config);
  context.after(() => first.close());
  context.after(() => second.close());
  assert.equal(await first.checkpoint("race", Buffer.from("first")), 1n);
  assert.equal(await second.checkpoint("race", Buffer.from("second")), 2n);
  assert.equal(second.welcome.nextClientSeq, 3n);
  assert.deepEqual(
    Buffer.from((await second.latestCheckpoint("race"))!.blob),
    Buffer.from("second"),
  );
});

test("lost acknowledgement stays pending and exact replay applies once", async (context) => {
  const fixture = await daemonFixture();
  context.after(fixture.stop);
  const proxyPath = path.join(fixture.directory, "lost-ack.sock");
  const { dropped } = await lostAckProxy(proxyPath, fixture.socket);
  const client = await Client.connect({
    socketPath: proxyPath,
    capabilityToken: fixture.token,
    connectionId: Uint8Array.from({ length: 16 }, () => 8),
    requestTimeoutMs: 2_000,
  });
  context.after(() => client.close());
  await assert.rejects(
    client.checkpoint("turn-lost", Buffer.from("once")),
    (error) => error instanceof PendingWriteError && error.effectState === "unknown",
  );
  assert.equal(client.pendingLength, 1);
  await Promise.race([
    dropped,
    new Promise<never>((_, reject) =>
      setTimeout(() => reject(new Error("lost-ack proxy did not observe a response")), 5_000),
    ),
  ]);
  const passthrough = await passthroughProxy(proxyPath, fixture.socket);
  context.after(passthrough);
  const latest = await client.latestCheckpoint("turn-lost");
  assert.equal(client.pendingLength, 0);
  assert.equal(latest?.lsn, 1n);
  assert.deepEqual(Buffer.from(latest!.blob), Buffer.from("once"));
});

async function listen(server: net.Server, socket: string): Promise<void> {
  await rm(socket, { force: true });
  await new Promise<void>((resolve, reject) => {
    server.once("error", reject);
    server.listen(socket, resolve);
  });
}

async function lostAckProxy(proxyPath: string, target: string): Promise<{ dropped: Promise<void> }> {
  let resolveDropped!: () => void;
  const dropped = new Promise<void>((resolve) => {
    resolveDropped = resolve;
  });
  const server = net.createServer((client) => {
    const backend = net.createConnection(target);
    client.on("data", (chunk) => backend.write(chunk));
    client.on("error", () => undefined);
    backend.on("error", () => undefined);
    let buffer = Buffer.alloc(0);
    let responses = 0;
    backend.on("data", (chunk) => {
      buffer = Buffer.concat([buffer, chunk]);
      while (buffer.length >= 8) {
        const length = buffer.readUInt32LE(0);
        if (buffer.length < length + 8) return;
        const frame = buffer.subarray(0, length + 8);
        buffer = buffer.subarray(length + 8);
        responses += 1;
        if (responses === 1) client.write(frame);
        else {
          client.destroy();
          backend.destroy();
          server.close(() => resolveDropped());
          return;
        }
      }
    });
  });
  await listen(server, proxyPath);
  return { dropped };
}

async function passthroughProxy(proxyPath: string, target: string): Promise<() => Promise<void>> {
  const sockets = new Set<net.Socket>();
  const server = net.createServer((client) => {
    const backend = net.createConnection(target);
    sockets.add(client);
    sockets.add(backend);
    client.once("close", () => sockets.delete(client));
    backend.once("close", () => sockets.delete(backend));
    client.pipe(backend).pipe(client);
  });
  await listen(server, proxyPath);
  return () => {
    for (const socket of sockets) socket.destroy();
    return new Promise((resolve) => server.close(() => resolve()));
  };
}
