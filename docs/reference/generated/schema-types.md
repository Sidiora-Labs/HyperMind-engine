# Schema type catalog

Generated from the canonical FlatBuffers and protobuf schemas. Field names, numeric values, defaults, and union members below are copied from source. Generated language bindings are representations of these types, not separate authority or storage formats.

## events.fbs::ResultStatus

<a id="schema-schemas-events-fbs-resultstatus"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Result Status when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
enum ResultStatus : ubyte { ok, error, outcome_unknown }
```

## events.fbs::ApprovalDecision

<a id="schema-schemas-events-fbs-approvaldecision"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Approval Decision when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
enum ApprovalDecision : ubyte { approved, denied }
```

## events.fbs::EffectState

<a id="schema-schemas-events-fbs-effectstate"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Effect State when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
enum EffectState : ubyte { dispatched, committed, returned, outcome_unknown }
```

## events.fbs::LoopCloseReason

<a id="schema-schemas-events-fbs-loopclosereason"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Loop Close Reason when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
enum LoopCloseReason : ubyte { done, abandoned, handed_off, superseded }
```

## events.fbs::BeliefType

<a id="schema-schemas-events-fbs-belieftype"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Belief Type when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
enum BeliefType : ubyte { fact, preference, constraint, goal, identity }
```

## events.fbs::AssertionClaim

<a id="schema-schemas-events-fbs-assertionclaim"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Assertion Claim when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
enum AssertionClaim : ubyte { affirmative, negative_existence }
```

## events.fbs::AttestationDisposition

<a id="schema-schemas-events-fbs-attestationdisposition"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Attestation Disposition when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
enum AttestationDisposition : ubyte { used, ignored, helpful, harmful }
```

## events.fbs::MemoryFadeReason

<a id="schema-schemas-events-fbs-memoryfadereason"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Memory Fade Reason when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
enum MemoryFadeReason : ubyte { low_retrievability, superseded, explicit }
```

## events.fbs::ReviewRating

<a id="schema-schemas-events-fbs-reviewrating"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Review Rating when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
enum ReviewRating : ubyte { again, hard, good, easy }
```

## events.fbs::ConsolidationPhaseName

<a id="schema-schemas-events-fbs-consolidationphasename"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Consolidation Phase Name when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
enum ConsolidationPhaseName : ubyte { nrem, connect, abstract, hindsight, review, publish, extract }
```

## events.fbs::ConsolidationPhaseState

<a id="schema-schemas-events-fbs-consolidationphasestate"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Consolidation Phase State when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
enum ConsolidationPhaseState : ubyte { started, completed, aborted }
```

## events.fbs::Authority

<a id="schema-schemas-events-fbs-authority"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Authority when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
enum Authority : ubyte {
  user_asserted,
  external_observed,
  tool_observed,
  runtime_fact,
  assistant_generated,
  derived_inference
}
```

## events.fbs::DocumentCut

<a id="schema-schemas-events-fbs-documentcut"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Document Cut when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
enum DocumentCut : ubyte { paragraph_end, paragraph_cut, row_end, row_cut }
```

## events.fbs::DocumentChange

<a id="schema-schemas-events-fbs-documentchange"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Document Change when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
enum DocumentChange : ubyte { retained, moved, replaced, added }
```

## events.fbs::Retention

<a id="schema-schemas-events-fbs-retention"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Retention when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
enum Retention : ubyte { current_state, daily, durable, do_not_store }
```

## events.fbs::Sensitivity

<a id="schema-schemas-events-fbs-sensitivity"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Sensitivity when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
enum Sensitivity : ubyte { public, personal, secret }
```

## events.fbs::ProvenanceRange

<a id="schema-schemas-events-fbs-provenancerange"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Provenance Range when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
struct ProvenanceRange {
  first_lsn:ulong;
  last_lsn:ulong;
  byte_start:uint;
  byte_end:uint;
}
```

## events.fbs::ModelProvenance

<a id="schema-schemas-events-fbs-modelprovenance"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Model Provenance when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table ModelProvenance {
  model_id:string (required);
  prompt_id:string (required);
  prompt_version:ushort;
  temperature:float;
  call_id:[ubyte];
  input_tokens:ulong;
  output_tokens:ulong;
  cache_read_tokens:ulong;
  cache_write_tokens:ulong;
  cost_microusd:ulong;
}
```

## events.fbs::UserMsg

<a id="schema-schemas-events-fbs-usermsg"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use User Msg when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table UserMsg {
  content:[ubyte] (required);
}
```

## events.fbs::DeliveredMsg

<a id="schema-schemas-events-fbs-deliveredmsg"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Delivered Msg when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table DeliveredMsg {
  content:[ubyte] (required);
}
```

## events.fbs::ToolCall

<a id="schema-schemas-events-fbs-toolcall"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Tool Call when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table ToolCall {
  call_id:[ubyte] (required);
  tool_name:string (required);
  arguments:[ubyte] (required);
}
```

## events.fbs::ToolResult

<a id="schema-schemas-events-fbs-toolresult"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Tool Result when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table ToolResult {
  call_id:[ubyte] (required);
  tool_call_lsn:ulong;
  status:ResultStatus;
  result:[ubyte] (required);
}
```

## events.fbs::Reasoning

<a id="schema-schemas-events-fbs-reasoning"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Reasoning when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table Reasoning {
  content:[ubyte] (required);
}
```

## events.fbs::ProviderFrame

<a id="schema-schemas-events-fbs-providerframe"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Provider Frame when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table ProviderFrame {
  provider:string (required);
  api_content:[ubyte] (required);
}
```

## events.fbs::MediaRef

