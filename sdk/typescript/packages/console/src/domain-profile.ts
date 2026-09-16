import { sha256 } from "@noble/hashes/sha2";
import { ConsoleDataError } from "./errors.js";
import { escapeText, section } from "./html.js";
import { ConsoleEnvelope, ConsoleTransport } from "./transport.js";

const SURFACE = "domain profile";
const SCHEMA = "hypermind.domain-profile.v1";
const CONVERSATION = "console-domain-profile";
const TIMELINE_LIMIT = 4096;
const KINDS = ["text", "number", "date", "identifier"];
const CARDINALITIES = ["one", "many"];

export const RESERVED_FIELD_NAMES: readonly string[] = [
  "authority",
  "conversation",
  "lsn",
  "provenance",
];

export interface DomainField {
  name: string;
  kind: "text" | "number" | "date" | "identifier";
  required: boolean;
}

export interface DomainRelation {
  name: string;
  target: string;
  cardinality: "one" | "many";
}

export interface DomainType {
  id: string;
  name: string;
  description: string;
  fields: DomainField[];
  relations: DomainRelation[];
}

export interface DomainProfile {
  schema: "hypermind.domain-profile.v1";
  version: number;
  types: DomainType[];
}

export type DomainAction =
  | { type: "add_type"; name: string; description?: string }
  | { type: "rename_type"; id: string; name: string }
  | { type: "delete_type"; id: string }
  | { type: "duplicate_type"; id: string }
  | { type: "add_field"; id: string; field: DomainField }
  | { type: "update_field"; id: string; name: string; field: DomainField }
  | { type: "delete_field"; id: string; name: string }
  | { type: "add_relation"; id: string; relation: DomainRelation }
  | { type: "delete_relation"; id: string; name: string };

export interface CompiledDomainProfile {
  document: string;
  digest: string;
  errors: string[];
}

export function emptyDomainProfile(): DomainProfile {
  return { schema: SCHEMA, version: 1, types: [] };
}

function canonical(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(canonical);
  if (typeof value === "object" && value !== null) {
    const source = value as Record<string, unknown>;
    const sorted: Record<string, unknown> = {};
    for (const key of Object.keys(source).sort()) sorted[key] = canonical(source[key]);
    return sorted;
  }
  return value;
}

function canonicalText(value: unknown): string {
  return JSON.stringify(canonical(value));
}

function copyField(field: DomainField): DomainField {
  return { name: field.name, kind: field.kind, required: field.required };
}

function copyRelation(relation: DomainRelation): DomainRelation {
  return { name: relation.name, target: relation.target, cardinality: relation.cardinality };
}

function mapType(
  profile: DomainProfile,
  id: string,
  change: (entry: DomainType) => DomainType,
): DomainType[] | undefined {
  if (!profile.types.some((entry) => entry.id === id)) return undefined;
  return profile.types.map((entry) => (entry.id === id ? change(entry) : entry));
}

function nextTypes(profile: DomainProfile, action: DomainAction): DomainType[] | undefined {
  switch (action.type) {
    case "add_type":
      return [
        ...profile.types,
        {
          id: `type-${profile.version}`,
          name: action.name,
          description: action.description ?? "",
          fields: [],
          relations: [],
        },
      ];
    case "rename_type":
      return mapType(profile, action.id, (entry) => ({ ...entry, name: action.name }));
    case "delete_type":
      return profile.types.some((entry) => entry.id === action.id)
        ? profile.types.filter((entry) => entry.id !== action.id)
        : undefined;
    case "duplicate_type": {
      const source = profile.types.find((entry) => entry.id === action.id);
      if (source === undefined) return undefined;
      return [
        ...profile.types,
        {
          id: `type-${profile.version}`,
          name: `${source.name} copy`,
          description: source.description,
          fields: source.fields.map(copyField),
          relations: source.relations.map(copyRelation),
        },
      ];
    }
    case "add_field":
      return mapType(profile, action.id, (entry) => ({
        ...entry,
        fields: [...entry.fields, copyField(action.field)],
      }));
    case "update_field":
      return mapType(profile, action.id, (entry) =>
        entry.fields.some((field) => field.name === action.name)
          ? {
              ...entry,
              fields: entry.fields.map((field) =>
                field.name === action.name ? copyField(action.field) : field,
              ),
            }
          : entry,
      );
    case "delete_field":
      return mapType(profile, action.id, (entry) => ({
        ...entry,
        fields: entry.fields.filter((field) => field.name !== action.name),
      }));
    case "add_relation":
      return mapType(profile, action.id, (entry) => ({
        ...entry,
        relations: [...entry.relations, copyRelation(action.relation)],
      }));
    case "delete_relation":
      return mapType(profile, action.id, (entry) => ({
        ...entry,
        relations: entry.relations.filter((relation) => relation.name !== action.name),
      }));
  }
}

