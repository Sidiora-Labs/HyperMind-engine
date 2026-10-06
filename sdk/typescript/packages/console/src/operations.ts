import type { Scope } from "@hypermind/client";
import { ConsoleDataError } from "./errors.js";
import { escapeText, section } from "./html.js";
import { ConsoleEnvelope, ConsoleTransport, requireItem } from "./transport.js";

export type OperationalRecord = Record<string, unknown>;
export function operationalRecord(value: unknown, field: string): OperationalRecord { if (typeof value !== "object" || value === null || Array.isArray(value)) throw new ConsoleDataError("operations", field); return value as OperationalRecord; }
export function operationalText(value: unknown, field: string): string { if (typeof value !== "string") throw new ConsoleDataError("operations", field); return value; }
export function operationalInteger(value: unknown, field: string): number { if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) throw new ConsoleDataError("operations", field); return value; }
export function operationalArray(value: unknown, field: string): unknown[] { if (!Array.isArray(value)) throw new ConsoleDataError("operations", field); return value; }
export function operationalScope(value: unknown): Scope { const s = operationalRecord(value, "scope"); return { owner_id: operationalText(s.owner_id, "owner_id"), project_id: operationalText(s.project_id, "project_id"), workspace_id: s.workspace_id === null ? null : operationalText(s.workspace_id, "workspace_id") }; }
export function operationalFacts(entries: [string, unknown][]): string { return `<dl>${entries.map(([label, value]) => `<dt>${escapeText(label)}</dt><dd>${escapeText(String(value ?? "Unavailable"))}</dd>`).join("")}</dl>`; }
export function operationalReply(envelope: ConsoleEnvelope, name: string): OperationalRecord { return operationalRecord(requireItem(envelope, name), "item"); }
export interface OperationsView {
  scope: Scope; backend: OperationalRecord; families: { family: string; available: boolean; reason?: string }[];
  operations: { operation: string; available: boolean; maxInput: number; maxTimeout: number }[];
  busy: boolean; active: OperationalRecord | null; receiptsAvailable: boolean; receipts?: OperationalRecord[];
  publications?: unknown[]; authority: string; cancelSemantics: string; writerEpoch?: number;
}
function flag(value: unknown, field: string): boolean { if (typeof value !== "boolean") throw new ConsoleDataError("operations", field); return value; }
export function buildOperationsView(envelope: ConsoleEnvelope): OperationsView {
  const v = operationalReply(envelope, "operations"); if (v.version !== 1) throw new ConsoleDataError("operations", "version");
  const d = operationalRecord(v.descriptors, "descriptors");
  const receiptsAvailable = flag(v.receipts_available, "receipts_available");
  return { scope: operationalScope(v.scope), backend: operationalRecord(d.backend, "backend"), busy: flag(v.runtime_busy, "runtime_busy"), active: v.active_effect === null ? null : operationalRecord(v.active_effect, "active_effect"), authority: operationalText(d.effect_authority, "effect_authority"), cancelSemantics: operationalText(d.cancel_semantics, "cancel_semantics"), writerEpoch: v.writer_epoch === undefined ? undefined : operationalInteger(v.writer_epoch, "writer_epoch"), receiptsAvailable,
    receipts: receiptsAvailable ? operationalArray(v.receipts, "receipts").map((r) => operationalRecord(r, "receipt")) : undefined, publications: receiptsAvailable ? operationalArray(v.publications, "publications") : undefined,
    families: operationalArray(d.families, "families").map((value) => { const f = operationalRecord(value, "family"); return { family: operationalText(f.family, "family"), available: flag(f.available, "available"), reason: f.reason === undefined ? undefined : operationalText(f.reason, "reason") }; }),
    operations: operationalArray(d.operations, "operations").map((value) => { const o = operationalRecord(value, "operation"); return { operation: operationalText(o.operation, "operation"), available: flag(o.available, "available"), maxInput: operationalInteger(o.max_input_bytes, "max_input_bytes"), maxTimeout: operationalInteger(o.max_timeout_ms, "max_timeout_ms") }; }),
  };
}
export async function inspectOperations(transport: ConsoleTransport, actor: number): Promise<OperationsView> { return buildOperationsView(await transport.callTool("inspect", { uri: `hm://${actor}/context-fabric` })); }
export type OperationsAction = { action: "dispatch"; operation: "digest"; bytes: number[]; timeout_ms: number } | { action: "cancel" | "reconcile"; effect_id: string };
export async function operationsAction(transport: ConsoleTransport, actor: number, owner: Scope, action: OperationsAction, requestId = `operation-${globalThis.crypto.randomUUID()}`): Promise<OperationalRecord> {
  return operationalReply(await transport.callTool("remember", { actor, conversation: "fabric", kind: "user", content: "", context: { operation: "fabric", request: { version: 1, scope: owner, request_id: requestId, action } } }), "operations");
}
export async function dispatchDigest(transport: ConsoleTransport, actor: number, view: OperationsView, text: string, timeoutMs: number, requestId?: string): Promise<OperationalRecord> {
  const operation = view.operations.find((o) => o.operation === "digest");
  if (!operation?.available) throw new ConsoleDataError("operations", "digest", "the selected runtime does not support digest dispatch");
  const bytes = Array.from(new TextEncoder().encode(text));
  if (bytes.length > operation.maxInput || !Number.isSafeInteger(timeoutMs) || timeoutMs <= 0 || timeoutMs > operation.maxTimeout) throw new ConsoleDataError("operations", "bounds", "input or timeout exceeds the selected runtime limit");
  return operationsAction(transport, actor, view.scope, { action: "dispatch", operation: "digest", bytes, timeout_ms: timeoutMs }, requestId);
}
export function renderOperations(view: OperationsView): string {
  const families = view.families.map((f) => `<li><strong>${escapeText(f.family.replace(/_/g, " "))}</strong>: ${f.available ? "Available" : `Unavailable${f.reason ? ` — ${escapeText(f.reason)}` : ""}`}</li>`).join("");
  const capabilities = operationalRecord(view.backend.bus_capabilities, "bus_capabilities");
  const receipts = view.receipts?.map((r) => { const intent = operationalRecord(r.intent, "receipt.intent"); const cursor = operationalRecord(r.cursor, "receipt.cursor"); const observation = intent.observation === null ? null : operationalRecord(intent.observation, "observation"); return `<article class="context-card">${operationalFacts([["Effect", intent.key], ["State", intent.state], ["Payload digest", intent.payload_digest], ["Revision", intent.version], ["Receipt cursor", `${cursor.epoch}:${cursor.sequence}`], ["Outcome", observation?.outcome ?? "Unknown"]])}${intent.state === "uncertain" ? '<p>Outcome uncertain. Automatic replay is blocked. Only trusted original evidence may reconcile this effect.</p>' : ""}</article>`; }).join("");
  const active = view.active ? operationalFacts([["Active effect", view.active.id], ["Phase", view.active.phase]]) + `<button type="button" data-operation-cancel="${escapeText(String(view.active.id))}">Cancel owned operation</button>` : '<p>No active operation.</p>';
  return section("Operational runtime", operationalFacts([["Owner", view.scope.owner_id], ["Project", view.scope.project_id], ["Selected operational backend", view.backend.operational_backend], ["Backend identity digest", view.backend.identity_digest], ["Owner epoch", view.backend.owner_epoch], ["Operational epoch", view.backend.operational_epoch], ["Writer epoch", view.writerEpoch], ["Effect authority", view.authority], ["Runtime", view.busy ? "Busy" : "Idle"]]) + section("Available families", `<ul>${families}</ul>${operationalFacts(Object.entries(capabilities).map(([key, value]) => [key.replace(/_/g, " "), typeof value === "object" ? "Reported capability" : value]))}`) + section("Owned operation", active + `<p>${escapeText(view.cancelSemantics)}</p>`) + section("Effect receipts", view.receiptsAvailable ? receipts || '<p>No effect receipts.</p>' : '<p>Receipts unavailable while the owned runtime operation is in progress.</p>') + section("Backend publications", view.publications === undefined ? '<p>Publication inspection unavailable while runtime is busy.</p>' : `<p>${view.publications.length} actual backend publication records reported.</p>`));
}
export async function mountOperations(mount: HTMLElement, transport: ConsoleTransport, actor: number): Promise<void> {
  const form = document.createElement("form"); form.innerHTML = '<label>Digest input<textarea name="digest-input" required></textarea></label><label>Deadline (ms)<input name="timeout" type="number" value="10000" min="1" max="30000" required /></label><button type="submit" disabled>Dispatch digest</button>';
  const refresh = document.createElement("button"); refresh.type = "button"; refresh.textContent = "Refresh operational state";
  const output = document.createElement("div"); const notice = document.createElement("p"); notice.setAttribute("aria-live", "polite");
  mount.append(refresh, form, notice, output);
  let view: OperationsView | undefined; let busy = false; let poll: ReturnType<typeof setInterval> | undefined; let generation = 0;
  const load = async () => { const current = ++generation; try { const next = await inspectOperations(transport, actor); if (current !== generation) return; view = next; output.innerHTML = renderOperations(next); const button = form.querySelector<HTMLButtonElement>("button"); if (button) button.disabled = busy || !next.operations.some((o) => o.operation === "digest" && o.available); } catch (error) { notice.textContent = `Operational runtime unavailable: ${error instanceof Error ? error.message : "inspection failed"}`; } };
  refresh.addEventListener("click", () => { void load(); });
  form.addEventListener("submit", (event) => { event.preventDefault(); if (!view || busy) return; const values = new FormData(form); busy = true; notice.textContent = "Dispatching owned digest operation…"; const button = form.querySelector<HTMLButtonElement>("button"); if (button) button.disabled = true; poll = setInterval(() => { if (!mount.isConnected) { if (poll) clearInterval(poll); return; } void load(); }, 500); void dispatchDigest(transport, actor, view, String(values.get("digest-input") ?? ""), Number(values.get("timeout"))).then((result) => { const value = operationalRecord(result.result, "result"); notice.textContent = `Digest result: ${String(value.digest)} · request ${String(value.id)}`; }).catch((error) => { notice.textContent = `Dispatch refused or outcome uncertain. Input preserved. ${error instanceof Error ? error.message : "runtime unavailable"}`; }).finally(() => { busy = false; if (poll) clearInterval(poll); void load(); }); });
  output.addEventListener("click", (event) => { const button = (event.target as HTMLElement).closest<HTMLButtonElement>("[data-operation-cancel]"); if (!button || !view) return; button.disabled = true; void operationsAction(transport, actor, view.scope, { action: "cancel", effect_id: button.dataset.operationCancel! }).then(() => { notice.textContent = "Cancellation recorded. Unknown outcomes remain held; automatic replay is disabled."; }).catch((error) => { notice.textContent = `Cancellation refused: ${error instanceof Error ? error.message : "unavailable"}`; }).finally(() => { void load(); }); });
  await load();
}
