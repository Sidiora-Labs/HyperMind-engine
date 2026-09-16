import { ConsoleDataError } from "./errors.js";
import { escapeText, section } from "./html.js";
import { ConsoleEnvelope } from "./transport.js";

const INDEX_SURFACE = "sources";
const DETAIL_SURFACE = "source_detail";

export interface SourceSummary {
  conversation: string;
  records: number;
  firstLsn: number;
  lastLsn: number;
  lastWallTimestampNs: number;
  kinds: Record<string, number>;
}

export interface SourceIndexView {
  sources: SourceSummary[];
}

export interface SourceDetailView {
  conversation: string;
  records: number;
  firstLsn: number;
  lastLsn: number;
  extractor: string;
  entities: Array<{ canonical: string; kind: string; aliases: string[]; lsns: number[] }>;
  sharedEntities: Array<{ canonical: string; conversations: string[] }>;
  citations: string[];
  truncated: boolean;
}

function record(surface: string, value: unknown, path: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new ConsoleDataError(surface, path);
  }
  return value as Record<string, unknown>;
}

function integer(surface: string, source: Record<string, unknown>, field: string, path: string): number {
  const value = source[field];
  if (typeof value !== "number" || !Number.isFinite(value)) throw new ConsoleDataError(surface, path);
  return value;
}

function text(surface: string, source: Record<string, unknown>, field: string, path: string): string {
  const value = source[field];
  if (typeof value !== "string") throw new ConsoleDataError(surface, path);
  return value;
}

function flag(surface: string, source: Record<string, unknown>, field: string, path: string): boolean {
  const value = source[field];
  if (typeof value !== "boolean") throw new ConsoleDataError(surface, path);
  return value;
}

function list(surface: string, source: Record<string, unknown>, field: string, path: string): unknown[] {
  const value = source[field];
  if (!Array.isArray(value)) throw new ConsoleDataError(surface, path);
  return value;
}

function texts(surface: string, source: Record<string, unknown>, field: string, path: string): string[] {
  return list(surface, source, field, path).map((entry, index) => {
    if (typeof entry !== "string") throw new ConsoleDataError(surface, `${path}[${index}]`);
    return entry;
  });
}

function integers(surface: string, source: Record<string, unknown>, field: string, path: string): number[] {
  return list(surface, source, field, path).map((entry, index) => {
    if (typeof entry !== "number" || !Number.isFinite(entry)) throw new ConsoleDataError(surface, `${path}[${index}]`);
    return entry;
  });
}

function tally(surface: string, source: Record<string, unknown>, field: string, path: string): Record<string, number> {
  const counts = record(surface, source[field], path);
  const kinds: Record<string, number> = {};
  for (const [kind, count] of Object.entries(counts)) {
    if (typeof count !== "number" || !Number.isFinite(count)) throw new ConsoleDataError(surface, `${path}.${kind}`);
    kinds[kind] = count;
  }
  return kinds;
}

function surfaced(envelope: ConsoleEnvelope, surface: string): Array<Record<string, unknown>> {
  if (!envelope.ok) throw new ConsoleDataError(surface, "ok", "the envelope reports a failed call");
  if (!Array.isArray(envelope.items)) throw new ConsoleDataError(surface, "items");
  const matching = envelope.items.filter(
    (item): item is Record<string, unknown> =>
      typeof item === "object"
      && item !== null
      && !Array.isArray(item)
      && (item as Record<string, unknown>)["surface"] === surface,
  );
  if (matching.length === 0) {
    throw new ConsoleDataError(surface, "items[].surface", `no item in the envelope carries the ${surface} surface`);
  }
  return matching;
}

export function buildSourceIndex(envelope: ConsoleEnvelope): SourceIndexView {
  const sources = surfaced(envelope, INDEX_SURFACE).map((item, index) => ({
    conversation: text(INDEX_SURFACE, item, "conversation", `items[${index}].conversation`),
    records: integer(INDEX_SURFACE, item, "records", `items[${index}].records`),
    firstLsn: integer(INDEX_SURFACE, item, "first_lsn", `items[${index}].first_lsn`),
    lastLsn: integer(INDEX_SURFACE, item, "last_lsn", `items[${index}].last_lsn`),
    lastWallTimestampNs: integer(
      INDEX_SURFACE,
      item,
      "last_wall_timestamp_ns",
      `items[${index}].last_wall_timestamp_ns`,
    ),
    kinds: tally(INDEX_SURFACE, item, "kinds", `items[${index}].kinds`),
  }));
  sources.sort((left, right) => left.firstLsn - right.firstLsn);
  return { sources };
}