<a id="schema-schemas-events-fbs-mediaref"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Media Ref when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table MediaRef {
  uri:string (required);
  media_type:string (required);
  digest:[ubyte] (required);
}
```

## events.fbs::Effect

<a id="schema-schemas-events-fbs-effect"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Effect when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table Effect {
  effect_id:[ubyte] (required);
  tool_call_lsn:ulong;
  state:EffectState;
}
```

## events.fbs::Approval

<a id="schema-schemas-events-fbs-approval"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Approval when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table Approval {
  effect_id:[ubyte] (required);
  decision:ApprovalDecision;
}
```

## events.fbs::Outcome

<a id="schema-schemas-events-fbs-outcome"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Outcome when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table Outcome {
  effect_id:[ubyte] (required);
  status:ResultStatus;
  detail:[ubyte] (required);
  evidence_lsns:[ulong];
}
```

## events.fbs::Checkpoint

<a id="schema-schemas-events-fbs-checkpoint"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Checkpoint when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table Checkpoint {
  cursor:[ubyte] (required);
}
```

## events.fbs::Supervisor

<a id="schema-schemas-events-fbs-supervisor"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Supervisor when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table Supervisor {
  code:string (required);
  evidence:[ubyte] (required);
}
```

## events.fbs::Recovery

<a id="schema-schemas-events-fbs-recovery"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Recovery when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table Recovery {
  code:string (required);
  target_lsn:ulong;
}
```

## events.fbs::IntentSet

<a id="schema-schemas-events-fbs-intentset"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Intent Set when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table IntentSet {
  objective:[ubyte] (required);
}
```

## events.fbs::LoopOpened

<a id="schema-schemas-events-fbs-loopopened"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Loop Opened when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table LoopOpened {
  loop_id:[ubyte] (required);
  objective:[ubyte] (required);
}
```

## events.fbs::LoopClosed

<a id="schema-schemas-events-fbs-loopclosed"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Loop Closed when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table LoopClosed {
  loop_id:[ubyte] (required);
  reason:LoopCloseReason;
  cause:[ubyte] (required);
  evidence_lsns:[ulong];
}
```

## events.fbs::Binding

<a id="schema-schemas-events-fbs-binding"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Binding when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table Binding {
  task:[ubyte];
  scope:[ubyte];
  canonical_entity:string (required);
  property:string (required);
  evidence_lsn:ulong;
  revision:[ubyte] (required);
  freshness_requirement_ns:ulong;
}
```

## events.fbs::Assertion

<a id="schema-schemas-events-fbs-assertion"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Assertion when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table Assertion {
  belief_id:[ubyte] (required);
  belief_type:BeliefType;
  canonical_identity:string (required);
  value:[ubyte] (required);
  valid_from_ns:long;
  valid_to_ns:long;
  provenance:[ProvenanceRange] (required);
  conflict_domain:string;
  claim:AssertionClaim = affirmative;
}
```

## events.fbs::Consolidation

<a id="schema-schemas-events-fbs-consolidation"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Consolidation when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table Consolidation {
  assertions:[Assertion] (required);
}
```

## events.fbs::ProposedAssertion

<a id="schema-schemas-events-fbs-proposedassertion"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Proposed Assertion when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table ProposedAssertion {
  belief_id:[ubyte] (required);
  belief_type:BeliefType;
  canonical_identity:string (required);
  value:[ubyte] (required);
  valid_from_ns:long;
  valid_to_ns:long;
  provenance:[ProvenanceRange] (required);
  conflict_domain:string;
  claim:AssertionClaim = affirmative;
}
```

## events.fbs::Embedding

<a id="schema-schemas-events-fbs-embedding"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Embedding when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table Embedding {
  target_lsn:ulong;
  dimension:uint;
  quantized:[byte] (required);
  binary_prefilter:[ubyte] (required);
  space_id:string (required);
}
```

## events.fbs::Retract

<a id="schema-schemas-events-fbs-retract"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Retract when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table Retract {
  belief_id:[ubyte] (required);
  provenance:[ProvenanceRange] (required);
}
```

## events.fbs::Attestation

<a id="schema-schemas-events-fbs-attestation"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Attestation when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table Attestation {
  target_lsn:ulong;
  disposition:AttestationDisposition;
}
```

## events.fbs::MemoryMinted

<a id="schema-schemas-events-fbs-memoryminted"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Memory Minted when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table MemoryMinted {
  memory_id:[ubyte] (required);
  name:string (required);
  definition:[ubyte] (required);
  tags:[string] (required);
  salience_micros:uint;
  citations:[ProvenanceRange] (required);
}
```

## events.fbs::MemoryRevised

<a id="schema-schemas-events-fbs-memoryrevised"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Memory Revised when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table MemoryRevised {
  memory_id:[ubyte] (required);
  previous_lsn:ulong;
  name:string (required);
  definition:[ubyte] (required);
  tags:[string] (required);
  salience_micros:uint;
  citations:[ProvenanceRange] (required);
}
```

## events.fbs::MemoryId

<a id="schema-schemas-events-fbs-memoryid"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Memory Id when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table MemoryId {
  value:[ubyte] (required);
}
```

## events.fbs::MemoryMerged

<a id="schema-schemas-events-fbs-memorymerged"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Memory Merged when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table MemoryMerged {
  memory_id:[ubyte] (required);
  merged_memory_ids:[MemoryId] (required);
  name:string (required);
  definition:[ubyte] (required);
  tags:[string] (required);
  salience_micros:uint;
  citations:[ProvenanceRange] (required);
}
```

## events.fbs::MemoryFaded

<a id="schema-schemas-events-fbs-memoryfaded"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Memory Faded when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table MemoryFaded {
  memory_id:[ubyte] (required);
  reason:MemoryFadeReason;
  evidence_lsns:[ulong];
}
```

