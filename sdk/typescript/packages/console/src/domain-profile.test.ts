import assert from "node:assert/strict";
import test from "node:test";
import { Client } from "@hypermind/client";
import {
  applyDomainAction,
  compileDomainProfile,
  DomainProfile,
  emptyDomainProfile,
  loadDomainProfile,
  persistDomainProfile,
  renderDomainProfile,
  RESERVED_FIELD_NAMES,
} from "./domain-profile.js";
import { ConsoleDataError } from "./errors.js";
import { clientTransport } from "./node-transport.js";
import { buildOverview } from "./overview.js";
import { ConsoleTransport } from "./transport.js";
import { daemonFixture } from "./test/daemon.js";

const DOCUMENT =
  '{"schema":"hypermind.domain-profile.v1","types":[{"description":"a party the operator serves",' +
  '"fields":[{"kind":"text","name":"display_name","required":true},' +
  '{"kind":"date","name":"opened_on","required":false}],"id":"type-1","name":"Customer",' +
  '"relations":[{"cardinality":"many","name":"orders","target":"Order"}]},' +
  '{"description":"one purchase the customer placed",' +
  '"fields":[{"kind":"identifier","name":"reference","required":true}],"id":"type-2",' +
  '"name":"Order","relations":[]}],"version":6}';
const DIGEST = "eeb246c4beb6d632fb75ffea2681e375e1e065cd3c7ae7c5985b079481e8c64e";

function catalogue(): DomainProfile {
  return {
    schema: "hypermind.domain-profile.v1",
    version: 6,
    types: [
      {
        id: "type-1",
        name: "Customer",
        description: "a party the operator serves",
        fields: [
          { name: "display_name", kind: "text", required: true },
          { name: "opened_on", kind: "date", required: false },
        ],
        relations: [{ name: "orders", target: "Order", cardinality: "many" }],
      },
      {
        id: "type-2",
        name: "Order",
        description: "one purchase the customer placed",
        fields: [{ name: "reference", kind: "identifier", required: true }],
        relations: [],
      },
    ],
  };
}

function broken(): DomainProfile {
  return {
    schema: "hypermind.domain-profile.v1",
    version: 3,
    types: [
      {
        id: "type-1",
        name: "Customer",
        description: "",
        fields: [
          { name: "reference", kind: "text", required: true },
          { name: "reference", kind: "text", required: false },
          { name: "lsn", kind: "number", required: false },
          { name: "", kind: "text", required: false },
        ],
        relations: [{ name: "orders", target: "Order", cardinality: "many" }],
      },
      { id: "type-2", name: "Customer", description: "", fields: [], relations: [] },
      { id: "type-3", name: "", description: "", fields: [], relations: [] },
    ],
  };
}

async function appliedLsn(transport: ConsoleTransport): Promise<number> {
  return buildOverview(await transport.callTool("inspect", {})).appliedLsn;
}

test("domain profile actions bump the version", () => {
  const start = emptyDomainProfile();
  assert.deepEqual(start, { schema: "hypermind.domain-profile.v1", version: 1, types: [] });

  const added = applyDomainAction(start, { type: "add_type", name: "Customer", description: "a party" });
  assert.equal(added.version, 2);
  assert.equal(added.types.length, 1);
  assert.equal(added.types[0]!.id, "type-1");
  assert.equal(start.version, 1);
  assert.equal(start.types.length, 0);

  const withField = applyDomainAction(added, {
    type: "add_field",
    id: "type-1",
    field: { name: "display_name", kind: "text", required: true },
  });
  assert.equal(withField.version, 3);
  assert.equal(added.types[0]!.fields.length, 0);

  const duplicated = applyDomainAction(withField, { type: "duplicate_type", id: "type-1" });
  assert.equal(duplicated.version, 4);
  assert.equal(duplicated.types.length, 2);
  assert.equal(duplicated.types[1]!.id, "type-3");
  assert.equal(duplicated.types[1]!.name, "Customer copy");
  assert.equal(withField.types.length, 1);

  const trimmed = applyDomainAction(duplicated, {
    type: "delete_field",
    id: "type-1",
    name: "display_name",
  });
  assert.equal(trimmed.version, 5);
  assert.equal(trimmed.types[0]!.fields.length, 0);
  assert.equal(duplicated.types[0]!.fields.length, 1);

  const related = applyDomainAction(trimmed, {
    type: "add_relation",
    id: "type-1",
    relation: { name: "copies", target: "Customer copy", cardinality: "many" },
  });
  assert.equal(related.version, 6);
  const renamed = applyDomainAction(related, { type: "rename_type", id: "type-3", name: "Order" });
  assert.equal(renamed.version, 7);
  assert.equal(renamed.types[1]!.name, "Order");
  const updated = applyDomainAction(renamed, {
    type: "update_field",
    id: "type-3",
    name: "display_name",
    field: { name: "reference", kind: "identifier", required: true },
  });
  assert.equal(updated.version, 8);
  assert.equal(updated.types[1]!.fields[0]!.name, "reference");
  const removed = applyDomainAction(updated, { type: "delete_relation", id: "type-1", name: "copies" });
  assert.equal(removed.version, 9);
  const dropped = applyDomainAction(removed, { type: "delete_type", id: "type-3" });
  assert.equal(dropped.version, 10);
  assert.equal(dropped.types.length, 1);

  for (const action of [
    { type: "rename_type", id: "type-404", name: "Ghost" },
    { type: "delete_type", id: "type-404" },
    { type: "duplicate_type", id: "type-404" },
    { type: "delete_field", id: "type-404", name: "display_name" },
    { type: "delete_relation", id: "type-404", name: "copies" },
    { type: "add_field", id: "type-404", field: { name: "x", kind: "text", required: false } },
  ] as const) {
    const unchanged = applyDomainAction(dropped, action);
    assert.equal(unchanged, dropped);
    assert.equal(unchanged.version, 10);
  }
});

