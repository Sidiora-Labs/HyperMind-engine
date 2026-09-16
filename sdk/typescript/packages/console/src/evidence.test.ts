import assert from "node:assert/strict";
import test, { TestContext } from "node:test";
import { Client } from "@hypermind/client";
import { ConsoleDataError } from "./errors.js";
import {
  AnswerLookupView,
  EvidenceClass,
  EvidencePathView,
  buildAnswerLookup,
  buildEvidencePath,
  classifyEvidence,
  renderAnswerLookup,
  renderEvidencePath,
} from "./evidence.js";
import { clientTransport } from "./node-transport.js";
import { ConsoleManifest, ConsoleTransport } from "./transport.js";
import { daemonFixture } from "./test/daemon.js";

const FORBIDDEN = ["reasoning", "trace", "thought"];
const TOKEN = "zarquonium";

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
  const item = envelope.items[0] as { first_lsn: number };
  return item.first_lsn;
}

async function attest(transport: ConsoleTransport, lsn: number, key: string): Promise<number> {
  const envelope = await transport.callTool("attest", {
    provenance: [`hm://7/lsn/${lsn}`],
    disposition: "used",
    idempotency_key: key,
  });
  assert.equal(envelope.ok, true);
  const item = envelope.items[0] as { first_lsn: number };
  return item.first_lsn;
}

async function lookup(transport: ConsoleTransport, lsn: number): Promise<AnswerLookupView> {
  return buildAnswerLookup(await transport.callTool("inspect", { uri: `hm://7/evidence/${lsn}` }));
}

function classOf(manifest: ConsoleManifest, evidence: AnswerLookupView[], lsn: number): EvidenceClass {
  const found = classifyEvidence(manifest, evidence).find((entry) => entry.lsn === lsn);
  assert.ok(found !== undefined, `lsn ${lsn} is missing from the classification`);
  return found;
}

test("console distinguishes retrieved included and attested", async (context) => {
  const transport = await connect(context);
  for (const content of [
    `the ${TOKEN} ledger records every activation`,
    `a ${TOKEN} bundle is trimmed to its budget`,
    `${TOKEN} evidence separates retrieval from inclusion`,
  ]) {
    await remember(transport, "console-evidence", content);
  }

  const activated = await transport.callTool("activate", {
    conversation: "console-evidence",
    query: TOKEN,
    budget_tokens: 4096,
  });
  assert.equal(activated.ok, true);
  const manifest = activated.manifest;
  assert.ok(manifest !== undefined, "activate carries a manifest");
  assert.ok(manifest.retrieved.length > 0, "the manifest retrieved at least one record");
  assert.ok(manifest.included.length > 0, "the manifest included at least one record");
  const included = manifest.included[0];
  assert.equal(typeof included, "number");
  const lsn = included as number;
  assert.ok(manifest.retrieved.includes(lsn), "an included record was also retrieved");

  const before = await lookup(transport, lsn);
  assert.equal(before.actor, 7);
  assert.equal(before.lsn, lsn);
  assert.deepEqual(before.counts, { used: 0, ignored: 0, helpful: 0, harmful: 0 });
  assert.deepEqual(before.answers, []);
  const unattested = classOf(manifest, [before], lsn);
  assert.equal(unattested.retrieved, true);
  assert.equal(unattested.included, true);
  assert.equal(unattested.attested, false);

  const attestationLsn = await attest(transport, lsn, "console-evidence");
  const after = await lookup(transport, lsn);
  assert.deepEqual(after.counts, { used: 1, ignored: 0, helpful: 0, harmful: 0 });
  assert.equal(after.answers.length, 1);
  assert.equal(after.answers[0].attestationLsn, attestationLsn);
  assert.equal(after.answers[0].disposition, "used");
  assert.equal(after.answers[0].conversation.length > 0, true);
  const attested = classOf(manifest, [after], lsn);
  assert.equal(attested.retrieved, true);
  assert.equal(attested.included, true);
  assert.equal(attested.attested, true);

  const html = renderAnswerLookup(after);
  assert.ok(html.includes(`hm://7/lsn/${attestationLsn}`));
  assert.ok(html.includes("<td>used</td>"));

  await assert.rejects(
    async () => buildAnswerLookup(await transport.callTool("inspect", {})),
    ConsoleDataError,
  );
});

