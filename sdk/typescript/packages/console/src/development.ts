import type { Scope } from "@hypermind/client";
import { ConsoleDataError } from "./errors.js";
import { escapeText, section, uriLink } from "./html.js";
import { ConsoleEnvelope, ConsoleTransport, requireItem } from "./transport.js";

type Row = Record<string, unknown>;
export interface DevelopmentWorker { id: string; kind: string; registered: boolean; provider_policy: string | null }
export interface DevelopmentProposal { id: string; kind: string; revision: number; digest: string; status: string; worker_id: string; evidence: Row; mutations: Row[] }
export interface DevelopmentView {
  scope: Scope; workers: DevelopmentWorker[]; unavailable: Record<string, string>; runtime: Row;
  capabilities: Row[]; proposals: DevelopmentProposal[]; receipts: Row[]; scheduler: Row;
}
export interface DevelopmentReviewDraft { proposalId: string; revision: number; digest: string; decision: "accept" | "reject"; note: string }
export interface DevelopmentState { view?: DevelopmentView; draft?: DevelopmentReviewDraft; notice: string; busy: boolean }
export type DevelopmentAction =
  | { action: "inspect" }
  | { action: "review"; decision: { request_id: string; proposal_id: string; expected_revision: number; expected_digest: string; decision: "accept" | "reject" } }
  | { action: "revoke"; capability_id: string; expected_revision: number }
  | { action: "enqueue"; schedule_id: string }
  | { action: "dispatch" | "cancel"; job_id: string }
  | { action: "configure"; schedule: { id: string; mode: { mode: "manual" | "disabled" } | { mode: "timed"; interval_ms: number }; worker_id: string; snapshot: { capability_id: string; source_ids: string[]; record_ids: string[] }; reservation: number; timeout_ms: number; backoff_ms: number; max_attempts: number; identical_failure_limit: number } };