export function applyDomainAction(profile: DomainProfile, action: DomainAction): DomainProfile {
  const types = nextTypes(profile, action);
  if (types === undefined || canonicalText(types) === canonicalText(profile.types)) return profile;
  return { schema: profile.schema, version: profile.version + 1, types };
}

function hex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

function validate(profile: DomainProfile): string[] {
  const errors = new Set<string>();
  if (profile.schema !== SCHEMA) errors.add(`the schema must be ${SCHEMA}`);
  if (!Number.isInteger(profile.version) || profile.version < 1) {
    errors.add("the version must be an integer of one or more");
  }
  const names = new Set<string>();
  const declared = new Set(profile.types.map((entry) => entry.name));
  for (const entry of profile.types) {
    if (entry.id === "") errors.add(`empty type id for the type named "${entry.name}"`);
    if (entry.name === "") {
      errors.add(`empty type name for the type with id "${entry.id}"`);
    } else if (names.has(entry.name)) {
      errors.add(`duplicate type name "${entry.name}"`);
    } else {
      names.add(entry.name);
    }
    const fields = new Set<string>();
    for (const field of entry.fields) {
      if (field.name === "") {
        errors.add(`empty field name in type "${entry.name}"`);
        continue;
      }
      if (fields.has(field.name)) {
        errors.add(`duplicate field name "${field.name}" in type "${entry.name}"`);
      }
      fields.add(field.name);
      if (RESERVED_FIELD_NAMES.includes(field.name)) {
        errors.add(`reserved field name "${field.name}" in type "${entry.name}"`);
      }
      if (!KINDS.includes(field.kind)) {
        errors.add(`unknown kind "${field.kind}" for field "${field.name}" in type "${entry.name}"`);
      }
    }
    const relations = new Set<string>();
    for (const relation of entry.relations) {
      if (relation.name === "") {
        errors.add(`empty relation name in type "${entry.name}"`);
        continue;
      }
      if (relations.has(relation.name)) {
        errors.add(`duplicate relation name "${relation.name}" in type "${entry.name}"`);
      }
      relations.add(relation.name);
      if (!declared.has(relation.target)) {
        errors.add(
          `relation "${relation.name}" in type "${entry.name}" targets undeclared type "${relation.target}"`,
        );
      }
      if (!CARDINALITIES.includes(relation.cardinality)) {
        errors.add(
          `unknown cardinality "${relation.cardinality}" for relation "${relation.name}" in type "${entry.name}"`,
        );
      }
    }
  }
  return [...errors];
}

export function compileDomainProfile(profile: DomainProfile): CompiledDomainProfile {
  const document = canonicalText(profile);
  return {
    document,
    digest: hex(sha256(new TextEncoder().encode(document))),
    errors: validate(profile),
  };
}

function record(value: unknown, field: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new ConsoleDataError(SURFACE, field);
  }
  return value as Record<string, unknown>;
}

function text(source: Record<string, unknown>, field: string, path: string): string {
  const value = source[field];
  if (typeof value !== "string") throw new ConsoleDataError(SURFACE, path);
  return value;
}

function parseField(value: unknown, path: string): DomainField {
  const source = record(value, path);
  const kind = text(source, "kind", `${path}.kind`);
  if (!KINDS.includes(kind)) {
    throw new ConsoleDataError(SURFACE, `${path}.kind`, `the stored kind ${kind} is not declared`);
  }
  const required = source["required"];
  if (typeof required !== "boolean") throw new ConsoleDataError(SURFACE, `${path}.required`);
  return { name: text(source, "name", `${path}.name`), kind: kind as DomainField["kind"], required };
}

function parseRelation(value: unknown, path: string): DomainRelation {
  const source = record(value, path);
  const cardinality = text(source, "cardinality", `${path}.cardinality`);
  if (!CARDINALITIES.includes(cardinality)) {
    throw new ConsoleDataError(
      SURFACE,
      `${path}.cardinality`,
      `the stored cardinality ${cardinality} is not declared`,
    );
  }
  return {
    name: text(source, "name", `${path}.name`),
    target: text(source, "target", `${path}.target`),
    cardinality: cardinality as DomainRelation["cardinality"],
  };
}

function parseType(value: unknown, path: string): DomainType {
  const source = record(value, path);
  const fields = source["fields"];
  const relations = source["relations"];
  if (!Array.isArray(fields)) throw new ConsoleDataError(SURFACE, `${path}.fields`);
  if (!Array.isArray(relations)) throw new ConsoleDataError(SURFACE, `${path}.relations`);
  return {
    id: text(source, "id", `${path}.id`),
    name: text(source, "name", `${path}.name`),
    description: text(source, "description", `${path}.description`),
    fields: fields.map((field, index) => parseField(field, `${path}.fields[${index}]`)),
    relations: relations.map((relation, index) =>
      parseRelation(relation, `${path}.relations[${index}]`),
    ),
  };
}

