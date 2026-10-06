import { ConsoleDataError } from "./errors.js";
import { escapeText, section } from "./html.js";
import { ConsoleEnvelope, ConsoleTransport, requireItem } from "./transport.js";

import type { ContextReport, Scope as ContextScope, Cursor as ContextCursor } from "@hypermind/client";
export type { ContextReport } from "@hypermind/client";

export interface ContextView { sessionId: string; report: ContextReport; operations: Record<string, unknown>; gaps: string[] }

function object(value: unknown, field: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new ConsoleDataError("context", field);
  return value as Record<string, unknown>;
}
function text(value: unknown, field: string): string {
  if (typeof value !== "string") throw new ConsoleDataError("context", field);
  return value;
}
function integer(value: unknown, field: string): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) throw new ConsoleDataError("context", field);
  return value;
}
function array(value: unknown, field: string): unknown[] {
  if (!Array.isArray(value)) throw new ConsoleDataError("context", field);
  return value;
}
function strings(value: unknown, field: string): string[] { return array(value, field).map((v) => text(v, field)); }
function authority(value: unknown): ContextReport["blocks"][number]["authority"] {
  const name = text(value, "authority");
  if (!["user_asserted", "external_observed", "tool_observed", "runtime_fact", "assistant_generated", "derived_inference"].includes(name)) throw new ConsoleDataError("context", "authority");
  return name as ContextReport["blocks"][number]["authority"];
}
function scope(value: unknown): ContextScope {
  const row = object(value, "scope");
  return { owner_id: text(row.owner_id, "owner_id"), project_id: text(row.project_id, "project_id"), workspace_id: row.workspace_id === null ? null : text(row.workspace_id, "workspace_id") };
}
function cursor(value: unknown): ContextCursor {
  const row = object(value, "cursor");
  const epoch = integer(row.epoch, "epoch");
  if (epoch === 0) throw new ConsoleDataError("context", "epoch");
  return { epoch, sequence: integer(row.sequence, "sequence") };
}
export function buildContextReport(value: unknown): ContextReport {
  const row = object(value, "report");
  if (row.version !== 1) throw new ConsoleDataError("context", "version");
  return {
    version: 1, scope: scope(row.scope), session_id: text(row.session_id, "session_id"), cursor: cursor(row.cursor),
    generation: integer(row.generation, "generation"), token_count: integer(row.token_count, "token_count"), digest: text(row.digest, "digest"),
    included: strings(row.included, "included"), gaps: strings(row.gaps, "gaps"),
    omitted: array(row.omitted, "omitted").map((v) => { const entry = object(v, "omitted"); return { id: text(entry.id, "id"), reason: text(entry.reason, "reason") }; }),
    blocks: array(row.blocks, "blocks").map((v) => {
      const block = object(v, "blocks");
      if (typeof block.required !== "boolean") throw new ConsoleDataError("context", "required");
      return { id: text(block.id, "id"), text: text(block.text, "text"), authority: authority(block.authority), tokens: integer(block.tokens, "tokens"), required: block.required,
        provenance: array(block.provenance, "provenance").map((p) => { const span = object(p, "span"); const start = integer(span.byte_start, "byte_start"); const end = integer(span.byte_end, "byte_end"); if (end < start) throw new ConsoleDataError("context", "byte_end"); return { source_id: text(span.source_id, "source_id"), source_digest: text(span.source_digest, "source_digest"), byte_start: start, byte_end: end }; }) };
    }),
  };
}
export function buildContextView(envelope: ConsoleEnvelope): ContextView {
  const row = object(requireItem(envelope, "context"), "item");
  if (row.version !== 1) throw new ConsoleDataError("context", "version");
  const report = buildContextReport(row.report);
  const sessionId = text(row.session_id, "session_id");
  if (sessionId !== report.session_id) throw new ConsoleDataError("context", "session_id", "report and resource differ");
  const operations: Record<string, unknown> = {};
  for (const name of ["history", "coverage", "cache", "jobs", "provenance", "migrations"]) operations[name] = row[name];
  return { sessionId, report, operations, gaps: strings(row.gaps, "gaps") };
}
export interface ContextSession { session_id: string; scope: ContextScope; generation: number; cursor: ContextCursor }
export function buildContextSessions(envelope: ConsoleEnvelope): ContextSession[] {
  const row = object(requireItem(envelope, "context"), "item");
  if (row.version !== 1) throw new ConsoleDataError("context", "version");
  return array(row.sessions, "sessions").map((value) => {
    const entry = object(value, "session");
    return { session_id: text(entry.session_id, "session_id"), scope: scope(entry.scope), generation: integer(entry.generation, "generation"), cursor: cursor(entry.cursor) };
  });
}
export function renderContextSessionOptions(sessions: ContextSession[]): string {
  return sessions.map((entry) => `<option value="${escapeText(entry.session_id)}">${escapeText(entry.session_id)} · ${escapeText(entry.scope.owner_id)} / ${escapeText(entry.scope.project_id)}${entry.scope.workspace_id === null ? "" : ` / ${escapeText(entry.scope.workspace_id)}`}</option>`).join("");
}
export async function inspectContext(transport: ConsoleTransport, actor: number, sessionId: string): Promise<ContextView> {
  return buildContextView(await transport.callTool("inspect", { uri: `hm://${actor}/context/${encodeURIComponent(sessionId)}` }));
}
function label(name: string): string { return name.replace(/_/g, " "); }
function details(value: unknown, depth = 0): string {
  if (value === undefined || value === null) return '<p class="context-unavailable">Unavailable: the engine has not reported this state.</p>';
  if (depth > 8) return '<p>Nested state exceeds the display limit.</p>';
  if (Array.isArray(value)) return value.length === 0 ? '<p class="context-empty">No records reported.</p>' : `<ul>${value.map((entry) => `<li>${details(entry, depth + 1)}</li>`).join("")}</ul>`;
  if (typeof value === "object" && "available" in value && (value as Record<string, unknown>).available === false) {
    const reason = (value as Record<string, unknown>).reason;
    return `<p class="context-unavailable">Unavailable: ${escapeText(typeof reason === "string" ? reason : "no reason reported")}</p>`;
  }
  if (typeof value === "object") return `<dl>${Object.entries(value).map(([key, entry]) => `<dt>${escapeText(label(key))}</dt><dd>${details(entry, depth + 1)}</dd>`).join("")}</dl>`;
  return escapeText(String(value));
}
export function renderContext(view: ContextView): string {
  const report = view.report;
  const included = new Set(report.included);
  const blocks = report.blocks.filter((block) => included.has(block.id));
  const missing = report.included.filter((id) => !report.blocks.some((block) => block.id === id));
  const blockCards = blocks.map((block) => `<article class="context-card"><h3>${escapeText(block.id)}</h3><p>${escapeText(block.authority)} · ${block.tokens} tokens${block.required ? " · required" : ""}</p><pre>${escapeText(block.text)}</pre><details><summary>Original source spans</summary>${details(block.provenance)}</details></article>`).join("");
  const omitted = report.omitted.length === 0 ? '<p class="context-empty">No context omitted.</p>' : `<ul>${report.omitted.map((entry) => `<li><strong>${escapeText(entry.id)}</strong>: ${escapeText(entry.reason)}</li>`).join("")}</ul>`;
  const gaps = [...new Set([...report.gaps, ...view.gaps])];
  const navigation = '<nav class="context-navigation" aria-label="Context inspection"><a href="#context-included">Included</a><a href="#context-omitted">Omitted</a><a href="#context-coverage">Coverage</a><a href="#context-cache">Cache</a><a href="#context-jobs">Jobs</a><a href="#context-provenance">Provenance</a><a href="#context-migrations">Migration</a></nav>';
  const operational = Object.entries(view.operations).map(([name, value]) => `<div id="context-${name}">${section(name === "jobs" ? "Historian and maintenance jobs" : label(name), details(value))}</div>`).join("");
  return section("Session context", `<p>Session <strong>${escapeText(view.sessionId)}</strong></p>${details({ scope: report.scope, cursor: report.cursor, generation: report.generation, token_count: report.token_count, digest: report.digest })}${navigation}${gaps.length ? section("Context gaps", details(gaps)) : ""}<div id="context-included">${section("Included context", (blockCards || '<p class="context-empty">No context blocks included.</p>') + (missing.length ? `<p class="context-unavailable">Included block content unavailable: ${missing.map(escapeText).join(", ")}</p>` : ""))}</div><div id="context-omitted">${section("Omitted context", omitted)}</div>${operational}`);
}
