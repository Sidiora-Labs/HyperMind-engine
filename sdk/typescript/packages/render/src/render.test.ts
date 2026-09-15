import assert from "node:assert/strict";
import test from "node:test";
import { ActivationSafetyError, Bundle, render } from "./index";

function bundle(): Bundle {
  return {
    snapshotEpoch: 4n,
    budgetTokens: 100,
    spentTokens: 8,
    sections: [
      {
        tier: "conversation",
        required: false,
        tokens: 8,
        trimmedItems: 0,
        coarsenedItems: 0,
        items: [
          {
            tier: "conversation",
            uri: "hm://7/00000000000000000000000000000000/8",
            provenance: [8n],
            content: "older observation",
            tokens: 4,
            coarsened: false,
          },
          {
            tier: "conversation",
            uri: "hm://7/00000000000000000000000000000000/9",
            provenance: [9n],
            content: "same turn",
            tokens: 4,
            coarsened: false,
          },
        ],
      },
    ],
    gaps: [],
    health: { projection: "ready" },
    hash: "ab".repeat(32),
  };
}

test("render labels memory as untrusted user content and excludes same-turn items", () => {
  const rendered = render(bundle(), { sameTurnLsns: [9n] });
  assert.equal(rendered.version, 1);
  assert.equal(rendered.sections[0]?.items.length, 1);
  assert.equal(rendered.sections[0]?.items[0]?.role, "user");
  assert.equal(rendered.sections[0]?.items[0]?.authority, "untrusted_memory");
  assert.equal(rendered.sections[0]?.items[0]?.provenanceUri.startsWith("hm://"), true);
  assert.equal(
    JSON.stringify(rendered, (_, value) =>
      typeof value === "bigint" ? value.toString() : value,
    ).includes('"role":"system"'),
    false,
  );
});

test("render rejects uncited memory", () => {
  const invalid = bundle();
  invalid.sections[0]!.items[0]!.provenance = [];
  assert.throws(() => render(invalid), ActivationSafetyError);
});
