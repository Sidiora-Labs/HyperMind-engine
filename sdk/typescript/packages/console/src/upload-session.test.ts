import assert from "node:assert/strict";
import { chmod, mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { Client } from "@hypermind/client";
import { clientTransport } from "./node-transport.js";
import { daemonFixture } from "./test/daemon.js";
import { fileSessionStore } from "./test/file-store.js";
import {
  MAXIMUM_SESSION_AGE_MS,
  UploadDocument,
  UploadSession,
  deleteSession,
  findResumableSession,
  newSessionId,
  purgeExpiredSessions,
  resumeUpload,
  saveSession,
  updateSession,
  uploadStatus,
} from "./upload-session.js";

function draft(documents: UploadDocument[], filesTotal: number): UploadSession {
  const nowMs = Date.now();
  return {
    schema: "hypermind.console-upload-session.v1",
    version: 1,
    id: newSessionId(),
    conversation: "console-upload",
    filesTotal,
    bytesTotal: documents.reduce((total, document) => total + Buffer.byteLength(document.text), 0),
    acceptedBatches: [],
    pending: [...documents],
    stage: "uploading",
    createdAtMs: nowMs,
    updatedAtMs: nowMs,
    lastProgressAtMs: nowMs,
    degraded: false,
    persisted: false,
  };
}

test("upload resumes after a reload", async (context) => {
  const fixture = await daemonFixture();
  context.after(fixture.stop);
  const client = await Client.connect({
    socketPath: fixture.socket,
    capabilityToken: fixture.token,
    connectionId: fixture.connectionId,
  });
  context.after(() => client.close());
  const transport = clientTransport(client);
  const directory = await mkdtemp(path.join(os.tmpdir(), "hypermind-upload-"));
  context.after(() => rm(directory, { recursive: true, force: true }));

  const documents: UploadDocument[] = [
    { name: "first.txt", text: "console upload first document" },
    { name: "second.txt", text: "console upload second document" },
    { name: "third.txt", text: "console upload third document" },
  ];
  const store = fileSessionStore(directory);
  const saved = await saveSession(store, draft([documents[0]], documents.length));
  assert.equal(saved.persisted, true);
  assert.equal(saved.degraded, false);

  const started = await resumeUpload(transport, store, saved);
  assert.equal(started.pending.length, 0);
  assert.equal(started.acceptedBatches.length, 1);
  assert.equal(uploadStatus(started), "in_progress");

  const reopened = fileSessionStore(directory);
  const found = await findResumableSession(reopened, "console-upload");
  assert.notEqual(found, null);
  assert.equal(found?.id, saved.id);
  assert.equal(found?.acceptedBatches.length, 1);

  const rechosen = await updateSession(reopened, { ...(found as UploadSession), pending: [documents[1], documents[2]] });
  assert.notEqual(rechosen, null);
  const finished = await resumeUpload(transport, reopened, rechosen as UploadSession);
  assert.equal(finished.pending.length, 0);
  assert.equal(finished.degraded, false);
  assert.equal(finished.acceptedBatches.length, 3);
  assert.equal(uploadStatus(finished), "complete");
  assert.ok(finished.lastProgressAtMs >= saved.lastProgressAtMs);
  for (const [index, batch] of finished.acceptedBatches.entries()) {
    assert.ok(batch.lastLsn >= batch.firstLsn, `batch ${index} spans ${batch.firstLsn}..${batch.lastLsn}`);
    assert.equal(batch.files, 1);
    assert.equal(batch.bytes, Buffer.byteLength(documents[index].text));
    if (index > 0) {
      assert.ok(
        batch.firstLsn > finished.acceptedBatches[index - 1].lastLsn,
        `batch ${index} starts at ${batch.firstLsn}`,
      );
    }
    const walked = await transport.callTool("inspect", { uri: `hm://7/lsn/${batch.firstLsn}` });
    assert.equal(walked.ok, true);
    assert.ok(
      walked.items.some((record) => (record as { lsn?: number }).lsn === batch.firstLsn),
      `hm://7/lsn/${batch.firstLsn} did not resolve to a ledger record`,
    );
    assert.ok(walked.provenance.includes(`hm://7/lsn/${batch.firstLsn}`));
  }

  const aged = await mkdtemp(path.join(os.tmpdir(), "hypermind-upload-age-"));
  context.after(() => rm(aged, { recursive: true, force: true }));
  const ageStore = fileSessionStore(aged);
  const nowMs = Date.now();
  const stale = draft([], 1);
  const kept = draft([], 1);
  await saveSession(ageStore, {
    ...stale,
    createdAtMs: nowMs - MAXIMUM_SESSION_AGE_MS - 1000,
    updatedAtMs: nowMs - MAXIMUM_SESSION_AGE_MS - 1000,
    lastProgressAtMs: nowMs - MAXIMUM_SESSION_AGE_MS - 1000,
  });
  await saveSession(ageStore, { ...kept, createdAtMs: nowMs - 1000, updatedAtMs: nowMs - 1000, lastProgressAtMs: nowMs - 1000 });
  await purgeExpiredSessions(ageStore);
  assert.equal(await ageStore.read(stale.id), undefined);
  assert.notEqual(await ageStore.read(kept.id), undefined);
});

test("upload reports a degraded store honestly", async (context) => {
  const base = await mkdtemp(path.join(os.tmpdir(), "hypermind-upload-degraded-"));
  const unwritable = path.join(base, "unwritable");
  await mkdir(unwritable);
  await chmod(unwritable, 0o500);
  context.after(async () => {
    await chmod(unwritable, 0o700).catch(() => undefined);
    await rm(base, { recursive: true, force: true });
  });
  const documents: UploadDocument[] = [
    { name: "first.txt", text: "console upload degraded document" },
    { name: "second.txt", text: "console upload second degraded document" },
  ];

  const denied = await writeFile(path.join(unwritable, "probe.json"), "{}", "utf8").then(() => false, () => true);
  if (denied) {
    const saved = await saveSession(fileSessionStore(unwritable), draft(documents, documents.length));
    assert.equal(saved.persisted, false);
    assert.equal(saved.degraded, true);
    assert.deepEqual(saved.pending, []);
    assert.equal(uploadStatus(saved), "incomplete_degraded");
  } else {
    assert.equal(
      typeof process.getuid === "function" ? process.getuid() : -1,
      0,
      "a directory at mode 0o500 accepted a write although this process is not root",
    );
    context.diagnostic("unverified leg: this process is root, so mode 0o500 does not deny writes; the degraded path is exercised below through a store rooted at a regular file");
  }

  const occupied = path.join(base, "occupied");
  await writeFile(occupied, "not a directory", "utf8");
  const blocked = await saveSession(fileSessionStore(occupied), draft(documents, documents.length));
  assert.equal(blocked.persisted, false);
  assert.equal(blocked.degraded, true);
  assert.deepEqual(blocked.pending, []);
  assert.equal(uploadStatus(blocked), "incomplete_degraded");

  const budgeted = path.join(base, "budgeted");
  const session = draft(documents, documents.length);
  const whole = Buffer.byteLength(JSON.stringify({ ...session, degraded: false, persisted: true }));
  const metadata = Buffer.byteLength(JSON.stringify({ ...session, pending: [], degraded: true, persisted: true }));
  assert.ok(whole > metadata, `the whole record is ${whole} bytes and its metadata is ${metadata} bytes`);
  const partial = await saveSession(fileSessionStore(budgeted, { maximumBytes: metadata }), session);
  assert.equal(partial.persisted, true);
  assert.equal(partial.degraded, true);
  assert.deepEqual(partial.pending, []);
  assert.equal(uploadStatus(partial), "incomplete_degraded");

  const stored = await fileSessionStore(budgeted).read(session.id);
  assert.notEqual(stored, undefined);
  assert.deepEqual(stored?.pending, []);
  assert.equal(stored?.degraded, true);
});

test("upload never resurrects a deleted session", async (context) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "hypermind-upload-deleted-"));
  context.after(() => rm(directory, { recursive: true, force: true }));
  const store = fileSessionStore(directory);
  const saved = await saveSession(store, draft([{ name: "first.txt", text: "console upload deleted document" }], 1));
  assert.notEqual(await store.read(saved.id), undefined);
  await deleteSession(store, saved.id);
  assert.equal(await store.read(saved.id), undefined);
  assert.equal(await updateSession(store, saved), null);
  assert.equal(await store.read(saved.id), undefined);
});