test("domain profile compilation rejects invalid shapes", () => {
  assert.deepEqual([...RESERVED_FIELD_NAMES].sort(), [
    "authority",
    "conversation",
    "lsn",
    "provenance",
  ]);

  const invalid = compileDomainProfile(broken());
  for (const expected of [
    'duplicate type name "Customer"',
    'duplicate field name "reference" in type "Customer"',
    'relation "orders" in type "Customer" targets undeclared type "Order"',
    'reserved field name "lsn" in type "Customer"',
    'empty field name in type "Customer"',
    'empty type name for the type with id "type-3"',
  ]) {
    assert.ok(
      invalid.errors.includes(expected),
      `${expected} is missing from ${JSON.stringify(invalid.errors)}`,
    );
  }

  const compiled = compileDomainProfile(catalogue());
  assert.deepEqual(compiled.errors, []);
  assert.equal(compiled.document, DOCUMENT);
  assert.equal(compiled.digest, DIGEST);
  const again = compileDomainProfile(catalogue());
  assert.equal(again.document, compiled.document);
  assert.equal(again.digest, compiled.digest);

  const html = renderDomainProfile(catalogue(), compiled);
  assert.ok(html.includes(DIGEST));
  assert.ok(html.includes("<dd>6</dd>"));
  assert.ok(html.includes("It does not change server-side extraction"));
  const escaped = renderDomainProfile(
    applyDomainAction(emptyDomainProfile(), { type: "add_type", name: "<script>alert(1)</script>" }),
    { digest: DIGEST, errors: ['empty field name in type "<img src=x>"'] },
  );
  assert.equal(escaped.includes("<script>"), false);
  assert.equal(escaped.includes("<img"), false);
});

test("domain profile round-trips through the ledger", async (context) => {
  const fixture = await daemonFixture();
  context.after(fixture.stop);
  const client = await Client.connect({
    socketPath: fixture.socket,
    capabilityToken: fixture.token,
    connectionId: fixture.connectionId,
  });
  context.after(() => client.close());
  const transport = clientTransport(client);

  assert.equal(await loadDomainProfile(transport), undefined);

  const provenance = await persistDomainProfile(transport, catalogue());
  assert.ok(provenance.length >= 1);
  for (const uri of provenance) assert.match(uri, /^hm:\/\/7\/lsn\/\d+$/);
  assert.deepEqual(await loadDomainProfile(transport), catalogue());

  const extended = applyDomainAction(catalogue(), { type: "add_type", name: "Supplier" });
  assert.equal(extended.version, 7);
  await persistDomainProfile(transport, extended);
  const loaded = await loadDomainProfile(transport);
  assert.deepEqual(loaded, extended);
  assert.equal(loaded?.schema, "hypermind.domain-profile.v1");
  assert.equal(loaded?.version, 7);
  assert.equal(loaded?.types.length, 3);

  const before = await appliedLsn(transport);
  await assert.rejects(() => persistDomainProfile(transport, broken()), ConsoleDataError);
  assert.equal(await appliedLsn(transport), before);
  assert.deepEqual(await loadDomainProfile(transport), extended);
});