## events.fbs::EdgeAsserted

<a id="schema-schemas-events-fbs-edgeasserted"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Edge Asserted when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table EdgeAsserted {
  edge_id:[ubyte] (required);
  source_id:[ubyte] (required);
  target_id:[ubyte] (required);
  relation:string (required);
  weight_micros:uint;
  valid_from_ns:long;
  valid_to_ns:long;
  citations:[ProvenanceRange] (required);
}
```

## events.fbs::EdgeRetracted

<a id="schema-schemas-events-fbs-edgeretracted"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Edge Retracted when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table EdgeRetracted {
  edge_id:[ubyte] (required);
  citations:[ProvenanceRange] (required);
}
```

## events.fbs::ConsolidationBudget

<a id="schema-schemas-events-fbs-consolidationbudget"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Consolidation Budget when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table ConsolidationBudget {
  max_llm_calls:ulong;
  max_tokens:ulong;
  max_microusd:ulong;
  max_wall_ms:ulong;
}
```

## events.fbs::PromptVersion

<a id="schema-schemas-events-fbs-promptversion"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Prompt Version when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table PromptVersion {
  prompt_id:string (required);
  version:ushort;
  model_id:string (required);
}
```

## events.fbs::ConsolidationOpened

<a id="schema-schemas-events-fbs-consolidationopened"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Consolidation Opened when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table ConsolidationOpened {
  scope_digest:[ubyte] (required);
  cadence_key:string (required);
  generation:ulong;
  expected_active_generation:ulong;
  phases:[ConsolidationPhaseName] (required);
  prompts:[PromptVersion] (required);
  budget:ConsolidationBudget (required);
  source_first_lsn:ulong;
  source_last_lsn:ulong;
}
```

## events.fbs::ConsolidationPhase

<a id="schema-schemas-events-fbs-consolidationphase"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Consolidation Phase when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table ConsolidationPhase {
  phase:ConsolidationPhaseName;
  state:ConsolidationPhaseState;
  attempt_prefix:[ubyte] (required);
  cursor:[ubyte];
  llm_calls:ulong;
  input_tokens:ulong;
  output_tokens:ulong;
  cost_microusd:ulong;
  dropped_candidates:ulong;
}
```

## events.fbs::ConsolidationClosed

<a id="schema-schemas-events-fbs-consolidationclosed"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Consolidation Closed when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table ConsolidationClosed {
  generation:ulong;
  expected_active_generation:ulong;
  derived_records:ulong;
  dropped_candidates:ulong;
  llm_calls:ulong;
  input_tokens:ulong;
  output_tokens:ulong;
  cost_microusd:ulong;
}
```

## events.fbs::ConsolidationRetracted

<a id="schema-schemas-events-fbs-consolidationretracted"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Consolidation Retracted when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table ConsolidationRetracted {
  target_run_id:[ubyte] (required);
  previous_generation:ulong;
  reason:string (required);
}
```

## events.fbs::Reviewed

<a id="schema-schemas-events-fbs-reviewed"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Reviewed when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table Reviewed {
  memory_id:[ubyte] (required);
  rating:ReviewRating;
  source_lsn:ulong;
  reviewed_at_ns:long;
  stability_millis:ulong;
  difficulty_micros:uint;
  due_at_ns:long;
}
```

## events.fbs::AttentionDecision

<a id="schema-schemas-events-fbs-attentiondecision"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Attention Decision when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
enum AttentionDecision : ubyte { ignore, remember, batch, schedule, ask_user, start_work, notify }
```

## events.fbs::PredicateKind

<a id="schema-schemas-events-fbs-predicatekind"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Predicate Kind when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
enum PredicateKind : ubyte {
  object_exists,
  revision_equals,
  digest_equals,
  receipt_matches,
  property_satisfies,
  process_terminated,
  answer_committed
}
```

## events.fbs::OutcomeAssessment

<a id="schema-schemas-events-fbs-outcomeassessment"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Outcome Assessment when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
enum OutcomeAssessment : ubyte { supported, contradicted, pending, unresolvable, not_executed }
```

## events.fbs::WakeAt

<a id="schema-schemas-events-fbs-wakeat"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Wake At when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table WakeAt {
  at_ns:long;
}
```

## events.fbs::WakeSchedule

<a id="schema-schemas-events-fbs-wakeschedule"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Wake Schedule when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table WakeSchedule {
  schedule:string (required);
}
```

## events.fbs::WakeChildTerminal

<a id="schema-schemas-events-fbs-wakechildterminal"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Wake Child Terminal when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table WakeChildTerminal {
  child_id:[ubyte] (required);
}
```

## events.fbs::WakeProcessExit

<a id="schema-schemas-events-fbs-wakeprocessexit"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Wake Process Exit when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table WakeProcessExit {
  process_id:[ubyte] (required);
}
```

## events.fbs::WakeFileChanged

<a id="schema-schemas-events-fbs-wakefilechanged"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Wake File Changed when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table WakeFileChanged {
  path:string (required);
}
```

## events.fbs::WakeRepositoryChanged

<a id="schema-schemas-events-fbs-wakerepositorychanged"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Wake Repository Changed when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table WakeRepositoryChanged {
  repository:string (required);
}
```

## events.fbs::WakeChannelMessage

<a id="schema-schemas-events-fbs-wakechannelmessage"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Wake Channel Message when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table WakeChannelMessage {
  channel:string (required);
}
```

## events.fbs::WakeExternalCondition

<a id="schema-schemas-events-fbs-wakeexternalcondition"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Wake External Condition when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table WakeExternalCondition {
  condition:string (required);
}
```

