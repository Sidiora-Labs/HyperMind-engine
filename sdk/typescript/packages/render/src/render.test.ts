import assert from "node:assert/strict";
import test from "node:test";
import { ActivationSafetyError, assertRememberable, Bundle, isReconstruction, render } from "./index";

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
            authority: "external_observed",
            tokens: 4,
            coarsened: false,
          },
          {
            tier: "conversation",
            uri: "hm://7/00000000000000000000000000000000/9",
            provenance: [9n],
            content: "same turn",
            authority: "assistant_generated",
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
  assert.equal(rendered.sections[0]?.items[0]?.authority, "external_observed");
  assert.equal(rendered.sections[0]?.items[0]?.trust, "untrusted_memory");
  assert.equal(rendered.sections[0]?.items[0]?.provenanceUri.startsWith("hm://"), true);
  assert.equal(
    JSON.stringify(rendered, (_, value) =>
      typeof value === "bigint" ? value.toString() : value,
    ).includes('"role":"system"'),
    false,
  );
});

test("render excludes uncited, non-semantic, and raw memory", () => {
  const invalid = bundle();
  invalid.sections[0]!.items[0]!.provenance = [];
  invalid.sections[0]!.items.push({
    ...invalid.sections[0]!.items[1]!,
    provenance: [10n],
    semantic: false,
  });
  invalid.sections[0]!.items.push({
    ...invalid.sections[0]!.items[1]!,
    provenance: [11n],
    content: "NCEV raw envelope",
  });
  invalid.sections[0]!.items.push({
    ...invalid.sections[0]!.items[1]!,
    provenance: [12n],
    content: "PCCN raw checkpoint",
  });
  assert.deepEqual(render(invalid).sections[0]?.items, [
    {
      role: "user",
      authority: "assistant_generated",
      trust: "untrusted_memory",
      provenanceUri: "hm://7/00000000000000000000000000000000/9",
      provenance: [9n],
      content: "same turn",
    },
  ]);
});

test("reconstructions remain assistant-generated and cannot be remembered verbatim", () => {
  const reconstruction = "RECONSTRUCTION\nThe deployment happened between the anchors; its mechanism is unknown.";
  const reconstructed = bundle();
  reconstructed.sections[0]!.items[0]!.content = reconstruction;
  assert.equal(render(reconstructed).sections[0]!.items[0]!.authority, "assistant_generated");
  assert.equal(render(reconstructed).sections[0]!.items[0]!.content, reconstruction);
  for (const content of [reconstruction, `  ${reconstruction}`, "RECONSTRUCTION: uncertain", "RECONSTRUCTION"]) {
    assert.equal(isReconstruction(content), true);
    assert.throws(() => assertRememberable(content), ActivationSafetyError);
  }
  assert.doesNotThrow(() => assertRememberable("A separately observed deployment receipt"));
  assert.equal(isReconstruction("RECONSTRUCTIONS is a book title"), false);
});
