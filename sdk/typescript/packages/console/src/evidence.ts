import { ConsoleDataError } from "./errors.js";
import { escapeText, section, uriLink } from "./html.js";
import { ConsoleEnvelope, ConsoleManifest, requireItem } from "./transport.js";

const PATH_SURFACE = "evidence_path";
const EVIDENCE_SURFACE = "evidence";

const CLASS_LABELS: ReadonlyArray<readonly [string, string]> = [
  ["retrieved", "the activation manifest lists the record among its candidates"],
  ["included", "the activation manifest lists the record among the records it put in the bundle"],
  ["attested", "an attestation names the record as used, ignored, helpful or harmful"],
];

export interface EvidenceIntegrity {
  leafHash: string;
  rootAtLsn: string;
  checkpointLsn: number;
}

export interface EvidenceStep {
  lsn: number;
  kind: string;
  authority: string;
  relation?: string;
  integrity: EvidenceIntegrity;
}

export interface EvidencePathView {
  label: "evidence_path";
  rootLsn: number;
  visited: number;
  steps: EvidenceStep[];
}

export interface EvidenceClass {
  lsn: number;
  retrieved: boolean;
  included: boolean;
  attested: boolean;
}

export interface AnswerLookupCounts {
  used: number;
  ignored: number;
  helpful: number;
  harmful: number;
}

export interface AnswerLookupAnswer {
  attestationLsn: number;
  conversation: string;
  disposition: string;
}

export interface AnswerLookupView {
  actor: number;
  lsn: number;
  counts: AnswerLookupCounts;
  answers: AnswerLookupAnswer[];
}

interface EvidenceEdge {
  from: number;
  to: number;
  relation: string;
}

interface WalkedRecord {
  lsn: number;
  kind: string;
  authority: string;
  integrity: EvidenceIntegrity;
}

function record(value: unknown, surface: string, field: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new ConsoleDataError(surface, field);
  }
  return value as Record<string, unknown>;
}

function integer(source: Record<string, unknown>, field: string, surface: string, path: string): number {
  const value = source[field];
  if (typeof value !== "number" || !Number.isFinite(value)) throw new ConsoleDataError(surface, path);
  return value;
}

function text(source: Record<string, unknown>, field: string, surface: string, path: string): string {
  const value = source[field];
  if (typeof value !== "string") throw new ConsoleDataError(surface, path);
  return value;
}

function list(source: Record<string, unknown>, field: string, surface: string, path: string): unknown[] {
  const value = source[field];
  if (!Array.isArray(value)) throw new ConsoleDataError(surface, path);
  return value;
}

function walkedRecords(envelope: ConsoleEnvelope): WalkedRecord[] {
  const records: WalkedRecord[] = [];
  for (let index = 1; index < envelope.items.length; index += 1) {
    const path = `items[${index}]`;
    const item = record(envelope.items[index], PATH_SURFACE, path);
    if (item["lsn"] === undefined) continue;
    const integrity = record(item["integrity"], PATH_SURFACE, `${path}.integrity`);
    records.push({
      lsn: integer(item, "lsn", PATH_SURFACE, `${path}.lsn`),
      kind: text(item, "kind", PATH_SURFACE, `${path}.kind`),
      authority: text(item, "authority", PATH_SURFACE, `${path}.authority`),
      integrity: {
        leafHash: text(integrity, "leaf_hash", PATH_SURFACE, `${path}.integrity.leaf_hash`),
        rootAtLsn: text(integrity, "root_at_lsn", PATH_SURFACE, `${path}.integrity.root_at_lsn`),
        checkpointLsn: integer(integrity, "checkpoint_lsn", PATH_SURFACE, `${path}.integrity.checkpoint_lsn`),
      },
    });
  }
  return records;
}

function step(found: WalkedRecord, relation: string | undefined): EvidenceStep {
  return {
    lsn: found.lsn,
    kind: found.kind,
    authority: found.authority,
    ...(relation === undefined ? {} : { relation }),
    integrity: found.integrity,
  };
}