## events.fbs::WakeUserResponse

<a id="schema-schemas-events-fbs-wakeuserresponse"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Wake User Response when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table WakeUserResponse {
  reply_to:[ubyte] (required);
}
```

## events.fbs::WakeEntityMentioned

<a id="schema-schemas-events-fbs-wakeentitymentioned"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Wake Entity Mentioned when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table WakeEntityMentioned {
  entity_id:[ubyte] (required);
}
```

## events.fbs::WakeLoopClosed

<a id="schema-schemas-events-fbs-wakeloopclosed"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Wake Loop Closed when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table WakeLoopClosed {
  loop_id:[ubyte] (required);
}
```

## events.fbs::WakePredictionResolved

<a id="schema-schemas-events-fbs-wakepredictionresolved"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Wake Prediction Resolved when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table WakePredictionResolved {
  prediction_id:[ubyte] (required);
}
```

## events.fbs::WakeBeliefChanged

<a id="schema-schemas-events-fbs-wakebeliefchanged"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Wake Belief Changed when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table WakeBeliefChanged {
  canonical_identity:string (required);
}
```

## events.fbs::WakeTrigger

<a id="schema-schemas-events-fbs-waketrigger"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Wake Trigger when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
union WakeTrigger {
  WakeAt,
  WakeSchedule,
  WakeChildTerminal,
  WakeProcessExit,
  WakeFileChanged,
  WakeRepositoryChanged,
  WakeChannelMessage,
  WakeExternalCondition,
  WakeUserResponse,
  WakeEntityMentioned,
  WakeLoopClosed,
  WakePredictionResolved,
  WakeBeliefChanged
}
```

## events.fbs::IntentionSet

<a id="schema-schemas-events-fbs-intentionset"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Intention Set when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table IntentionSet {
  intention_id:[ubyte] (required);
  objective:[ubyte] (required);
  trigger:WakeTrigger;
  expires_at_ns:long;
  reply_route:string (required);
}
```

## events.fbs::IntentionFired

<a id="schema-schemas-events-fbs-intentionfired"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Intention Fired when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table IntentionFired {
  intention_id:[ubyte] (required);
  wake_id:[ubyte] (required);
  trigger_lsn:ulong;
}
```

## events.fbs::AttentionDecided

<a id="schema-schemas-events-fbs-attentiondecided"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Attention Decided when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table AttentionDecided {
  intention_id:[ubyte] (required);
  wake_id:[ubyte] (required);
  decision:AttentionDecision;
  reason:string (required);
}
```

## events.fbs::IntentionCancelled

<a id="schema-schemas-events-fbs-intentioncancelled"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Intention Cancelled when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table IntentionCancelled {
  intention_id:[ubyte] (required);
  reason:string (required);
}
```

## events.fbs::ExpectedPredicate

<a id="schema-schemas-events-fbs-expectedpredicate"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Expected Predicate when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table ExpectedPredicate {
  kind:PredicateKind;
  scope:string (required);
  property:string;
  expected:[ubyte];
}
```

## events.fbs::Predicted

<a id="schema-schemas-events-fbs-predicted"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Predicted when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table Predicted {
  prediction_id:[ubyte] (required);
  revision:uint;
  task_id:[ubyte];
  attempt_id:[ubyte];
  operation_id:[ubyte];
  mechanism:string (required);
  predicates:[ExpectedPredicate] (required);
  deadline_ns:long;
  uncertainty:string (required);
}
```

## events.fbs::OutcomeObserved

<a id="schema-schemas-events-fbs-outcomeobserved"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Outcome Observed when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table OutcomeObserved {
  prediction_id:[ubyte] (required);
  revision:uint;
  assessment:OutcomeAssessment;
  observation_lsns:[ulong] (required);
  evaluator_version:string (required);
}
```

## events.fbs::ProcedureSupport

<a id="schema-schemas-events-fbs-proceduresupport"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Procedure Support when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table ProcedureSupport {
  source_root:[ubyte] (required);
  conversation:[ubyte] (required);
  episode_lsn:ulong;
}
```

## events.fbs::ProcedureMined

<a id="schema-schemas-events-fbs-proceduremined"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Procedure Mined when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table ProcedureMined {
  procedure_id:[ubyte] (required);
  strategy:string (required);
  expected_outcomes:[string] (required);
  preconditions:[string] (required);
  supports:[ProcedureSupport] (required);
  failures:[ulong];
  counterexamples:[ulong];
}
```

## events.fbs::ProcedureRevised

<a id="schema-schemas-events-fbs-procedurerevised"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Procedure Revised when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table ProcedureRevised {
  procedure_id:[ubyte] (required);
  previous_lsn:ulong;
  strategy:string (required);
  expected_outcomes:[string] (required);
  preconditions:[string] (required);
  supports:[ProcedureSupport] (required);
  failures:[ulong];
  counterexamples:[ulong];
}
```

## events.fbs::ProcedureAdopted

<a id="schema-schemas-events-fbs-procedureadopted"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Procedure Adopted when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table ProcedureAdopted {
  procedure_id:[ubyte] (required);
  procedure_lsn:ulong;
}
```

## events.fbs::ProcedureImported

<a id="schema-schemas-events-fbs-procedureimported"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Procedure Imported when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table ProcedureImported {
  procedure_id:[ubyte] (required);
  name:string (required);
  strategy:string (required);
  expected_outcomes:[string] (required);
  preconditions:[string] (required);
  instructions:[ubyte] (required);
  declared_tools:[string] (required);
  source_uri:string (required);
  source_digest:[ubyte] (required);
  playbook_version:ushort;
}
```