test("console walks the evidence path", async (context) => {
  const transport = await connect(context);
  const recordLsn = await remember(transport, "console-path", `${TOKEN} anchors one walkable record`);
  const attestationLsn = await attest(transport, recordLsn, "console-evidence-path");

  const walked = await transport.callTool("inspect", { uri: `hm://7/lsn/${attestationLsn}` });
  assert.equal(walked.ok, true);
  const view = buildEvidencePath(walked);
  assert.equal(view.label, "evidence_path");
  assert.equal(view.rootLsn, attestationLsn);
  assert.ok(view.visited >= 2, `visited ${view.visited}`);
  assert.equal(view.steps.length, view.visited);
  assert.equal(view.steps[0].lsn, attestationLsn);
  assert.equal(view.steps[0].relation, undefined);
  assert.equal(view.steps[0].kind.length > 0, true);
  assert.equal(view.steps[0].authority.length > 0, true);
  assert.equal(view.steps[0].integrity.leafHash.length % 2, 0);
  assert.ok(view.steps[0].integrity.leafHash.length > 0);
  assert.equal(view.steps[1].lsn, recordLsn);
  assert.equal(view.steps[1].relation, "target_lsn");
  assert.ok(view.steps[1].integrity.leafHash.length > 0);
  assert.equal(view.steps[1].integrity.checkpointLsn >= 0, true);

  const html = renderEvidencePath(view);
  assert.ok(html.includes("Evidence path"));
  for (const word of FORBIDDEN) {
    assert.equal(html.toLowerCase().includes(word), false, `the rendered path uses ${word}`);
  }
});

test("console labels the evidence path honestly", () => {
  const view: EvidencePathView = {
    label: "evidence_path",
    rootLsn: 4,
    visited: 2,
    steps: [
      {
        lsn: 4,
        kind: "attestation",
        authority: "<img src=x>",
        integrity: { leafHash: "aa", rootAtLsn: "bb", checkpointLsn: 0 },
      },
      {
        lsn: 1,
        kind: "usermsg",
        authority: "external_observed",
        relation: "target_lsn",
        integrity: { leafHash: "cc", rootAtLsn: "dd", checkpointLsn: 0 },
      },
    ],
  };
  const html = renderEvidencePath(view);
  assert.ok(html.includes("Evidence path"));
  for (const word of FORBIDDEN) {
    assert.equal(html.toLowerCase().includes(word), false, `the rendered path uses ${word}`);
  }
  for (const label of ["retrieved", "included", "attested"]) {
    assert.ok(html.includes(`<span class="evidence-class-name">${label}</span>`), `${label} is not a label`);
  }
  assert.equal(html.includes("<img"), false);
  assert.ok(html.includes("&lt;img src=x&gt;"));
  assert.ok(html.includes("<td>target_lsn</td>"));
  assert.ok(html.includes("<td>root</td>"));
});

test("console evidence refuses a foreign surface", () => {
  const empty = { ok: true, items: [] as unknown[], provenance: [] as string[], budget: null, gaps: [], health: {}, warnings: [] };
  assert.throws(() => buildAnswerLookup(empty), ConsoleDataError);
  assert.throws(
    () => buildAnswerLookup({ ...empty, items: [{ surface: "sources" }], provenance: ["hm://7/lsn/1"] }),
    (error) => error instanceof ConsoleDataError && error.field === "surface",
  );
  assert.throws(() => buildEvidencePath(empty), ConsoleDataError);
});
