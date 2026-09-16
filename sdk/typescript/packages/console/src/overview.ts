import { ConsoleDataError } from "./errors.js";
import { escapeText, section } from "./html.js";
import { ConsoleEnvelope, requireItem } from "./transport.js";

const SURFACE = "overview";

export interface OverviewProjection {
  name: string;
  appliedLsn: number;
}

export interface OverviewView {
  actor: number;
  appliedLsn: number;
  logEvents: number;
  logBytes: number;
  verified: boolean;
  rootHex: string;
  lastCheckpointLsn: number;
  projections: OverviewProjection[];
}

function record(value: unknown, field: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new ConsoleDataError(SURFACE, field);
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

export function buildOverview(envelope: ConsoleEnvelope): OverviewView {
  const item = record(requireItem<unknown>(envelope, SURFACE), "items[0]");
  const verification = record(item["verification"], "verification");
  const projections = item["projections"];
  if (!Array.isArray(projections)) throw new ConsoleDataError(SURFACE, "projections");
  return {
    actor: integer(item, "actor", "actor"),
    appliedLsn: integer(item, "applied_lsn", "applied_lsn"),
    logEvents: integer(item, "log_events", "log_events"),
    logBytes: integer(item, "log_bytes", "log_bytes"),
    verified: flag(verification, "verified", "verification.verified"),
    rootHex: text(verification, "root", "verification.root"),
    lastCheckpointLsn: integer(verification, "last_checkpoint_lsn", "verification.last_checkpoint_lsn"),
    projections: projections.map((entry, index) => {
      const projection = record(entry, `projections[${index}]`);
      return {
        name: text(projection, "name", `projections[${index}].name`),
        appliedLsn: integer(projection, "applied_lsn", `projections[${index}].applied_lsn`),
      };
    }),
  };
}

export function renderOverview(view: OverviewView): string {
  const rows = view.projections
    .map((projection) => `<tr><td>${escapeText(projection.name)}</td><td>${escapeText(String(projection.appliedLsn))}</td></tr>`)
    .join("");
  const identity = [
    `<dt>actor</dt><dd>${escapeText(String(view.actor))}</dd>`,
    `<dt>applied lsn</dt><dd>${escapeText(String(view.appliedLsn))}</dd>`,
    `<dt>log events</dt><dd>${escapeText(String(view.logEvents))}</dd>`,
    `<dt>log bytes</dt><dd>${escapeText(String(view.logBytes))}</dd>`,
    `<dt>mmr verified</dt><dd>${escapeText(String(view.verified))}</dd>`,
    `<dt>mmr root</dt><dd>${escapeText(view.rootHex)}</dd>`,
    `<dt>last checkpoint lsn</dt><dd>${escapeText(String(view.lastCheckpointLsn))}</dd>`,
  ].join("");
  return section("Overview", `<dl>${identity}</dl><table><thead><tr><th>projection</th><th>applied lsn</th></tr></thead><tbody>${rows}</tbody></table>`);
}
