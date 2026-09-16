import assert from "node:assert/strict";
import test, { TestContext } from "node:test";
import { Client } from "@hypermind/client";
import { ConsoleDataError } from "./errors.js";
import { clientTransport } from "./node-transport.js";
import {
  buildSourceDetail,
  buildSourceIndex,
  renderSourceDetail,
  renderSourceIndex,
} from "./sources.js";
import { ConsoleTransport } from "./transport.js";
import { daemonFixture } from "./test/daemon.js";

const ALPHA_FIRST = "Halberd Systems delivered the quarterly ledger to https://records.example.org/exports.";
const ALPHA_SECOND = "The reviewer for Halberd Systems opened /var/log/<b>.log and records.example.org.";
const BETA_FIRST = "Marlowe Freight audited records.example.org against the Halberd Systems contract.";

async function seeded(context: TestContext): Promise<ConsoleTransport> {
  const fixture = await daemonFixture();
  context.after(fixture.stop);
  const client = await Client.connect({
    socketPath: fixture.socket,
    capabilityToken: fixture.token,
    connectionId: fixture.connectionId,
  });
  context.after(() => client.close());
  const transport = clientTransport(client);
  const documents: Array<[string, string]> = [
    ["alpha", ALPHA_FIRST],
    ["alpha", ALPHA_SECOND],
    ["beta", BETA_FIRST],
  ];
  for (const [conversation, content] of documents) {
    const stored = await transport.callTool("remember", { conversation, content, kind: "document" });
    assert.equal(stored.ok, true, JSON.stringify(stored.items));
  }
  return transport;
}

test("console lists sources from the daemon", async (context) => {
  const transport = await seeded(context);
  const view = buildSourceIndex(await transport.callTool("inspect", { uri: "hm://7/sources" }));
  assert.equal(view.sources.length, 2);
  const [alpha, beta] = view.sources;
  assert.equal(alpha.conversation.length, 32);
  assert.equal(beta.conversation.length, 32);
  assert.equal(alpha.records, 2);
  assert.equal(beta.records, 1);
  assert.ok(alpha.firstLsn < beta.firstLsn, `${alpha.firstLsn} precedes ${beta.firstLsn}`);
  assert.ok(alpha.lastLsn >= alpha.firstLsn);
  assert.ok(alpha.lastWallTimestampNs > 0);
  assert.equal(alpha.kinds["usermsg"], 2);
  assert.equal(beta.kinds["usermsg"], 1);
  const html = renderSourceIndex(view);
  assert.ok(html.includes(`data-conversation="${alpha.conversation}"`));
  assert.ok(html.includes(`data-conversation="${beta.conversation}"`));
  assert.ok(html.includes("<td>2</td>"));
  assert.ok(html.includes("usermsg 2"));
});

test("console explains one source", async (context) => {
  const transport = await seeded(context);
  const index = buildSourceIndex(await transport.callTool("inspect", { uri: "hm://7/sources" }));
  const alpha = index.sources[0];
  const beta = index.sources[1];
  const view = buildSourceDetail(
    await transport.callTool("inspect", { uri: `hm://7/sources/${alpha.conversation}` }),
  );
  assert.equal(view.conversation, alpha.conversation);
  assert.equal(view.records, 2);
  assert.equal(view.extractor, "entity_rules");
  assert.equal(view.truncated, false);
  assert.ok(view.entities.length > 0);
  for (const entity of view.entities) {
    assert.ok(entity.canonical.length > 0);
    assert.ok(entity.kind.length > 0);
    for (const lsn of entity.lsns) {
      assert.ok(lsn >= view.firstLsn && lsn <= view.lastLsn, `lsn ${lsn} is inside the source`);
    }
  }
  const shared = view.sharedEntities.filter((entity) => entity.canonical === "Halberd Systems");
  assert.equal(shared.length, 1, JSON.stringify(view.sharedEntities));
  assert.ok(shared[0].conversations.includes(alpha.conversation));
  assert.ok(shared[0].conversations.includes(beta.conversation));
  assert.equal(view.citations.length, 2);

  const html = renderSourceDetail(view);
  assert.equal(html.split('href="hm://7/').length - 1, view.citations.length);
  for (const citation of view.citations) {
    assert.ok(citation.startsWith(`hm://7/${alpha.conversation}/`), citation);
    assert.ok(html.includes(`href="${citation}"`));
  }
  const marked = view.entities.filter((entity) => entity.canonical.includes("<b>"));
  assert.equal(marked.length, 1, JSON.stringify(view.entities.map((entity) => entity.canonical)));
  assert.equal(html.includes("<b>"), false);
  assert.ok(html.includes("&lt;b&gt;"));
  assert.equal(html.includes("truncated"), false);
});

test("console refuses an unusable sources envelope", () => {
  const empty = { items: [], provenance: [], budget: null, gaps: [], health: {}, warnings: [] };
  assert.throws(() => buildSourceIndex({ ok: false, ...empty }), ConsoleDataError);
  assert.throws(() => buildSourceDetail({ ok: false, ...empty }), ConsoleDataError);
  assert.throws(
    () => buildSourceIndex({ ok: true, ...empty, items: [{ surface: "source_detail" }] }),
    (error) => error instanceof ConsoleDataError && error.field === "items[].surface",
  );
  assert.throws(
    () => buildSourceDetail({ ok: true, ...empty, items: [{ surface: "sources" }] }),
    (error) => error instanceof ConsoleDataError && error.field === "items[].surface",
  );
  const withoutRecords = {
    surface: "sources",
    conversation: "a".repeat(32),
    first_lsn: 1,
    last_lsn: 2,
    last_wall_timestamp_ns: 3,
    kinds: { usermsg: 2 },
  };
  assert.throws(
    () => buildSourceIndex({ ok: true, ...empty, items: [withoutRecords] }),
    (error) => error instanceof ConsoleDataError && error.field === "items[0].records",
  );
});

test("console renders a truncated source only when the server says so", () => {
  const view = {
    conversation: "b".repeat(32),
    records: 1,
    firstLsn: 1,
    lastLsn: 1,
    extractor: "entity_rules",
    entities: [{ canonical: "<script>alert(1)</script>", kind: "proper_noun", aliases: ["<b>"], lsns: [1] }],
    sharedEntities: [{ canonical: "<b>shared", conversations: ["b".repeat(32), "c".repeat(32)] }],
    citations: [`hm://7/${"b".repeat(32)}/1`],
    truncated: false,
  };
  const plain = renderSourceDetail(view);
  assert.equal(plain.includes("truncated"), false);
  assert.equal(plain.includes("<script>"), false);
  assert.equal(plain.includes("<b>"), false);
  assert.ok(plain.includes("&lt;script&gt;alert(1)&lt;/script&gt;"));
  assert.ok(renderSourceDetail({ ...view, truncated: true }).includes("truncated"));
});