export function buildSourceDetail(envelope: ConsoleEnvelope): SourceDetailView {
  const item = surfaced(envelope, DETAIL_SURFACE)[0];
  return {
    conversation: text(DETAIL_SURFACE, item, "conversation", "conversation"),
    records: integer(DETAIL_SURFACE, item, "records", "records"),
    firstLsn: integer(DETAIL_SURFACE, item, "first_lsn", "first_lsn"),
    lastLsn: integer(DETAIL_SURFACE, item, "last_lsn", "last_lsn"),
    extractor: text(DETAIL_SURFACE, item, "extractor", "extractor"),
    entities: list(DETAIL_SURFACE, item, "entities", "entities").map((entry, index) => {
      const entity = record(DETAIL_SURFACE, entry, `entities[${index}]`);
      return {
        canonical: text(DETAIL_SURFACE, entity, "canonical", `entities[${index}].canonical`),
        kind: text(DETAIL_SURFACE, entity, "kind", `entities[${index}].kind`),
        aliases: texts(DETAIL_SURFACE, entity, "aliases", `entities[${index}].aliases`),
        lsns: integers(DETAIL_SURFACE, entity, "lsns", `entities[${index}].lsns`),
      };
    }),
    sharedEntities: list(DETAIL_SURFACE, item, "shared_entities", "shared_entities").map((entry, index) => {
      const shared = record(DETAIL_SURFACE, entry, `shared_entities[${index}]`);
      return {
        canonical: text(DETAIL_SURFACE, shared, "canonical", `shared_entities[${index}].canonical`),
        conversations: texts(
          DETAIL_SURFACE,
          shared,
          "conversations",
          `shared_entities[${index}].conversations`,
        ),
      };
    }),
    citations: texts(DETAIL_SURFACE, item, "citations", "citations"),
    truncated: flag(DETAIL_SURFACE, item, "truncated", "truncated"),
  };
}

function conversationLink(conversation: string): string {
  return `<a class="source" href="#sources/${escapeText(encodeURIComponent(conversation))}" data-conversation="${escapeText(conversation)}">${escapeText(conversation)}</a>`;
}

export function renderSourceIndex(view: SourceIndexView): string {
  const rows = view.sources
    .map((source) => {
      const kinds = Object.entries(source.kinds)
        .map(([kind, count]) => `${escapeText(kind)} ${escapeText(String(count))}`)
        .join(", ");
      return `<tr><td>${conversationLink(source.conversation)}</td><td>${escapeText(String(source.records))}</td><td>${escapeText(String(source.firstLsn))}</td><td>${escapeText(String(source.lastLsn))}</td><td>${escapeText(String(source.lastWallTimestampNs))}</td><td>${kinds}</td></tr>`;
    })
    .join("");
  return section(
    "Sources",
    `<table><thead><tr><th>conversation</th><th>records</th><th>first lsn</th><th>last lsn</th><th>last wall timestamp ns</th><th>kinds</th></tr></thead><tbody>${rows}</tbody></table>`,
  );
}

export function renderSourceDetail(view: SourceDetailView): string {
  const facts = [
    `<dt>conversation</dt><dd>${escapeText(view.conversation)}</dd>`,
    `<dt>records</dt><dd>${escapeText(String(view.records))}</dd>`,
    `<dt>first lsn</dt><dd>${escapeText(String(view.firstLsn))}</dd>`,
    `<dt>last lsn</dt><dd>${escapeText(String(view.lastLsn))}</dd>`,
    `<dt>extractor</dt><dd>${escapeText(view.extractor)}</dd>`,
  ].join("");
  const entities = view.entities
    .map(
      (entity) =>
        `<tr><td>${escapeText(entity.canonical)}</td><td>${escapeText(entity.kind)}</td><td>${entity.aliases
          .map((alias) => escapeText(alias))
          .join(", ")}</td><td>${entity.lsns.map((lsn) => escapeText(String(lsn))).join(" ")}</td></tr>`,
    )
    .join("");
  const shared = view.sharedEntities
    .map(
      (entity) =>
        `<li><span class="entity">${escapeText(entity.canonical)}</span> ${entity.conversations
          .map((conversation) => conversationLink(conversation))
          .join(" ")}</li>`,
    )
    .join("");
  const citations = view.citations
    .map((citation) => `<li><a class="citation" href="${escapeText(citation)}">${escapeText(citation)}</a></li>`)
    .join("");
  const note = view.truncated
    ? `<p class="note">the server truncated this source: it did not list every entity, contributing record or sharing conversation</p>`
    : "";
  return section(
    "Source",
    `<dl>${facts}</dl>${note}<h3>entities</h3><table><thead><tr><th>canonical</th><th>kind</th><th>aliases</th><th>lsns</th></tr></thead><tbody>${entities}</tbody></table><h3>shared entities</h3><ul class="shared">${shared}</ul><h3>citations</h3><ul class="citations">${citations}</ul>`,
  );
}