## events.fbs::ProcedureImprovementProposed

<a id="schema-schemas-events-fbs-procedureimprovementproposed"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Procedure Improvement Proposed when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table ProcedureImprovementProposed {
  proposal_id:[ubyte] (required);
  procedure_id:[ubyte] (required);
  base_lsn:ulong;
  strategy:string (required);
  expected_outcomes:[string] (required);
  preconditions:[string] (required);
  rationale:string (required);
  failure_lsns:[ulong] (required);
}
```

## events.fbs::VocabularyCategory

<a id="schema-schemas-events-fbs-vocabularycategory"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Vocabulary Category when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
enum VocabularyCategory : ubyte { entity_type, entity_instance, relation }
```

## events.fbs::VocabularyTerm

<a id="schema-schemas-events-fbs-vocabularyterm"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Vocabulary Term when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table VocabularyTerm {
  term_id:string (required);
  canonical_name:string (required);
  category:VocabularyCategory;
  parent_term_id:string;
  aliases:[string];
}
```

## events.fbs::VocabularyImported

<a id="schema-schemas-events-fbs-vocabularyimported"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Vocabulary Imported when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table VocabularyImported {
  vocabulary_id:[ubyte] (required);
  version:ushort;
  source_uri:string (required);
  source_media_type:string (required);
  source_digest:[ubyte] (required);
  terms:[VocabularyTerm] (required);
  ignored_triples:uint;
}
```

## events.fbs::DocumentPageSpan

<a id="schema-schemas-events-fbs-documentpagespan"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Document Page Span when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table DocumentPageSpan {
  page_number:uint;
  byte_start:uint;
  byte_end:uint;
}
```

## events.fbs::DocumentIngested

<a id="schema-schemas-events-fbs-documentingested"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Document Ingested when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table DocumentIngested {
  document_id:[ubyte] (required);
  name:string (required);
  media_type:string (required);
  content:[ubyte] (required);
  content_digest:[ubyte] (required);
}
```

## events.fbs::DocumentExtracted

<a id="schema-schemas-events-fbs-documentextracted"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Document Extracted when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table DocumentExtracted {
  document_id:[ubyte] (required);
  source_lsn:ulong;
  loader_id:string (required);
  extraction_version:ushort;
  text:[ubyte] (required);
  text_digest:[ubyte] (required);
  page_spans:[DocumentPageSpan] (required);
  partial_reason:string;
  failed_units:[uint];
}
```

## events.fbs::DocumentChunk

<a id="schema-schemas-events-fbs-documentchunk"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Document Chunk when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table DocumentChunk {
  chunk_id:[ubyte] (required);
  content_hash:[ubyte] (required);
  occurrence:uint;
  ordinal:uint;
  byte_start:uint;
  byte_end:uint;
  cut:DocumentCut;
  change:DocumentChange;
  page_number:uint;
  row_index:uint;
  column_start:uint;
  column_end:uint;
  token_estimate:uint;
}
```

## events.fbs::DocumentChunked

<a id="schema-schemas-events-fbs-documentchunked"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Document Chunked when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table DocumentChunked {
  document_id:[ubyte] (required);
  source_lsn:ulong;
  extraction_version:ushort;
  chunker_id:string (required);
  token_budget:uint;
  previous_chunked_lsn:ulong;
  chunks:[DocumentChunk] (required);
}
```

## events.fbs::SourceSignatureScheme

<a id="schema-schemas-events-fbs-sourcesignaturescheme"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Source Signature Scheme when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
enum SourceSignatureScheme : ubyte { hmac_sha256_v0 }
```

## events.fbs::SourceDeliveryState

<a id="schema-schemas-events-fbs-sourcedeliverystate"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Source Delivery State when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
enum SourceDeliveryState : ubyte { accepted, applied, failed, abandoned }
```

## events.fbs::ConnectorState

<a id="schema-schemas-events-fbs-connectorstate"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Connector State when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
enum ConnectorState : ubyte { bound, revoked }
```

## events.fbs::SourceConnectorBound

<a id="schema-schemas-events-fbs-sourceconnectorbound"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Source Connector Bound when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table SourceConnectorBound {
  connector_id:[ubyte] (required);
  provider:string (required);
  external_account:string (required);
  consent_nonce:[ubyte] (required);
  consent_expires_at_ns:long;
  credential_version:uint;
  signature_scheme:SourceSignatureScheme;
  scopes:[string] (required);
  state:ConnectorState;
}
```

## events.fbs::SourceDeliveryAccepted

<a id="schema-schemas-events-fbs-sourcedeliveryaccepted"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Source Delivery Accepted when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table SourceDeliveryAccepted {
  connector_id:[ubyte] (required);
  delivery_id:[ubyte] (required);
  signature_scheme:SourceSignatureScheme;
  credential_version:uint;
  signed_at_ns:long;
  body_digest:[ubyte] (required);
  body_bytes:ulong;
  event_name:string (required);
}
```

## events.fbs::SourceDeliverySettled

<a id="schema-schemas-events-fbs-sourcedeliverysettled"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Source Delivery Settled when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table SourceDeliverySettled {
  connector_id:[ubyte] (required);
  delivery_id:[ubyte] (required);
  accepted_lsn:ulong;
  attempt:uint;
  state:SourceDeliveryState;
  next_attempt_at_ns:long;
  detail:string (required);
}
```

## events.fbs::SourceRevisionObserved

<a id="schema-schemas-events-fbs-sourcerevisionobserved"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Source Revision Observed when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table SourceRevisionObserved {
  connector_id:[ubyte] (required);
  source_id:string (required);
  revision:[ubyte] (required);
  content_digest:[ubyte] (required);
  observed_at_ns:long;
  delivery_lsn:ulong;
}
```

