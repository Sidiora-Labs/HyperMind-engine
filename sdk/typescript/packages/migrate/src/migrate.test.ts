import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import test from 'node:test';
import { exportCortex, KINDS } from './index';

const ROOT = path.resolve(__dirname, '../../../../..');
const FIXTURES = path.join(ROOT, 'eval/fixtures/cortex-store');

test('original Cortex JSON and real SQLite vectors survive read-only export and CLI import', context => {
  const directory = mkdtempSync(path.join(tmpdir(), 'hm-cortex-import-'));
  context.after(() => rmSync(directory, {recursive: true, force: true}));
  const store = JSON.parse(readFileSync(path.join(FIXTURES, 'donor-store.json'), 'utf8'));
  const database = path.join(directory, 'source.sqlite');
  const db = new DatabaseSync(database);
  db.exec(readFileSync(path.join(FIXTURES, 'donor-schema.sql'), 'utf8'));
  for (const kind of KINDS) {
    if (kind === 'generic') {
      for (const [collection, docs] of Object.entries(store.generic)) {
        for (const [id, doc] of Object.entries(docs as object)) db.prepare('INSERT INTO generic_docs VALUES (?, ?, ?)').run(collection, id, JSON.stringify(doc));
      }
      continue;
    }
    const columns = new Set(db.prepare(`PRAGMA table_info(${kind})`).all().map(column => column.name));
    for (const value of Object.values(store[kind])) {
      const row = {...value as Record<string, unknown>};
      if (kind === 'memories') {
        for (const [key, value] of Object.entries(row.fsrs as object)) row[`fsrs_${key}`] = value;
        delete row.fsrs;
      }
      const entries = Object.entries(row).filter(([key]) => columns.has(key)).map(([key, value]) => {
        if (key === 'embedding' && Array.isArray(value)) {
          const bytes = Buffer.alloc(value.length * 4);
          value.forEach((v, i) => bytes.writeFloatLE(v, i * 4));
          return [key, bytes] as const;
        }
        if (Array.isArray(value)) return [key, JSON.stringify(value)] as const;
        if (typeof value === 'boolean') return [key, Number(value)] as const;
        return [key, value] as const;
      });
      db.prepare(`INSERT INTO ${kind} (${entries.map(([key]) => key).join(',')}) VALUES (${entries.map(() => '?').join(',')})`).run(...entries.map(([,value]) => value as string | number | null | Uint8Array));
    }
  }
  db.close();
  const before = readFileSync(database);
  const sqlite = exportCortex({source: database, format: 'sqlite'});
  assert.deepEqual(readFileSync(database), before);
  const json = exportCortex({source: path.join(FIXTURES, 'donor-store.json'), format: 'json'});
  assert.deepEqual(sqlite.manifest.counts, {observations:2, memories:3, edges:2, beliefs:2, ops:2, signals:2, generic:2});
  assert.deepEqual(sqlite.manifest.counts, json.manifest.counts);
  assert.deepEqual(sqlite.manifest.embedding_samples, json.manifest.embedding_samples);
  const sample = Buffer.from(sqlite.manifest.embedding_samples[0].f32_le, 'base64');
  assert.deepEqual([sample.readFloatLE(0), sample.readFloatLE(4), sample.readFloatLE(8)], [Math.fround(.1), Math.fround(.2), Math.fround(.3)]);
  const stream = path.join(directory, 'import.jsonl');
  writeFileSync(stream, sqlite.jsonl, {mode:0o600});
  const config = path.join(directory, 'hm.conf');
  writeFileSync(config, [`socket=${directory}/hm.sock`,`data=${directory}/data`,`user=${'11'.repeat(16)}`,`kek=${'22'.repeat(32)}`,`admin_token=${'33'.repeat(32)}`,`actor=7:${'44'.repeat(32)}`,'projection_map_bytes=67108864',''].join('\n'), {mode:0o600});
  const first = JSON.parse(execFileSync(path.join(ROOT, 'target/debug/hm'), ['import','--config',config,'--input',stream], {encoding:'utf8'}));
  assert.equal(first.verified, true);
  assert.deepEqual(first.counts, sqlite.manifest.counts);
  assert.equal(first.embedding_samples_verified, 3);
  const repeat = JSON.parse(execFileSync(path.join(ROOT, 'target/debug/hm'), ['import','--config',config,'--input',stream], {encoding:'utf8'}));
  assert.equal(repeat.resumed, true);
  assert.equal(repeat.events, first.events);
});

test('export rejects malformed vectors and IDs before emitting a stream', context => {
  const directory = mkdtempSync(path.join(tmpdir(), 'hm-cortex-bad-'));
  context.after(() => rmSync(directory, {recursive:true,force:true}));
  const source = path.join(directory, 'source.json');
  const store = JSON.parse(readFileSync(path.join(FIXTURES, 'donor-store.json'), 'utf8'));
  store.memories['mem-1'].embedding = [1, 'bad'];
  writeFileSync(source, JSON.stringify(store));
  assert.throws(() => exportCortex({source,format:'json'}), /finite float32/);
  store.memories['mem-1'].embedding = [.1,.2,.3];
  store.memories['mem-1'].id = 'wrong';
  writeFileSync(source, JSON.stringify(store));
  assert.throws(() => exportCortex({source,format:'json'}), /id mismatch/);
});
