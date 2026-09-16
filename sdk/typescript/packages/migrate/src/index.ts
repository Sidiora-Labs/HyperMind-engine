import { readFileSync } from 'node:fs';
import { DatabaseSync } from 'node:sqlite';

export const FORMAT = 'hypermind.cortex-import.v1';
export const KINDS = ['observations', 'memories', 'edges', 'beliefs', 'ops', 'signals', 'generic'] as const;
export type Kind = typeof KINDS[number];
type Row = Record<string, unknown>;
export interface SourceRecord {
  type: 'record';
  kind: Kind;
  id: string;
  record: Row;
  times_ns: Record<string, string>;
  embedding_f32_le?: string;
}
export interface Manifest {
  type: 'manifest';
  format: typeof FORMAT;
  namespace: string;
  counts: Record<Kind, number>;
  embedding_samples: {kind: Kind; id: string; f32_le: string}[];
}
export interface ExportOptions {source: string; format: 'json' | 'sqlite'; namespace?: string}

function object(value: unknown, label: string): Row {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error(`Invalid ${label}`);
  return value as Row;
}

function embedding(value: unknown): number[] | null {
  if (value === null || value === undefined) return null;
  if (typeof value === 'string') value = JSON.parse(value);
  if (value instanceof Uint8Array) {
    if (value.byteLength % 4 !== 0) throw new Error('Invalid float32 embedding byte length');
    const bytes = Buffer.from(value);
    value = Array.from({length: bytes.length / 4}, (_, index) => bytes.readFloatLE(index * 4));
  }
  if (!Array.isArray(value) || value.some(v => typeof v !== 'number' || !Number.isFinite(v) || !Number.isFinite(Math.fround(v)))) {
    throw new Error('Embedding must contain finite float32 numbers');
  }
  return value.map(Math.fround);
}

function normalize(kind: Kind, raw: Row, sqlite: boolean): Row {
  const row = {...raw};
  if (sqlite) {
    for (const key of ['source_files', 'tags', 'keywords', 'source_tags', 'concept_ids']) {
      if (typeof row[key] === 'string') row[key] = JSON.parse(row[key] as string);
    }
    for (const key of ['processed', 'faded', 'resolved']) {
      if (key in row) row[key] = row[key] === 1;
    }
    if (kind === 'memories') {
      row.fsrs = Object.fromEntries(['stability', 'difficulty', 'reps', 'lapses', 'state', 'last_review'].map(key => [key, row[`fsrs_${key}`]]));
      for (const key of Object.keys(row).filter(key => key.startsWith('fsrs_'))) delete row[key];
    }
    if (row.prov_model_id) row.provenance = {
      model_id: row.prov_model_id, model_family: row.prov_model_family ?? '',
      client: row.prov_client ?? '', agent: row.prov_agent ?? '',
    };
    for (const key of Object.keys(row).filter(key => key.startsWith('prov_'))) delete row[key];
  }
  if ('embedding' in row) row.embedding = embedding(row.embedding);
  return row;
}

function sourceRows(options: ExportOptions): [Kind, Row][] {
  const rows: [Kind, Row][] = [];
  const namespace = options.namespace ?? '';
  if (!/^[A-Za-z0-9_]*$/.test(namespace)) throw new Error('Invalid namespace');
  if (options.format === 'json') {
    const store = object(JSON.parse(readFileSync(options.source, 'utf8')), 'Cortex JSON store');
    if (namespace && store.namespace !== namespace) throw new Error('Source namespace mismatch');
    if (!('memories' in store) || !('observations' in store)) throw new Error('Not a Cortex JSON store');
    for (const kind of KINDS) {
      const collection = object(store[kind] ?? {}, kind);
      if (kind === 'generic') {
        for (const [collectionName, documents] of Object.entries(collection)) {
          for (const [id, record] of Object.entries(object(documents, collectionName))) {
            rows.push([kind, {id: `${collectionName}/${id}`, collection: collectionName, source_id: id, data: object(record, id)}]);
          }
        }
      } else {
        for (const [id, record] of Object.entries(collection)) {
          const row = object(record, id);
          if (row.id !== id) throw new Error(`Source id mismatch: ${kind}/${id}`);
          rows.push([kind, normalize(kind, row, false)]);
        }
      }
    }
  } else {
    const db = new DatabaseSync(options.source, {readOnly: true});
    try {
      db.exec('BEGIN');
      const tables = new Set((db.prepare("SELECT name FROM sqlite_master WHERE type='table'").all()).map(row => row.name));
      for (const kind of KINDS) {
        const table = `${namespace ? `${namespace}_` : ''}${kind === 'generic' ? 'generic_docs' : kind}`;
        if (!tables.has(table)) {
          if (kind === 'memories' || kind === 'observations') throw new Error(`Missing Cortex table ${table}`);
          continue;
        }
        for (const raw of db.prepare(`SELECT * FROM "${table}"`).all()) {
          if (kind === 'generic') rows.push([kind, {
            id: `${raw.collection}/${raw.id}`, collection: raw.collection, source_id: raw.id,
            data: object(JSON.parse(String(raw.data)), 'generic data'),
          }]);
          else rows.push([kind, normalize(kind, raw, true)]);
        }
      }
      db.exec('COMMIT');
    } finally { db.close(); }
  }
  return rows.sort(([a, x], [b, y]) => KINDS.indexOf(a) - KINDS.indexOf(b) || String(x.id).localeCompare(String(y.id), 'en'));
}

export function exportCortex(options: ExportOptions): {manifest: Manifest; records: SourceRecord[]; jsonl: string} {
  const records = sourceRows(options).map(([kind, record]): SourceRecord => {
    if (typeof record.id !== 'string' || !record.id || record.id.length > 1024) throw new Error('Invalid source id');
    const times_ns: Record<string, string> = {};
    const dates: Row = {...record, fsrs_last_review: (record.fsrs as Row | undefined)?.last_review};
    for (const key of ['created_at', 'updated_at', 'last_accessed', 'changed_at', 'valid_from', 'valid_to', 'fsrs_last_review']) {
      const value = dates[key];
      if (value === undefined || value === null) continue;
      const millis = Date.parse(String(value));
      if (!Number.isSafeInteger(millis)) throw new Error(`Invalid ${kind}/${record.id}/${key}`);
      times_ns[key] = (BigInt(millis) * 1_000_000n).toString();
    }
    const row: SourceRecord = {type: 'record', kind, id: record.id, record, times_ns};
    if (Array.isArray(record.embedding) && record.embedding.length) {
      const bytes = Buffer.alloc(record.embedding.length * 4);
      record.embedding.forEach((v, index) => bytes.writeFloatLE(v, index * 4));
      row.embedding_f32_le = bytes.toString('base64');
    }
    return row;
  });
  const seen = new Set<string>();
  for (const row of records) {
    const key = `${row.kind}/${row.id}`;
    if (seen.has(key)) throw new Error(`Duplicate source id ${key}`);
    seen.add(key);
  }
  const manifest: Manifest = {
    type: 'manifest', format: FORMAT, namespace: options.namespace ?? '',
    counts: Object.fromEntries(KINDS.map(kind => [kind, records.filter(row => row.kind === kind).length])) as Record<Kind, number>,
    embedding_samples: records.filter(row => row.embedding_f32_le).slice(0, 16).map(row => ({kind: row.kind, id: row.id, f32_le: row.embedding_f32_le!})),
  };
  return {manifest, records, jsonl: [manifest, ...records].map(row => JSON.stringify(row)).join('\n') + '\n'};
}