function parseProfile(content: string): DomainProfile {
  let value: unknown;
  try {
    value = JSON.parse(content);
  } catch {
    throw new ConsoleDataError(SURFACE, "document", "the stored record is not a JSON document");
  }
  const source = record(value, "document");
  const schema = source["schema"];
  if (schema !== SCHEMA) {
    throw new ConsoleDataError(
      SURFACE,
      "schema",
      `the stored document declares ${JSON.stringify(schema)} instead of ${SCHEMA}`,
    );
  }
  const version = source["version"];
  if (typeof version !== "number" || !Number.isInteger(version) || version < 1) {
    throw new ConsoleDataError(SURFACE, "version");
  }
  const types = source["types"];
  if (!Array.isArray(types)) throw new ConsoleDataError(SURFACE, "types");
  return {
    schema: SCHEMA,
    version,
    types: types.map((entry, index) => parseType(entry, `types[${index}]`)),
  };
}

export async function persistDomainProfile(
  transport: ConsoleTransport,
  profile: DomainProfile,
): Promise<string[]> {
  const compiled = compileDomainProfile(profile);
  if (compiled.errors.length > 0) {
    throw new ConsoleDataError(
      SURFACE,
      "errors",
      `the profile does not compile: ${compiled.errors.join("; ")}`,
    );
  }
  const envelope: ConsoleEnvelope = await transport.callTool("remember", {
    conversation: CONVERSATION,
    content: compiled.document,
    kind: "user",
  });
  if (!envelope.ok) {
    throw new ConsoleDataError(SURFACE, "ok", "the daemon refused to store the profile");
  }
  if (!Array.isArray(envelope.provenance) || envelope.provenance.length === 0) {
    throw new ConsoleDataError(SURFACE, "provenance");
  }
  return [...envelope.provenance];
}

export async function loadDomainProfile(
  transport: ConsoleTransport,
): Promise<DomainProfile | undefined> {
  const envelope: ConsoleEnvelope = await transport.callTool("recall", {
    mode: "timeline",
    conversation: CONVERSATION,
    limit: TIMELINE_LIMIT,
  });
  if (!envelope.ok) {
    throw new ConsoleDataError(SURFACE, "ok", "the daemon refused to read the profile timeline");
  }
  if (!Array.isArray(envelope.items)) throw new ConsoleDataError(SURFACE, "items");
  let latest: { lsn: number; content: string } | undefined;
  for (const [index, entry] of envelope.items.entries()) {
    const item = record(entry, `items[${index}]`);
    const lsn = item["lsn"];
    if (typeof lsn !== "number") throw new ConsoleDataError(SURFACE, `items[${index}].lsn`);
    const content = text(item, "content", `items[${index}].content`);
    if (latest === undefined || lsn > latest.lsn) latest = { lsn, content };
  }
  return latest === undefined ? undefined : parseProfile(latest.content);
}

export function renderDomainProfile(
  profile: DomainProfile,
  compiled: { digest: string; errors: string[] },
): string {
  const header = [
    `<dt>schema</dt><dd>${escapeText(profile.schema)}</dd>`,
    `<dt>version</dt><dd>${escapeText(String(profile.version))}</dd>`,
    `<dt>digest</dt><dd>${escapeText(compiled.digest)}</dd>`,
  ].join("");
  const errors =
    compiled.errors.length === 0
      ? `<p class="compiles">The profile compiles with no validation errors.</p>`
      : `<ul class="errors">${compiled.errors
          .map((error) => `<li>${escapeText(error)}</li>`)
          .join("")}</ul>`;
  const types = profile.types
    .map((entry) => {
      const fields = entry.fields
        .map(
          (field) =>
            `<li>${escapeText(field.name)} (${escapeText(field.kind)}${field.required ? ", required" : ""})</li>`,
        )
        .join("");
      const relations = entry.relations
        .map(
          (relation) =>
            `<li>${escapeText(relation.name)} to ${escapeText(relation.target)} (${escapeText(relation.cardinality)})</li>`,
        )
        .join("");
      return `<article class="domain-type"><h3>${escapeText(entry.name)}</h3><p class="identity">${escapeText(entry.id)}</p><p class="description">${escapeText(entry.description)}</p><ul class="fields">${fields}</ul><ul class="relations">${relations}</ul></article>`;
    })
    .join("");
  const note = `<p class="note">This profile groups and labels the entities the server already extracted. It does not change server-side extraction, which stays deterministic in the kernel.</p>`;
  return section("Domain profile", `${note}<dl>${header}</dl>${errors}${types}`);
}
