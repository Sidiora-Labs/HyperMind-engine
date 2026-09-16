import { ConsoleDataError } from "./errors.js";
import { escapeText, section, uriLink } from "./html.js";
import { ConsoleEnvelope, requireItem } from "./transport.js";

const SURFACE = "removal";

const NOTICE =
  "This is a diagnostic preview. Nothing has been removed: no record has been retracted, no event was appended and the ledger is exactly as it was.";

export interface RemovalAttestationCounts {
  used: number;
  ignored: number;
  helpful: number;
  harmful: number;
}

export interface RemovalPreviewView {
  actor: number;
  targetLsn: number;
  directDependents: number[];
  orphanedLsns: number[];
  islandCount: number;
  attestationCounts: RemovalAttestationCounts;
  scannedFrames: number;
  performsRetraction: false;
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

function integers(source: Record<string, unknown>, field: string, path: string): number[] {
  const value = source[field];
  if (!Array.isArray(value)) throw new ConsoleDataError(SURFACE, path);
  return value.map((entry, index) => {
    if (typeof entry !== "number" || !Number.isFinite(entry)) {
      throw new ConsoleDataError(SURFACE, `${path}[${index}]`);
    }
    return entry;
  });
}

function previewActor(envelope: ConsoleEnvelope): number {
  if (Array.isArray(envelope.provenance)) {
    for (const uri of envelope.provenance) {
      const matched = /^hm:\/\/(\d+)\//.exec(uri);
      if (matched !== null) return Number(matched[1]);
    }
  }
  throw new ConsoleDataError(SURFACE, "provenance");
}

function previewIndex(envelope: ConsoleEnvelope): number {
  if (!Array.isArray(envelope.items)) return 0;
  const index = envelope.items.findIndex(
    (entry) =>
      typeof entry === "object" && entry !== null && !Array.isArray(entry) && "diagnostic" in entry,
  );
  return index === -1 ? 0 : index;
}

export function buildRemovalPreview(envelope: ConsoleEnvelope): RemovalPreviewView {
  const index = previewIndex(envelope);
  const item = record(requireItem<unknown>(envelope, SURFACE, index), `items[${index}]`);
  const diagnostic = text(item, "diagnostic", "diagnostic");
  if (diagnostic !== "removal_preview") {
    throw new ConsoleDataError(SURFACE, "diagnostic", `the item reports the diagnostic ${diagnostic}`);
  }
  if (item["performs_retraction"] !== false) {
    throw new ConsoleDataError(
      SURFACE,
      "performs_retraction",
      "the item does not declare that the preview performs no retraction",
    );
  }
  const counts = record(item["attestation_counts"], "attestation_counts");
  return {
    actor: previewActor(envelope),
    targetLsn: integer(item, "target_lsn", "target_lsn"),
    directDependents: integers(item, "direct_dependents", "direct_dependents"),
    orphanedLsns: integers(item, "orphaned_lsns", "orphaned_lsns"),
    islandCount: integer(item, "island_count", "island_count"),
    attestationCounts: {
      used: integer(counts, "used", "attestation_counts.used"),
      ignored: integer(counts, "ignored", "attestation_counts.ignored"),
      helpful: integer(counts, "helpful", "attestation_counts.helpful"),
      harmful: integer(counts, "harmful", "attestation_counts.harmful"),
    },
    scannedFrames: integer(item, "scanned_frames", "scanned_frames"),
    performsRetraction: false,
  };
}

export function renderRemovalPreview(view: RemovalPreviewView): string {
  const facts = [
    `<dt>target record</dt><dd>${uriLink(`hm://${view.actor}/lsn/${view.targetLsn}`)}</dd>`,
    `<dt>records that reference the target</dt><dd>${escapeText(String(view.directDependents.length))}</dd>`,
    `<dt>islands the target holds together</dt><dd>${escapeText(String(view.islandCount))}</dd>`,
    `<dt>records that would be stranded</dt><dd>${escapeText(String(view.orphanedLsns.length))}</dd>`,
    `<dt>attestations naming the target</dt><dd>used ${escapeText(String(view.attestationCounts.used))}, ignored ${escapeText(String(view.attestationCounts.ignored))}, helpful ${escapeText(String(view.attestationCounts.helpful))}, harmful ${escapeText(String(view.attestationCounts.harmful))}</dd>`,
  ].join("");
  const dependents = view.directDependents
    .map((lsn) => `<li class="removal-dependent">${uriLink(`hm://${view.actor}/lsn/${lsn}`)}</li>`)
    .join("");
  const orphans = view.orphanedLsns
    .map((lsn) => `<li class="removal-orphan">${uriLink(`hm://${view.actor}/lsn/${lsn}`)}</li>`)
    .join("");
  const scope = `<p class="removal-scope">This diagnostic preview read ${escapeText(String(view.scannedFrames))} ledger frames and answers for the evidence reference graph inside that window only, not for the whole store.</p>`;
  return section(
    "Diagnostic removal preview",
    [
      `<p class="removal-notice">${escapeText(NOTICE)}</p>`,
      `<dl class="removal-facts">${facts}</dl>`,
      `<h3>Records that reference the target</h3><ul class="removal-dependents">${dependents}</ul>`,
      `<h3>Records that would be stranded</h3><ul class="removal-orphans">${orphans}</ul>`,
      scope,
    ].join(""),
  );
}
