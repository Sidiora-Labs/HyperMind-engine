import assert from "node:assert/strict";
import test from "node:test";
import { buildContextReport, buildContextSessions, buildContextView, ContextReport, renderContext, renderContextSessionOptions } from "./context.js";
import { ConsoleDataError } from "./errors.js";
import { ConsoleEnvelope } from "./transport.js";

const report: ContextReport = {
  version: 1, session_id: "review", scope: { owner_id: "owner", project_id: "project", workspace_id: null },
  cursor: { epoch: 1, sequence: 2 }, generation: 3, blocks: [{ id: "source", text: "Original <script>content</script>", authority: "user_asserted", provenance: [{ source_id: "message", source_digest: "digest", byte_start: 0, byte_end: 32 }], tokens: 8, required: true }],
  included: ["source"], omitted: [{ id: "older", reason: "budget exhausted" }], gaps: ["retrieval unavailable"], token_count: 8, digest: "report-digest",
};
function envelope(item: unknown): ConsoleEnvelope { return { ok: true, items: [item], provenance: [], budget: null, gaps: [], health: {}, warnings: [] }; }

test("context report preserves source authority, scope, cursor, omissions and spans", () => {
  assert.deepEqual(buildContextReport(report), report);
  const view = buildContextView(envelope({ version: 1, session_id: "review", report, coverage: { sources: 1 }, cache: { generation: 3 }, jobs: { available: false, reason: "worker not connected" }, gaps: [] }));
  const html = renderContext(view);
  for (const value of ["user_asserted", "report-digest", "budget exhausted", "worker not connected", "retrieval unavailable", "Original source spans", "context-cache", "context-migrations"]) assert.ok(html.includes(value));
  assert.ok(html.includes("&lt;script&gt;content&lt;/script&gt;"));
  assert.equal(html.includes("<script>"), false);
  assert.ok(html.includes("Unavailable: the engine has not reported this state."));
});

test("context empty states retain zero generation and distinguish absent operations", () => {
  const empty = { ...report, generation: 0, blocks: [], included: [], omitted: [], gaps: [], token_count: 0 };
  const html = renderContext(buildContextView(envelope({ version: 1, session_id: "review", report: empty, jobs: [], gaps: [] })));
  assert.ok(html.includes("No context blocks included."));
  assert.ok(html.includes("No context omitted."));
  assert.ok(html.includes("No records reported."));
  assert.ok(html.includes("<dt>generation</dt><dd>0</dd>"));
});

test("context rejects failed, inconsistent and malformed authoritative data", () => {
  const item = { version: 1, session_id: "review", report, gaps: [] };
  for (const value of [{ ...envelope(item), ok: false }, envelope({ ...item, session_id: "other" }), envelope({ ...item, report: { ...report, generation: -1 } }), envelope({ ...item, report: { ...report, version: 2 } }), envelope({ ...item, report: { ...report, cursor: { epoch: 0, sequence: 0 } } })]) assert.throws(() => buildContextView(value), ConsoleDataError);
});

test("scoped session selector uses actual index values and escapes labels and attributes", () => {
  const sessions = [{ session_id: 'a"<b>', scope: { owner_id: "<owner>", project_id: "project", workspace_id: "workspace" }, generation: 0, cursor: { epoch: 1, sequence: 0 } }];
  assert.deepEqual(buildContextSessions(envelope({ version: 1, sessions })), sessions);
  const html = renderContextSessionOptions(sessions);
  assert.ok(html.includes('value="a&quot;&lt;b&gt;"'));
  assert.ok(html.includes("&lt;owner&gt; / project / workspace"));
  assert.equal(renderContextSessionOptions(buildContextSessions(envelope({ version: 1, sessions: [] }))), "");
});
