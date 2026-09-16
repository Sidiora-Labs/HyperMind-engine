import assert from "node:assert/strict";
import test, { TestContext } from "node:test";
import { Client } from "@hypermind/client";
import { ConsoleDataError } from "./errors.js";
import { buildRemovalPreview, renderRemovalPreview } from "./removal.js";
import { clientTransport } from "./node-transport.js";
import { ConsoleEnvelope, ConsoleTransport } from "./transport.js";
import { daemonFixture } from "./test/daemon.js";

async function connect(context: TestContext): Promise<ConsoleTransport> {
  const fixture = await daemonFixture();
  context.after(fixture.stop);
  const client = await Client.connect({
    socketPath: fixture.socket,
    capabilityToken: fixture.token,
    connectionId: fixture.connectionId,
  });
  context.after(() => client.close());
  return clientTransport(client);
}

async function remember(transport: ConsoleTransport, conversation: string, content: string): Promise<number> {
  const envelope = await transport.callTool("remember", { conversation, content, kind: "document" });
  assert.equal(envelope.ok, true);
  const item = envelope.items[0] as { first_lsn: number; last_lsn: number };
  assert.equal(item.first_lsn, item.last_lsn);
  return item.first_lsn;
}

async function attest(
  transport: ConsoleTransport,
  lsn: number,
  disposition: string,
  key: string,
): Promise<number> {
  const envelope = await transport.callTool("attest", {
    provenance: [`hm://7/lsn/${lsn}`],
    disposition,
    idempotency_key: key,
  });
  assert.equal(envelope.ok, true);
  const item = envelope.items[0] as { first_lsn: number };
  return item.first_lsn;
}

async function appliedLsn(transport: ConsoleTransport): Promise<number> {
  const envelope = await transport.callTool("inspect", {});
  assert.equal(envelope.ok, true);
  return (envelope.items[0] as { applied_lsn: number }).applied_lsn;
}

function occurrences(haystack: string, needle: string): number {
  return haystack.split(needle).length - 1;
}

function envelopeWith(item: unknown): ConsoleEnvelope {
  return {
    ok: true,
    items: [item],
    provenance: ["hm://7/lsn/4"],
    budget: null,
    gaps: [],
    health: {},
    warnings: [],
  };
}

test("console previews removal as a diagnostic", async (context) => {
  const transport = await connect(context);
  const first = await remember(transport, "alpha", "The shipping ledger closed on Tuesday.");
  const second = await remember(transport, "beta", "The audit reopened the shipping ledger.");
  const used = await attest(transport, first, "used", "console-removal-1");
  const helpful = await attest(transport, first, "helpful", "console-removal-2");
  await attest(transport, second, "ignored", "console-removal-3");

  const before = await appliedLsn(transport);
  const envelope = await transport.callTool("inspect", { uri: `hm://7/removal/${first}` });
  assert.equal(envelope.ok, true);
  const view = buildRemovalPreview(envelope);

  assert.equal(view.actor, 7);
  assert.equal(view.targetLsn, first);
  assert.equal(view.performsRetraction, false);
  assert.equal(view.islandCount, 2);
  assert.deepEqual(view.orphanedLsns, [used, helpful]);
  assert.deepEqual(view.directDependents, [used, helpful]);
  assert.deepEqual(view.attestationCounts, { used: 1, ignored: 0, helpful: 1, harmful: 0 });
  assert.ok(view.scannedFrames >= 5, `scanned only ${view.scannedFrames} frames`);

  const html = renderRemovalPreview(view);
  assert.ok(html.includes("Diagnostic"));
  assert.ok(
    html.includes("Nothing has been removed: no record has been retracted"),
    "the preview does not say that nothing has been removed",
  );
  assert.equal(
    occurrences(html, "retracted"),
    1,
    "the rendered preview says retracted somewhere other than its denial",
  );
  assert.equal(occurrences(html, String(view.scannedFrames)) >= 1, true);
  assert.ok(html.includes("ledger frames"));
  const orphanList = html.slice(html.indexOf(`<ul class="removal-orphans">`));
  assert.equal(occurrences(orphanList, `<li class="removal-orphan">`), view.orphanedLsns.length);
  for (const lsn of view.orphanedLsns) {
    assert.ok(orphanList.includes(`hm://7/lsn/${lsn}</a>`), `lsn ${lsn} has no hm:// anchor`);
  }

  assert.equal(await appliedLsn(transport), before);
});

test("console refuses a mislabelled preview", () => {
  const body = {
    surface: "removal",
    performs_retraction: false,
    target_lsn: 4,
    direct_dependents: [],
    orphaned_lsns: [],
    island_count: 0,
    attestation_counts: { used: 0, ignored: 0, helpful: 0, harmful: 0 },
    scanned_frames: 7,
  };
  assert.throws(
    () => buildRemovalPreview(envelopeWith(body)),
    (error) => error instanceof ConsoleDataError && error.field === "diagnostic",
  );
  assert.throws(
    () => buildRemovalPreview(envelopeWith({ ...body, diagnostic: "retraction" })),
    (error) => error instanceof ConsoleDataError && error.field === "diagnostic",
  );
  assert.throws(
    () =>
      buildRemovalPreview(
        envelopeWith({ ...body, diagnostic: "removal_preview", performs_retraction: true }),
      ),
    (error) => error instanceof ConsoleDataError && error.field === "performs_retraction",
  );
  assert.throws(
    () => buildRemovalPreview({ ...envelopeWith(body), ok: false }),
    ConsoleDataError,
  );
});
