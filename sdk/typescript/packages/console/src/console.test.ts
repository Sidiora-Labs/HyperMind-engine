import assert from "node:assert/strict";
import { readdir } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { Client } from "@hypermind/client";
import { ConsoleDataError } from "./errors.js";
import { escapeText } from "./html.js";
import { clientTransport } from "./node-transport.js";
import { buildOverview, OverviewView, renderOverview } from "./overview.js";
import { daemonFixture } from "./test/daemon.js";

const SERVABLE = /^[a-z][a-z0-9-]{0,63}\.(html|js|css|json|svg)$/;

test("console overview reads the real daemon", async (context) => {
  const fixture = await daemonFixture();
  context.after(fixture.stop);
  const client = await Client.connect({
    socketPath: fixture.socket,
    capabilityToken: fixture.token,
    connectionId: fixture.connectionId,
  });
  context.after(() => client.close());
  const transport = clientTransport(client);
  for (const content of ["console overview first record", "console overview second record"]) {
    const remembered = await transport.callTool("remember", {
      conversation: "console",
      content,
      kind: "document",
    });
    assert.equal(remembered.ok, true);
  }
  const envelope = await transport.callTool("inspect", {});
  const view = buildOverview(envelope);
  assert.equal(view.actor, 7);
  assert.ok(view.appliedLsn >= 2, `applied lsn ${view.appliedLsn}`);
  assert.equal(view.verified, true);
  assert.ok(view.rootHex.length > 0);
  assert.equal(view.rootHex.length % 2, 0);
  assert.ok(view.logEvents >= 2, `log events ${view.logEvents}`);
  assert.ok(view.projections.some((projection) => projection.name === "bm25"));
  const html = renderOverview(view);
  assert.ok(html.includes(escapeText(view.rootHex)));
  assert.ok(html.includes("<td>bm25</td>"));
});

test("console overview refuses an unusable envelope", () => {
  const empty = { items: [], provenance: [], budget: null, gaps: [], health: {}, warnings: [] };
  assert.throws(() => buildOverview({ ok: false, ...empty }), ConsoleDataError);
  assert.throws(() => buildOverview({ ok: true, ...empty }), ConsoleDataError);
  const partial = {
    actor: 7,
    verification: { verified: true, root: "aa", last_checkpoint_lsn: 0 },
    projections: [],
  };
  assert.throws(
    () => buildOverview({ ok: true, ...empty, items: [partial] }),
    (error) => error instanceof ConsoleDataError && error.field === "applied_lsn",
  );
});

test("console escapes server text", () => {
  assert.equal(escapeText("<script>&\"'"), "&lt;script&gt;&amp;&quot;&#39;");
  const view: OverviewView = {
    actor: 7,
    appliedLsn: 2,
    logEvents: 2,
    logBytes: 4096,
    verified: true,
    rootHex: "<img src=x>",
    lastCheckpointLsn: 0,
    projections: [{ name: "<script>alert(1)</script>", appliedLsn: 2 }],
  };
  const html = renderOverview(view);
  assert.equal(html.includes("<img"), false);
  assert.equal(html.includes("<script>"), false);
  assert.ok(html.includes("&lt;script&gt;alert(1)&lt;/script&gt;"));
  assert.ok(html.includes("&lt;img src=x&gt;"));
});

test("console browser output is servable", async () => {
  const site = path.resolve(__dirname, "site");
  const names = await readdir(site);
  for (const expected of ["index.html", "app.js", "overview.js", "transport.js", "html.js", "errors.js"]) {
    assert.ok(names.includes(expected), `${expected} is missing from ${site}`);
  }
  assert.equal(names.includes("node-transport.js"), false);
  assert.equal(names.some((name) => name.endsWith(".test.js")), false);
  for (const name of names) {
    assert.match(name, SERVABLE);
  }
});