function row(value: unknown, field: string): Row {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new ConsoleDataError("development", field);
  return value as Row;
}
function text(value: unknown, field: string): string { if (typeof value !== "string") throw new ConsoleDataError("development", field); return value; }
function integer(value: unknown, field: string): number { if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) throw new ConsoleDataError("development", field); return value; }
function rows(value: unknown, field: string): Row[] { return Object.values(row(value, field)).map((v) => row(v, field)); }
function items(value: unknown, field: string): unknown[] { if (!Array.isArray(value)) throw new ConsoleDataError("development", field); return value; }
function scope(value: unknown): Scope { const v = row(value, "scope"); return { owner_id: text(v.owner_id, "scope.owner_id"), project_id: text(v.project_id, "scope.project_id"), workspace_id: v.workspace_id === null ? null : text(v.workspace_id, "scope.workspace_id") }; }
export function buildDevelopmentView(envelope: ConsoleEnvelope): DevelopmentView {
  const v = row(requireItem(envelope, "development"), "item");
  if (v.version !== 1) throw new ConsoleDataError("development", "version");
  const scheduler = row(v.scheduler, "scheduler");
  for (const name of ["schedules", "progress", "jobs", "accounting"]) row(scheduler[name], `scheduler.${name}`);
  const unavailable: Record<string, string> = {};
  for (const [key, reason] of Object.entries(row(v.worker_unavailability, "worker_unavailability"))) unavailable[key] = text(reason, "worker_unavailability");
  return {
    scope: scope(v.scope), scheduler, unavailable, runtime: row(v.runtime, "runtime"),
    workers: items(v.runtime_workers, "runtime_workers").map((value) => { const worker = row(value, "worker"); if (typeof worker.registered !== "boolean") throw new ConsoleDataError("development", "registered"); return { id: text(worker.id, "worker.id"), kind: text(worker.kind, "worker.kind"), registered: worker.registered, provider_policy: worker.provider_policy === null ? null : text(worker.provider_policy, "worker.provider_policy") }; }),
    capabilities: rows(v.capabilities, "capabilities"), receipts: rows(v.receipts, "receipts"),
    proposals: rows(v.proposals, "proposals").map((p) => ({ id: text(p.id, "proposal.id"), kind: text(p.kind, "proposal.kind"), revision: integer(p.revision, "proposal.revision"), digest: text(p.digest, "proposal.digest"), status: text(p.status, "proposal.status"), worker_id: text(p.worker_id, "proposal.worker_id"), evidence: row(p.evidence, "proposal.evidence"), mutations: items(p.mutations, "proposal.mutations").map((m) => row(m, "mutation")) })),
  };
}
export async function inspectDevelopment(transport: ConsoleTransport, actor: number): Promise<DevelopmentView> {
  return buildDevelopmentView(await transport.callTool("inspect", { uri: `hm://${actor}/context-development` }));
}
export async function developmentAction(transport: ConsoleTransport, actor: number, conversation: string, owner: Scope, action: DevelopmentAction, requestId = `development-${globalThis.crypto.randomUUID()}`): Promise<ConsoleEnvelope> {
  const envelope = await transport.callTool("remember", { actor, conversation, kind: "user", content: "", context: { operation: "development", request: { version: 1, scope: owner, request_id: requestId, action } } });
  if (!envelope.ok) throw new ConsoleDataError("development", "ok", envelope.warnings.join("; ") || "the engine refused the operation; refresh current evidence before retrying");
  return envelope;
}
export async function submitDevelopmentReview(state: DevelopmentState, transport: ConsoleTransport, actor: number, conversation: string): Promise<boolean> {
  if (!state.view || !state.draft || state.busy) return false;
  const draft = state.draft;
  const requestId = `review-${globalThis.crypto.randomUUID()}`;
  state.busy = true;
  try {
    await developmentAction(transport, actor, conversation, state.view.scope, { action: "review", decision: { request_id: requestId, proposal_id: draft.proposalId, expected_revision: draft.revision, expected_digest: draft.digest, decision: draft.decision } }, requestId);
    state.draft = undefined;
    state.notice = "Owner decision recorded. Refreshing durable state.";
    return true;
  } catch (error) {
    state.notice = `Review refused. Your draft and its original revision and digest are preserved. ${error instanceof Error ? error.message : "Inspection unavailable."}`;
    return false;
  } finally { state.busy = false; }
}
const readable = (value: unknown): string => escapeText(String(value ?? "Unavailable"));
const human = (value: unknown): string => readable(value).replace(/_/g, " ");
function facts(entries: [string, unknown][]): string { return `<dl>${entries.map(([key, value]) => `<dt>${escapeText(key)}</dt><dd>${readable(value)}</dd>`).join("")}</dl>`; }
function sourceEvidence(evidence: Row): string {
  const sources = Array.isArray(evidence.sources) ? evidence.sources : [];
  const records = Array.isArray(evidence.records) ? evidence.records : [];
  return facts([["Snapshot digest", evidence.digest], ["Source cursor", typeof evidence.source_cursor === "object" && evidence.source_cursor ? `${(evidence.source_cursor as Row).epoch}:${(evidence.source_cursor as Row).sequence}` : "Unavailable"], ["Policy revision", evidence.policy_revision], ["Ledger tail", evidence.ledger_tail]]) + `<ul>${sources.map((value) => { const s = row(value, "source"); return `<li>${readable(s.id)} · revision ${readable(s.revision)} · digest ${readable(s.digest)}</li>`; }).join("")}${records.map((value) => { const r = row(value, "record"); return `<li>${readable(r.id)} · revision ${readable(r.revision)} · digest ${readable(r.revision_digest)}</li>`; }).join("")}</ul>`;
}
export function renderDevelopment(view: DevelopmentView, actor?: number): string {
  const runtimeUnavailable = Array.isArray(view.runtime.unavailable_families) ? view.runtime.unavailable_families : [];
  const workers = view.workers.map((w) => `<article class="context-card"><h3>${human(w.kind)}</h3>${facts([["Worker", w.id], ["Runtime", w.registered ? "Registered" : "Unavailable"], ["Provider policy", w.provider_policy ?? "Not required"]])}</article>`).join("") || '<p>No runtime workers are registered.</p>';
  const capabilities = view.capabilities.map((c) => `<article class="context-card"><h3>${readable(c.id)}</h3>${facts([["Worker", c.worker_id], ["Revision", c.revision], ["State", c.revoked ? "Revoked" : "Active"], ["Session", c.session_id], ["Allowed kinds", Array.isArray(c.allowed_kinds) ? c.allowed_kinds.join(", ") : "Unavailable"]])}<details><summary>Approved evidence and limits</summary>${facts([["Source IDs", Array.isArray(c.source_ids) ? c.source_ids.join(", ") : "Unavailable"], ["Record IDs", Array.isArray(c.record_ids) ? c.record_ids.join(", ") : "Unavailable"], ["Lease expiry (ns)", c.lease ? row(c.lease, "lease").expires_at_ns : "Unavailable"], ["Reserved tokens", c.budget ? row(c.budget, "budget").reserved_tokens : "Unavailable"]])}</details>${!c.revoked ? `<button type="button" data-revoke="${readable(c.id)}">Revoke this worker permission</button>` : ""}</article>`).join("") || '<p>No worker capabilities granted.</p>';
  const proposals = view.proposals.map((p) => `<article class="context-card"><h3>${human(p.kind)} · ${readable(p.id)}</h3>${facts([["State", p.status], ["Revision", p.revision], ["Digest", p.digest], ["Worker", p.worker_id], ["Planned changes", p.mutations.length]])}<details><summary>Source and revision evidence</summary>${sourceEvidence(p.evidence)}</details><button type="button" data-preview="${escapeText(p.id)}">Reveal proposed changes</button><div data-preview-output="${escapeText(p.id)}"></div>${p.status === "pending" ? `<button type="button" data-review="${escapeText(p.id)}">Review exact proposal</button>` : ""}</article>`).join("") || '<p>No profile or documentation proposals await review.</p>';
  const schedules = rows(view.scheduler.schedules, "schedules").map((s) => { const mode = row(s.mode, "mode"); const progress = row(view.scheduler.progress, "progress")[String(s.id)]; return `<article class="context-card"><h3>${readable(s.id)}</h3>${facts([["Worker", s.worker_id], ["Mode", mode.mode], ["Interval (ms)", mode.interval_ms ?? "Manual"], ["Token reservation", s.reservation], ["Attempt limit", s.max_attempts], ["Timeout (ms)", s.timeout_ms]])}${progress ? facts([["Completed frontier", row(progress, "progress").completed_frontier], ["Circuit", row(progress, "progress").circuit_open ? "Open" : "Closed"], ["Last error", row(progress, "progress").last_error ?? "None"]]) : ""}<button data-enqueue="${readable(s.id)}" type="button"${mode.mode === "disabled" ? " disabled" : ""}>Queue work</button></article>`; }).join("") || '<p>No schedules configured.</p>';
  const jobs = rows(view.scheduler.jobs, "jobs");
  const jobCards = jobs.map((j) => { const status = row(j.status, "status"); return `<article class="context-card"><h3>${readable(j.id)}</h3>${facts([["Schedule", j.schedule_id], ["State", status.state], ["Attempt", j.attempt], ["Input digest", j.input_digest], ["Receipt", status.receipt_id ?? "Not published"]])}${status.state === "pending" ? `<button type="button" data-dispatch="${readable(j.id)}">Run queued work</button>` : ""}${["pending", "running"].includes(String(status.state)) ? `<button type="button" data-cancel="${readable(j.id)}">Cancel job</button>` : ""}</article>`; }).join("") || '<p>No development jobs.</p>';
  const accounting = row(view.scheduler.accounting, "accounting");
  const unknown = row(accounting.unknown_usage, "unknown_usage");
  const receipts = view.receipts.map((r) => `<article class="context-card"><h3>${readable(r.plan_id)}</h3>${facts([["Ledger LSN", r.ledger_lsn], ["Cursor", r.cursor], ["Snapshot digest", r.snapshot_digest], ["Plan digest", r.plan_digest], ["Proposal", r.proposal_id ?? "None"], ["Replay", r.replayed]])}${typeof r.ledger_lsn === "number" && actor !== undefined ? uriLink(`hm://${actor}/lsn/${r.ledger_lsn}`) : ""}</article>`).join("") || '<p>No publication receipts.</p>';
  return section("Development", facts([["Owner", view.scope.owner_id], ["Project", view.scope.project_id], ["Workspace", view.scope.workspace_id ?? "None"], ["Provider", view.runtime.provider], ["Readiness", view.runtime.readiness]]) + section("Runtime workers", workers + (runtimeUnavailable.length ? `<p>Unavailable worker families: ${runtimeUnavailable.map(human).join(", ")}.</p>` : "") + Object.entries(view.unavailable).map(([id, reason]) => `<p>${readable(id)}: ${readable(reason)}</p>`).join("")) + section("Budget and backlog", facts([["Budget tokens", view.scheduler.budget], ["Spent tokens", accounting.spent], ["Maximum concurrency", view.scheduler.max_concurrency], ["Pending jobs", jobs.filter((j) => row(j.status, "status").state === "pending").length], ["Unknown usage reservations", Object.values(unknown).reduce<number>((sum, v) => sum + integer(v, "unknown usage"), 0)]]) + (Object.keys(unknown).length ? '<p>Usage remains unknown. Reservations remain held until trusted settlement.</p>' : '<p>No unknown usage reported.</p>')) + section("Worker permissions", capabilities) + section("Schedules", schedules) + section("Jobs", jobCards) + section("Owner proposal review", proposals) + section("Publication receipts", receipts));
}
export function renderProposalPreview(proposal: DevelopmentProposal): string {
  return proposal.mutations.map((m) => {
    const record = m.record && typeof m.record === "object" ? row(m.record, "record") : undefined;
    return `<article>${facts([["Operation", m.operation], ["Record", record?.id ?? m.id], ["Expected revision", m.expected_revision ?? "New record"], ["Expected digest", m.expected_digest ?? "New record"]])}${record ? `<pre>${readable(record.content)}</pre>` : ""}</article>`;
  }).join("") || '<p>No proposed changes.</p>';
}
export function renderDevelopmentDraft(draft: DevelopmentReviewDraft): string {
  return `<form data-development-review>${facts([["Proposal", draft.proposalId], ["Exact revision", draft.revision], ["Exact digest", draft.digest]])}<label>Decision<select name="decision"><option value="accept"${draft.decision === "accept" ? " selected" : ""}>Accept</option><option value="reject"${draft.decision === "reject" ? " selected" : ""}>Reject</option></select></label><label>Local review note<textarea name="note">${escapeText(draft.note)}</textarea></label><p>The note stays in this browser draft. The owner decision targets the revision and digest above.</p><button type="submit">Record owner decision</button></form>`;
}
export async function mountDevelopment(mount: HTMLElement, transport: ConsoleTransport, actor: number, conversation: string): Promise<DevelopmentState> {
  const state: DevelopmentState = { notice: "", busy: false };
  let current = 0;
  const body = document.createElement("div");
  const notice = document.createElement("p"); notice.setAttribute("aria-live", "polite");
  const draft = document.createElement("div");
  const refresh = document.createElement("button"); refresh.textContent = "Refresh development state"; refresh.type = "button";
  mount.append(refresh, notice, body, draft);
  const paintDraft = () => { draft.innerHTML = state.draft ? renderDevelopmentDraft(state.draft) : ""; notice.textContent = state.notice; };
  const load = async () => { const version = ++current; try { const view = await inspectDevelopment(transport, actor); if (version !== current) return; state.view = view; body.innerHTML = renderDevelopment(view, actor); } catch (error) { state.notice = `Development unavailable: ${error instanceof Error ? error.message : "inspection failed"}`; } paintDraft(); };
  refresh.addEventListener("click", () => { void load(); });
  body.addEventListener("click", (event) => {
    const button = (event.target as HTMLElement).closest<HTMLButtonElement>("button"); if (!button || !state.view || state.busy) return;
    const proposal = state.view.proposals.find((p) => p.id === (button.dataset.preview ?? button.dataset.review));
    if (button.dataset.preview && proposal) { const output = Array.from(body.querySelectorAll<HTMLElement>("[data-preview-output]")).find((entry) => entry.dataset.previewOutput === proposal.id); if (output) output.innerHTML = renderProposalPreview(proposal); return; }
    if (button.dataset.review && proposal) { state.draft = { proposalId: proposal.id, revision: proposal.revision, digest: proposal.digest, decision: "reject", note: "" }; paintDraft(); return; }
    const cap = state.view.capabilities.find((c) => c.id === button.dataset.revoke);
    const action: DevelopmentAction | undefined = cap ? { action: "revoke", capability_id: text(cap.id, "capability.id"), expected_revision: integer(cap.revision, "capability.revision") } : button.dataset.enqueue ? { action: "enqueue", schedule_id: button.dataset.enqueue } : button.dataset.dispatch ? { action: "dispatch", job_id: button.dataset.dispatch } : button.dataset.cancel ? { action: "cancel", job_id: button.dataset.cancel } : undefined;
    if (action) { state.busy = true; button.disabled = true; void developmentAction(transport, actor, conversation, state.view.scope, action).then(() => { state.notice = "Operation recorded."; }).catch((error) => { state.notice = `Operation refused: ${error instanceof Error ? error.message : "unavailable"}`; }).finally(() => { state.busy = false; void load(); }); }
  });
  draft.addEventListener("input", (event) => { if (!state.draft) return; const target = event.target as HTMLInputElement; if (target.name === "note") state.draft.note = target.value; if (target.name === "decision" && ["accept", "reject"].includes(target.value)) state.draft.decision = target.value as "accept" | "reject"; });
  draft.addEventListener("submit", (event) => { event.preventDefault(); void submitDevelopmentReview(state, transport, actor, conversation).then((accepted) => { paintDraft(); if (accepted) void load(); }); });
  await load(); return state;
}
