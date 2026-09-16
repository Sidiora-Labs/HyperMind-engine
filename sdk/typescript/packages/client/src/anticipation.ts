export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };
export type LedgerInteger = number | bigint;

export interface ToolEnvelope {
  ok: boolean;
  items: unknown[];
  provenance: string[];
  budget?: unknown;
  gaps: Array<{ kind: string; detail?: string }>;
  health: Record<string, unknown>;
  warnings: string[];
  effect_state?: "not_dispatched" | "unknown" | "rejected";
}

export type PredicateKind =
  | "object_exists" | "revision_equals" | "digest_equals" | "receipt_matches"
  | "property_satisfies" | "process_terminated" | "answer_committed";

export interface ExpectedPredicateInput {
  kind: PredicateKind;
  scope: string;
  property?: string;
  expected?: JsonValue;
}

export interface PredictInput {
  prediction_id: string;
  revision: number;
  task_id?: string;
  attempt_id?: string;
  operation_id?: string;
  mechanism: string;
  predicates: ExpectedPredicateInput[];
  deadline_ns: LedgerInteger;
  uncertainty: string;
}

export interface OutcomeInput {
  prediction_id: string;
  revision: number;
  observation_lsns: LedgerInteger[];
}

export interface InspectInput {
  uri?: string;
}

export type WakeTrigger =
  | { kind: "at"; at_ns: LedgerInteger }
  | { kind: "schedule"; schedule: string }
  | { kind: "child_terminal"; child_id: string }
  | { kind: "process_exit"; process_id: string }
  | { kind: "file_changed"; path: string }
  | { kind: "repository_changed"; repository: string }
  | { kind: "channel_message"; channel: string }
  | { kind: "external_condition"; condition: string }
  | { kind: "user_response"; reply_to: string }
  | { kind: "entity_mentioned"; entity_id: string }
  | { kind: "loop_closed"; loop_id: string }
  | { kind: "prediction_resolved"; prediction_id: string }
  | { kind: "belief_changed"; canonical_identity: string };

export interface AttentionFactors {
  urgency: number;
  expected_value: number;
  confidence: number;
  interruption_cost: number;
  resource_cost: number;
  duplication_penalty: number;
  quiet_hours: boolean;
  notifications_remaining: number;
  workload: number;
}

export type IntendAction =
  | { kind: "set_objective"; objective: string }
  | { kind: "open_loop"; loop_id: string; objective: string }
  | { kind: "close_loop"; loop_id: string; reason: "done" | "abandoned" | "handed_off" | "superseded"; cause?: string; evidence_lsns?: LedgerInteger[] }
  | { kind: "set_intention"; intention_id: string; objective: string; trigger: WakeTrigger; expires_at_ns: LedgerInteger; reply_route: string }
  | { kind: "cancel_intention"; intention_id: string; reason: string }
  | { kind: "evaluate_wake"; observation_lsn: LedgerInteger; factors: AttentionFactors; rearm_at_ns?: LedgerInteger }
  | { kind: "adopt_procedure"; procedure_id: string; procedure_lsn: LedgerInteger };

export interface ReconstructionRecallOptions {
  mode: "reconstruct";
  anchorLsns: LedgerInteger[];
  maximumOutputTokens?: number;
}

export function encodeToolArguments(value: unknown): string {
  const ancestors = new Set<object>();
  function encode(item: unknown): string {
    if (typeof item === "bigint") return item.toString();
    if (typeof item === "number") {
      if (!Number.isFinite(item) || Number.isInteger(item) && !Number.isSafeInteger(item)) {
        throw new RangeError("unsafe JSON number; use bigint for ledger integers");
      }
      return JSON.stringify(item);
    }
    if (item === null || typeof item === "boolean" || typeof item === "string") return JSON.stringify(item);
    if (typeof item !== "object") throw new TypeError("unsupported tool argument");
    if (ancestors.has(item)) throw new TypeError("cyclic tool argument");
    ancestors.add(item);
    const result = Array.isArray(item)
      ? `[${item.map((entry) => encode(entry ?? null)).join(",")}]`
      : `{${Object.entries(item).filter(([, entry]) => entry !== undefined)
          .map(([key, entry]) => `${JSON.stringify(key)}:${encode(entry)}`).join(",")}}`;
    ancestors.delete(item);
    return result;
  }
  return encode(value);
}
