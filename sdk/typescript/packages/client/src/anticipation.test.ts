import assert from "node:assert/strict";
import test from "node:test";
import { encodeToolArguments, IntendAction, WakeTrigger } from "./anticipation";

test("anticipation JSON preserves ledger integers without precision loss", () => {
  assert.equal(encodeToolArguments({ deadline_ns: 1_789_524_945_000_000_001n }),
    '{"deadline_ns":1789524945000000001}');
  assert.throws(() => encodeToolArguments({ deadline_ns: Number.MAX_SAFE_INTEGER + 1 }), /unsafe JSON number/);
  assert.throws(() => encodeToolArguments({ value: Number.NaN }), /unsafe JSON number/);
  assert.equal(encodeToolArguments({ missing: undefined, value: [true, null, "quoted\"text", 0.25] }),
    '{"value":[true,null,"quoted\\\"text",0.25]}');
  const recursive: Record<string, unknown> = {};
  recursive.child = recursive;
  assert.throws(() => encodeToolArguments(recursive), /cyclic/);
});

test("the typed SDK accepts the complete wake-trigger vocabulary", () => {
  const triggers: WakeTrigger[] = [
    { kind: "at", at_ns: 10n },
    { kind: "schedule", schedule: "daily" },
    { kind: "child_terminal", child_id: "child" },
    { kind: "process_exit", process_id: "process" },
    { kind: "file_changed", path: "src/main.rs" },
    { kind: "repository_changed", repository: "hypermind" },
    { kind: "channel_message", channel: "ops" },
    { kind: "external_condition", condition: "available" },
    { kind: "user_response", reply_to: "question" },
    { kind: "entity_mentioned", entity_id: "entity" },
    { kind: "loop_closed", loop_id: "loop" },
    { kind: "prediction_resolved", prediction_id: "prediction" },
    { kind: "belief_changed", canonical_identity: "region" },
  ];
  for (const trigger of triggers) {
    const action: IntendAction = {
      kind: "set_intention", intention_id: trigger.kind, objective: "follow up",
      trigger, expires_at_ns: 9_000_000_000_000_000_000n, reply_route: "conversation",
    };
    assert.equal(JSON.parse(encodeToolArguments(action)).trigger.kind, trigger.kind);
  }
});