export function buildEvidencePath(envelope: ConsoleEnvelope): EvidencePathView {
  const head = record(requireItem<unknown>(envelope, PATH_SURFACE), PATH_SURFACE, "items[0]");
  const path = record(head["evidence_path"], PATH_SURFACE, "evidence_path");
  const label = text(path, "label", PATH_SURFACE, "evidence_path.label");
  if (label !== "evidence_path") {
    throw new ConsoleDataError(PATH_SURFACE, "evidence_path.label", `the envelope labels the walk ${label}`);
  }
  const rootLsn = integer(path, "root_lsn", PATH_SURFACE, "evidence_path.root_lsn");
  const visited = integer(path, "visited", PATH_SURFACE, "evidence_path.visited");
  const edges: EvidenceEdge[] = list(path, "edges", PATH_SURFACE, "evidence_path.edges").map((entry, index) => {
    const edge = record(entry, PATH_SURFACE, `evidence_path.edges[${index}]`);
    return {
      from: integer(edge, "from", PATH_SURFACE, `evidence_path.edges[${index}].from`),
      to: integer(edge, "to", PATH_SURFACE, `evidence_path.edges[${index}].to`),
      relation: text(edge, "relation", PATH_SURFACE, `evidence_path.edges[${index}].relation`),
    };
  });
  const records = new Map<number, WalkedRecord>();
  for (const walked of walkedRecords(envelope)) {
    if (!records.has(walked.lsn)) records.set(walked.lsn, walked);
  }
  const outgoing = new Map<number, EvidenceEdge[]>();
  for (const edge of edges) {
    const existing = outgoing.get(edge.from);
    if (existing === undefined) outgoing.set(edge.from, [edge]);
    else existing.push(edge);
  }
  const steps: EvidenceStep[] = [];
  const emitted = new Set<number>();
  const queue: Array<{ lsn: number; relation?: string }> = [{ lsn: rootLsn }];
  while (queue.length > 0) {
    const next = queue.shift();
    if (next === undefined || emitted.has(next.lsn)) continue;
    const found = records.get(next.lsn);
    if (found === undefined) {
      throw new ConsoleDataError(PATH_SURFACE, `items[lsn ${next.lsn}]`, "the walk reports a record the envelope does not carry");
    }
    emitted.add(next.lsn);
    steps.push(step(found, next.relation));
    for (const edge of outgoing.get(next.lsn) ?? []) {
      if (!emitted.has(edge.to)) queue.push({ lsn: edge.to, relation: edge.relation });
    }
  }
  for (const walked of records.values()) {
    if (emitted.has(walked.lsn)) continue;
    emitted.add(walked.lsn);
    steps.push(step(walked, undefined));
  }
  return { label: "evidence_path", rootLsn, visited, steps };
}

function manifestLsns(values: unknown[], field: string): Set<number> {
  const lsns = new Set<number>();
  values.forEach((value, index) => {
    if (typeof value !== "number" || !Number.isFinite(value)) {
      throw new ConsoleDataError(EVIDENCE_SURFACE, `${field}[${index}]`);
    }
    lsns.add(value);
  });
  return lsns;
}

export function classifyEvidence(manifest: ConsoleManifest, evidence: AnswerLookupView[]): EvidenceClass[] {
  if (!Array.isArray(manifest.retrieved)) throw new ConsoleDataError(EVIDENCE_SURFACE, "manifest.retrieved");
  if (!Array.isArray(manifest.included)) throw new ConsoleDataError(EVIDENCE_SURFACE, "manifest.included");
  const retrieved = manifestLsns(manifest.retrieved, "manifest.retrieved");
  const included = manifestLsns(manifest.included, "manifest.included");
  const attested = new Map<number, boolean>();
  for (const lookup of evidence) {
    const counts = lookup.counts;
    attested.set(
      lookup.lsn,
      counts.used !== 0 || counts.ignored !== 0 || counts.helpful !== 0 || counts.harmful !== 0,
    );
  }
  const lsns = [...new Set([...retrieved, ...included, ...attested.keys()])].sort((left, right) => left - right);
  return lsns.map((lsn) => ({
    lsn,
    retrieved: retrieved.has(lsn),
    included: included.has(lsn),
    attested: attested.get(lsn) ?? false,
  }));
}

function evidenceActor(envelope: ConsoleEnvelope): number {
  for (const uri of envelope.provenance) {
    const matched = /^hm:\/\/(\d+)\//.exec(uri);
    if (matched !== null) return Number(matched[1]);
  }
  throw new ConsoleDataError(EVIDENCE_SURFACE, "provenance");
}

