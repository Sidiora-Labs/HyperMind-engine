import { ConsoleDataError } from "./errors.js";
import { escapeText, section } from "./html.js";
import { ConsoleEnvelope, requireItem } from "./transport.js";

const SURFACE = "access";

const PROTECTED_REASON =
  "a consolidation run's direct write to this belief type is rejected and surfaces as a proposal";

export interface AccessVerbRow {
  verb: string;
  mutating: boolean;
  adminTokenRequiredActions: string[];
}

export interface AccessView {
  actor: number;
  isolation: string;
  adminTokenConfigured: boolean;
  adminOperations: string[];
  verbs: AccessVerbRow[];
  protectedBeliefTypes: string[];
}

function record(value: unknown, path: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new ConsoleDataError(SURFACE, path);
  }
  return value as Record<string, unknown>;
}

function integer(source: Record<string, unknown>, field: string, path: string): number {
  const value = source[field];
  if (typeof value !== "number" || !Number.isFinite(value)) throw new ConsoleDataError(SURFACE, path);
  return value;
}

function text(source: Record<string, unknown>, field: string, path: string): string {
  const value = source[field];
  if (typeof value !== "string") throw new ConsoleDataError(SURFACE, path);
  return value;
}

function flag(source: Record<string, unknown>, field: string, path: string): boolean {
  const value = source[field];
  if (typeof value !== "boolean") throw new ConsoleDataError(SURFACE, path);
  return value;
}

function list(source: Record<string, unknown>, field: string, path: string): unknown[] {
  const value = source[field];
  if (!Array.isArray(value)) throw new ConsoleDataError(SURFACE, path);
  return value;
}

function texts(source: Record<string, unknown>, field: string, path: string): string[] {
  return list(source, field, path).map((entry, index) => {
    if (typeof entry !== "string") throw new ConsoleDataError(SURFACE, `${path}[${index}]`);
    return entry;
  });
}

function verbRow(entry: unknown, index: number): AccessVerbRow {
  const path = `verbs[${index}]`;
  const row = record(entry, path);
  return {
    verb: text(row, "verb", `${path}.verb`),
    mutating: flag(row, "mutating", `${path}.mutating`),
    adminTokenRequiredActions: texts(
      row,
      "admin_token_required_actions",
      `${path}.admin_token_required_actions`,
    ),
  };
}

function accessIndex(envelope: ConsoleEnvelope): number {
  if (!Array.isArray(envelope.items)) return 0;
  const index = envelope.items.findIndex(
    (entry) => typeof entry === "object" && entry !== null && !Array.isArray(entry) && "surface" in entry,
  );
  return index === -1 ? 0 : index;
}

export function buildAccessView(envelope: ConsoleEnvelope): AccessView {
  const index = accessIndex(envelope);
  const item = record(requireItem<unknown>(envelope, SURFACE, index), `items[${index}]`);
  const source = text(item, "source", "source");
  if (source !== "server") {
    throw new ConsoleDataError(SURFACE, "source", `the item reports the source ${source}`);
  }
  const surface = text(item, "surface", "surface");
  if (surface !== SURFACE) {
    throw new ConsoleDataError(SURFACE, "surface", `the envelope reports the surface ${surface}`);
  }
  return {
    actor: integer(item, "actor", "actor"),
    isolation: text(item, "isolation", "isolation"),
    adminTokenConfigured: flag(item, "admin_token_configured", "admin_token_configured"),
    adminOperations: texts(item, "admin_operations", "admin_operations"),
    verbs: list(item, "verbs", "verbs").map(verbRow),
    protectedBeliefTypes: texts(item, "protected_belief_types", "protected_belief_types"),
  };
}

export function renderAccess(view: AccessView): string {
  const facts = [
    `<dt>actor</dt><dd>${escapeText(String(view.actor))}</dd>`,
    `<dt>isolation</dt><dd>${escapeText(view.isolation)}</dd>`,
    `<dt>admin capability configured</dt><dd>${escapeText(String(view.adminTokenConfigured))}</dd>`,
    `<dt>admin only operations</dt><dd>${view.adminOperations.map((name) => escapeText(name)).join(", ")}</dd>`,
  ].join("");
  const rows = view.verbs
    .map(
      (row) =>
        `<tr><td>${escapeText(row.verb)}</td><td class="access-mode">${row.mutating ? "write" : "read"}</td><td class="access-admin">${row.adminTokenRequiredActions.map((action) => escapeText(action)).join(", ")}</td></tr>`,
    )
    .join("");
  const verbs = `<table class="access-verbs"><thead><tr><th>verb</th><th>access</th><th>additionally needs the admin capability</th></tr></thead><tbody>${rows}</tbody></table>`;
  const protectedRows = view.protectedBeliefTypes
    .map(
      (belief) =>
        `<tr><td>${escapeText(belief)}</td><td>${escapeText(PROTECTED_REASON)}</td></tr>`,
    )
    .join("");
  const beliefs = `<table class="access-protected-belief-types"><thead><tr><th>protected belief type</th><th>why it is protected</th></tr></thead><tbody>${protectedRows}</tbody></table>`;
  const preamble = `<p>The daemon reports these permissions for actor ${escapeText(String(view.actor))}, the single actor this connection is bound to. The server states its isolation as ${escapeText(view.isolation)}.</p>`;
  return section("Governance", `${preamble}<dl>${facts}</dl>${verbs}${beliefs}`);
}