## events.fbs::EventPayload

<a id="schema-schemas-events-fbs-eventpayload"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Event Payload when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
union EventPayload {
  UserMsg,
  DeliveredMsg,
  ToolCall,
  ToolResult,
  Reasoning,
  ProviderFrame,
  MediaRef,
  Effect,
  Approval,
  Outcome,
  Checkpoint,
  Supervisor,
  Recovery,
  IntentSet,
  LoopOpened,
  LoopClosed,
  Assertion,
  Consolidation,
  Embedding,
  Retract,
  Attestation,
  Binding,
  ProposedAssertion,
  MemoryMinted,
  MemoryRevised,
  MemoryMerged,
  MemoryFaded,
  EdgeAsserted,
  EdgeRetracted,
  ConsolidationOpened,
  ConsolidationPhase,
  ConsolidationClosed,
  ConsolidationRetracted,
  Reviewed,
  IntentionSet,
  IntentionFired,
  AttentionDecided,
  IntentionCancelled,
  Predicted,
  OutcomeObserved,
  ProcedureMined,
  ProcedureRevised,
  ProcedureAdopted,
  VocabularyImported,
  DocumentIngested,
  DocumentExtracted,
  DocumentChunked,
  SourceConnectorBound,
  SourceDeliveryAccepted,
  SourceDeliverySettled,
  SourceRevisionObserved,
  ProcedureImported,
  ProcedureImprovementProposed
}
```

## events.fbs::EventEnvelope

<a id="schema-schemas-events-fbs-eventenvelope"></a>

Source: [`schemas/events.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/events.fbs).

When to use: Use Event Envelope when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.

Do not use: Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.


```text
table EventEnvelope {
  schema_version:ushort = 1;
  payload:EventPayload (required);
  connection_id:[ubyte];
  client_seq:ulong;
  client_event_index:uint;
  client_event_count:uint;
  origin_actor:ushort;
  run_id:[ubyte];
  model_provenance:ModelProvenance;
  authority:Authority = user_asserted;
  retention:Retention = current_state;
  sensitivity:Sensitivity = public;
  event_time_ns:long;
}
```

## protocol.fbs::ResponseStatus

<a id="schema-schemas-protocol-fbs-responsestatus"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Response Status when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
enum ResponseStatus : ubyte { ok, error }
```

## protocol.fbs::MutationEffectState

<a id="schema-schemas-protocol-fbs-mutationeffectstate"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Mutation Effect State when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
enum MutationEffectState : ubyte { none, not_dispatched, unknown, rejected }
```

## protocol.fbs::Hello

<a id="schema-schemas-protocol-fbs-hello"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Hello when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table Hello {
  proto_version:ushort;
  connection_id:[ubyte] (required);
  capability_token:[ubyte] (required);
}
```

## protocol.fbs::Welcome

<a id="schema-schemas-protocol-fbs-welcome"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Welcome when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table Welcome {
  proto_version:ushort;
  actor_ns:ushort;
  admin:bool;
  maximum_frame_bytes:uint;
  maximum_batch_events:uint;
  maximum_subscriptions:uint;
  next_client_seq:ulong;
}
```

## protocol.fbs::AppendEvent

<a id="schema-schemas-protocol-fbs-appendevent"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Append Event when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table AppendEvent {
  kind:ubyte;
  conversation:[ubyte] (required);
  payload:[ubyte] (required);
}
```

## protocol.fbs::Append

<a id="schema-schemas-protocol-fbs-append"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Append when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table Append {
  client_seq:ulong;
  events:[AppendEvent] (required);
}
```

## protocol.fbs::Activate

<a id="schema-schemas-protocol-fbs-activate"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Activate when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table Activate {
  conversation:[ubyte] (required);
  query:[ubyte] (required);
  turn_text:[ubyte];
  budget_tokens:ulong;
  query_embedding:[byte];
  query_binary_prefilter:[ubyte];
  temporal_from_ns:long;
  temporal_to_ns:long;
  token_weights:[ushort];
  token_item_overhead:uint;
}
```

## protocol.fbs::Transcript

<a id="schema-schemas-protocol-fbs-transcript"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Transcript when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table Transcript {
  conversation:[ubyte] (required);
  since_lsn:ulong;
  limit:uint;
}
```

## protocol.fbs::RecallMode

<a id="schema-schemas-protocol-fbs-recallmode"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Recall Mode when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
enum RecallMode : ubyte { list_windows, open_window, resolve_members }
```

## protocol.fbs::Recall

<a id="schema-schemas-protocol-fbs-recall"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Recall when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table Recall {
  query:[ubyte] (required);
  limit:uint;
  mode:RecallMode;
  level:ubyte;
  start_ns:long;
  end_ns:long;
}
```

## protocol.fbs::AsOf

<a id="schema-schemas-protocol-fbs-asof"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use As Of when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table AsOf {
  belief_type:ubyte;
  canonical_identity:string (required);
  valid_time_ns:long;
  transaction_lsn:ulong;
  known_lsn:ulong;
}
```

## protocol.fbs::Checkpoint

<a id="schema-schemas-protocol-fbs-checkpoint"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Checkpoint when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table Checkpoint {
  turn_id:[ubyte] (required);
  blob:[ubyte] (required);
  client_seq:ulong;
}
```

## protocol.fbs::LatestCheckpoint

<a id="schema-schemas-protocol-fbs-latestcheckpoint"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Latest Checkpoint when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table LatestCheckpoint {
  turn_id:[ubyte] (required);
}
```

## protocol.fbs::Attest

<a id="schema-schemas-protocol-fbs-attest"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Attest when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table Attest {
  used:[ulong];
  ignored:[ulong];
  client_seq:ulong;
  helpful:[ulong];
  harmful:[ulong];
}
```