export function buildAnswerLookup(envelope: ConsoleEnvelope): AnswerLookupView {
  const index = envelope.items.findIndex(
    (entry) => typeof entry === "object" && entry !== null && !Array.isArray(entry) && "surface" in entry,
  );
  const item = record(
    requireItem<unknown>(envelope, EVIDENCE_SURFACE, index === -1 ? 0 : index),
    EVIDENCE_SURFACE,
    "items[0]",
  );
  const surface = text(item, "surface", EVIDENCE_SURFACE, "surface");
  if (surface !== EVIDENCE_SURFACE) {
    throw new ConsoleDataError(EVIDENCE_SURFACE, "surface", `the envelope reports the surface ${surface}`);
  }
  const counts = record(item["attestations"], EVIDENCE_SURFACE, "attestations");
  const answers = list(item, "answers", EVIDENCE_SURFACE, "answers").map((entry, position) => {
    const answer = record(entry, EVIDENCE_SURFACE, `answers[${position}]`);
    return {
      attestationLsn: integer(answer, "attestation_lsn", EVIDENCE_SURFACE, `answers[${position}].attestation_lsn`),
      conversation: text(answer, "conversation", EVIDENCE_SURFACE, `answers[${position}].conversation`),
      disposition: text(answer, "disposition", EVIDENCE_SURFACE, `answers[${position}].disposition`),
    };
  });
  return {
    actor: evidenceActor(envelope),
    lsn: integer(item, "lsn", EVIDENCE_SURFACE, "lsn"),
    counts: {
      used: integer(counts, "used", EVIDENCE_SURFACE, "attestations.used"),
      ignored: integer(counts, "ignored", EVIDENCE_SURFACE, "attestations.ignored"),
      helpful: integer(counts, "helpful", EVIDENCE_SURFACE, "attestations.helpful"),
      harmful: integer(counts, "harmful", EVIDENCE_SURFACE, "attestations.harmful"),
    },
    answers,
  };
}

export function renderEvidencePath(view: EvidencePathView): string {
  const labels = CLASS_LABELS.map(
    ([name, meaning]) =>
      `<li class="evidence-class evidence-class-${escapeText(name)}"><span class="evidence-class-name">${escapeText(name)}</span><span class="evidence-class-meaning">${escapeText(meaning)}</span></li>`,
  ).join("");
  const rows = view.steps
    .map(
      (entry, position) =>
        `<tr><td>${escapeText(String(position))}</td><td>${escapeText(String(entry.lsn))}</td><td>${escapeText(entry.kind)}</td><td>${escapeText(entry.authority)}</td><td>${escapeText(entry.relation ?? "root")}</td><td>${escapeText(entry.integrity.leafHash)}</td></tr>`,
    )
    .join("");
  const identity = [
    `<dt>root lsn</dt><dd>${escapeText(String(view.rootLsn))}</dd>`,
    `<dt>records walked</dt><dd>${escapeText(String(view.visited))}</dd>`,
    `<dt>steps</dt><dd>${escapeText(String(view.steps.length))}</dd>`,
  ].join("");
  const table = `<table><thead><tr><th>step</th><th>lsn</th><th>kind</th><th>authority</th><th>relation</th><th>leaf hash</th></tr></thead><tbody>${rows}</tbody></table>`;
  return section("Evidence path", `<dl>${identity}</dl><ul class="evidence-classes">${labels}</ul>${table}`);
}

export function renderAnswerLookup(view: AnswerLookupView): string {
  const counts = [
    `<dt>record lsn</dt><dd>${escapeText(String(view.lsn))}</dd>`,
    `<dt>used</dt><dd>${escapeText(String(view.counts.used))}</dd>`,
    `<dt>ignored</dt><dd>${escapeText(String(view.counts.ignored))}</dd>`,
    `<dt>helpful</dt><dd>${escapeText(String(view.counts.helpful))}</dd>`,
    `<dt>harmful</dt><dd>${escapeText(String(view.counts.harmful))}</dd>`,
  ].join("");
  const rows = view.answers
    .map(
      (answer) =>
        `<tr><td>${uriLink(`hm://${view.actor}/lsn/${answer.attestationLsn}`)}</td><td>${escapeText(answer.conversation)}</td><td>${escapeText(answer.disposition)}</td></tr>`,
    )
    .join("");
  const table = `<table><thead><tr><th>attestation</th><th>conversation</th><th>disposition</th></tr></thead><tbody>${rows}</tbody></table>`;
  return section("Answers that attest this record", `<dl>${counts}</dl>${table}`);
}