## protocol.fbs::Subscribe

<a id="schema-schemas-protocol-fbs-subscribe"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Subscribe when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table Subscribe {
  conversation:[ubyte];
  since_lsn:ulong;
}
```

## protocol.fbs::Health

<a id="schema-schemas-protocol-fbs-health"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Health when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table Health {}
```

## protocol.fbs::Stats

<a id="schema-schemas-protocol-fbs-stats"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Stats when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table Stats {
  actor:ushort;
}
```

## protocol.fbs::LatencyHistograms

<a id="schema-schemas-protocol-fbs-latencyhistograms"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Latency Histograms when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table LatencyHistograms {}
```

## protocol.fbs::VerifyStatus

<a id="schema-schemas-protocol-fbs-verifystatus"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Verify Status when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table VerifyStatus {
  actor:ushort;
}
```

## protocol.fbs::RebuildProjection

<a id="schema-schemas-protocol-fbs-rebuildprojection"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Rebuild Projection when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table RebuildProjection {
  actor:ushort;
  name:string (required);
}
```

## protocol.fbs::CryptoDelete

<a id="schema-schemas-protocol-fbs-cryptodelete"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Crypto Delete when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table CryptoDelete {
  actor:ushort;
}
```

## protocol.fbs::ToolRequest

<a id="schema-schemas-protocol-fbs-toolrequest"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Tool Request when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table ToolRequest {
  verb:string (required);
  arguments_json:[ubyte] (required);
}
```

## protocol.fbs::RequestPayload

<a id="schema-schemas-protocol-fbs-requestpayload"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Request Payload when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
union RequestPayload {
  Append,
  Activate,
  Transcript,
  Recall,
  AsOf,
  Checkpoint,
  LatestCheckpoint,
  Attest,
  Subscribe,
  Health,
  Stats,
  LatencyHistograms,
  VerifyStatus,
  RebuildProjection,
  CryptoDelete,
  ToolRequest
}
```

## protocol.fbs::Request

<a id="schema-schemas-protocol-fbs-request"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Request when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table Request {
  request_id:ulong;
  payload:RequestPayload (required);
}
```

## protocol.fbs::ErrorDetail

<a id="schema-schemas-protocol-fbs-errordetail"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Error Detail when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table ErrorDetail {
  code:ubyte;
  system_error:int;
  lsn:ulong;
  offset:ulong;
  effect_state:MutationEffectState = none;
}
```

## protocol.fbs::AppendAck

<a id="schema-schemas-protocol-fbs-appendack"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Append Ack when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table AppendAck {
  client_seq:ulong;
  first_lsn:ulong;
  last_lsn:ulong;
  duplicate:bool;
  leaf_count:ulong;
  last_leaf_hash:[ubyte];
  mmr_root:[ubyte];
}
```

## protocol.fbs::BytesResult

<a id="schema-schemas-protocol-fbs-bytesresult"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Bytes Result when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table BytesResult {
  bytes:[ubyte] (required);
}
```

## protocol.fbs::SubscriptionAck

<a id="schema-schemas-protocol-fbs-subscriptionack"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Subscription Ack when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table SubscriptionAck {
  subscription_id:ulong;
}
```

## protocol.fbs::HealthResult

<a id="schema-schemas-protocol-fbs-healthresult"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Health Result when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table HealthResult {
  ready:bool;
  actor_count:uint;
  active_connections:uint;
}
```

## protocol.fbs::ProjectionStat

<a id="schema-schemas-protocol-fbs-projectionstat"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Projection Stat when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table ProjectionStat {
  name:string (required);
  applied_lsn:ulong;
}
```

## protocol.fbs::StatsResult

<a id="schema-schemas-protocol-fbs-statsresult"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Stats Result when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table StatsResult {
  actor:ushort;
  log_events:ulong;
  log_bytes:ulong;
  projection_stats:[ProjectionStat] (required);
}
```

## protocol.fbs::LatencyBucket

<a id="schema-schemas-protocol-fbs-latencybucket"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Latency Bucket when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table LatencyBucket {
  operation:string (required);
  upper_bound_ns:ulong;
  count:ulong;
}
```

## protocol.fbs::LatencyResult

<a id="schema-schemas-protocol-fbs-latencyresult"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Latency Result when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table LatencyResult {
  buckets:[LatencyBucket] (required);
}
```

## protocol.fbs::VerifyResult

<a id="schema-schemas-protocol-fbs-verifyresult"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Verify Result when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table VerifyResult {
  actor:ushort;
  root:[ubyte] (required);
  leaf_count:ulong;
  last_checkpoint_lsn:ulong;
  verified:bool;
}
```

## protocol.fbs::RebuildResult

<a id="schema-schemas-protocol-fbs-rebuildresult"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Rebuild Result when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table RebuildResult {
  actor:ushort;
  name:string (required);
  applied_lsn:ulong;
}
```

## protocol.fbs::DeleteResult

<a id="schema-schemas-protocol-fbs-deleteresult"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Delete Result when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table DeleteResult {
  actor:ushort;
  receipt:[ubyte] (required);
}
```

## protocol.fbs::FrameRecord

<a id="schema-schemas-protocol-fbs-framerecord"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Frame Record when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table FrameRecord {
  lsn:ulong;
  kind:ubyte;
  wall_timestamp_ns:long;
  actor:ushort;
  conversation:[ubyte] (required);
  payload:[ubyte] (required);
}
```

## protocol.fbs::TranscriptResult

<a id="schema-schemas-protocol-fbs-transcriptresult"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Transcript Result when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table TranscriptResult {
  records:[FrameRecord] (required);
  truncated:bool;
}
```

## protocol.fbs::TemporalWindowRecord

<a id="schema-schemas-protocol-fbs-temporalwindowrecord"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Temporal Window Record when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table TemporalWindowRecord {
  level:ubyte;
  start_ns:long;
  end_ns:long;
  member_count:ulong;
}
```

## protocol.fbs::RecallResult

<a id="schema-schemas-protocol-fbs-recallresult"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Recall Result when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table RecallResult {
  windows:[TemporalWindowRecord];
  members:[ulong];
}
```

## protocol.fbs::BeliefProvenanceRecord

<a id="schema-schemas-protocol-fbs-beliefprovenancerecord"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Belief Provenance Record when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table BeliefProvenanceRecord {
  first_lsn:ulong;
  last_lsn:ulong;
  byte_start:uint;
  byte_end:uint;
}
```

## protocol.fbs::BeliefConflictRecord

<a id="schema-schemas-protocol-fbs-beliefconflictrecord"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Belief Conflict Record when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table BeliefConflictRecord {
  other_type:ubyte;
  other_canonical_identity:string (required);
  created_lsn:ulong;
  resolved_lsn:ulong;
  obligated_surfacing:bool;
}
```

## protocol.fbs::BeliefResult

<a id="schema-schemas-protocol-fbs-beliefresult"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Belief Result when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table BeliefResult {
  present:bool;
  belief_type:ubyte;
  belief_id:[ubyte];
  canonical_identity:string;
  conflict_domain:string;
  value:[ubyte];
  claim:ubyte;
  valid_from_ns:long;
  valid_to_ns:long;
  transaction_lsn:ulong;
  version:ulong;
  supersedes_version:ulong;
  provenance:[BeliefProvenanceRecord];
  conflict_edges:[BeliefConflictRecord];
  tombstoned:bool;
}
```

## protocol.fbs::CheckpointAck

<a id="schema-schemas-protocol-fbs-checkpointack"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Checkpoint Ack when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table CheckpointAck {
  lsn:ulong;
}
```

## protocol.fbs::CheckpointResult

<a id="schema-schemas-protocol-fbs-checkpointresult"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Checkpoint Result when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table CheckpointResult {
  present:bool;
  lsn:ulong;
  blob:[ubyte];
}
```

## protocol.fbs::AttestAck

<a id="schema-schemas-protocol-fbs-attestack"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Attest Ack when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table AttestAck {
  first_lsn:ulong;
  last_lsn:ulong;
  count:uint;
}
```

## protocol.fbs::ResponsePayload

<a id="schema-schemas-protocol-fbs-responsepayload"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Response Payload when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
union ResponsePayload {
  ErrorDetail,
  AppendAck,
  BytesResult,
  SubscriptionAck,
  HealthResult,
  StatsResult,
  LatencyResult,
  VerifyResult,
  RebuildResult,
  DeleteResult,
  TranscriptResult,
  RecallResult,
  BeliefResult,
  CheckpointAck,
  CheckpointResult,
  AttestAck
}
```

## protocol.fbs::Response

<a id="schema-schemas-protocol-fbs-response"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Response when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table Response {
  request_id:ulong;
  status:ResponseStatus;
  payload:ResponsePayload;
}
```

## protocol.fbs::Event

<a id="schema-schemas-protocol-fbs-event"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Event when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table Event {
  subscription_id:ulong;
  lsn:ulong;
  kind:ubyte;
  wall_timestamp_ns:long;
  actor:ushort;
  conversation:[ubyte] (required);
  payload:[ubyte] (required);
}
```

## protocol.fbs::WirePayload

<a id="schema-schemas-protocol-fbs-wirepayload"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Wire Payload when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
union WirePayload {
  Hello,
  Welcome,
  Request,
  Response,
  Event
}
```

## protocol.fbs::WireEnvelope

<a id="schema-schemas-protocol-fbs-wireenvelope"></a>

Source: [`schemas/protocol.fbs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/protocol.fbs).

When to use: Use Wire Envelope when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```text
table WireEnvelope {
  proto_version:ushort;
  payload:WirePayload (required);
}
```

## hypermind.proto::HyperMind

<a id="schema-schemas-hypermind-proto-hypermind"></a>

Source: [`schemas/hypermind.proto`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/hypermind.proto).

When to use: Use Hyper Mind when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```protobuf
service HyperMind {
  // Validates Hello and returns Welcome, including the durable next_client_seq.
  rpc Connect(Envelope) returns (Envelope);
  // Reuses the Hello connection_id for durable idempotency across RPCs.
  rpc Exchange(ExchangeRequest) returns (Envelope);
  // The request must contain Subscribe. First response is SubscriptionAck,
  // followed by unchanged NCPR Event envelopes. Resume using the last LSN.
  // A saturated bounded queue terminates with RESOURCE_EXHAUSTED.
  rpc Subscribe(ExchangeRequest) returns (stream Envelope);
}
```

## hypermind.proto::Envelope

<a id="schema-schemas-hypermind-proto-envelope"></a>

Source: [`schemas/hypermind.proto`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/hypermind.proto).

When to use: Use Envelope when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```protobuf
message Envelope {
  bytes ncpr = 1;
}
```

## hypermind.proto::ExchangeRequest

<a id="schema-schemas-hypermind-proto-exchangerequest"></a>

Source: [`schemas/hypermind.proto`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/schemas/hypermind.proto).

When to use: Use Exchange Request when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.

Do not use: Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.


```protobuf
message ExchangeRequest {
  bytes hello = 1;
  bytes request = 2;
}
```
