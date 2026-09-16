# Rust public type catalog

Generated from authored `pub struct`, `enum`, `trait`, and `type` declarations in every crate’s `src` directory, including macro-defined core identifiers. FlatBuffers-generated builders, offsets, and object wrappers are represented by their canonical definitions in the schema catalog rather than duplicated here. Public declarations in internal modules are included conservatively; this catalog does not promise every path is a stable external API.

## hm-capi::HmEngine

<a id="rust-crates-hm-capi-src-engine-rs-hmengine"></a>

Source: [`crates/hm-capi/src/engine.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-capi/src/engine.rs).

When to use: Use `HmEngine` for the C application binary interface over an embedded actor: opening a handle from sealed identity material, sharing it by reference count, and shutting it down deterministically.

Do not use: Do not open one actor directory from a second owner, free a handle while a call is in flight, retain a borrowed callback string after the callback returns, or treat a returned envelope as proof of an external effect.


```rust
pub struct HmEngine {
    pub(crate) inner: Arc<EngineState>,
}
```

## hm-capi::HmStatus

<a id="rust-crates-hm-capi-src-status-rs-hmstatus"></a>

Source: [`crates/hm-capi/src/status.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-capi/src/status.rs).

When to use: Use `HmStatus` for the C application binary interface over an embedded actor: opening a handle from sealed identity material, sharing it by reference count, and shutting it down deterministically.

Do not use: Do not open one actor directory from a second owner, free a handle while a call is in flight, retain a borrowed callback string after the callback returns, or treat a returned envelope as proof of an external effect.


```rust
pub enum HmStatus {
    Ok = 0,
    NullPointer = 1,
    InvalidUtf8 = 2,
    InvalidArgument = 3,
    HandleClosed = 4,
    Runtime = 5,
    Panic = 6,
    Kernel = 7,
}
```

## hm-cli::Command

<a id="rust-crates-hm-cli-src-actors-rs-command"></a>

Source: [`crates/hm-cli/src/actors.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cli/src/actors.rs).

When to use: Use `Command` for command-line orchestration and validated migration into the actor ledger.

Do not use: Do not run embedded import against a live writer, overwrite original migration data, or claim verification without the command report.


```rust
pub enum Command {
    Add {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        actor: u16,
    },
    List {
        #[arg(long)]
        config: PathBuf,
    },
    RotateKeys {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        new_kek_file: PathBuf,
    },
    Shred {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        actor: u16,
        #[arg(long)]
        confirm: u16,
    },
}
```

## hm-cli::ArchiveMember

<a id="rust-crates-hm-cli-src-archive-rs-archivemember"></a>

Source: [`crates/hm-cli/src/archive.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cli/src/archive.rs).

When to use: Use `ArchiveMember` for command-line orchestration and validated migration into the actor ledger.

Do not use: Do not run embedded import against a live writer, overwrite original migration data, or claim verification without the command report.


```rust
pub struct ArchiveMember {
    pub name: String,
    pub bytes: u64,
    pub digest: String,
}
```

## hm-cli::ArchiveManifest

<a id="rust-crates-hm-cli-src-archive-rs-archivemanifest"></a>

Source: [`crates/hm-cli/src/archive.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cli/src/archive.rs).

When to use: Use `ArchiveManifest` for command-line orchestration and validated migration into the actor ledger.

Do not use: Do not run embedded import against a live writer, overwrite original migration data, or claim verification without the command report.


```rust
pub struct ArchiveManifest {
    pub format: String,
    pub archive_version: u16,
    pub actor: u16,
    pub events: u64,
    pub created_ns: i64,
    pub members: Vec<ArchiveMember>,
    pub public_key: String,
    pub signature: String,
}
```

## hm-cli::Command

<a id="rust-crates-hm-cli-src-archive-rs-command"></a>

Source: [`crates/hm-cli/src/archive.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cli/src/archive.rs).

When to use: Use `Command` for command-line orchestration and validated migration into the actor ledger.

Do not use: Do not run embedded import against a live writer, overwrite original migration data, or claim verification without the command report.


```rust
pub enum Command {
    Pack {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Verify {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        public_key: Option<String>,
    },
}
```

## hm-cli::Command

<a id="rust-crates-hm-cli-src-consolidate-rs-command"></a>

Source: [`crates/hm-cli/src/consolidate.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cli/src/consolidate.rs).

When to use: Use `Command` for command-line orchestration and validated migration into the actor ledger.

Do not use: Do not run embedded import against a live writer, overwrite original migration data, or claim verification without the command report.


```rust
pub enum Command {
    Run {
        #[arg(long)]
        config: PathBuf,
        #[arg(long, value_enum)]
        mode: Mode,
        #[arg(long, default_value = "actor")]
        scope: String,
        #[arg(long)]
        cadence_key: String,
        #[arg(long)]
        max_llm_calls: u64,
        #[arg(long)]
        max_tokens: u64,
        #[arg(long)]
        max_microusd: u64,
        #[arg(long)]
        max_wall_ms: u64,
    },
    List {
        #[arg(long)]
        config: PathBuf,
    },
    Retract {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        reason: String,
    },
}
```

## hm-cli::Mode

<a id="rust-crates-hm-cli-src-consolidate-rs-mode"></a>

Source: [`crates/hm-cli/src/consolidate.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cli/src/consolidate.rs).

When to use: Use `Mode` for command-line orchestration and validated migration into the actor ledger.

Do not use: Do not run embedded import against a live writer, overwrite original migration data, or claim verification without the command report.


```rust
pub enum Mode {
    Nrem,
    Rem,
    Both,
}
```

## hm-cli::Choice

<a id="rust-crates-hm-cli-src-models-rs-choice"></a>

Source: [`crates/hm-cli/src/models.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cli/src/models.rs).

When to use: Use `Choice` for command-line orchestration and validated migration into the actor ledger.

Do not use: Do not run embedded import against a live writer, overwrite original migration data, or claim verification without the command report.


```rust
pub enum Choice {
    BgeSmall,
    Nomic,
}
```

## hm-cli::Command

<a id="rust-crates-hm-cli-src-models-rs-command"></a>

Source: [`crates/hm-cli/src/models.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cli/src/models.rs).

When to use: Use `Command` for command-line orchestration and validated migration into the actor ledger.

Do not use: Do not run embedded import against a live writer, overwrite original migration data, or claim verification without the command report.


```rust
pub enum Command {
    Ensure {
        #[arg(long)]
        directory: PathBuf,
        #[arg(long, value_enum)]
        model: Choice,
    },
    List {
        #[arg(long)]
        directory: PathBuf,
    },
}
```

## hm-cli::TuiOptions

<a id="rust-crates-hm-cli-src-operations-rs-tuioptions"></a>

Source: [`crates/hm-cli/src/operations.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cli/src/operations.rs).

When to use: Use `TuiOptions` for command-line orchestration and validated migration into the actor ledger. Set explicit deployment limits before opening the associated resource.

Do not use: Do not run embedded import against a live writer, overwrite original migration data, or claim verification without the command report. Do not log secret fields or substitute defaults for an explicitly authorized budget.


```rust
pub struct TuiOptions<'a> {
    pub config: &'a Path,
    pub conversation: &'a str,
    pub query: &'a str,
    pub budget_tokens: usize,
    pub interval_ms: u64,
    pub frames: Option<usize>,
    pub json: bool,
}
```

## hm-compose::BudgetProfile

<a id="rust-crates-hm-compose-src-budget-rs-budgetprofile"></a>

Source: [`crates/hm-compose/src/budget.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/budget.rs).

When to use: Use `BudgetProfile` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct BudgetProfile {
    pub mandatory_percent: u8,
    pub intent_bindings_work_percent: u8,
    pub conversation_percent: u8,
    pub recall_percent: u8,
    pub tools_percent: u8,
    pub reserve_percent: u8,
}
```

## hm-compose::BudgetAllocation

<a id="rust-crates-hm-compose-src-budget-rs-budgetallocation"></a>

Source: [`crates/hm-compose/src/budget.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/budget.rs).

When to use: Use `BudgetAllocation` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct BudgetAllocation {
    pub mandatory: usize,
    pub intent_bindings_work: usize,
    pub conversation: usize,
    pub recall: usize,
    pub tools: usize,
    pub reserve: usize,
}
```

## hm-compose::Tier

<a id="rust-crates-hm-compose-src-bundle-rs-tier"></a>

Source: [`crates/hm-compose/src/bundle.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/bundle.rs).

When to use: Use `Tier` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub enum Tier {
    Resident,
    Intent,
    Bindings,
    WorkLedger,
    Prospective,
    Conversation,
    Entity,
    Conflicts,
    Fused,
    Temporal,
}
```

## hm-compose::RetrievalLane

<a id="rust-crates-hm-compose-src-bundle-rs-retrievallane"></a>

Source: [`crates/hm-compose/src/bundle.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/bundle.rs).

When to use: Use `RetrievalLane` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub enum RetrievalLane {
    Lexical,
    Vector,
    Entity,
    Temporal,
    Graph,
    Belief,
    Timeline,
    Reconstruct,
    Relation,
}
```

## hm-compose::WhyCode

<a id="rust-crates-hm-compose-src-bundle-rs-whycode"></a>

Source: [`crates/hm-compose/src/bundle.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/bundle.rs).

When to use: Use `WhyCode` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub enum WhyCode {
    Conversation,
    Lexical,
    Vector,
    Entity,
    Temporal,
    Fused,
    Intent,
    Binding,
    WorkLedger,
    Belief,
    Conflict,
    Prospective,
    Procedure,
    Relation,
}
```

## hm-compose::ActivationRequest

<a id="rust-crates-hm-compose-src-bundle-rs-activationrequest"></a>

Source: [`crates/hm-compose/src/bundle.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/bundle.rs).

When to use: Use `ActivationRequest` for planning, combining, budgeting, and safely rendering provenance-bearing activation context. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct ActivationRequest<'model> {
    pub actor: ActorId,
    pub conversation: ConversationId,
    pub query: String,
    pub turn_text: String,
    pub budget_tokens: usize,
    pub token_counter: &'model TokenCounter,
    pub maximum_candidates: usize,
    pub maximum_conversation_records: usize,
}
```

## hm-compose::ActivationContext

<a id="rust-crates-hm-compose-src-bundle-rs-activationcontext"></a>

Source: [`crates/hm-compose/src/bundle.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/bundle.rs).

When to use: Use `ActivationContext` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct ActivationContext {
    pub task: Option<Vec<u8>>,
    pub required_bindings: Vec<hm_proj::bindings::BindingRequirement>,
    pub now_ns: Option<UtcNanos>,
    pub budget_profile: BudgetProfile,
}
```

## hm-compose::ActivationItem

<a id="rust-crates-hm-compose-src-bundle-rs-activationitem"></a>

Source: [`crates/hm-compose/src/bundle.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/bundle.rs).

When to use: Use `ActivationItem` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct ActivationItem {
    pub tier: Tier,
    pub uri: String,
    pub provenance: Vec<LSN>,
    pub content: Vec<u8>,
    pub authority: Authority,
    pub tokens: usize,
    pub coarsened: bool,
    pub vector_rank: u32,
    pub lexical_rank: u32,
    pub why: WhyCode,
}
```

## hm-compose::ActivationSection

<a id="rust-crates-hm-compose-src-bundle-rs-activationsection"></a>

Source: [`crates/hm-compose/src/bundle.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/bundle.rs).

When to use: Use `ActivationSection` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct ActivationSection {
    pub tier: Tier,
    pub required: bool,
    pub items: Vec<ActivationItem>,
    pub tokens: usize,
    pub trimmed_items: usize,
    pub coarsened_items: usize,
}
```

## hm-compose::GapKind

<a id="rust-crates-hm-compose-src-bundle-rs-gapkind"></a>

Source: [`crates/hm-compose/src/bundle.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/bundle.rs).

When to use: Use `GapKind` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub enum GapKind {
    TruncatedLane,
    DroppedTier,
    NarrowedSubtask,
    MissingBinding,
    StaleBinding,
    ConflictingBinding,
    IndexLag,
    PendingProtectedProposal,
}
```

## hm-compose::Gap

<a id="rust-crates-hm-compose-src-bundle-rs-gap"></a>

Source: [`crates/hm-compose/src/bundle.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/bundle.rs).

When to use: Use `Gap` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct Gap {
    pub kind: GapKind,
    pub tier: Option<Tier>,
    pub lane: Option<RetrievalLane>,
    pub detail: String,
}
```

## hm-compose::HealthStatus

<a id="rust-crates-hm-compose-src-bundle-rs-healthstatus"></a>

Source: [`crates/hm-compose/src/bundle.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/bundle.rs).

When to use: Use `HealthStatus` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub enum HealthStatus {
    SemanticReady,
    SemanticLagging,
    LexicalOnly,
    Unavailable,
}
```

## hm-compose::BundleHealth

<a id="rust-crates-hm-compose-src-bundle-rs-bundlehealth"></a>

Source: [`crates/hm-compose/src/bundle.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/bundle.rs).

When to use: Use `BundleHealth` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct BundleHealth {
    pub encoder: HealthStatus,
    pub backlog: HealthStatus,
    pub projection: HealthStatus,
    pub inclusion: HealthStatus,
}
```

## hm-compose::RetrievalManifest

<a id="rust-crates-hm-compose-src-bundle-rs-retrievalmanifest"></a>

Source: [`crates/hm-compose/src/bundle.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/bundle.rs).

When to use: Use `RetrievalManifest` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct RetrievalManifest {
    pub manifest_id: [u8; 32],
    pub query_digest: [u8; 32],
    pub snapshot_epoch: u64,
    pub encoder: String,
    pub index_generation: u64,
    pub candidate_lanes: Vec<RetrievalLane>,
    pub candidates: Vec<LSN>,
    pub selected: Vec<LSN>,
    pub included: Vec<LSN>,
    pub used: Vec<LSN>,
}
```

## hm-compose::ActivationBundle

<a id="rust-crates-hm-compose-src-bundle-rs-activationbundle"></a>

Source: [`crates/hm-compose/src/bundle.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/bundle.rs).

When to use: Use `ActivationBundle` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct ActivationBundle {
    pub snapshot_epoch: u64,
    pub budget_tokens: usize,
    pub spent_tokens: usize,
    pub sections: [ActivationSection; 10],
    pub manifest: RetrievalManifest,
    pub gaps: Vec<Gap>,
    pub health: BundleHealth,
    pub bundle_hash: [u8; 32],
}
```

## hm-compose::AttestationRequest

<a id="rust-crates-hm-compose-src-bundle-rs-attestationrequest"></a>

Source: [`crates/hm-compose/src/bundle.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/bundle.rs).

When to use: Use `AttestationRequest` for planning, combining, budgeting, and safely rendering provenance-bearing activation context. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct AttestationRequest {
    pub first_lsn: LSN,
    pub actor: ActorId,
    pub conversation: ConversationId,
    pub wall_timestamp_ns: UtcNanos,
    pub disposition: AttestationDisposition,
}
```

## hm-compose::DeadlineBundle

<a id="rust-crates-hm-compose-src-deadline-rs-deadlinebundle"></a>

Source: [`crates/hm-compose/src/deadline.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/deadline.rs).

When to use: Use `DeadlineBundle` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct DeadlineBundle {
    pub bundle: ActivationBundle,
    pub degraded: bool,
    pub stale_by_lsn: u64,
}
```

## hm-compose::DeadlineActivator

<a id="rust-crates-hm-compose-src-deadline-rs-deadlineactivator"></a>

Source: [`crates/hm-compose/src/deadline.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/deadline.rs).

When to use: Use `DeadlineActivator` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct DeadlineActivator {
    shared: Arc<Shared>,
}
```

## hm-compose::RankedCandidate

<a id="rust-crates-hm-compose-src-fusion-rs-rankedcandidate"></a>

Source: [`crates/hm-compose/src/fusion.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/fusion.rs).

When to use: Use `RankedCandidate` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct RankedCandidate {
    pub canonical_id: Vec<u8>,
    pub lsn: LSN,
    pub fsrs_retrievability_q16: u32,
    pub salience_q16: u32,
    pub recency_q16: u32,
}
```

## hm-compose::LaneRanking

<a id="rust-crates-hm-compose-src-fusion-rs-laneranking"></a>

Source: [`crates/hm-compose/src/fusion.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/fusion.rs).

When to use: Use `LaneRanking` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct LaneRanking {
    pub lane: RetrievalLane,
    pub weight_q16: u32,
    pub candidates: Vec<RankedCandidate>,
}
```

## hm-compose::FusedHit

<a id="rust-crates-hm-compose-src-fusion-rs-fusedhit"></a>

Source: [`crates/hm-compose/src/fusion.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/fusion.rs).

When to use: Use `FusedHit` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct FusedHit {
    pub canonical_id: Vec<u8>,
    pub lsn: LSN,
    pub score_q32: u64,
    pub lane_ranks: BTreeMap<RetrievalLane, u32>,
    pub why: WhyCode,
    pub uri: String,
    pub preference_q16: u32,
}
```

## hm-compose::GeometryConfig

<a id="rust-crates-hm-compose-src-geometry-rs-geometryconfig"></a>

Source: [`crates/hm-compose/src/geometry.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/geometry.rs).

When to use: Use `GeometryConfig` for planning, combining, budgeting, and safely rendering provenance-bearing activation context. Set explicit deployment limits before opening the associated resource.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions. Do not log secret fields or substitute defaults for an explicitly authorized budget.


```rust
pub struct GeometryConfig {
    pub enabled: bool,
    pub deployment_id: String,
    pub version: u16,
    pub evidence: Option<GeometryEvidence>,
}
```

## hm-compose::GeometryEvidence

<a id="rust-crates-hm-compose-src-geometry-rs-geometryevidence"></a>

Source: [`crates/hm-compose/src/geometry.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/geometry.rs).

When to use: Use `GeometryEvidence` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct GeometryEvidence {
    pub producer: String,
    pub deployment_id: String,
    pub version: u16,
    pub evaluation_id: String,
    pub evaluated_queries: u64,
    pub baseline_mrr_at_10: f64,
    pub boosted_mrr_at_10: f64,
}
```

## hm-compose::GeometryError

<a id="rust-crates-hm-compose-src-geometry-rs-geometryerror"></a>

Source: [`crates/hm-compose/src/geometry.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/geometry.rs).

When to use: Use `GeometryError` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub enum GeometryError {
    UnqualifiedDeployment,
    InvalidArgument,
    DimensionMismatch,
    CapacityExceeded,
}
```

## hm-compose::AlignmentReport

<a id="rust-crates-hm-compose-src-geometry-rs-alignmentreport"></a>

Source: [`crates/hm-compose/src/geometry.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/geometry.rs).

When to use: Use `AlignmentReport` for planning, combining, budgeting, and safely rendering provenance-bearing activation context. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct AlignmentReport {
    pub applied: usize,
    pub neutral: usize,
    pub minimum_factor_q16: i64,
    pub maximum_factor_q16: i64,
}
```

## hm-compose::AlignmentBoost

<a id="rust-crates-hm-compose-src-geometry-rs-alignmentboost"></a>

Source: [`crates/hm-compose/src/geometry.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/geometry.rs).

When to use: Use `AlignmentBoost` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct AlignmentBoost {
    enabled: bool,
}
```

## hm-compose::EncoderState

<a id="rust-crates-hm-compose-src-health-rs-encoderstate"></a>

Source: [`crates/hm-compose/src/health.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/health.rs).

When to use: Use `EncoderState` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub enum EncoderState {
    Unconfigured,
    Loading,
    Ready,
}
```

## hm-compose::HealthInput

<a id="rust-crates-hm-compose-src-health-rs-healthinput"></a>

Source: [`crates/hm-compose/src/health.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/health.rs).

When to use: Use `HealthInput` for planning, combining, budgeting, and safely rendering provenance-bearing activation context. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct HealthInput {
    pub encoder: EncoderState,
    pub embedding_backlog: usize,
    pub projection_lsn: LSN,
    pub ledger_lsn: LSN,
    pub semantic_included: bool,
    pub lexical_included: bool,
}
```

## hm-compose::AsOfRecall

<a id="rust-crates-hm-compose-src-lanes-belief-rs-asofrecall"></a>

Source: [`crates/hm-compose/src/lanes/belief.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/lanes/belief.rs).

When to use: Use `AsOfRecall` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct AsOfRecall {
    pub axis: BeliefAsOfAxis,
    pub record: Option<BeliefRecord>,
}
```

## hm-compose::TimelineRecall

<a id="rust-crates-hm-compose-src-lanes-belief-rs-timelinerecall"></a>

Source: [`crates/hm-compose/src/lanes/belief.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/lanes/belief.rs).

When to use: Use `TimelineRecall` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct TimelineRecall {
    pub windows: Vec<TemporalWindow>,
    pub members: Vec<LSN>,
}
```

## hm-compose::GraphSeed

<a id="rust-crates-hm-compose-src-lanes-graph-rs-graphseed"></a>

Source: [`crates/hm-compose/src/lanes/graph.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/lanes/graph.rs).

When to use: Use `GraphSeed` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct GraphSeed {
    pub node_id: Vec<u8>,
    pub source_lsn: LSN,
}
```

## hm-compose::RelationHit

<a id="rust-crates-hm-compose-src-lanes-relation-rs-relationhit"></a>

Source: [`crates/hm-compose/src/lanes/relation.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/lanes/relation.rs).

When to use: Use `RelationHit` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct RelationHit {
    pub edge_id: Vec<u8>,
    pub event_lsn: LSN,
    pub weight_micros: u32,
    pub score: i64,
    pub support_lsns: Vec<LSN>,
}
```

## hm-compose::RelationRanking

<a id="rust-crates-hm-compose-src-lanes-relation-rs-relationranking"></a>

Source: [`crates/hm-compose/src/lanes/relation.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/lanes/relation.rs).

When to use: Use `RelationRanking` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct RelationRanking {
    pub ranking: LaneRanking,
    pub dropped: usize,
}
```

## hm-compose::TemporalCandidate

<a id="rust-crates-hm-compose-src-lanes-temporal-rs-temporalcandidate"></a>

Source: [`crates/hm-compose/src/lanes/temporal.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/lanes/temporal.rs).

When to use: Use `TemporalCandidate` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct TemporalCandidate {
    pub canonical_id: Vec<u8>,
    pub lsn: LSN,
    pub event_time_ns: UtcNanos,
}
```

## hm-compose::RecallMode

<a id="rust-crates-hm-compose-src-planner-rs-recallmode"></a>

Source: [`crates/hm-compose/src/planner.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/planner.rs).

When to use: Use `RecallMode` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub enum RecallMode {
    Semantic,
    Lexical,
    Entity,
    Temporal,
    Graph,
    AsOf,
    Timeline,
    Reconstruct,
    Near,
    Relation,
}
```

## hm-compose::QueryShape

<a id="rust-crates-hm-compose-src-planner-rs-queryshape"></a>

Source: [`crates/hm-compose/src/planner.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/planner.rs).

When to use: Use `QueryShape` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub enum QueryShape {
    Identifier,
    Temporal,
    Explanatory,
    Semantic,
    Anchored,
}
```

## hm-compose::AnchorFacet

<a id="rust-crates-hm-compose-src-planner-rs-anchorfacet"></a>

Source: [`crates/hm-compose/src/planner.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/planner.rs).

When to use: Use `AnchorFacet` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub enum AnchorFacet {
    Path,
    Symbol,
    Url,
    Entity,
}
```

## hm-compose::Anchor

<a id="rust-crates-hm-compose-src-planner-rs-anchor"></a>

Source: [`crates/hm-compose/src/planner.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/planner.rs).

When to use: Use `Anchor` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct Anchor {
    pub facet: AnchorFacet,
    pub value: String,
}
```

## hm-compose::LanePlan

<a id="rust-crates-hm-compose-src-planner-rs-laneplan"></a>

Source: [`crates/hm-compose/src/planner.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/planner.rs).

When to use: Use `LanePlan` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct LanePlan {
    pub lane: RetrievalLane,
    pub weight_q16: u32,
}
```

## hm-compose::QueryPlan

<a id="rust-crates-hm-compose-src-planner-rs-queryplan"></a>

Source: [`crates/hm-compose/src/planner.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/planner.rs).

When to use: Use `QueryPlan` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct QueryPlan {
    pub shape: QueryShape,
    pub lanes: Vec<LanePlan>,
}
```

## hm-compose::PreferenceProfile

<a id="rust-crates-hm-compose-src-preference-rs-preferenceprofile"></a>

Source: [`crates/hm-compose/src/preference.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/preference.rs).

When to use: Use `PreferenceProfile` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct PreferenceProfile {
    weights: BTreeMap<u64, u32>,
}
```

## hm-compose::ReconstructionAnchor

<a id="rust-crates-hm-compose-src-reconstruct-rs-reconstructionanchor"></a>

Source: [`crates/hm-compose/src/reconstruct.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/reconstruct.rs).

When to use: Use `ReconstructionAnchor` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct ReconstructionAnchor {
    pub lsn: LSN,
    pub uri: String,
    pub content: String,
    pub authority: Authority,
}
```

## hm-compose::Reconstruction

<a id="rust-crates-hm-compose-src-reconstruct-rs-reconstruction"></a>

Source: [`crates/hm-compose/src/reconstruct.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/reconstruct.rs).

When to use: Use `Reconstruction` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct Reconstruction {
    pub content: String,
    pub authority: Authority,
    pub anchor_lsns: Vec<LSN>,
    pub model_id: String,
    pub prompt_id: &'static str,
    pub usage: Usage,
}
```

## hm-compose::DeploymentConfig

<a id="rust-crates-hm-compose-src-rerank-rs-deploymentconfig"></a>

Source: [`crates/hm-compose/src/rerank.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/rerank.rs).

When to use: Use `DeploymentConfig` for planning, combining, budgeting, and safely rendering provenance-bearing activation context. Set explicit deployment limits before opening the associated resource.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions. Do not log secret fields or substitute defaults for an explicitly authorized budget.


```rust
pub struct DeploymentConfig {
    pub enabled: bool,
    pub deployment_id: String,
    pub evidence: Option<EvaluationEvidence>,
}
```

## hm-compose::EvaluationEvidence

<a id="rust-crates-hm-compose-src-rerank-rs-evaluationevidence"></a>

Source: [`crates/hm-compose/src/rerank.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/rerank.rs).

When to use: Use `EvaluationEvidence` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct EvaluationEvidence {
    pub producer: String,
    pub deployment_id: String,
    pub model_sha256: String,
    pub tokenizer_sha256: String,
    pub evaluation_id: String,
    pub evaluated_queries: u64,
    pub baseline_mrr_at_10: f64,
    pub reranked_mrr_at_10: f64,
}
```

## hm-compose::RerankError

<a id="rust-crates-hm-compose-src-rerank-rs-rerankerror"></a>

Source: [`crates/hm-compose/src/rerank.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/rerank.rs).

When to use: Use `RerankError` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub enum RerankError {
    UnqualifiedDeployment,
    InvalidArgument,
    Artifact(String),
    DigestMismatch,
    Tokenizer(String),
    Runtime(String),
    Shape,
}
```

## hm-compose::DeploymentReranker

<a id="rust-crates-hm-compose-src-rerank-rs-deploymentreranker"></a>

Source: [`crates/hm-compose/src/rerank.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/rerank.rs).

When to use: Use `DeploymentReranker` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct DeploymentReranker {
    model: Option<CrossEncoder>,
}
```

## hm-compose::CrossEncoder

<a id="rust-crates-hm-compose-src-rerank-rs-crossencoder"></a>

Source: [`crates/hm-compose/src/rerank.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/rerank.rs).

When to use: Use `CrossEncoder` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct CrossEncoder {
    tokenizer: Tokenizer,
    session: Mutex<Session>,
}
```

## hm-compose::SafeContent

<a id="rust-crates-hm-compose-src-safety-rs-safecontent"></a>

Source: [`crates/hm-compose/src/safety.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/safety.rs).

When to use: Use `SafeContent` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct SafeContent {
    pub bytes: Vec<u8>,
    pub authority: Authority,
}
```

## hm-compose::PromptItem

<a id="rust-crates-hm-compose-src-safety-rs-promptitem"></a>

Source: [`crates/hm-compose/src/safety.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/safety.rs).

When to use: Use `PromptItem` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct PromptItem {
    pub tier: Tier,
    pub role: &'static str,
    pub authority: Authority,
    pub provenance_uri: String,
    pub provenance: Vec<LSN>,
    pub content: String,
}
```

## hm-compose::PromptSection

<a id="rust-crates-hm-compose-src-safety-rs-promptsection"></a>

Source: [`crates/hm-compose/src/safety.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/safety.rs).

When to use: Use `PromptSection` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct PromptSection {
    pub tier: Tier,
    pub items: Vec<PromptItem>,
}
```

## hm-compose::RenderedPrompt

<a id="rust-crates-hm-compose-src-safety-rs-renderedprompt"></a>

Source: [`crates/hm-compose/src/safety.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/safety.rs).

When to use: Use `RenderedPrompt` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct RenderedPrompt {
    pub sections: Vec<PromptSection>,
    pub bundle_hash: [u8; 32],
}
```

## hm-compose::ConflictTier

<a id="rust-crates-hm-compose-src-tiers-conflicts-rs-conflicttier"></a>

Source: [`crates/hm-compose/src/tiers/conflicts.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/tiers/conflicts.rs).

When to use: Use `ConflictTier` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct ConflictTier {
    pub items: Vec<ActivationItem>,
    pub gaps: Vec<Gap>,
}
```

## hm-compose::FallbackWeights

<a id="rust-crates-hm-compose-src-tokens-rs-fallbackweights"></a>

Source: [`crates/hm-compose/src/tokens.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/tokens.rs).

When to use: Use `FallbackWeights` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub struct FallbackWeights {
    pub per_byte_q8: [u16; 256],
    pub item_overhead: u32,
}
```

## hm-compose::TokenCounter

<a id="rust-crates-hm-compose-src-tokens-rs-tokencounter"></a>

Source: [`crates/hm-compose/src/tokens.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-compose/src/tokens.rs).

When to use: Use `TokenCounter` for planning, combining, budgeting, and safely rendering provenance-bearing activation context.

Do not use: Do not shed required bindings silently, omit degraded health, or render memory into system/developer instructions.


```rust
pub enum TokenCounter {
    Tiktoken {
        model_id: String,
    },
    HuggingFace {
        model_id: String,
        tokenizer: Box<Tokenizer>,
    },
    Fallback {
        model_id: String,
        weights: Box<FallbackWeights>,
    },
}
```

## hm-core::ErrorCode

<a id="rust-crates-hm-core-src-error-rs-errorcode"></a>

Source: [`crates/hm-core/src/error.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-core/src/error.rs).

When to use: Use `ErrorCode` for typed identities, revisions, timestamps, and stable error handling across kernel boundaries.

Do not use: Do not interchange actor, conversation, entity, and log identities or interpret wall time as ledger order.


```rust
pub enum ErrorCode {
    InvalidArgument = 0,
    InvalidLength = 1,
    InvalidKind = 2,
    SequenceViolation = 3,
    OpenFailed = 4,
    ReadFailed = 5,
    WriteFailed = 6,
    SyncFailed = 7,
    CloseFailed = 8,
    Truncated = 9,
    ChecksumMismatch = 10,
    InteriorCorruption = 11,
    ManifestCorrupt = 12,
    BackendUnavailable = 13,
    SegmentFull = 14,
    ProofInvalid = 15,
    SignatureInvalid = 16,
    CheckpointMismatch = 17,
    AlreadyExists = 18,
    WriterViolation = 19,
    ProcessKilled = 20,
    DurabilityFailure = 21,
    ProjectionCheckpoint = 22,
    MapFull = 23,
    CryptoAuthentication = 24,
    KeyDestroyed = 25,
    LegacyPlaintext = 26,
    SchemaInvalid = 27,
    SchemaVersion = 28,
    ForbiddenKind = 29,
    OrderingViolation = 30,
    BeliefWriteGate = 31,
    BeliefIdConflict = 32,
    BeliefNotFound = 33,
    BeliefCorrupt = 34,
    NegativeExistenceUncorroborated = 35,
    EntityIndexCorrupt = 36,
    VectorIndexCorrupt = 37,
    LexicalIndexCorrupt = 38,
    TemporalLadderCorrupt = 39,
    IntentFrameCorrupt = 40,
    LoopNotFound = 41,
    WorkLedgerCorrupt = 42,
    WorkItemNotFound = 43,
    InvariantViolation = 44,
    ProtocolInvalid = 45,
    ProtocolVersion = 46,
    CapabilityDenied = 47,
    CapacityExceeded = 48,
    OperationUnavailable = 49,
    IdempotencyConflict = 50,
    ProtectedTypeWrite = 51,
    CitationInvalid = 52,
    DeadlineMissed = 53,
    Tripwire = 54,
}
```

## hm-core::Error

<a id="rust-crates-hm-core-src-error-rs-error"></a>

Source: [`crates/hm-core/src/error.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-core/src/error.rs).

When to use: Use `Error` for typed identities, revisions, timestamps, and stable error handling across kernel boundaries.

Do not use: Do not interchange actor, conversation, entity, and log identities or interpret wall time as ledger order.


```rust
pub struct Error {
    pub code: ErrorCode,
    pub system_error: i32,
    pub lsn: LSN,
    pub offset: u64,
}
```

## hm-core::LSN

<a id="rust-crates-hm-core-src-ids-rs-lsn"></a>

Source: [`crates/hm-core/src/ids.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-core/src/ids.rs).

When to use: Use `LSN` for typed identities, revisions, timestamps, and stable error handling across kernel boundaries.

Do not use: Do not interchange actor, conversation, entity, and log identities or interpret wall time as ledger order.


```rust
pub type LSN = Lsn;
```

## hm-core::Lsn

<a id="rust-crates-hm-core-src-ids-rs-lsn"></a>

Source: [`crates/hm-core/src/ids.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-core/src/ids.rs).

When to use: Use `Lsn` for typed identities, revisions, timestamps, and stable error handling across kernel boundaries.

Do not use: Do not interchange actor, conversation, entity, and log identities or interpret wall time as ledger order.


```rust
pub struct Lsn(u64);
```

## hm-core::ActorId

<a id="rust-crates-hm-core-src-ids-rs-actorid"></a>

Source: [`crates/hm-core/src/ids.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-core/src/ids.rs).

When to use: Use `ActorId` for typed identities, revisions, timestamps, and stable error handling across kernel boundaries.

Do not use: Do not interchange actor, conversation, entity, and log identities or interpret wall time as ledger order.


```rust
pub struct ActorId(u16);
```

## hm-core::SchemaVersion

<a id="rust-crates-hm-core-src-ids-rs-schemaversion"></a>

Source: [`crates/hm-core/src/ids.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-core/src/ids.rs).

When to use: Use `SchemaVersion` for typed identities, revisions, timestamps, and stable error handling across kernel boundaries.

Do not use: Do not interchange actor, conversation, entity, and log identities or interpret wall time as ledger order.


```rust
pub struct SchemaVersion(u16);
```

## hm-core::UtcNanos

<a id="rust-crates-hm-core-src-ids-rs-utcnanos"></a>

Source: [`crates/hm-core/src/ids.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-core/src/ids.rs).

When to use: Use `UtcNanos` for typed identities, revisions, timestamps, and stable error handling across kernel boundaries.

Do not use: Do not interchange actor, conversation, entity, and log identities or interpret wall time as ledger order.


```rust
pub struct UtcNanos(i64);
```

## hm-core::EntityId

<a id="rust-crates-hm-core-src-ids-rs-entityid"></a>

Source: [`crates/hm-core/src/ids.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-core/src/ids.rs).

When to use: Use `EntityId` for typed identities, revisions, timestamps, and stable error handling across kernel boundaries.

Do not use: Do not interchange actor, conversation, entity, and log identities or interpret wall time as ledger order.


```rust
pub struct EntityId([u8; 16]);
```

## hm-core::ConversationId

<a id="rust-crates-hm-core-src-ids-rs-conversationid"></a>

Source: [`crates/hm-core/src/ids.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-core/src/ids.rs).

When to use: Use `ConversationId` for typed identities, revisions, timestamps, and stable error handling across kernel boundaries.

Do not use: Do not interchange actor, conversation, entity, and log identities or interpret wall time as ledger order.


```rust
pub struct ConversationId([u8; 16]);
```

## hm-core::SpanKind

<a id="rust-crates-hm-core-src-telemetry-rs-spankind"></a>

Source: [`crates/hm-core/src/telemetry.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-core/src/telemetry.rs).

When to use: Use `SpanKind` for typed identities, revisions, timestamps, and stable error handling across kernel boundaries.

Do not use: Do not interchange actor, conversation, entity, and log identities or interpret wall time as ledger order.


```rust
pub enum SpanKind {
    Request,
    Ingestion,
    Extraction,
    Provider,
}
```

## hm-core::SpanOutcome

<a id="rust-crates-hm-core-src-telemetry-rs-spanoutcome"></a>

Source: [`crates/hm-core/src/telemetry.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-core/src/telemetry.rs).

When to use: Use `SpanOutcome` for typed identities, revisions, timestamps, and stable error handling across kernel boundaries.

Do not use: Do not interchange actor, conversation, entity, and log identities or interpret wall time as ledger order.


```rust
pub enum SpanOutcome {
    Ok,
    Error,
}
```

## hm-core::Attribute

<a id="rust-crates-hm-core-src-telemetry-rs-attribute"></a>

Source: [`crates/hm-core/src/telemetry.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-core/src/telemetry.rs).

When to use: Use `Attribute` for typed identities, revisions, timestamps, and stable error handling across kernel boundaries.

Do not use: Do not interchange actor, conversation, entity, and log identities or interpret wall time as ledger order.


```rust
pub enum Attribute {
    Integer(&'static str, i64),
    Text(&'static str, &'static str),
    Boolean(&'static str, bool),
}
```

## hm-core::SpanRecord

<a id="rust-crates-hm-core-src-telemetry-rs-spanrecord"></a>

Source: [`crates/hm-core/src/telemetry.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-core/src/telemetry.rs).

When to use: Use `SpanRecord` for typed identities, revisions, timestamps, and stable error handling across kernel boundaries.

Do not use: Do not interchange actor, conversation, entity, and log identities or interpret wall time as ledger order.


```rust
pub struct SpanRecord {
    pub kind: SpanKind,
    pub name: &'static str,
    pub trace_id: [u8; 16],
    pub span_id: [u8; 8],
    pub start_unix_nanos: u128,
    pub duration_nanos: u64,
    pub outcome: SpanOutcome,
    pub attributes: Vec<Attribute>,
}
```

## hm-core::SpanSink

<a id="rust-crates-hm-core-src-telemetry-rs-spansink"></a>

Source: [`crates/hm-core/src/telemetry.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-core/src/telemetry.rs).

When to use: Use `SpanSink` for typed identities, revisions, timestamps, and stable error handling across kernel boundaries.

Do not use: Do not interchange actor, conversation, entity, and log identities or interpret wall time as ledger order.


```rust
pub trait SpanSink: Send + Sync {
    fn enabled(&self) -> bool;
    fn record(&self, span: &SpanRecord);
    fn flush(&self);
}
```

## hm-core::SpanBuilder

<a id="rust-crates-hm-core-src-telemetry-rs-spanbuilder"></a>

Source: [`crates/hm-core/src/telemetry.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-core/src/telemetry.rs).

When to use: Use `SpanBuilder` for typed identities, revisions, timestamps, and stable error handling across kernel boundaries.

Do not use: Do not interchange actor, conversation, entity, and log identities or interpret wall time as ledger order.


```rust
pub struct SpanBuilder {
    kind: SpanKind,
    name: &'static str,
    trace_id: [u8; 16],
    span_id: [u8; 8],
    start_unix_nanos: u128,
    started: Instant,
    attributes: Vec<Attribute>,
}
```

## hm-cortex::AdjudicationConflict

<a id="rust-crates-hm-cortex-src-adjudicate-rs-adjudicationconflict"></a>

Source: [`crates/hm-cortex/src/adjudicate.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/adjudicate.rs).

When to use: Use `AdjudicationConflict` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct AdjudicationConflict {
    pub left_belief_id: Vec<u8>,
    pub right_belief_id: Vec<u8>,
    pub conflict_domain: String,
    pub obligated_surfacing: bool,
}
```

## hm-cortex::AdjudicationOutcome

<a id="rust-crates-hm-cortex-src-adjudicate-rs-adjudicationoutcome"></a>

Source: [`crates/hm-cortex/src/adjudicate.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/adjudicate.rs).

When to use: Use `AdjudicationOutcome` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum AdjudicationOutcome {
    Replace {
        retract: Retract,
        assertion: Assertion,
    },
    Conflict(AdjudicationConflict),
    UnverifiedTension,
    NoChange,
}
```

## hm-cortex::AdjudicationReport

<a id="rust-crates-hm-cortex-src-adjudicate-rs-adjudicationreport"></a>

Source: [`crates/hm-cortex/src/adjudicate.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/adjudicate.rs).

When to use: Use `AdjudicationReport` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct AdjudicationReport {
    pub nli: BidirectionalNli,
    pub outcome: AdjudicationOutcome,
    pub model_id: Option<String>,
    pub usage: Usage,
}
```

## hm-cortex::AdjudicationError

<a id="rust-crates-hm-cortex-src-adjudicate-rs-adjudicationerror"></a>

Source: [`crates/hm-cortex/src/adjudicate.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/adjudicate.rs).

When to use: Use `AdjudicationError` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum AdjudicationError {
    InvalidUtf8,
    InvalidBelief,
    Nli(NliError),
    Llm(LlmError),
    InvalidStructuredOutput,
}
```

## hm-cortex::AttentionFactors

<a id="rust-crates-hm-cortex-src-attention-rs-attentionfactors"></a>

Source: [`crates/hm-cortex/src/attention.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/attention.rs).

When to use: Use `AttentionFactors` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct AttentionFactors {
    pub urgency: u32,
    pub expected_value: u32,
    pub confidence: u32,
    pub interruption_cost: u32,
    pub resource_cost: u32,
    pub duplication_penalty: u32,
    pub quiet_hours: bool,
    pub notifications_remaining: u32,
    pub workload: u32,
}
```

## hm-cortex::CitedSource

<a id="rust-crates-hm-cortex-src-authority-rs-citedsource"></a>

Source: [`crates/hm-cortex/src/authority.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/authority.rs).

When to use: Use `CitedSource` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct CitedSource<'a> {
    pub authority: Authority,
    pub bytes: &'a [u8],
    pub cited_range: Range<usize>,
}
```

## hm-cortex::BudgetUsage

<a id="rust-crates-hm-cortex-src-budget-rs-budgetusage"></a>

Source: [`crates/hm-cortex/src/budget.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/budget.rs).

When to use: Use `BudgetUsage` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct BudgetUsage {
    pub llm_calls: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_microusd: u64,
    pub wall_ms: u64,
}
```

## hm-cortex::BudgetDimension

<a id="rust-crates-hm-cortex-src-budget-rs-budgetdimension"></a>

Source: [`crates/hm-cortex/src/budget.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/budget.rs).

When to use: Use `BudgetDimension` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum BudgetDimension {
    LlmCalls,
    Tokens,
    Cost,
    WallTime,
}
```

## hm-cortex::BudgetExceeded

<a id="rust-crates-hm-cortex-src-budget-rs-budgetexceeded"></a>

Source: [`crates/hm-cortex/src/budget.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/budget.rs).

When to use: Use `BudgetExceeded` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct BudgetExceeded {
    pub dimension: BudgetDimension,
    pub limit: u64,
    pub attempted: u64,
}
```

## hm-cortex::BudgetTracker

<a id="rust-crates-hm-cortex-src-budget-rs-budgettracker"></a>

Source: [`crates/hm-cortex/src/budget.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/budget.rs).

When to use: Use `BudgetTracker` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct BudgetTracker {
    limits: ConsolidationBudget,
    usage: BudgetUsage,
    started_at_ms: u64,
}
```

## hm-cortex::CallReservation

<a id="rust-crates-hm-cortex-src-budget-rs-callreservation"></a>

Source: [`crates/hm-cortex/src/budget.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/budget.rs).

When to use: Use `CallReservation` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct CallReservation {
    pub llm_calls: u64,
    pub output_tokens: u64,
}
```

## hm-cortex::ReservationTicket

<a id="rust-crates-hm-cortex-src-budget-rs-reservationticket"></a>

Source: [`crates/hm-cortex/src/budget.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/budget.rs).

When to use: Use `ReservationTicket` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct ReservationTicket {
    reserved: CallReservation,
}
```

## hm-cortex::RunReservation

<a id="rust-crates-hm-cortex-src-budget-rs-runreservation"></a>

Source: [`crates/hm-cortex/src/budget.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/budget.rs).

When to use: Use `RunReservation` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct RunReservation {
    limits: ConsolidationBudget,
    settled: BudgetUsage,
    outstanding: CallReservation,
}
```

## hm-cortex::SourceKind

<a id="rust-crates-hm-cortex-src-citations-rs-sourcekind"></a>

Source: [`crates/hm-cortex/src/citations.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/citations.rs).

When to use: Use `SourceKind` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum SourceKind {
    Declarative,
    Reflective,
    Speculation,
    Interrogative,
}
```

## hm-cortex::FrozenCandidate

<a id="rust-crates-hm-cortex-src-citations-rs-frozencandidate"></a>

Source: [`crates/hm-cortex/src/citations.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/citations.rs).

When to use: Use `FrozenCandidate` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct FrozenCandidate {
    pub lsn: u64,
    pub conversation: [u8; 16],
    pub source_root: [u8; 32],
    pub content: Vec<u8>,
    pub kind: SourceKind,
    pub authority: Authority,
}
```

## hm-cortex::CitationClaim

<a id="rust-crates-hm-cortex-src-citations-rs-citationclaim"></a>

Source: [`crates/hm-cortex/src/citations.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/citations.rs).

When to use: Use `CitationClaim` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct CitationClaim {
    pub lsn: u64,
    pub byte_start: u32,
    pub byte_end: u32,
    pub quote: Vec<u8>,
}
```

## hm-cortex::ValidatedCitations

<a id="rust-crates-hm-cortex-src-citations-rs-validatedcitations"></a>

Source: [`crates/hm-cortex/src/citations.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/citations.rs).

When to use: Use `ValidatedCitations` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct ValidatedCitations {
    pub ranges: Vec<ProvenanceRange>,
    pub authority: Authority,
}
```

## hm-cortex::CitationError

<a id="rust-crates-hm-cortex-src-citations-rs-citationerror"></a>

Source: [`crates/hm-cortex/src/citations.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/citations.rs).

When to use: Use `CitationError` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum CitationError {
    EmptyCandidates,
    DuplicateLsn(u64),
    MissingCitation,
    UnknownLsn(u64),
    InvalidRange(u64),
    QuoteOutsideRange(u64),
    UncitedDerivedQuote(Vec<u8>),
}
```

## hm-cortex::FrozenCandidateSet

<a id="rust-crates-hm-cortex-src-citations-rs-frozencandidateset"></a>

Source: [`crates/hm-cortex/src/citations.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/citations.rs).

When to use: Use `FrozenCandidateSet` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct FrozenCandidateSet {
    candidates: BTreeMap<u64, FrozenCandidate>,
}
```

## hm-cortex::DeliveryEnvelope

<a id="rust-crates-hm-cortex-src-connectors-rs-deliveryenvelope"></a>

Source: [`crates/hm-cortex/src/connectors.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/connectors.rs).

When to use: Use `DeliveryEnvelope` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct DeliveryEnvelope<'a> {
    pub connector_id: [u8; 16],
    pub delivery_id: &'a [u8],
    pub event_name: &'a str,
    pub signed_at_ns: i64,
    pub body: &'a [u8],
    pub signature: &'a [u8],
}
```

## hm-cortex::VerifiedDelivery

<a id="rust-crates-hm-cortex-src-connectors-rs-verifieddelivery"></a>

Source: [`crates/hm-cortex/src/connectors.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/connectors.rs).

When to use: Use `VerifiedDelivery` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct VerifiedDelivery {
    pub body_digest: [u8; 32],
    pub body_bytes: u64,
}
```

## hm-cortex::ConsentState

<a id="rust-crates-hm-cortex-src-connectors-rs-consentstate"></a>

Source: [`crates/hm-cortex/src/connectors.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/connectors.rs).

When to use: Use `ConsentState` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct ConsentState {
    pub encoded: String,
    pub nonce: [u8; 16],
    pub expires_at_ns: i64,
}
```

## hm-cortex::ConsentGrant

<a id="rust-crates-hm-cortex-src-connectors-rs-consentgrant"></a>

Source: [`crates/hm-cortex/src/connectors.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/connectors.rs).

When to use: Use `ConsentGrant` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct ConsentGrant {
    pub nonce: [u8; 16],
    pub expires_at_ns: i64,
}
```

## hm-cortex::FsrsState

<a id="rust-crates-hm-cortex-src-fsrs-rs-fsrsstate"></a>

Source: [`crates/hm-cortex/src/fsrs.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/fsrs.rs).

When to use: Use `FsrsState` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum FsrsState {
    New,
    Learning,
    Review,
    Relearning,
}
```

## hm-cortex::FsrsData

<a id="rust-crates-hm-cortex-src-fsrs-rs-fsrsdata"></a>

Source: [`crates/hm-cortex/src/fsrs.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/fsrs.rs).

When to use: Use `FsrsData` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct FsrsData {
    pub stability: f64,
    pub difficulty: f64,
    pub repetitions: u32,
    pub lapses: u32,
    pub state: FsrsState,
    pub last_review_ns: Option<i64>,
}
```

## hm-cortex::ScheduleResult

<a id="rust-crates-hm-cortex-src-fsrs-rs-scheduleresult"></a>

Source: [`crates/hm-cortex/src/fsrs.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/fsrs.rs).

When to use: Use `ScheduleResult` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct ScheduleResult {
    pub stability: f64,
    pub difficulty: f64,
    pub interval_days: u32,
    pub state: FsrsState,
    pub repetitions: u32,
    pub lapses: u32,
}
```

## hm-cortex::IngestSource

<a id="rust-crates-hm-cortex-src-ingest-rs-ingestsource"></a>

Source: [`crates/hm-cortex/src/ingest.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/ingest.rs).

When to use: Use `IngestSource` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum IngestSource {
    User,
    Assistant,
    ToolRuntime,
    Kernel,
    ExternalFeed,
    Derived,
    Document,
    Memory(String),
}
```

## hm-cortex::DoNotStoreReceipt

<a id="rust-crates-hm-cortex-src-ingest-rs-donotstorereceipt"></a>

Source: [`crates/hm-cortex/src/ingest.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/ingest.rs).

When to use: Use `DoNotStoreReceipt` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct DoNotStoreReceipt {
    pub kind: EventKind,
    pub authority: Authority,
    pub content_hash: [u8; 32],
}
```

## hm-cortex::IngestResult

<a id="rust-crates-hm-cortex-src-ingest-rs-ingestresult"></a>

Source: [`crates/hm-cortex/src/ingest.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/ingest.rs).

When to use: Use `IngestResult` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub enum IngestResult {
    Append(EventEnvelope),
    DoNotStore(DoNotStoreReceipt),
}
```

## hm-cortex::NliScores

<a id="rust-crates-hm-cortex-src-nli-rs-nliscores"></a>

Source: [`crates/hm-cortex/src/nli.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/nli.rs).

When to use: Use `NliScores` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct NliScores {
    pub contradiction: f32,
    pub entailment: f32,
    pub neutral: f32,
}
```

## hm-cortex::NliVerdict

<a id="rust-crates-hm-cortex-src-nli-rs-nliverdict"></a>

Source: [`crates/hm-cortex/src/nli.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/nli.rs).

When to use: Use `NliVerdict` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum NliVerdict {
    Genuine,
    Tension,
    Complementary,
    Unrelated,
}
```

## hm-cortex::BidirectionalNli

<a id="rust-crates-hm-cortex-src-nli-rs-bidirectionalnli"></a>

Source: [`crates/hm-cortex/src/nli.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/nli.rs).

When to use: Use `BidirectionalNli` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct BidirectionalNli {
    pub forward: NliScores,
    pub reverse: NliScores,
    pub verdict: NliVerdict,
    pub confidence: f32,
}
```

## hm-cortex::NliError

<a id="rust-crates-hm-cortex-src-nli-rs-nlierror"></a>

Source: [`crates/hm-cortex/src/nli.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/nli.rs).

When to use: Use `NliError` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum NliError {
    InvalidArgument,
    Download(String),
    Digest { path: PathBuf },
    Tokenizer(String),
    Runtime(String),
    Shape,
}
```

## hm-cortex::NliModel

<a id="rust-crates-hm-cortex-src-nli-rs-nlimodel"></a>

Source: [`crates/hm-cortex/src/nli.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/nli.rs).

When to use: Use `NliModel` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct NliModel {
    tokenizer: Tokenizer,
    session: Mutex<Session>,
}
```

## hm-cortex::PendingObservation

<a id="rust-crates-hm-cortex-src-nrem-cluster-rs-pendingobservation"></a>

Source: [`crates/hm-cortex/src/nrem/cluster.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/nrem/cluster.rs).

When to use: Use `PendingObservation` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct PendingObservation {
    pub source: FrozenCandidate,
    pub salience_micros: u32,
    pub event_time_ns: i64,
    pub embedding: Vec<i8>,
    pub entities: Vec<String>,
}
```

## hm-cortex::ClusterOptions

<a id="rust-crates-hm-cortex-src-nrem-cluster-rs-clusteroptions"></a>

Source: [`crates/hm-cortex/src/nrem/cluster.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/nrem/cluster.rs).

When to use: Use `ClusterOptions` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning. Set explicit deployment limits before opening the associated resource.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation. Do not log secret fields or substitute defaults for an explicitly authorized budget.


```rust
pub struct ClusterOptions {
    pub maximum_observations: usize,
    pub cosine_threshold_micros: u32,
    pub entity_overlap_threshold_micros: u32,
}
```

## hm-cortex::ObservationCluster

<a id="rust-crates-hm-cortex-src-nrem-cluster-rs-observationcluster"></a>

Source: [`crates/hm-cortex/src/nrem/cluster.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/nrem/cluster.rs).

When to use: Use `ObservationCluster` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct ObservationCluster {
    pub cluster_id: [u8; 32],
    pub priority: u64,
    pub observations: Vec<PendingObservation>,
}
```

## hm-cortex::ExistingMemory

<a id="rust-crates-hm-cortex-src-nrem-merge-rs-existingmemory"></a>

Source: [`crates/hm-cortex/src/nrem/merge.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/nrem/merge.rs).

When to use: Use `ExistingMemory` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct ExistingMemory {
    pub memory_id: Vec<u8>,
    pub name: String,
    pub definition: Vec<u8>,
    pub faded: bool,
}
```

## hm-cortex::MergeAction

<a id="rust-crates-hm-cortex-src-nrem-merge-rs-mergeaction"></a>

Source: [`crates/hm-cortex/src/nrem/merge.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/nrem/merge.rs).

When to use: Use `MergeAction` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum MergeAction {
    Attach,
    Revise,
    Mint,
}
```

## hm-cortex::NremDecision

<a id="rust-crates-hm-cortex-src-nrem-merge-rs-nremdecision"></a>

Source: [`crates/hm-cortex/src/nrem/merge.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/nrem/merge.rs).

When to use: Use `NremDecision` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct NremDecision {
    pub cluster_id: [u8; 32],
    pub action: MergeAction,
    pub target_id: Option<Vec<u8>>,
    pub name: String,
    pub definition: Vec<u8>,
    pub tags: Vec<String>,
    pub salience_micros: u32,
    pub citations: Vec<ProvenanceRange>,
    pub authority: Authority,
    pub model_provenance: ModelProvenance,
    pub thought_quality: ThoughtQualityResult,
    pub rewrite_guard: Option<RewriteGuardResult>,
}
```

## hm-cortex::DropReason

<a id="rust-crates-hm-cortex-src-nrem-merge-rs-dropreason"></a>

Source: [`crates/hm-cortex/src/nrem/merge.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/nrem/merge.rs).

When to use: Use `DropReason` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum DropReason {
    InvalidStructuredOutput,
    Citation(CitationError),
    InsufficientIndependentRoots,
    InsufficientConversations,
    SpeculationPoisoned,
    ThoughtQuality(Vec<String>),
    RewriteGuard(Vec<String>),
    MissingTarget,
    InvalidCandidateEncoding,
}
```

## hm-cortex::DroppedCandidate

<a id="rust-crates-hm-cortex-src-nrem-merge-rs-droppedcandidate"></a>

Source: [`crates/hm-cortex/src/nrem/merge.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/nrem/merge.rs).

When to use: Use `DroppedCandidate` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct DroppedCandidate {
    pub cluster_id: [u8; 32],
    pub reason: DropReason,
}
```

## hm-cortex::NremReport

<a id="rust-crates-hm-cortex-src-nrem-merge-rs-nremreport"></a>

Source: [`crates/hm-cortex/src/nrem/merge.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/nrem/merge.rs).

When to use: Use `NremReport` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct NremReport {
    pub decisions: Vec<NremDecision>,
    pub dropped: Vec<DroppedCandidate>,
    pub citation_invalid: u64,
    pub llm_calls: u64,
    pub cost: RunCost,
}
```

## hm-cortex::NremError

<a id="rust-crates-hm-cortex-src-nrem-merge-rs-nremerror"></a>

Source: [`crates/hm-cortex/src/nrem/merge.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/nrem/merge.rs).

When to use: Use `NremError` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum NremError {
    Llm(LlmError),
    Cost(LlmError),
}
```

## hm-cortex::ClusterOutcome

<a id="rust-crates-hm-cortex-src-nrem-merge-rs-clusteroutcome"></a>

Source: [`crates/hm-cortex/src/nrem/merge.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/nrem/merge.rs).

When to use: Use `ClusterOutcome` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum ClusterOutcome {
    Decided(Box<NremDecision>),
    Dropped(DroppedCandidate),
}
```

## hm-cortex::ClusterExtraction

<a id="rust-crates-hm-cortex-src-nrem-merge-rs-clusterextraction"></a>

Source: [`crates/hm-cortex/src/nrem/merge.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/nrem/merge.rs).

When to use: Use `ClusterExtraction` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct ClusterExtraction {
    pub outcome: ClusterOutcome,
    pub usage: hm_llm::Usage,
    pub llm_calls: u64,
    pub citation_invalid: bool,
}
```

## hm-cortex::Observation

<a id="rust-crates-hm-cortex-src-predict-rs-observation"></a>

Source: [`crates/hm-cortex/src/predict.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/predict.rs).

When to use: Use `Observation` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct Observation {
    pub lsn: u64,
    pub kind: PredicateKind,
    pub scope: String,
    pub property: Option<String>,
    pub value: Option<Vec<u8>>,
    pub executed: bool,
    pub resolvable: bool,
}
```

## hm-cortex::RevisionGap

<a id="rust-crates-hm-cortex-src-predict-rs-revisiongap"></a>

Source: [`crates/hm-cortex/src/predict.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/predict.rs).

When to use: Use `RevisionGap` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct RevisionGap {
    pub revision_required: bool,
    pub revision_rounds_remaining: u8,
    pub probes_remaining: u8,
}
```

## hm-cortex::Episode

<a id="rust-crates-hm-cortex-src-procedures-rs-episode"></a>

Source: [`crates/hm-cortex/src/procedures.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/procedures.rs).

When to use: Use `Episode` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct Episode {
    pub tool_call_lsn: u64,
    pub tool_call_id: Vec<u8>,
    pub tool_result_lsn: u64,
    pub tool_result_call_id: Vec<u8>,
    pub effect_lsn: u64,
    pub effect_tool_call_lsn: u64,
    pub effect_id: Vec<u8>,
    pub outcome_lsn: u64,
    pub outcome_effect_id: Vec<u8>,
    pub loop_closed_lsn: u64,
    pub loop_evidence_lsns: Vec<u64>,
    pub source_root: Vec<u8>,
    pub conversation: Vec<u8>,
    pub successful: bool,
}
```

## hm-cortex::WakeSignal

<a id="rust-crates-hm-cortex-src-prospective-rs-wakesignal"></a>

Source: [`crates/hm-cortex/src/prospective.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/prospective.rs).

When to use: Use `WakeSignal` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct WakeSignal {
    pub trigger_lsn: u64,
    pub now_ns: i64,
    pub kind: &'static str,
    pub key: Vec<u8>,
}
```

## hm-cortex::ThoughtQualityOptions

<a id="rust-crates-hm-cortex-src-quality-rs-thoughtqualityoptions"></a>

Source: [`crates/hm-cortex/src/quality.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/quality.rs).

When to use: Use `ThoughtQualityOptions` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning. Set explicit deployment limits before opening the associated resource.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation. Do not log secret fields or substitute defaults for an explicitly authorized budget.


```rust
pub struct ThoughtQualityOptions {
    pub minimum_grounding_micros: u32,
    pub require_sentence_end: bool,
    pub minimum_length: usize,
    pub maximum_length: usize,
}
```

## hm-cortex::ThoughtQualityResult

<a id="rust-crates-hm-cortex-src-quality-rs-thoughtqualityresult"></a>

Source: [`crates/hm-cortex/src/quality.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/quality.rs).

When to use: Use `ThoughtQualityResult` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct ThoughtQualityResult {
    pub accepted: bool,
    pub grounding_micros: Option<u32>,
    pub generic_hits: Vec<String>,
    pub reasons: Vec<String>,
}
```

## hm-cortex::RewriteGuardResult

<a id="rust-crates-hm-cortex-src-quality-rs-rewriteguardresult"></a>

Source: [`crates/hm-cortex/src/quality.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/quality.rs).

When to use: Use `RewriteGuardResult` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct RewriteGuardResult {
    pub accepted: bool,
    pub reasons: Vec<String>,
}
```

## hm-cortex::LabelledDecision

<a id="rust-crates-hm-cortex-src-quality-rs-labelleddecision"></a>

Source: [`crates/hm-cortex/src/quality.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/quality.rs).

When to use: Use `LabelledDecision` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct LabelledDecision {
    pub accepted: bool,
    pub expected_accepted: bool,
}
```

## hm-cortex::ClassifierMetrics

<a id="rust-crates-hm-cortex-src-quality-rs-classifiermetrics"></a>

Source: [`crates/hm-cortex/src/quality.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/quality.rs).

When to use: Use `ClassifierMetrics` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct ClassifierMetrics {
    pub examples: u64,
    pub predicted_rejections: u64,
    pub true_rejections: u64,
    pub rejection_precision_micros: Option<u32>,
}
```

## hm-cortex::RelationSource

<a id="rust-crates-hm-cortex-src-relations-rs-relationsource"></a>

Source: [`crates/hm-cortex/src/relations.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/relations.rs).

When to use: Use `RelationSource` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct RelationSource {
    pub edge_id: Vec<u8>,
    pub edge_lsn: u64,
    pub source_id: Vec<u8>,
    pub target_id: Vec<u8>,
    pub relation: String,
    pub support_lsns: Vec<u64>,
}
```

## hm-cortex::RelationRepresentation

<a id="rust-crates-hm-cortex-src-relations-rs-relationrepresentation"></a>

Source: [`crates/hm-cortex/src/relations.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/relations.rs).

When to use: Use `RelationRepresentation` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct RelationRepresentation {
    pub relation_key: [u8; 32],
    pub edge_lsn: u64,
    pub text: String,
    pub support_lsns: Vec<u64>,
}
```

## hm-cortex::RelationOptions

<a id="rust-crates-hm-cortex-src-relations-rs-relationoptions"></a>

Source: [`crates/hm-cortex/src/relations.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/relations.rs).

When to use: Use `RelationOptions` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning. Set explicit deployment limits before opening the associated resource.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation. Do not log secret fields or substitute defaults for an explicitly authorized budget.


```rust
pub struct RelationOptions {
    pub maximum_relations: usize,
    pub maximum_text_bytes: usize,
}
```

## hm-cortex::RelationDrop

<a id="rust-crates-hm-cortex-src-relations-rs-relationdrop"></a>

Source: [`crates/hm-cortex/src/relations.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/relations.rs).

When to use: Use `RelationDrop` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum RelationDrop {
    ZeroLsn,
    EmptyRelation,
    EmptyEndpoint,
    NoSupport,
    TextTooLong,
}
```

## hm-cortex::RelationPlan

<a id="rust-crates-hm-cortex-src-relations-rs-relationplan"></a>

Source: [`crates/hm-cortex/src/relations.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/relations.rs).

When to use: Use `RelationPlan` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct RelationPlan {
    pub representations: Vec<RelationRepresentation>,
    pub dropped: Vec<(u64, RelationDrop)>,
}
```

## hm-cortex::ExistingAbstract

<a id="rust-crates-hm-cortex-src-rem-abstract-rs-existingabstract"></a>

Source: [`crates/hm-cortex/src/rem/abstract.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/rem/abstract.rs).

When to use: Use `ExistingAbstract` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct ExistingAbstract {
    pub definition: String,
    pub faded: bool,
}
```

## hm-cortex::AbstractOptions

<a id="rust-crates-hm-cortex-src-rem-abstract-rs-abstractoptions"></a>

Source: [`crates/hm-cortex/src/rem/abstract.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/rem/abstract.rs).

When to use: Use `AbstractOptions` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning. Set explicit deployment limits before opening the associated resource.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation. Do not log secret fields or substitute defaults for an explicitly authorized budget.


```rust
pub struct AbstractOptions {
    pub maximum_similarity_micros: u32,
    pub minimum_grounding_micros: u32,
}
```

## hm-cortex::AbstractDecision

<a id="rust-crates-hm-cortex-src-rem-abstract-rs-abstractdecision"></a>

Source: [`crates/hm-cortex/src/rem/abstract.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/rem/abstract.rs).

When to use: Use `AbstractDecision` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct AbstractDecision {
    pub run_id: Vec<u8>,
    pub name: String,
    pub definition: Vec<u8>,
    pub tag: String,
    pub citations: Vec<ProvenanceRange>,
    pub authority: Authority,
    pub model_provenance: ModelProvenance,
    pub quality: ThoughtQualityResult,
    pub maximum_similarity_micros: u32,
}
```

## hm-cortex::AbstractDrop

<a id="rust-crates-hm-cortex-src-rem-abstract-rs-abstractdrop"></a>

Source: [`crates/hm-cortex/src/rem/abstract.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/rem/abstract.rs).

When to use: Use `AbstractDrop` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum AbstractDrop {
    InvalidStructuredOutput,
    InvalidEncoding,
    Citation(CitationError),
    Quality(Vec<String>),
    Duplicate { similarity_micros: u32 },
}
```

## hm-cortex::AbstractError

<a id="rust-crates-hm-cortex-src-rem-abstract-rs-abstracterror"></a>

Source: [`crates/hm-cortex/src/rem/abstract.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/rem/abstract.rs).

When to use: Use `AbstractError` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum AbstractError {
    Llm(LlmError),
    Drop(AbstractDrop),
}
```

## hm-cortex::ConnectMemory

<a id="rust-crates-hm-cortex-src-rem-connect-rs-connectmemory"></a>

Source: [`crates/hm-cortex/src/rem/connect.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/rem/connect.rs).

When to use: Use `ConnectMemory` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct ConnectMemory {
    pub memory_id: Vec<u8>,
    pub cluster_ordinal: u64,
    pub name: String,
    pub definition: Vec<u8>,
    pub entities: BTreeSet<String>,
    pub evidence: Vec<FrozenCandidate>,
    pub faded: bool,
}
```

## hm-cortex::ConnectedEdge

<a id="rust-crates-hm-cortex-src-rem-connect-rs-connectededge"></a>

Source: [`crates/hm-cortex/src/rem/connect.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/rem/connect.rs).

When to use: Use `ConnectedEdge` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct ConnectedEdge {
    pub run_id: Vec<u8>,
    pub event: EdgeAsserted,
    pub model_provenance: ModelProvenance,
}
```

## hm-cortex::ConnectReport

<a id="rust-crates-hm-cortex-src-rem-connect-rs-connectreport"></a>

Source: [`crates/hm-cortex/src/rem/connect.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/rem/connect.rs).

When to use: Use `ConnectReport` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct ConnectReport {
    pub edges: Vec<ConnectedEdge>,
    pub dropped: u64,
    pub llm_calls: u64,
    pub cost: RunCost,
}
```

## hm-cortex::ConnectError

<a id="rust-crates-hm-cortex-src-rem-connect-rs-connecterror"></a>

Source: [`crates/hm-cortex/src/rem/connect.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/rem/connect.rs).

When to use: Use `ConnectError` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum ConnectError {
    InvalidEncoding,
    InvalidStructuredOutput,
    Llm(LlmError),
    Cost(LlmError),
}
```

## hm-cortex::ConcernSourceKind

<a id="rust-crates-hm-cortex-src-rem-hindsight-rs-concernsourcekind"></a>

Source: [`crates/hm-cortex/src/rem/hindsight.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/rem/hindsight.rs).

When to use: Use `ConcernSourceKind` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum ConcernSourceKind {
    Neighbour,
    BeliefHistory,
}
```

## hm-cortex::ConcernSource

<a id="rust-crates-hm-cortex-src-rem-hindsight-rs-concernsource"></a>

Source: [`crates/hm-cortex/src/rem/hindsight.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/rem/hindsight.rs).

When to use: Use `ConcernSource` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct ConcernSource {
    pub kind: ConcernSourceKind,
    pub candidate: FrozenCandidate,
}
```

## hm-cortex::HindsightMemory

<a id="rust-crates-hm-cortex-src-rem-hindsight-rs-hindsightmemory"></a>

Source: [`crates/hm-cortex/src/rem/hindsight.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/rem/hindsight.rs).

When to use: Use `HindsightMemory` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct HindsightMemory {
    pub memory_id: Vec<u8>,
    pub name: String,
    pub candidate: FrozenCandidate,
    pub faded: bool,
}
```

## hm-cortex::HindsightRevision

<a id="rust-crates-hm-cortex-src-rem-hindsight-rs-hindsightrevision"></a>

Source: [`crates/hm-cortex/src/rem/hindsight.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/rem/hindsight.rs).

When to use: Use `HindsightRevision` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct HindsightRevision {
    pub run_id: Vec<u8>,
    pub definition: Vec<u8>,
    pub concern: String,
    pub citations: Vec<ProvenanceRange>,
    pub authority: Authority,
    pub model_provenance: ModelProvenance,
    pub rewrite_guard: RewriteGuardResult,
}
```

## hm-cortex::HindsightDecision

<a id="rust-crates-hm-cortex-src-rem-hindsight-rs-hindsightdecision"></a>

Source: [`crates/hm-cortex/src/rem/hindsight.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/rem/hindsight.rs).

When to use: Use `HindsightDecision` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum HindsightDecision {
    NoChange,
    Revised(Box<HindsightRevision>),
    Declined(Vec<String>),
}
```

## hm-cortex::HindsightError

<a id="rust-crates-hm-cortex-src-rem-hindsight-rs-hindsighterror"></a>

Source: [`crates/hm-cortex/src/rem/hindsight.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/rem/hindsight.rs).

When to use: Use `HindsightError` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum HindsightError {
    InvalidEncoding,
    Llm(LlmError),
}
```

## hm-cortex::RepoFactKind

<a id="rust-crates-hm-cortex-src-repograph-rs-repofactkind"></a>

Source: [`crates/hm-cortex/src/repograph.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/repograph.rs).

When to use: Use `RepoFactKind` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum RepoFactKind {
    File,
    Symbol,
    Dependency,
    Route,
    Test,
    Storage,
}
```

## hm-cortex::RepoCitation

<a id="rust-crates-hm-cortex-src-repograph-rs-repocitation"></a>

Source: [`crates/hm-cortex/src/repograph.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/repograph.rs).

When to use: Use `RepoCitation` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct RepoCitation {
    pub lsn: u64,
    pub byte_start: u32,
    pub byte_end: u32,
}
```

## hm-cortex::RepoRelation

<a id="rust-crates-hm-cortex-src-repograph-rs-reporelation"></a>

Source: [`crates/hm-cortex/src/repograph.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/repograph.rs).

When to use: Use `RepoRelation` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct RepoRelation {
    pub relation: String,
    pub target: String,
}
```

## hm-cortex::RepoFact

<a id="rust-crates-hm-cortex-src-repograph-rs-repofact"></a>

Source: [`crates/hm-cortex/src/repograph.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/repograph.rs).

When to use: Use `RepoFact` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct RepoFact {
    pub kind: RepoFactKind,
    pub name: String,
    pub path: Option<String>,
    pub line: Option<u32>,
    pub end_line: Option<u32>,
    pub attributes: BTreeMap<String, String>,
    pub relations: Vec<RepoRelation>,
    pub source: RepoCitation,
}
```

## hm-cortex::RepoSnapshotShard

<a id="rust-crates-hm-cortex-src-repograph-rs-reposnapshotshard"></a>

Source: [`crates/hm-cortex/src/repograph.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/repograph.rs).

When to use: Use `RepoSnapshotShard` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct RepoSnapshotShard {
    pub lsn: u64,
    pub content: Vec<u8>,
}
```

## hm-cortex::RepoSnapshot

<a id="rust-crates-hm-cortex-src-repograph-rs-reposnapshot"></a>

Source: [`crates/hm-cortex/src/repograph.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/repograph.rs).

When to use: Use `RepoSnapshot` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct RepoSnapshot {
    pub repository: String,
    pub digest: [u8; 32],
    pub facts: Vec<RepoFact>,
    pub skipped: u64,
}
```

## hm-cortex::RepoGraphError

<a id="rust-crates-hm-cortex-src-repograph-rs-repographerror"></a>

Source: [`crates/hm-cortex/src/repograph.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/repograph.rs).

When to use: Use `RepoGraphError` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum RepoGraphError {
    Empty,
    Contract,
    Header,
    Truncated,
    Count,
    TooManyFacts,
}
```

## hm-cortex::RepoNode

<a id="rust-crates-hm-cortex-src-repograph-rs-reponode"></a>

Source: [`crates/hm-cortex/src/repograph.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/repograph.rs).

When to use: Use `RepoNode` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct RepoNode {
    pub node_id: [u8; 32],
    pub kind: RepoFactKind,
    pub name: String,
    pub display_name: String,
    pub definition: Vec<u8>,
    pub tags: Vec<String>,
    pub citation: RepoCitation,
}
```

## hm-cortex::RepoEdge

<a id="rust-crates-hm-cortex-src-repograph-rs-repoedge"></a>

Source: [`crates/hm-cortex/src/repograph.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/repograph.rs).

When to use: Use `RepoEdge` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct RepoEdge {
    pub edge_id: [u8; 32],
    pub source_id: [u8; 32],
    pub target_id: [u8; 32],
    pub relation: String,
    pub weight_micros: u32,
    pub citation: RepoCitation,
}
```

## hm-cortex::RepoDropReason

<a id="rust-crates-hm-cortex-src-repograph-rs-repodropreason"></a>

Source: [`crates/hm-cortex/src/repograph.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/repograph.rs).

When to use: Use `RepoDropReason` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum RepoDropReason {
    UnknownRelation,
    UnresolvedTarget,
    AmbiguousTarget,
    SelfReference,
    NodeLimit,
    EdgeLimit,
}
```

## hm-cortex::RepoDrop

<a id="rust-crates-hm-cortex-src-repograph-rs-repodrop"></a>

Source: [`crates/hm-cortex/src/repograph.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/repograph.rs).

When to use: Use `RepoDrop` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct RepoDrop {
    pub source: String,
    pub relation: String,
    pub target: String,
    pub reason: RepoDropReason,
}
```

## hm-cortex::RepoGraph

<a id="rust-crates-hm-cortex-src-repograph-rs-repograph"></a>

Source: [`crates/hm-cortex/src/repograph.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/repograph.rs).

When to use: Use `RepoGraph` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct RepoGraph {
    pub repository: String,
    pub digest: [u8; 32],
    pub nodes: Vec<RepoNode>,
    pub edges: Vec<RepoEdge>,
    pub dropped: Vec<RepoDrop>,
    pub skipped: u64,
}
```

## hm-cortex::AttestationSignal

<a id="rust-crates-hm-cortex-src-review-rs-attestationsignal"></a>

Source: [`crates/hm-cortex/src/review.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/review.rs).

When to use: Use `AttestationSignal` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum AttestationSignal {
    Used,
    Ignored,
    Helpful,
    Harmful,
}
```

## hm-cortex::ReviewCandidate

<a id="rust-crates-hm-cortex-src-review-rs-reviewcandidate"></a>

Source: [`crates/hm-cortex/src/review.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/review.rs).

When to use: Use `ReviewCandidate` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct ReviewCandidate {
    pub memory_id: Vec<u8>,
    pub retention: Retention,
    pub provenance_lsns: BTreeSet<u64>,
    pub fsrs: FsrsData,
    pub source_lsn: u64,
    pub signals: Vec<AttestationSignal>,
    pub retrieval_rank: Option<u32>,
    pub contradiction_involved: bool,
    pub dream_access_count: u32,
    pub faded: bool,
}
```

## hm-cortex::ProtectionSet

<a id="rust-crates-hm-cortex-src-review-rs-protectionset"></a>

Source: [`crates/hm-cortex/src/review.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/review.rs).

When to use: Use `ProtectionSet` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct ProtectionSet {
    lsns: BTreeSet<u64>,
}
```

## hm-cortex::ReviewDecision

<a id="rust-crates-hm-cortex-src-review-rs-reviewdecision"></a>

Source: [`crates/hm-cortex/src/review.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/review.rs).

When to use: Use `ReviewDecision` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct ReviewDecision {
    pub score: u64,
    pub event: Reviewed,
}
```

## hm-cortex::FadePolicy

<a id="rust-crates-hm-cortex-src-review-rs-fadepolicy"></a>

Source: [`crates/hm-cortex/src/review.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/review.rs).

When to use: Use `FadePolicy` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct FadePolicy {
    pub maximum_retrievability_micros: u32,
}
```

## hm-cortex::ContradictedOutcome

<a id="rust-crates-hm-cortex-src-review-rs-contradictedoutcome"></a>

Source: [`crates/hm-cortex/src/review.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/review.rs).

When to use: Use `ContradictedOutcome` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct ContradictedOutcome {
    pub domain: String,
    pub observation_lsn: u64,
    pub observed_at_ns: i64,
}
```

## hm-cortex::PersistedPhase

<a id="rust-crates-hm-cortex-src-run-rs-persistedphase"></a>

Source: [`crates/hm-cortex/src/run.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/run.rs).

When to use: Use `PersistedPhase` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct PersistedPhase {
    pub phase: ConsolidationPhaseName,
    pub state: ConsolidationPhaseState,
    pub attempt: u32,
    pub cursor: Option<Vec<u8>>,
}
```

## hm-cortex::PhaseWork

<a id="rust-crates-hm-cortex-src-run-rs-phasework"></a>

Source: [`crates/hm-cortex/src/run.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/run.rs).

When to use: Use `PhaseWork` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct PhaseWork {
    pub phase: ConsolidationPhaseName,
    pub attempt: u32,
    pub cursor: Option<Vec<u8>>,
    pub resumed: bool,
}
```

## hm-cortex::PhaseMachine

<a id="rust-crates-hm-cortex-src-run-rs-phasemachine"></a>

Source: [`crates/hm-cortex/src/run.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/run.rs).

When to use: Use `PhaseMachine` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct PhaseMachine {
    run_id: Vec<u8>,
    phases: Vec<ConsolidationPhaseName>,
    progress: BTreeMap<u8, PersistedPhase>,
}
```

## hm-cortex::Cadence

<a id="rust-crates-hm-cortex-src-run-rs-cadence"></a>

Source: [`crates/hm-cortex/src/run.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/run.rs).

When to use: Use `Cadence` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct Cadence {
    pub every_days: u32,
    pub local_hour: u8,
    pub local_minute: u8,
}
```

## hm-cortex::CadenceError

<a id="rust-crates-hm-cortex-src-run-rs-cadenceerror"></a>

Source: [`crates/hm-cortex/src/run.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/run.rs).

When to use: Use `CadenceError` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub enum CadenceError {
    UnknownTimezone,
    InvalidCadence,
    TimestampOutOfRange,
}
```

## hm-cortex::VocabularySource

<a id="rust-crates-hm-cortex-src-vocabulary-rs-vocabularysource"></a>

Source: [`crates/hm-cortex/src/vocabulary.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cortex/src/vocabulary.rs).

When to use: Use `VocabularySource` for background ingestion, consolidation, attention, prediction assessment, and evidence-based procedural learning.

Do not use: Do not promote inferred claims to observed authority, adopt procedures without user authority, or skip citation validation.


```rust
pub struct VocabularySource<'a> {
    pub vocabulary_id: &'a [u8],
    pub version: u16,
    pub source_uri: &'a str,
    pub document: &'a str,
    pub retention: Retention,
    pub sensitivity: Sensitivity,
}
```

## hm-docs::MailLoader

<a id="rust-crates-hm-docs-src-formats-mail-rs-mailloader"></a>

Source: [`crates/hm-docs/src/formats/mail.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-docs/src/formats/mail.rs).

When to use: Use `MailLoader` for document ingestion, format extraction, chunking and validated change plans for documents entering the ledger.

Do not use: Do not use it to write to the ledger or a projection, and do not treat extracted or chunked text as observed evidence.


```rust
pub struct MailLoader;
```

## hm-docs::PdfLoader

<a id="rust-crates-hm-docs-src-formats-pdf-rs-pdfloader"></a>

Source: [`crates/hm-docs/src/formats/pdf.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-docs/src/formats/pdf.rs).

When to use: Use `PdfLoader` for document ingestion, format extraction, chunking and validated change plans for documents entering the ledger.

Do not use: Do not use it to write to the ledger or a projection, and do not treat extracted or chunked text as observed evidence.


```rust
pub struct PdfLoader;
```

## hm-docs::TableLoader

<a id="rust-crates-hm-docs-src-formats-table-rs-tableloader"></a>

Source: [`crates/hm-docs/src/formats/table.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-docs/src/formats/table.rs).

When to use: Use `TableLoader` for document ingestion, format extraction, chunking and validated change plans for documents entering the ledger.

Do not use: Do not use it to write to the ledger or a projection, and do not treat extracted or chunked text as observed evidence.


```rust
pub struct TableLoader;
```

## hm-docs::TextLoader

<a id="rust-crates-hm-docs-src-formats-text-rs-textloader"></a>

Source: [`crates/hm-docs/src/formats/text.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-docs/src/formats/text.rs).

When to use: Use `TextLoader` for document ingestion, format extraction, chunking and validated change plans for documents entering the ledger.

Do not use: Do not use it to write to the ledger or a projection, and do not treat extracted or chunked text as observed evidence.


```rust
pub struct TextLoader;
```

## hm-docs::LoaderId

<a id="rust-crates-hm-docs-src-loader-rs-loaderid"></a>

Source: [`crates/hm-docs/src/loader.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-docs/src/loader.rs).

When to use: Use `LoaderId` for document ingestion, format extraction, chunking and validated change plans for documents entering the ledger.

Do not use: Do not use it to write to the ledger or a projection, and do not treat extracted or chunked text as observed evidence.


```rust
pub enum LoaderId {
    Text,
    Table,
    Pdf,
    Mail,
}
```

## hm-docs::PageSpan

<a id="rust-crates-hm-docs-src-loader-rs-pagespan"></a>

Source: [`crates/hm-docs/src/loader.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-docs/src/loader.rs).

When to use: Use `PageSpan` for document ingestion, format extraction, chunking and validated change plans for documents entering the ledger.

Do not use: Do not use it to write to the ledger or a projection, and do not treat extracted or chunked text as observed evidence.


```rust
pub struct PageSpan {
    pub page_number: u32,
    pub byte_start: usize,
    pub byte_end: usize,
}
```

## hm-docs::FieldSpan

<a id="rust-crates-hm-docs-src-loader-rs-fieldspan"></a>

Source: [`crates/hm-docs/src/loader.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-docs/src/loader.rs).

When to use: Use `FieldSpan` for document ingestion, format extraction, chunking and validated change plans for documents entering the ledger.

Do not use: Do not use it to write to the ledger or a projection, and do not treat extracted or chunked text as observed evidence.


```rust
pub struct FieldSpan {
    pub column_index: u32,
    pub byte_start: usize,
    pub byte_end: usize,
}
```

## hm-docs::RowSpan

<a id="rust-crates-hm-docs-src-loader-rs-rowspan"></a>

Source: [`crates/hm-docs/src/loader.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-docs/src/loader.rs).

When to use: Use `RowSpan` for document ingestion, format extraction, chunking and validated change plans for documents entering the ledger.

Do not use: Do not use it to write to the ledger or a projection, and do not treat extracted or chunked text as observed evidence.


```rust
pub struct RowSpan {
    pub row_index: u32,
    pub byte_start: usize,
    pub byte_end: usize,
    pub fields: Vec<FieldSpan>,
}
```

## hm-docs::TableLayout

<a id="rust-crates-hm-docs-src-loader-rs-tablelayout"></a>

Source: [`crates/hm-docs/src/loader.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-docs/src/loader.rs).

When to use: Use `TableLayout` for document ingestion, format extraction, chunking and validated change plans for documents entering the ledger.

Do not use: Do not use it to write to the ledger or a projection, and do not treat extracted or chunked text as observed evidence.


```rust
pub struct TableLayout {
    pub header: Vec<String>,
    pub rows: Vec<RowSpan>,
}
```

## hm-docs::PartialExtraction

<a id="rust-crates-hm-docs-src-loader-rs-partialextraction"></a>

Source: [`crates/hm-docs/src/loader.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-docs/src/loader.rs).

When to use: Use `PartialExtraction` for document ingestion, format extraction, chunking and validated change plans for documents entering the ledger.

Do not use: Do not use it to write to the ledger or a projection, and do not treat extracted or chunked text as observed evidence.


```rust
pub struct PartialExtraction {
    pub reason: String,
    pub failed_units: Vec<u32>,
}
```

## hm-docs::Extraction

<a id="rust-crates-hm-docs-src-loader-rs-extraction"></a>

Source: [`crates/hm-docs/src/loader.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-docs/src/loader.rs).

When to use: Use `Extraction` for document ingestion, format extraction, chunking and validated change plans for documents entering the ledger.

Do not use: Do not use it to write to the ledger or a projection, and do not treat extracted or chunked text as observed evidence.


```rust
pub struct Extraction {
    pub loader: LoaderId,
    pub extraction_version: u16,
    pub text: String,
    pub page_spans: Vec<PageSpan>,
    pub table: Option<TableLayout>,
    pub partial: Option<PartialExtraction>,
}
```

## hm-docs::DocumentLoader

<a id="rust-crates-hm-docs-src-loader-rs-documentloader"></a>

Source: [`crates/hm-docs/src/loader.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-docs/src/loader.rs).

When to use: Use `DocumentLoader` for document ingestion, format extraction, chunking and validated change plans for documents entering the ledger.

Do not use: Do not use it to write to the ledger or a projection, and do not treat extracted or chunked text as observed evidence.


```rust
pub trait DocumentLoader {
    fn loader_id(&self) -> LoaderId;

    fn extraction_version(&self) -> u16;

    fn extract(&self, bytes: &[u8]) -> Result<Extraction, Error>;
}
```

## hm-docs::LoaderRegistry

<a id="rust-crates-hm-docs-src-loader-rs-loaderregistry"></a>

Source: [`crates/hm-docs/src/loader.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-docs/src/loader.rs).

When to use: Use `LoaderRegistry` for document ingestion, format extraction, chunking and validated change plans for documents entering the ledger.

Do not use: Do not use it to write to the ledger or a projection, and do not treat extracted or chunked text as observed evidence.


```rust
pub struct LoaderRegistry {
    text: TextLoader,
    table: TableLoader,
    pdf: PdfLoader,
    mail: MailLoader,
}
```

## hm-embed::CachedEmbedder

<a id="rust-crates-hm-embed-src-cache-rs-cachedembedder"></a>

Source: [`crates/hm-embed/src/cache.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/cache.rs).

When to use: Use `CachedEmbedder` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub struct CachedEmbedder<E> {
    inner: E,
    maximum_batch: usize,
    cache: Mutex<HashMap<[u8; 32], Embedding>>,
}
```

## hm-embed::HashFeatureEmbedder

<a id="rust-crates-hm-embed-src-hash-feature-rs-hashfeatureembedder"></a>

Source: [`crates/hm-embed/src/hash_feature.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/hash_feature.rs).

When to use: Use `HashFeatureEmbedder` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub struct HashFeatureEmbedder {
    dimensions: usize,
}
```

## hm-embed::OnnxEmbedder

<a id="rust-crates-hm-embed-src-local-rs-onnxembedder"></a>

Source: [`crates/hm-embed/src/local.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/local.rs).

When to use: Use `OnnxEmbedder` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub struct OnnxEmbedder {
    kind: ModelKind,
    tokenizer: Tokenizer,
    session: Mutex<Session>,
}
```

## hm-embed::ModelKind

<a id="rust-crates-hm-embed-src-model-rs-modelkind"></a>

Source: [`crates/hm-embed/src/model.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/model.rs).

When to use: Use `ModelKind` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub enum ModelKind {
    NomicEmbedTextV15,
    BgeSmallEnV15,
}
```

## hm-embed::Artifact

<a id="rust-crates-hm-embed-src-model-rs-artifact"></a>

Source: [`crates/hm-embed/src/model.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/model.rs).

When to use: Use `Artifact` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub struct Artifact {
    pub name: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
}
```

## hm-embed::ModelSpec

<a id="rust-crates-hm-embed-src-model-rs-modelspec"></a>

Source: [`crates/hm-embed/src/model.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/model.rs).

When to use: Use `ModelSpec` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub struct ModelSpec {
    pub kind: ModelKind,
    pub encoder_id: &'static str,
    pub revision: &'static str,
    pub dimensions: usize,
    pub maximum_tokens: usize,
    pub model: Artifact,
    pub tokenizer: Artifact,
}
```

## hm-embed::ArtifactFetcher

<a id="rust-crates-hm-embed-src-model-rs-artifactfetcher"></a>

Source: [`crates/hm-embed/src/model.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/model.rs).

When to use: Use `ArtifactFetcher` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub trait ArtifactFetcher: Send + Sync {
    fn fetch(&self, url: &str) -> Result<Vec<u8>, EmbedError>;
}
```

## hm-embed::HttpFetcher

<a id="rust-crates-hm-embed-src-model-rs-httpfetcher"></a>

Source: [`crates/hm-embed/src/model.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/model.rs).

When to use: Use `HttpFetcher` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub struct HttpFetcher;
```

## hm-embed::ModelStore

<a id="rust-crates-hm-embed-src-model-rs-modelstore"></a>

Source: [`crates/hm-embed/src/model.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/model.rs).

When to use: Use `ModelStore` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub struct ModelStore {
    root: PathBuf,
}
```

## hm-embed::ModelFiles

<a id="rust-crates-hm-embed-src-model-rs-modelfiles"></a>

Source: [`crates/hm-embed/src/model.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/model.rs).

When to use: Use `ModelFiles` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub struct ModelFiles {
    pub model: PathBuf,
    pub tokenizer: PathBuf,
}
```

## hm-embed::QuantizedEmbedding

<a id="rust-crates-hm-embed-src-quantize-rs-quantizedembedding"></a>

Source: [`crates/hm-embed/src/quantize.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/quantize.rs).

When to use: Use `QuantizedEmbedding` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub struct QuantizedEmbedding {
    pub space: SpaceIdentity,
    pub values: Vec<i8>,
    pub binary_prefilter: Vec<u8>,
    pub inverse_scale: f32,
}
```

## hm-embed::Provider

<a id="rust-crates-hm-embed-src-remote-rs-provider"></a>

Source: [`crates/hm-embed/src/remote.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/remote.rs).

When to use: Use `Provider` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub enum Provider {
    OpenAi,
    Voyage,
    Vertex,
    Ollama,
}
```

## hm-embed::RemoteConfig

<a id="rust-crates-hm-embed-src-remote-rs-remoteconfig"></a>

Source: [`crates/hm-embed/src/remote.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/remote.rs).

When to use: Use `RemoteConfig` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity. Set explicit deployment limits before opening the associated resource.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials. Do not log secret fields or substitute defaults for an explicitly authorized budget.


```rust
pub struct RemoteConfig {
    pub endpoint: String,
    pub api_key: Option<String>,
    pub model: String,
    pub revision: String,
    pub dimensions: usize,
    pub maximum_batch: usize,
}
```

## hm-embed::WireRequest

<a id="rust-crates-hm-embed-src-remote-rs-wirerequest"></a>

Source: [`crates/hm-embed/src/remote.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/remote.rs).

When to use: Use `WireRequest` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct WireRequest {
    pub method: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub body: Value,
}
```

## hm-embed::WireResponse

<a id="rust-crates-hm-embed-src-remote-rs-wireresponse"></a>

Source: [`crates/hm-embed/src/remote.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/remote.rs).

When to use: Use `WireResponse` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct WireResponse {
    pub status: u16,
    pub body: Value,
}
```

## hm-embed::WireTransport

<a id="rust-crates-hm-embed-src-remote-rs-wiretransport"></a>

Source: [`crates/hm-embed/src/remote.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/remote.rs).

When to use: Use `WireTransport` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub trait WireTransport: Send + Sync {
    fn send(&self, request: &WireRequest) -> Result<WireResponse, EmbedError>;
}
```

## hm-embed::WireFixture

<a id="rust-crates-hm-embed-src-remote-rs-wirefixture"></a>

Source: [`crates/hm-embed/src/remote.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/remote.rs).

When to use: Use `WireFixture` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub struct WireFixture {
    pub request: WireRequest,
    pub response: WireResponse,
}
```

## hm-embed::RecordedTransport

<a id="rust-crates-hm-embed-src-remote-rs-recordedtransport"></a>

Source: [`crates/hm-embed/src/remote.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/remote.rs).

When to use: Use `RecordedTransport` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub struct RecordedTransport {
    fixtures: Mutex<Vec<WireFixture>>,
}
```

## hm-embed::HttpTransport

<a id="rust-crates-hm-embed-src-remote-rs-httptransport"></a>

Source: [`crates/hm-embed/src/remote.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/remote.rs).

When to use: Use `HttpTransport` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub struct HttpTransport {
    client: reqwest::blocking::Client,
}
```

## hm-embed::RemoteEmbedder

<a id="rust-crates-hm-embed-src-remote-rs-remoteembedder"></a>

Source: [`crates/hm-embed/src/remote.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/remote.rs).

When to use: Use `RemoteEmbedder` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub struct RemoteEmbedder<T> {
    provider: Provider,
    config: RemoteConfig,
    transport: T,
}
```

## hm-embed::Distance

<a id="rust-crates-hm-embed-src-types-rs-distance"></a>

Source: [`crates/hm-embed/src/types.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/types.rs).

When to use: Use `Distance` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub enum Distance {
    Cosine,
    Dot,
}
```

## hm-embed::Normalization

<a id="rust-crates-hm-embed-src-types-rs-normalization"></a>

Source: [`crates/hm-embed/src/types.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/types.rs).

When to use: Use `Normalization` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub enum Normalization {
    None,
    L2,
}
```

## hm-embed::InputRole

<a id="rust-crates-hm-embed-src-types-rs-inputrole"></a>

Source: [`crates/hm-embed/src/types.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/types.rs).

When to use: Use `InputRole` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub enum InputRole {
    Query,
    Document,
}
```

## hm-embed::SpaceIdentity

<a id="rust-crates-hm-embed-src-types-rs-spaceidentity"></a>

Source: [`crates/hm-embed/src/types.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/types.rs).

When to use: Use `SpaceIdentity` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub struct SpaceIdentity {
    pub encoder_id: String,
    pub revision: String,
    pub dimensions: usize,
    pub distance: Distance,
    pub normalization: Normalization,
    pub input_role: InputRole,
}
```

## hm-embed::Embedding

<a id="rust-crates-hm-embed-src-types-rs-embedding"></a>

Source: [`crates/hm-embed/src/types.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/types.rs).

When to use: Use `Embedding` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub struct Embedding {
    pub space: SpaceIdentity,
    pub values: Vec<f32>,
}
```

## hm-embed::Embedder

<a id="rust-crates-hm-embed-src-types-rs-embedder"></a>

Source: [`crates/hm-embed/src/types.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/types.rs).

When to use: Use `Embedder` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub trait Embedder: Send + Sync {
    fn identity(&self, role: InputRole) -> SpaceIdentity;

    fn embed_query(&self, query: &str) -> Result<Embedding, EmbedError>;

    fn embed_documents(&self, documents: &[&str]) -> Result<Vec<Embedding>, EmbedError>;

    fn report_label(&self) -> String {
        self.identity(InputRole::Document).encoder_id
    }
}
```

## hm-embed::EmbedError

<a id="rust-crates-hm-embed-src-types-rs-embederror"></a>

Source: [`crates/hm-embed/src/types.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-embed/src/types.rs).

When to use: Use `EmbedError` for pre-append or query embedding with explicit model, dimension, space identity, and cache identity.

Do not use: Do not mix embedding spaces, embed historical documents on the recall hot path, or expose provider credentials.


```rust
pub enum EmbedError {
    InvalidArgument(&'static str),
    Dimension { expected: usize, actual: usize },
    ArtifactDigest { name: String },
    Io(std::io::Error),
    Network(String),
    Runtime(String),
    Tokenizer(String),
    Wire(String),
}
```

## hm-eval::ProbeKind

<a id="rust-crates-hm-eval-src-bench-beam-rs-probekind"></a>

Source: [`crates/hm-eval/src/bench/beam.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/beam.rs).

When to use: Use `ProbeKind` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub enum ProbeKind {
    InformationExtraction,
    TemporalReasoning,
    MultiSessionReasoning,
    ContradictionResolution,
    EventOrdering,
    KnowledgeUpdate,
    Summarization,
    Abstention,
    PreferenceFollowing,
    InstructionFollowing,
}
```

## hm-eval::Message

<a id="rust-crates-hm-eval-src-bench-beam-rs-message"></a>

Source: [`crates/hm-eval/src/bench/beam.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/beam.rs).

When to use: Use `Message` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct Message {
    pub id: u64,
    pub role: BenchRole,
    pub speaker: String,
    pub text: String,
    pub occurred_at: String,
}
```

## hm-eval::Session

<a id="rust-crates-hm-eval-src-bench-beam-rs-session"></a>

Source: [`crates/hm-eval/src/bench/beam.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/beam.rs).

When to use: Use `Session` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct Session {
    pub index: usize,
    pub label: String,
    pub messages: Vec<Message>,
}
```

## hm-eval::Probe

<a id="rust-crates-hm-eval-src-bench-beam-rs-probe"></a>

Source: [`crates/hm-eval/src/bench/beam.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/beam.rs).

When to use: Use `Probe` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct Probe {
    pub id: String,
    pub kind: ProbeKind,
    pub question: String,
    pub reference_answer: String,
    pub criteria: Vec<String>,
    pub difficulty: String,
    pub evidence_message_ids: Vec<u64>,
}
```

## hm-eval::Conversation

<a id="rust-crates-hm-eval-src-bench-beam-rs-conversation"></a>

Source: [`crates/hm-eval/src/bench/beam.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/beam.rs).

When to use: Use `Conversation` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct Conversation {
    pub id: String,
    pub sessions: Vec<Session>,
    pub probes: Vec<Probe>,
}
```

## hm-eval::ProbeSet

<a id="rust-crates-hm-eval-src-bench-beam-rs-probeset"></a>

Source: [`crates/hm-eval/src/bench/beam.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/beam.rs).

When to use: Use `ProbeSet` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct ProbeSet {
    pub format: String,
    pub source_digest: String,
    pub conversations: Vec<Conversation>,
}
```

## hm-eval::DynError

<a id="rust-crates-hm-eval-src-bench-gateway-rs-dynerror"></a>

Source: [`crates/hm-eval/src/bench/gateway.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/gateway.rs).

When to use: Use `DynError` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub type DynError = Box<dyn std::error::Error + Send + Sync>;
```

## hm-eval::CompletionSettings

<a id="rust-crates-hm-eval-src-bench-gateway-rs-completionsettings"></a>

Source: [`crates/hm-eval/src/bench/gateway.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/gateway.rs).

When to use: Use `CompletionSettings` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct CompletionSettings {
    pub max_output_tokens: u32,
    pub reasoning_effort: &'static str,
}
```

## hm-eval::Usage

<a id="rust-crates-hm-eval-src-bench-gateway-rs-usage"></a>

Source: [`crates/hm-eval/src/bench/gateway.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/gateway.rs).

When to use: Use `Usage` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub reasoning_tokens: u64,
    pub cached_input_tokens: u64,
    pub cost_microusd: u64,
    pub cost_source: String,
}
```

## hm-eval::Completion

<a id="rust-crates-hm-eval-src-bench-gateway-rs-completion"></a>

Source: [`crates/hm-eval/src/bench/gateway.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/gateway.rs).

When to use: Use `Completion` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct Completion {
    pub text: String,
    pub model: String,
    pub response_model: String,
    pub canonical_snapshot: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub canonical_model: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model_identity_kind: String,
    pub model_catalog_source: String,
    pub model_catalog_observed_at: String,
    pub request_hash: String,
    pub cached: bool,
    pub usage: Usage,
}
```

## hm-eval::BudgetSummary

<a id="rust-crates-hm-eval-src-bench-gateway-rs-budgetsummary"></a>

Source: [`crates/hm-eval/src/bench/gateway.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/gateway.rs).

When to use: Use `BudgetSummary` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct BudgetSummary {
    pub budget_microusd: u64,
    pub charged_microusd: u64,
    pub retained_reservations_microusd: u64,
    pub available_microusd: u64,
    pub dispatched_calls: usize,
    pub completed_calls: usize,
    pub unknown_calls: usize,
    #[serde(default)]
    pub upstream_usage_accounted_microusd: u64,
}
```

## hm-eval::Gateway

<a id="rust-crates-hm-eval-src-bench-gateway-rs-gateway"></a>

Source: [`crates/hm-eval/src/bench/gateway.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/gateway.rs).

When to use: Use `Gateway` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct Gateway {
    inner: Arc<GatewayState>,
}
```

## hm-eval::Dataset

<a id="rust-crates-hm-eval-src-bench-locomo-rs-dataset"></a>

Source: [`crates/hm-eval/src/bench/locomo.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/locomo.rs).

When to use: Use `Dataset` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct Dataset {
    pub conversations: Vec<Conversation>,
    pub sha256: String,
}
```

## hm-eval::Conversation

<a id="rust-crates-hm-eval-src-bench-locomo-rs-conversation"></a>

Source: [`crates/hm-eval/src/bench/locomo.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/locomo.rs).

When to use: Use `Conversation` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct Conversation {
    pub sample_id: String,
    pub conversation: BTreeMap<String, Value>,
    pub qa: Vec<Question>,
}
```

## hm-eval::Question

<a id="rust-crates-hm-eval-src-bench-locomo-rs-question"></a>

Source: [`crates/hm-eval/src/bench/locomo.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/locomo.rs).

When to use: Use `Question` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct Question {
    pub question: String,
    #[serde(default)]
    pub answer: Value,
    pub category: u8,
}
```

## hm-eval::Prediction

<a id="rust-crates-hm-eval-src-bench-locomo-rs-prediction"></a>

Source: [`crates/hm-eval/src/bench/locomo.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/locomo.rs).

When to use: Use `Prediction` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct Prediction {
    pub question_id: String,
    #[serde(alias = "answer", alias = "response")]
    pub hypothesis: String,
}
```

## hm-eval::CategoryResult

<a id="rust-crates-hm-eval-src-bench-locomo-rs-categoryresult"></a>

Source: [`crates/hm-eval/src/bench/locomo.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/locomo.rs).

When to use: Use `CategoryResult` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct CategoryResult {
    pub total: usize,
    pub evaluated: usize,
    pub mean: f64,
}
```

## hm-eval::JudgeFreeMetrics

<a id="rust-crates-hm-eval-src-bench-locomo-rs-judgefreemetrics"></a>

Source: [`crates/hm-eval/src/bench/locomo.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/locomo.rs).

When to use: Use `JudgeFreeMetrics` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct JudgeFreeMetrics {
    pub official_all_categories_mean: f64,
    pub non_adversarial_f1: f64,
    pub non_adversarial_count: usize,
    pub adversarial_accuracy: f64,
    pub adversarial_count: usize,
    pub by_category: BTreeMap<u8, CategoryResult>,
}
```

## hm-eval::JudgedMetrics

<a id="rust-crates-hm-eval-src-bench-locomo-rs-judgedmetrics"></a>

Source: [`crates/hm-eval/src/bench/locomo.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/locomo.rs).

When to use: Use `JudgedMetrics` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct JudgedMetrics {
    pub model: String,
    pub prompt_version: String,
    pub total: usize,
    pub evaluated: usize,
    pub correct: usize,
    pub accuracy: f64,
    pub cache_hits: usize,
}
```

## hm-eval::BenchmarkResult

<a id="rust-crates-hm-eval-src-bench-locomo-rs-benchmarkresult"></a>

Source: [`crates/hm-eval/src/bench/locomo.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/locomo.rs).

When to use: Use `BenchmarkResult` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct BenchmarkResult {
    pub schema_version: u32,
    pub dataset_revision: String,
    pub dataset_sha256: String,
    pub scorer_sha256: String,
    pub input_variant: String,
    pub total: usize,
    pub evaluated: usize,
    pub complete: bool,
    pub reader_model: String,
    pub retrieval_mode: String,
    pub reader_cache_hits: usize,
    pub judge_free: JudgeFreeMetrics,
    pub judged: Option<JudgedMetrics>,
    pub failure: Option<String>,
}
```

## hm-eval::Example

<a id="rust-crates-hm-eval-src-bench-longmemeval-rs-example"></a>

Source: [`crates/hm-eval/src/bench/longmemeval.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/longmemeval.rs).

When to use: Use `Example` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct Example {
    pub question_id: String,
    pub question: String,
    #[serde(deserialize_with = "answer_text")]
    pub answer: String,
}
```

## hm-eval::Prediction

<a id="rust-crates-hm-eval-src-bench-longmemeval-rs-prediction"></a>

Source: [`crates/hm-eval/src/bench/longmemeval.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/longmemeval.rs).

When to use: Use `Prediction` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct Prediction {
    pub question_id: String,
    #[serde(alias = "response", alias = "answer")]
    pub hypothesis: String,
}
```

## hm-eval::BenchmarkResult

<a id="rust-crates-hm-eval-src-bench-longmemeval-rs-benchmarkresult"></a>

Source: [`crates/hm-eval/src/bench/longmemeval.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/longmemeval.rs).

When to use: Use `BenchmarkResult` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct BenchmarkResult {
    pub total: usize,
    pub evaluated: usize,
    pub correct: usize,
    pub cache_hits: usize,
}
```

## hm-eval::GatewayJudge

<a id="rust-crates-hm-eval-src-bench-longmemeval-rs-gatewayjudge"></a>

Source: [`crates/hm-eval/src/bench/longmemeval.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/longmemeval.rs).

When to use: Use `GatewayJudge` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct GatewayJudge {
    endpoint: String,
    api_key: String,
    client: reqwest::blocking::Client,
}
```

## hm-eval::HistoryTurn

<a id="rust-crates-hm-eval-src-bench-longmemeval-rs-historyturn"></a>

Source: [`crates/hm-eval/src/bench/longmemeval.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/longmemeval.rs).

When to use: Use `HistoryTurn` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct HistoryTurn {
    pub role: String,
    pub content: String,
}
```

## hm-eval::FullExample

<a id="rust-crates-hm-eval-src-bench-longmemeval-rs-fullexample"></a>

Source: [`crates/hm-eval/src/bench/longmemeval.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/longmemeval.rs).

When to use: Use `FullExample` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct FullExample {
    #[serde(flatten)]
    pub example: Example,
    pub question_type: String,
    pub question_date: String,
    pub haystack_dates: Vec<String>,
    pub haystack_session_ids: Vec<String>,
    pub haystack_sessions: Vec<Vec<HistoryTurn>>,
}
```

## hm-eval::CategoryResult

<a id="rust-crates-hm-eval-src-bench-longmemeval-rs-categoryresult"></a>

Source: [`crates/hm-eval/src/bench/longmemeval.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/longmemeval.rs).

When to use: Use `CategoryResult` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct CategoryResult {
    pub total: usize,
    pub evaluated: usize,
    pub correct: usize,
}
```

## hm-eval::FullBenchmarkResult

<a id="rust-crates-hm-eval-src-bench-longmemeval-rs-fullbenchmarkresult"></a>

Source: [`crates/hm-eval/src/bench/longmemeval.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/longmemeval.rs).

When to use: Use `FullBenchmarkResult` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct FullBenchmarkResult {
    pub diagnostic: bool,
    pub selected_question_ids: Vec<String>,
    pub question_outcomes: BTreeMap<String, bool>,
    pub dataset_revision: String,
    pub dataset_sha256: String,
    pub dataset_url: String,
    pub scorer_revision: String,
    pub prompt_version: String,
    pub reader_model: String,
    pub reader_canonical_model: String,
    pub reader_canonical_snapshot: String,
    pub model_identity_kind: String,
    pub judge_model: String,
    pub reader_settings: Value,
    pub judge_settings: Value,
    pub retrieval_mode: String,
    pub total: usize,
    pub generated: usize,
    pub evaluated: usize,
    pub correct: usize,
    pub reader_cache_hits: usize,
    pub judge_cache_hits: usize,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub cached_input_tokens: u64,
    pub cost_microusd: u64,
    pub new_cost_microusd: u64,
    pub cost_basis: String,
    pub reported_cost_microusd: u64,
    pub reserved_cost_microusd: u64,
    pub new_reported_cost_microusd: u64,
    pub new_reserved_cost_microusd: u64,
    pub by_category: BTreeMap<String, CategoryResult>,
    pub failure: Option<String>,
}
```

## hm-eval::BenchRole

<a id="rust-crates-hm-eval-src-bench-pipeline-rs-benchrole"></a>

Source: [`crates/hm-eval/src/bench/pipeline.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/pipeline.rs).

When to use: Use `BenchRole` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub enum BenchRole {
    User,
    Assistant,
    Other,
}
```

## hm-eval::BenchDocument

<a id="rust-crates-hm-eval-src-bench-pipeline-rs-benchdocument"></a>

Source: [`crates/hm-eval/src/bench/pipeline.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/pipeline.rs).

When to use: Use `BenchDocument` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct BenchDocument {
    pub id: String,
    pub conversation: String,
    pub date: String,
    pub role: BenchRole,
    pub speaker: String,
    pub text: String,
}
```

## hm-eval::BenchQuestion

<a id="rust-crates-hm-eval-src-bench-pipeline-rs-benchquestion"></a>

Source: [`crates/hm-eval/src/bench/pipeline.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/pipeline.rs).

When to use: Use `BenchQuestion` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct BenchQuestion {
    pub id: String,
    pub text: String,
    pub date: Option<String>,
}
```

## hm-eval::BenchCitation

<a id="rust-crates-hm-eval-src-bench-pipeline-rs-benchcitation"></a>

Source: [`crates/hm-eval/src/bench/pipeline.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/pipeline.rs).

When to use: Use `BenchCitation` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct BenchCitation {
    pub source_ref: String,
    pub session_ref: String,
    pub lsn: u64,
    pub uri: String,
    pub rank: usize,
    pub document_ids: Vec<String>,
    pub conversation: String,
    pub date: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub role: BenchRole,
    pub speaker: String,
    pub source_byte_start: usize,
    pub source_byte_end: usize,
    pub source_document_bytes: usize,
    pub partial_document: bool,
}
```

## hm-eval::ContextCoverage

<a id="rust-crates-hm-eval-src-bench-pipeline-rs-contextcoverage"></a>

Source: [`crates/hm-eval/src/bench/pipeline.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/pipeline.rs).

When to use: Use `ContextCoverage` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct ContextCoverage {
    pub candidate_chunks: usize,
    pub candidate_passages: usize,
    pub candidate_sessions: usize,
    pub included_passages: usize,
    pub included_sessions: usize,
    pub candidate_characters_by_role: BTreeMap<String, usize>,
    pub included_characters_by_role: BTreeMap<String, usize>,
    pub skipped_budget_passages: usize,
    pub skipped_oversize_passages: usize,
    pub partial_document_passages: usize,
    pub context_characters: usize,
    pub source_characters: usize,
    pub context_limit: usize,
    pub truncated: bool,
}
```

## hm-eval::RetrievedContext

<a id="rust-crates-hm-eval-src-bench-pipeline-rs-retrievedcontext"></a>

Source: [`crates/hm-eval/src/bench/pipeline.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/pipeline.rs).

When to use: Use `RetrievedContext` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct RetrievedContext {
    pub provenance: Vec<BenchCitation>,
    pub excerpts: Vec<serde_json::Value>,
    pub coverage: ContextCoverage,
}
```

## hm-eval::BenchAnswer

<a id="rust-crates-hm-eval-src-bench-pipeline-rs-benchanswer"></a>

Source: [`crates/hm-eval/src/bench/pipeline.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/pipeline.rs).

When to use: Use `BenchAnswer` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct BenchAnswer {
    pub question_id: String,
    pub answer: String,
    pub reader: Completion,
    pub provenance: Vec<BenchCitation>,
    pub retrieval_mode: String,
    pub history_digest: String,
    pub dataset_hash: String,
    pub pipeline_version: String,
    pub implementation_digest: String,
    pub question_digest: String,
    pub ingested_documents: usize,
    pub ingested_chunks: usize,
    pub retrieved_characters: usize,
    pub context_coverage: ContextCoverage,
    pub request_policy_id: String,
}
```

## hm-eval::MemoryPipeline

<a id="rust-crates-hm-eval-src-bench-pipeline-rs-memorypipeline"></a>

Source: [`crates/hm-eval/src/bench/pipeline.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/bench/pipeline.rs).

When to use: Use `MemoryPipeline` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct MemoryPipeline {
    actor: ActorEngine,
    root: PathBuf,
    dataset_hash: String,
    history_id: String,
    history_digest: String,
    implementation_digest: String,
    documents: usize,
    chunks: Vec<Chunk>,
    _process_lock: File,
}
```

## hm-eval::Phase

<a id="rust-crates-hm-eval-src-contract-mod-rs-phase"></a>

Source: [`crates/hm-eval/src/contract/mod.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/contract/mod.rs).

When to use: Use `Phase` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub enum Phase {
    BeforeRestart,
    AfterRestart,
}
```

## hm-eval::ScenarioCall

<a id="rust-crates-hm-eval-src-contract-mod-rs-scenariocall"></a>

Source: [`crates/hm-eval/src/contract/mod.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/contract/mod.rs).

When to use: Use `ScenarioCall` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct ScenarioCall {
    pub step: &'static str,
    pub verb: &'static str,
    pub arguments: Value,
    pub phase: Phase,
}
```

## hm-eval::StepObservation

<a id="rust-crates-hm-eval-src-contract-mod-rs-stepobservation"></a>

Source: [`crates/hm-eval/src/contract/mod.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/contract/mod.rs).

When to use: Use `StepObservation` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct StepObservation {
    pub step: String,
    pub verb: String,
    pub ok: bool,
    pub error_code: Option<String>,
    pub effect_state: Option<String>,
    pub item_identifiers: Vec<String>,
    pub provenance: Vec<String>,
    pub authority: Vec<String>,
    pub health: BTreeMap<String, String>,
    pub gap_kinds: Vec<String>,
    pub warning_kinds: Vec<String>,
}
```

## hm-eval::ContractObservation

<a id="rust-crates-hm-eval-src-contract-mod-rs-contractobservation"></a>

Source: [`crates/hm-eval/src/contract/mod.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/contract/mod.rs).

When to use: Use `ContractObservation` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct ContractObservation {
    pub format: String,
    pub sdk: String,
    pub transport: String,
    pub steps: Vec<StepObservation>,
}
```

## hm-eval::RawCall

<a id="rust-crates-hm-eval-src-contract-mod-rs-rawcall"></a>

Source: [`crates/hm-eval/src/contract/mod.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/contract/mod.rs).

When to use: Use `RawCall` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct RawCall {
    pub step: String,
    pub verb: String,
    pub envelope: Value,
}
```

## hm-eval::RawDocument

<a id="rust-crates-hm-eval-src-contract-mod-rs-rawdocument"></a>

Source: [`crates/hm-eval/src/contract/mod.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/contract/mod.rs).

When to use: Use `RawDocument` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct RawDocument {
    pub format: String,
    pub sdk: String,
    pub transport: String,
    pub calls: Vec<RawCall>,
}
```

## hm-eval::Unavailable

<a id="rust-crates-hm-eval-src-contract-mod-rs-unavailable"></a>

Source: [`crates/hm-eval/src/contract/mod.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/contract/mod.rs).

When to use: Use `Unavailable` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct Unavailable {
    pub sdk: String,
    pub reason: String,
}
```

## hm-eval::Unsupported

<a id="rust-crates-hm-eval-src-contract-mod-rs-unsupported"></a>

Source: [`crates/hm-eval/src/contract/mod.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/contract/mod.rs).

When to use: Use `Unsupported` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct Unsupported {
    pub sdk: String,
    pub step: String,
}
```

## hm-eval::Divergence

<a id="rust-crates-hm-eval-src-contract-mod-rs-divergence"></a>

Source: [`crates/hm-eval/src/contract/mod.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/contract/mod.rs).

When to use: Use `Divergence` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct Divergence {
    pub step: String,
    pub field: String,
    pub values: BTreeMap<String, String>,
}
```

## hm-eval::ContractComparison

<a id="rust-crates-hm-eval-src-contract-mod-rs-contractcomparison"></a>

Source: [`crates/hm-eval/src/contract/mod.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/contract/mod.rs).

When to use: Use `ContractComparison` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct ContractComparison {
    pub format: String,
    pub participants: Vec<String>,
    pub unavailable: Vec<Unavailable>,
    pub unsupported: Vec<Unsupported>,
    pub steps_compared: usize,
    pub divergences: Vec<Divergence>,
    pub agreed: bool,
}
```

## hm-eval::Metric

<a id="rust-crates-hm-eval-src-slice1-rs-metric"></a>

Source: [`crates/hm-eval/src/slice1.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/slice1.rs).

When to use: Use `Metric` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct Metric {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub tolerance: f64,
    pub higher_is_better: bool,
    pub judged: bool,
}
```

## hm-eval::GateResult

<a id="rust-crates-hm-eval-src-slice1-rs-gateresult"></a>

Source: [`crates/hm-eval/src/slice1.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/slice1.rs).

When to use: Use `GateResult` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct GateResult {
    pub schema_version: u32,
    pub slice: String,
    pub encoder: String,
    pub judge_model: Option<String>,
    pub suites: Vec<String>,
    pub judge_free: Vec<Metric>,
    pub judged: Vec<Metric>,
}
```

## hm-eval::SeedManifest

<a id="rust-crates-hm-eval-src-slice1-rs-seedmanifest"></a>

Source: [`crates/hm-eval/src/slice1.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/slice1.rs).

When to use: Use `SeedManifest` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct SeedManifest {
    pub seed: u64,
    pub event_counts: Vec<usize>,
    pub probe_count: usize,
}
```

## hm-eval::AttentionResult

<a id="rust-crates-hm-eval-src-suites-attention-rs-attentionresult"></a>

Source: [`crates/hm-eval/src/suites/attention.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/suites/attention.rs).

When to use: Use `AttentionResult` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct AttentionResult {
    pub fixture_hash: String,
    pub cases: usize,
    pub true_suppression: usize,
    pub false_suppression: usize,
    pub missed_suppression: usize,
    pub true_interruptions: usize,
    pub reasons_present: usize,
}
```

## hm-eval::CalibrationRow

<a id="rust-crates-hm-eval-src-suites-calibration-rs-calibrationrow"></a>

Source: [`crates/hm-eval/src/suites/calibration.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/suites/calibration.rs).

When to use: Use `CalibrationRow` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct CalibrationRow {
    pub predicate_kind: String,
    pub counts: CalibrationCounters,
    pub supported_among_resolved: f64,
}
```

## hm-eval::CalibrationResult

<a id="rust-crates-hm-eval-src-suites-calibration-rs-calibrationresult"></a>

Source: [`crates/hm-eval/src/suites/calibration.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/suites/calibration.rs).

When to use: Use `CalibrationResult` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct CalibrationResult {
    pub cases: usize,
    pub assessment_mismatches: usize,
    pub duplicate_writes: usize,
    pub restart_identical: bool,
    pub revision_required: bool,
    pub per_kind: Vec<CalibrationRow>,
}
```

## hm-eval::CitationResult

<a id="rust-crates-hm-eval-src-suites-citations-rs-citationresult"></a>

Source: [`crates/hm-eval/src/suites/citations.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/suites/citations.rs).

When to use: Use `CitationResult` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct CitationResult {
    pub attempted: usize,
    pub accepted: usize,
    pub dropped: usize,
    pub escaped_invalid: usize,
}
```

## hm-eval::ContinuityResult

<a id="rust-crates-hm-eval-src-suites-continuity-rs-continuityresult"></a>

Source: [`crates/hm-eval/src/suites/continuity.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/suites/continuity.rs).

When to use: Use `ContinuityResult` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct ContinuityResult {
    pub rust_trials: usize,
    pub typescript_passed: bool,
}
```

## hm-eval::DegradationResult

<a id="rust-crates-hm-eval-src-suites-degradation-rs-degradationresult"></a>

Source: [`crates/hm-eval/src/suites/degradation.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/suites/degradation.rs).

When to use: Use `DegradationResult` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct DegradationResult {
    pub forced_conditions: usize,
    pub visible_conditions: usize,
}
```

## hm-eval::DreamResult

<a id="rust-crates-hm-eval-src-suites-dream-rs-dreamresult"></a>

Source: [`crates/hm-eval/src/suites/dream.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/suites/dream.rs).

When to use: Use `DreamResult` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct DreamResult {
    pub regression_cases: usize,
    pub lossy_rewrites: usize,
    pub preserved_cases: usize,
    pub duplicate_abstractions: usize,
    pub valid_mints: usize,
    pub ungrounded_mints: usize,
}
```

## hm-eval::GenerationResult

<a id="rust-crates-hm-eval-src-suites-generations-rs-generationresult"></a>

Source: [`crates/hm-eval/src/suites/generations.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/suites/generations.rs).

When to use: Use `GenerationResult` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct GenerationResult {
    pub staged_hidden_after_kill: bool,
    pub publish_after_restart: bool,
    pub lost_ack_deduplicated: bool,
    pub published_generation_count: usize,
    pub rollback_immediate: bool,
    pub leased_view_overridden: bool,
}
```

## hm-eval::ParityResult

<a id="rust-crates-hm-eval-src-suites-hnsw-parity-rs-parityresult"></a>

Source: [`crates/hm-eval/src/suites/hnsw_parity.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/suites/hnsw_parity.rs).

When to use: Use `ParityResult` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct ParityResult {
    pub encoder: &'static str,
    pub seed: u64,
    pub vectors: usize,
    pub queries: usize,
    pub matched: usize,
    pub expected: usize,
    pub recall_at_10: f64,
}
```

## hm-eval::LaunderingResult

<a id="rust-crates-hm-eval-src-suites-laundering-rs-launderingresult"></a>

Source: [`crates/hm-eval/src/suites/laundering.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/suites/laundering.rs).

When to use: Use `LaunderingResult` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct LaunderingResult {
    pub attempts: usize,
    pub violations: usize,
}
```

## hm-eval::ProtectedResult

<a id="rust-crates-hm-eval-src-suites-protected-rs-protectedresult"></a>

Source: [`crates/hm-eval/src/suites/protected.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/suites/protected.rs).

When to use: Use `ProtectedResult` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct ProtectedResult {
    pub attempts: usize,
    pub rejected: usize,
    pub surfaced_proposals: usize,
}
```

## hm-eval::EncoderRun

<a id="rust-crates-hm-eval-src-suites-recall-rs-encoderrun"></a>

Source: [`crates/hm-eval/src/suites/recall.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/suites/recall.rs).

When to use: Use `EncoderRun` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass.


```rust
pub struct EncoderRun {
    pub encoder: String,
    pub metrics: Vec<Metric>,
}
```

## hm-eval::TemporalResult

<a id="rust-crates-hm-eval-src-suites-temporal-rs-temporalresult"></a>

Source: [`crates/hm-eval/src/suites/temporal.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-eval/src/suites/temporal.rs).

When to use: Use `TemporalResult` for reproducible benchmark inputs, metrics, coverage, budget accounting, and declared qualification gates. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not present selected diagnostics, fixture responses, missing rows, or incomplete coverage as a full live benchmark pass. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct TemporalResult {
    pub axis_cases: usize,
    pub axis_correct: usize,
    pub stale_claims: usize,
    pub stated_claims: usize,
}
```

## hm-index::EntityKind

<a id="rust-crates-hm-index-src-entity-rules-rs-entitykind"></a>

Source: [`crates/hm-index/src/entity_rules.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-index/src/entity_rules.rs).

When to use: Use `EntityKind` for bounded lexical, entity, or vector candidate retrieval and reproducible index scoring.

Do not use: Do not compare raw scores across embedding spaces or represent lexical-only results as semantic retrieval.


```rust
pub enum EntityKind {
    Url,
    Domain,
    Path,
    HexId,
    StructuredId,
    ProperNoun,
}
```

## hm-index::Entity

<a id="rust-crates-hm-index-src-entity-rules-rs-entity"></a>

Source: [`crates/hm-index/src/entity_rules.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-index/src/entity_rules.rs).

When to use: Use `Entity` for bounded lexical, entity, or vector candidate retrieval and reproducible index scoring.

Do not use: Do not compare raw scores across embedding spaces or represent lexical-only results as semantic retrieval.


```rust
pub struct Entity {
    pub canonical: String,
    pub aliases: Vec<String>,
    pub kind: EntityKind,
}
```

## hm-index::HnswIndex

<a id="rust-crates-hm-index-src-hnsw-rs-hnswindex"></a>

Source: [`crates/hm-index/src/hnsw.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-index/src/hnsw.rs).

When to use: Use `HnswIndex` for bounded lexical, entity, or vector candidate retrieval and reproducible index scoring.

Do not use: Do not compare raw scores across embedding spaces or represent lexical-only results as semantic retrieval.


```rust
pub struct HnswIndex {
    index: Index,
    dimensions: usize,
}
```

## hm-index::AliasSuggestion

<a id="rust-crates-hm-index-src-vocabulary-rs-aliassuggestion"></a>

Source: [`crates/hm-index/src/vocabulary.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-index/src/vocabulary.rs).

When to use: Use `AliasSuggestion` for bounded lexical, entity, or vector candidate retrieval and reproducible index scoring.

Do not use: Do not compare raw scores across embedding spaces or represent lexical-only results as semantic retrieval.


```rust
pub struct AliasSuggestion {
    pub candidate: String,
    pub similarity_q16: u32,
}
```

## hm-ledger::Clock

<a id="rust-crates-hm-ledger-src-apply-rs-clock"></a>

Source: [`crates/hm-ledger/src/apply.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/apply.rs).

When to use: Use `Clock` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub trait Clock: Send {
    fn now_ns(&mut self) -> Result<UtcNanos, Error>;
}
```

## hm-ledger::StorageCommit

<a id="rust-crates-hm-ledger-src-apply-rs-storagecommit"></a>

Source: [`crates/hm-ledger/src/apply.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/apply.rs).

When to use: Use `StorageCommit` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct StorageCommit {
    pub first_lsn: LSN,
    pub last_lsn: LSN,
}
```

## hm-ledger::Storage

<a id="rust-crates-hm-ledger-src-apply-rs-storage"></a>

Source: [`crates/hm-ledger/src/apply.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/apply.rs).

When to use: Use `Storage` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub trait Storage: Send {
    fn append(&mut self, frames: &[Frame]) -> Result<StorageCommit, Error>;
    fn recover(&mut self) -> Result<Vec<Frame>, Error>;
    fn read_from(&mut self, first_lsn: LSN, maximum_frames: usize) -> Result<Vec<Frame>, Error>;
}
```

## hm-ledger::ApplyRequest

<a id="rust-crates-hm-ledger-src-apply-rs-applyrequest"></a>

Source: [`crates/hm-ledger/src/apply.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/apply.rs).

When to use: Use `ApplyRequest` for durable encrypted append, recovery, key lifecycle, and integrity verification. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct ApplyRequest {
    pub kind: EventKind,
    pub conversation: ConversationId,
    pub plaintext_payload: Vec<u8>,
}
```

## hm-ledger::ApplyAck

<a id="rust-crates-hm-ledger-src-apply-rs-applyack"></a>

Source: [`crates/hm-ledger/src/apply.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/apply.rs).

When to use: Use `ApplyAck` for durable encrypted append, recovery, key lifecycle, and integrity verification. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct ApplyAck {
    pub lsn: LSN,
    pub wall_timestamp_ns: UtcNanos,
}
```

## hm-ledger::AppliedState

<a id="rust-crates-hm-ledger-src-apply-rs-appliedstate"></a>

Source: [`crates/hm-ledger/src/apply.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/apply.rs).

When to use: Use `AppliedState` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct AppliedState {
    last_lsn: LSN,
    event_count: u64,
    kind_counts: [u64; EVENT_KIND_COUNT],
    digest: [u8; 32],
}
```

## hm-ledger::ApplyLoop

<a id="rust-crates-hm-ledger-src-apply-rs-applyloop"></a>

Source: [`crates/hm-ledger/src/apply.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/apply.rs).

When to use: Use `ApplyLoop` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct ApplyLoop<'environment> {
    clock: &'environment mut dyn Clock,
    storage: &'environment mut dyn Storage,
    actor: ActorId,
    maximum_batch_size: usize,
    writer_thread: ThreadId,
    state: AppliedState,
}
```

## hm-ledger::SigningSeed

<a id="rust-crates-hm-ledger-src-checkpoint-rs-signingseed"></a>

Source: [`crates/hm-ledger/src/checkpoint.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/checkpoint.rs).

When to use: Use `SigningSeed` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub type SigningSeed = [u8;
```

## hm-ledger::PublicKey

<a id="rust-crates-hm-ledger-src-checkpoint-rs-publickey"></a>

Source: [`crates/hm-ledger/src/checkpoint.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/checkpoint.rs).

When to use: Use `PublicKey` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub type PublicKey = [u8;
```

## hm-ledger::Signature

<a id="rust-crates-hm-ledger-src-checkpoint-rs-signature"></a>

Source: [`crates/hm-ledger/src/checkpoint.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/checkpoint.rs).

When to use: Use `Signature` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub type Signature = [u8;
```

## hm-ledger::SigningKeyPair

<a id="rust-crates-hm-ledger-src-checkpoint-rs-signingkeypair"></a>

Source: [`crates/hm-ledger/src/checkpoint.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/checkpoint.rs).

When to use: Use `SigningKeyPair` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct SigningKeyPair {
    seed: Zeroizing<SigningSeed>,
    pub public_key: PublicKey,
}
```

## hm-ledger::Checkpoint

<a id="rust-crates-hm-ledger-src-checkpoint-rs-checkpoint"></a>

Source: [`crates/hm-ledger/src/checkpoint.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/checkpoint.rs).

When to use: Use `Checkpoint` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct Checkpoint {
    pub actor: ActorId,
    pub lsn: LSN,
    pub leaf_count: u64,
    pub root: Hash,
    pub public_key: PublicKey,
    pub signature: Signature,
}
```

## hm-ledger::CredentialVault

<a id="rust-crates-hm-ledger-src-credentials-rs-credentialvault"></a>

Source: [`crates/hm-ledger/src/credentials.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/credentials.rs).

When to use: Use `CredentialVault` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct CredentialVault {
    directory: PathBuf,
}
```

## hm-ledger::EventKind

<a id="rust-crates-hm-ledger-src-frame-rs-eventkind"></a>

Source: [`crates/hm-ledger/src/frame.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/frame.rs).

When to use: Use `EventKind` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub enum EventKind {
    UserMsg = 1,
    DeliveredMsg = 2,
    ToolCall = 3,
    ToolResult = 4,
    Reasoning = 5,
    ProviderFrame = 6,
    MediaRef = 7,
    Effect = 8,
    Approval = 9,
    Outcome = 10,
    Checkpoint = 11,
    Supervisor = 12,
    Recovery = 13,
    IntentSet = 14,
    LoopOpened = 15,
    LoopClosed = 16,
    Assertion = 17,
    Consolidation = 18,
    Embedding = 19,
    Retract = 20,
    Attestation = 21,
    Binding = 22,
    ProposedAssertion = 23,
    MemoryMinted = 24,
    MemoryRevised = 25,
    MemoryMerged = 26,
    MemoryFaded = 27,
    EdgeAsserted = 28,
    EdgeRetracted = 29,
    ConsolidationOpened = 30,
    ConsolidationPhase = 31,
    ConsolidationClosed = 32,
    ConsolidationRetracted = 33,
    Reviewed = 34,
    IntentionSet = 35,
    IntentionFired = 36,
    AttentionDecided = 37,
    IntentionCancelled = 38,
    Predicted = 39,
    OutcomeObserved = 40,
    ProcedureMined = 41,
    ProcedureRevised = 42,
    ProcedureAdopted = 43,
    VocabularyImported = 44,
    DocumentIngested = 45,
    DocumentExtracted = 46,
    DocumentChunked = 47,
    SourceConnectorBound = 48,
    SourceDeliveryAccepted = 49,
    SourceDeliverySettled = 50,
    SourceRevisionObserved = 51,
    ProcedureImported = 52,
    ProcedureImprovementProposed = 53,
}
```

## hm-ledger::FrameHeader

<a id="rust-crates-hm-ledger-src-frame-rs-frameheader"></a>

Source: [`crates/hm-ledger/src/frame.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/frame.rs).

When to use: Use `FrameHeader` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct FrameHeader {
    pub lsn: LSN,
    pub kind: EventKind,
    pub wall_timestamp_ns: UtcNanos,
    pub actor: ActorId,
    pub conversation: ConversationId,
}
```

## hm-ledger::Frame

<a id="rust-crates-hm-ledger-src-frame-rs-frame"></a>

Source: [`crates/hm-ledger/src/frame.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/frame.rs).

When to use: Use `Frame` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct Frame {
    pub header: FrameHeader,
    pub sealed_payload: Vec<u8>,
}
```

## hm-ledger::BeliefEvidence

<a id="rust-crates-hm-ledger-src-gate-rs-beliefevidence"></a>

Source: [`crates/hm-ledger/src/gate.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/gate.rs).

When to use: Use `BeliefEvidence` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub trait BeliefEvidence {
    fn has_successful_tool_observed_result(&self, first_lsn: LSN, last_lsn: LSN) -> bool;
}
```

## hm-ledger::LiveBelief

<a id="rust-crates-hm-ledger-src-gate-rs-livebelief"></a>

Source: [`crates/hm-ledger/src/gate.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/gate.rs).

When to use: Use `LiveBelief` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct LiveBelief {
    pub belief_id: Vec<u8>,
    pub belief_type: BeliefType,
    pub canonical_identity: String,
    pub conflict_domain: String,
}
```

## hm-ledger::BeliefConflict

<a id="rust-crates-hm-ledger-src-gate-rs-beliefconflict"></a>

Source: [`crates/hm-ledger/src/gate.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/gate.rs).

When to use: Use `BeliefConflict` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct BeliefConflict {
    pub event_index: usize,
    pub incoming_belief_id: Vec<u8>,
    pub existing_belief_id: Vec<u8>,
    pub belief_type: BeliefType,
    pub conflict_domain: String,
    pub incoming_identity: String,
    pub existing_identity: String,
}
```

## hm-ledger::GateRejection

<a id="rust-crates-hm-ledger-src-gate-rs-gaterejection"></a>

Source: [`crates/hm-ledger/src/gate.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/gate.rs).

When to use: Use `GateRejection` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct GateRejection {
    pub event_index: usize,
    pub error: Error,
}
```

## hm-ledger::BatchAdmission

<a id="rust-crates-hm-ledger-src-gate-rs-batchadmission"></a>

Source: [`crates/hm-ledger/src/gate.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/gate.rs).

When to use: Use `BatchAdmission` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct BatchAdmission {
    pub conflicts: Vec<BeliefConflict>,
}
```

## hm-ledger::ApplyAdmission

<a id="rust-crates-hm-ledger-src-gate-rs-applyadmission"></a>

Source: [`crates/hm-ledger/src/gate.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/gate.rs).

When to use: Use `ApplyAdmission` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct ApplyAdmission {
    pub admitted_indices: Vec<usize>,
    pub skipped: Vec<GateRejection>,
    pub conflicts: Vec<BeliefConflict>,
}
```

## hm-ledger::ConnectionId

<a id="rust-crates-hm-ledger-src-idempotency-rs-connectionid"></a>

Source: [`crates/hm-ledger/src/idempotency.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/idempotency.rs).

When to use: Use `ConnectionId` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub type ConnectionId = [u8;
```

## hm-ledger::BatchEvent

<a id="rust-crates-hm-ledger-src-idempotency-rs-batchevent"></a>

Source: [`crates/hm-ledger/src/idempotency.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/idempotency.rs).

When to use: Use `BatchEvent` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct BatchEvent<'payload> {
    pub kind: EventKind,
    pub conversation: ConversationId,
    pub plaintext_payload: &'payload [u8],
}
```

## hm-ledger::BatchIdentity

<a id="rust-crates-hm-ledger-src-idempotency-rs-batchidentity"></a>

Source: [`crates/hm-ledger/src/idempotency.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/idempotency.rs).

When to use: Use `BatchIdentity` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct BatchIdentity<'payload> {
    pub connection_id: ConnectionId,
    pub client_seq: u64,
    pub events: &'payload [BatchEvent<'payload>],
}
```

## hm-ledger::DedupState

<a id="rust-crates-hm-ledger-src-idempotency-rs-dedupstate"></a>

Source: [`crates/hm-ledger/src/idempotency.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/idempotency.rs).

When to use: Use `DedupState` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct DedupState {
    pub client_seq: u64,
    pub first_lsn: LSN,
    pub last_lsn: LSN,
    pub digest: [u8; 32],
}
```

## hm-ledger::Admission

<a id="rust-crates-hm-ledger-src-idempotency-rs-admission"></a>

Source: [`crates/hm-ledger/src/idempotency.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/idempotency.rs).

When to use: Use `Admission` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub enum Admission {
    Fresh { digest: [u8; 32] },
    Duplicate(DedupState),
}
```

## hm-ledger::DedupTable

<a id="rust-crates-hm-ledger-src-idempotency-rs-deduptable"></a>

Source: [`crates/hm-ledger/src/idempotency.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/idempotency.rs).

When to use: Use `DedupTable` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct DedupTable {
    entries: BTreeMap<ConnectionId, DedupState>,
    maximum_connections: usize,
}
```

## hm-ledger::KeyEncryptionKey

<a id="rust-crates-hm-ledger-src-keyring-rs-keyencryptionkey"></a>

Source: [`crates/hm-ledger/src/keyring.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/keyring.rs).

When to use: Use `KeyEncryptionKey` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub type KeyEncryptionKey = [u8;
```

## hm-ledger::UserId

<a id="rust-crates-hm-ledger-src-keyring-rs-userid"></a>

Source: [`crates/hm-ledger/src/keyring.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/keyring.rs).

When to use: Use `UserId` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub type UserId = [u8;
```

## hm-ledger::EntropySource

<a id="rust-crates-hm-ledger-src-keyring-rs-entropysource"></a>

Source: [`crates/hm-ledger/src/keyring.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/keyring.rs).

When to use: Use `EntropySource` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub trait EntropySource {
    fn fill(&mut self, destination: &mut [u8]) -> Result<(), Error>;
}
```

## hm-ledger::OsEntropy

<a id="rust-crates-hm-ledger-src-keyring-rs-osentropy"></a>

Source: [`crates/hm-ledger/src/keyring.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/keyring.rs).

When to use: Use `OsEntropy` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct OsEntropy;
```

## hm-ledger::KeyHierarchy

<a id="rust-crates-hm-ledger-src-keyring-rs-keyhierarchy"></a>

Source: [`crates/hm-ledger/src/keyring.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/keyring.rs).

When to use: Use `KeyHierarchy` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct KeyHierarchy {
    pub(crate) actor: ActorId,
    pub(crate) user: UserId,
    _user_key: Zeroizing<[u8; 32]>,
    pub(crate) data_key: Zeroizing<[u8; 32]>,
    keyring_path: PathBuf,
}
```

## hm-ledger::VerificationStatus

<a id="rust-crates-hm-ledger-src-mmr-store-rs-verificationstatus"></a>

Source: [`crates/hm-ledger/src/mmr_store.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/mmr_store.rs).

When to use: Use `VerificationStatus` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct VerificationStatus {
    pub root: Hash,
    pub leaf_count: u64,
    pub last_checkpoint_lsn: LSN,
    pub verified: bool,
}
```

## hm-ledger::RepairStatus

<a id="rust-crates-hm-ledger-src-mmr-store-rs-repairstatus"></a>

Source: [`crates/hm-ledger/src/mmr_store.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/mmr_store.rs).

When to use: Use `RepairStatus` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct RepairStatus {
    pub repaired_leaves: usize,
    pub leaf_count: u64,
    pub complete: bool,
}
```

## hm-ledger::MmrStore

<a id="rust-crates-hm-ledger-src-mmr-store-rs-mmrstore"></a>

Source: [`crates/hm-ledger/src/mmr_store.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/mmr_store.rs).

When to use: Use `MmrStore` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct MmrStore {
    mmr_directory: PathBuf,
    checkpoint_directory: PathBuf,
    actor: ActorId,
    expected_public_key: PublicKey,
    mmr: Mmr,
    status: VerificationStatus,
}
```

## hm-ledger::Hash

<a id="rust-crates-hm-ledger-src-mmr-rs-hash"></a>

Source: [`crates/hm-ledger/src/mmr.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/mmr.rs).

When to use: Use `Hash` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub type Hash = [u8;
```

## hm-ledger::Node

<a id="rust-crates-hm-ledger-src-mmr-rs-node"></a>

Source: [`crates/hm-ledger/src/mmr.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/mmr.rs).

When to use: Use `Node` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct Node {
    pub height: u8,
    pub start: u64,
    pub leaf_count: u64,
    pub hash: Hash,
}
```

## hm-ledger::AppendResult

<a id="rust-crates-hm-ledger-src-mmr-rs-appendresult"></a>

Source: [`crates/hm-ledger/src/mmr.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/mmr.rs).

When to use: Use `AppendResult` for durable encrypted append, recovery, key lifecycle, and integrity verification. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct AppendResult {
    pub root: Hash,
    pub created_nodes: Vec<Node>,
}
```

## hm-ledger::RangeProof

<a id="rust-crates-hm-ledger-src-mmr-rs-rangeproof"></a>

Source: [`crates/hm-ledger/src/mmr.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/mmr.rs).

When to use: Use `RangeProof` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct RangeProof {
    pub total_leaf_count: u64,
    pub range_start: u64,
    pub range_leaf_count: u64,
    pub expected_root: Hash,
    pub boundary_nodes: Vec<Node>,
}
```

## hm-ledger::Mmr

<a id="rust-crates-hm-ledger-src-mmr-rs-mmr"></a>

Source: [`crates/hm-ledger/src/mmr.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/mmr.rs).

When to use: Use `Mmr` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct Mmr {
    leaves: Vec<Hash>,
    peaks: Vec<Node>,
    nodes: BTreeMap<(u64, u64), Node>,
    leaf_count: u64,
    retain_history: bool,
}
```

## hm-ledger::WriteBackend

<a id="rust-crates-hm-ledger-src-segment-rs-writebackend"></a>

Source: [`crates/hm-ledger/src/segment.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/segment.rs).

When to use: Use `WriteBackend` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub enum WriteBackend {
    Auto,
    Pwrite,
    IoUringDirect,
}
```

## hm-ledger::SegmentLogOptions

<a id="rust-crates-hm-ledger-src-segment-rs-segmentlogoptions"></a>

Source: [`crates/hm-ledger/src/segment.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/segment.rs).

When to use: Use `SegmentLogOptions` for durable encrypted append, recovery, key lifecycle, and integrity verification. Set explicit deployment limits before opening the associated resource.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted. Do not log secret fields or substitute defaults for an explicitly authorized budget.


```rust
pub struct SegmentLogOptions {
    pub segment_bytes: u64,
    pub maximum_frame_bytes: usize,
    pub direct_alignment: u32,
    pub maximum_io_bytes: usize,
    pub backend: WriteBackend,
}
```

## hm-ledger::AppendRequest

<a id="rust-crates-hm-ledger-src-segment-rs-appendrequest"></a>

Source: [`crates/hm-ledger/src/segment.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/segment.rs).

When to use: Use `AppendRequest` for durable encrypted append, recovery, key lifecycle, and integrity verification. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct AppendRequest {
    pub kind: EventKind,
    pub wall_timestamp_ns: UtcNanos,
    pub conversation: ConversationId,
    pub sealed_payload: Vec<u8>,
}
```

## hm-ledger::CommitResult

<a id="rust-crates-hm-ledger-src-segment-rs-commitresult"></a>

Source: [`crates/hm-ledger/src/segment.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/segment.rs).

When to use: Use `CommitResult` for durable encrypted append, recovery, key lifecycle, and integrity verification. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct CommitResult {
    pub first_lsn: LSN,
    pub last_lsn: LSN,
    pub backend: WriteBackend,
    pub segment_count: u32,
}
```

## hm-ledger::RecoveryReport

<a id="rust-crates-hm-ledger-src-segment-rs-recoveryreport"></a>

Source: [`crates/hm-ledger/src/segment.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/segment.rs).

When to use: Use `RecoveryReport` for durable encrypted append, recovery, key lifecycle, and integrity verification. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct RecoveryReport {
    pub next_lsn: LSN,
    pub recovered_tail_lsn: LSN,
    pub segment_count: u32,
    pub truncated_torn_tail: bool,
    pub rebuilt_manifest: bool,
}
```

## hm-ledger::SegmentLog

<a id="rust-crates-hm-ledger-src-segment-rs-segmentlog"></a>

Source: [`crates/hm-ledger/src/segment.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/segment.rs).

When to use: Use `SegmentLog` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct SegmentLog {
    log_directory: PathBuf,
    actor: ActorId,
    options: SegmentLogOptions,
    segments: Vec<SegmentEntry>,
    recovery_report: RecoveryReport,
    next_lsn: u64,
}
```

## hm-ledger::DeletionReceipt

<a id="rust-crates-hm-ledger-src-shred-rs-deletionreceipt"></a>

Source: [`crates/hm-ledger/src/shred.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/shred.rs).

When to use: Use `DeletionReceipt` for durable encrypted append, recovery, key lifecycle, and integrity verification. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct DeletionReceipt {
    pub actor: ActorId,
    pub user: UserId,
    pub deleted_at_lsn: LSN,
    pub key_fingerprint: Hash,
    pub checkpoint_root: Hash,
    pub public_key: PublicKey,
    pub signature: Signature,
}
```

## hm-ledger::TripwireSet

<a id="rust-crates-hm-ledger-src-tripwire-rs-tripwireset"></a>

Source: [`crates/hm-ledger/src/tripwire.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-ledger/src/tripwire.rs).

When to use: Use `TripwireSet` for durable encrypted append, recovery, key lifecycle, and integrity verification.

Do not use: Do not rewrite committed records, share an actor writer, expose keys, or treat an unverified prefix as trusted.


```rust
pub struct TripwireSet {
    lsns: BTreeSet<LSN>,
}
```

## hm-llm::AdmissionLimits

<a id="rust-crates-hm-llm-src-admission-rs-admissionlimits"></a>

Source: [`crates/hm-llm/src/admission.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/admission.rs).

When to use: Use `AdmissionLimits` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub struct AdmissionLimits {
    pub maximum_in_flight: usize,
    pub maximum_in_flight_per_actor: usize,
    pub maximum_wait_ms: u64,
}
```

## hm-llm::CallAdmission

<a id="rust-crates-hm-llm-src-admission-rs-calladmission"></a>

Source: [`crates/hm-llm/src/admission.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/admission.rs).

When to use: Use `CallAdmission` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub struct CallAdmission {
    limits: AdmissionLimits,
    state: Mutex<AdmissionState>,
    released: Condvar,
}
```

## hm-llm::AdmissionPermit

<a id="rust-crates-hm-llm-src-admission-rs-admissionpermit"></a>

Source: [`crates/hm-llm/src/admission.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/admission.rs).

When to use: Use `AdmissionPermit` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub struct AdmissionPermit {
    admission: Arc<CallAdmission>,
    actor: u16,
}
```

## hm-llm::AdmittedProvider

<a id="rust-crates-hm-llm-src-admission-rs-admittedprovider"></a>

Source: [`crates/hm-llm/src/admission.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/admission.rs).

When to use: Use `AdmittedProvider` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub struct AdmittedProvider<P> {
    admission: Arc<CallAdmission>,
    actor: u16,
    inner: P,
}
```

## hm-llm::Anthropic

<a id="rust-crates-hm-llm-src-anthropic-rs-anthropic"></a>

Source: [`crates/hm-llm/src/anthropic.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/anthropic.rs).

When to use: Use `Anthropic` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub struct Anthropic<T> {
    config: ProviderConfig,
    transport: T,
}
```

## hm-llm::FieldShape

<a id="rust-crates-hm-llm-src-contract-rs-fieldshape"></a>

Source: [`crates/hm-llm/src/contract.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/contract.rs).

When to use: Use `FieldShape` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub enum FieldShape {
    Identifier,
    NullableIdentifier,
    Prose,
    IdentifierList,
    Count,
    Flag,
    Opaque,
}
```

## hm-llm::FieldRule

<a id="rust-crates-hm-llm-src-contract-rs-fieldrule"></a>

Source: [`crates/hm-llm/src/contract.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/contract.rs).

When to use: Use `FieldRule` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub struct FieldRule {
    pub name: &'static str,
    pub shape: FieldShape,
}
```

## hm-llm::ExtractionContract

<a id="rust-crates-hm-llm-src-contract-rs-extractioncontract"></a>

Source: [`crates/hm-llm/src/contract.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/contract.rs).

When to use: Use `ExtractionContract` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub struct ExtractionContract {
    pub contract_id: &'static str,
    pub version: u16,
    pub fields: &'static [FieldRule],
}
```

## hm-llm::ContractViolation

<a id="rust-crates-hm-llm-src-contract-rs-contractviolation"></a>

Source: [`crates/hm-llm/src/contract.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/contract.rs).

When to use: Use `ContractViolation` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub enum ContractViolation {
    NotAnObject,
    MissingField(String),
    UnknownField(String),
    WrongShape(String),
    EmptyIdentifier(String),
    IdentifierTooLong {
        field: String,
        characters: usize,
        limit: usize,
    },
    ProseTooLong {
        field: String,
        characters: usize,
        limit: usize,
    },
    ListTooLong {
        field: String,
        items: usize,
        limit: usize,
    },
}
```

## hm-llm::PriceEntry

<a id="rust-crates-hm-llm-src-cost-rs-priceentry"></a>

Source: [`crates/hm-llm/src/cost.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/cost.rs).

When to use: Use `PriceEntry` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub struct PriceEntry {
    pub model: &'static str,
    pub pricing: Pricing,
}
```

## hm-llm::RunCost

<a id="rust-crates-hm-llm-src-cost-rs-runcost"></a>

Source: [`crates/hm-llm/src/cost.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/cost.rs).

When to use: Use `RunCost` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub struct RunCost {
    pub calls: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub cost_microusd: u64,
}
```

## hm-llm::RunBudget

<a id="rust-crates-hm-llm-src-cost-rs-runbudget"></a>

Source: [`crates/hm-llm/src/cost.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/cost.rs).

When to use: Use `RunBudget` for budgeted provider requests, structured responses, prompt identities, and measured usage. Set explicit deployment limits before opening the associated resource.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation. Do not log secret fields or substitute defaults for an explicitly authorized budget.


```rust
pub struct RunBudget {
    pub max_calls: u64,
    pub max_tokens: u64,
    pub max_microusd: u64,
}
```

## hm-llm::Gemini

<a id="rust-crates-hm-llm-src-gemini-rs-gemini"></a>

Source: [`crates/hm-llm/src/gemini.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/gemini.rs).

When to use: Use `Gemini` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub struct Gemini<T> {
    config: ProviderConfig,
    transport: T,
}
```

## hm-llm::ModelTier

<a id="rust-crates-hm-llm-src-lib-rs-modeltier"></a>

Source: [`crates/hm-llm/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/lib.rs).

When to use: Use `ModelTier` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub enum ModelTier {
    Economy,
    Standard,
    Capable,
}
```

## hm-llm::Pricing

<a id="rust-crates-hm-llm-src-lib-rs-pricing"></a>

Source: [`crates/hm-llm/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/lib.rs).

When to use: Use `Pricing` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub struct Pricing {
    pub input_microusd_per_million_tokens: u64,
    pub output_microusd_per_million_tokens: u64,
}
```

## hm-llm::Usage

<a id="rust-crates-hm-llm-src-lib-rs-usage"></a>

Source: [`crates/hm-llm/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/lib.rs).

When to use: Use `Usage` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub cost_microusd: u64,
}
```

## hm-llm::StructuredRequest

<a id="rust-crates-hm-llm-src-lib-rs-structuredrequest"></a>

Source: [`crates/hm-llm/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/lib.rs).

When to use: Use `StructuredRequest` for budgeted provider requests, structured responses, prompt identities, and measured usage. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct StructuredRequest {
    pub prompt_id: String,
    pub system: String,
    pub prompt: String,
    pub json_schema: Value,
    pub maximum_output_tokens: u32,
}
```

## hm-llm::StructuredResponse

<a id="rust-crates-hm-llm-src-lib-rs-structuredresponse"></a>

Source: [`crates/hm-llm/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/lib.rs).

When to use: Use `StructuredResponse` for budgeted provider requests, structured responses, prompt identities, and measured usage. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct StructuredResponse {
    pub model_id: String,
    pub tier: ModelTier,
    pub value: Value,
    pub usage: Usage,
}
```

## hm-llm::LlmError

<a id="rust-crates-hm-llm-src-lib-rs-llmerror"></a>

Source: [`crates/hm-llm/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/lib.rs).

When to use: Use `LlmError` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub enum LlmError {
    InvalidArgument(&'static str),
    Network(String),
    Wire(String),
    Schema(String),
    Response(outcome::ResponseFault),
    Capacity,
    Admission(&'static str),
}
```

## hm-llm::LlmProvider

<a id="rust-crates-hm-llm-src-lib-rs-llmprovider"></a>

Source: [`crates/hm-llm/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/lib.rs).

When to use: Use `LlmProvider` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub trait LlmProvider: Send + Sync {
    fn model_id(&self) -> &str;
    fn tier(&self) -> ModelTier;
    fn generate_structured(
        &self,
        request: &StructuredRequest,
    ) -> Result<StructuredResponse, LlmError>;
}
```

## hm-llm::ProviderConfig

<a id="rust-crates-hm-llm-src-lib-rs-providerconfig"></a>

Source: [`crates/hm-llm/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/lib.rs).

When to use: Use `ProviderConfig` for budgeted provider requests, structured responses, prompt identities, and measured usage. Set explicit deployment limits before opening the associated resource.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation. Do not log secret fields or substitute defaults for an explicitly authorized budget.


```rust
pub struct ProviderConfig {
    pub endpoint: String,
    pub api_key: Option<String>,
    pub model: String,
    pub tier: ModelTier,
    pub pricing: Pricing,
}
```

## hm-llm::WireRequest

<a id="rust-crates-hm-llm-src-lib-rs-wirerequest"></a>

Source: [`crates/hm-llm/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/lib.rs).

When to use: Use `WireRequest` for budgeted provider requests, structured responses, prompt identities, and measured usage. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct WireRequest {
    pub method: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub body: Value,
}
```

## hm-llm::WireResponse

<a id="rust-crates-hm-llm-src-lib-rs-wireresponse"></a>

Source: [`crates/hm-llm/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/lib.rs).

When to use: Use `WireResponse` for budgeted provider requests, structured responses, prompt identities, and measured usage. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct WireResponse {
    pub status: u16,
    pub body: Value,
}
```

## hm-llm::WireTransport

<a id="rust-crates-hm-llm-src-lib-rs-wiretransport"></a>

Source: [`crates/hm-llm/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/lib.rs).

When to use: Use `WireTransport` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub trait WireTransport: Send + Sync {
    fn send(&self, request: &WireRequest) -> Result<WireResponse, LlmError>;
}
```

## hm-llm::WireFixture

<a id="rust-crates-hm-llm-src-lib-rs-wirefixture"></a>

Source: [`crates/hm-llm/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/lib.rs).

When to use: Use `WireFixture` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub struct WireFixture {
    pub request: WireRequest,
    pub response: WireResponse,
}
```

## hm-llm::RecordedTransport

<a id="rust-crates-hm-llm-src-lib-rs-recordedtransport"></a>

Source: [`crates/hm-llm/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/lib.rs).

When to use: Use `RecordedTransport` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub struct RecordedTransport {
    fixtures: Mutex<Vec<WireFixture>>,
}
```

## hm-llm::HttpTransport

<a id="rust-crates-hm-llm-src-lib-rs-httptransport"></a>

Source: [`crates/hm-llm/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/lib.rs).

When to use: Use `HttpTransport` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub struct HttpTransport {
    client: reqwest::blocking::Client,
}
```

## hm-llm::MediaModality

<a id="rust-crates-hm-llm-src-media-rs-mediamodality"></a>

Source: [`crates/hm-llm/src/media.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/media.rs).

When to use: Use `MediaModality` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub enum MediaModality {
    Audio,
    Image,
}
```

## hm-llm::MediaAttachment

<a id="rust-crates-hm-llm-src-media-rs-mediaattachment"></a>

Source: [`crates/hm-llm/src/media.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/media.rs).

When to use: Use `MediaAttachment` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub struct MediaAttachment {
    pub modality: MediaModality,
    pub media_type: String,
    pub bytes: Vec<u8>,
}
```

## hm-llm::MediaRequest

<a id="rust-crates-hm-llm-src-media-rs-mediarequest"></a>

Source: [`crates/hm-llm/src/media.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/media.rs).

When to use: Use `MediaRequest` for budgeted provider requests, structured responses, prompt identities, and measured usage. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct MediaRequest {
    pub prompt_id: String,
    pub system: String,
    pub prompt: String,
    pub json_schema: Value,
    pub maximum_output_tokens: u32,
    pub attachment: MediaAttachment,
}
```

## hm-llm::MediaProvider

<a id="rust-crates-hm-llm-src-media-rs-mediaprovider"></a>

Source: [`crates/hm-llm/src/media.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/media.rs).

When to use: Use `MediaProvider` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub trait MediaProvider: Send + Sync {
    fn model_id(&self) -> &str;
    fn tier(&self) -> ModelTier;
    fn describe_media(&self, request: &MediaRequest) -> Result<StructuredResponse, LlmError>;
}
```

## hm-llm::Ollama

<a id="rust-crates-hm-llm-src-ollama-rs-ollama"></a>

Source: [`crates/hm-llm/src/ollama.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/ollama.rs).

When to use: Use `Ollama` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub struct Ollama<T> {
    config: ProviderConfig,
    transport: T,
}
```

## hm-llm::OpenAiCompatible

<a id="rust-crates-hm-llm-src-openai-compat-rs-openaicompatible"></a>

Source: [`crates/hm-llm/src/openai_compat.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/openai_compat.rs).

When to use: Use `OpenAiCompatible` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub struct OpenAiCompatible<T> {
    config: ProviderConfig,
    transport: T,
}
```

## hm-llm::ResponseOutcome

<a id="rust-crates-hm-llm-src-outcome-rs-responseoutcome"></a>

Source: [`crates/hm-llm/src/outcome.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/outcome.rs).

When to use: Use `ResponseOutcome` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub enum ResponseOutcome {
    Refused,
    Truncated,
    Malformed,
    Incomplete,
}
```

## hm-llm::ResponseFault

<a id="rust-crates-hm-llm-src-outcome-rs-responsefault"></a>

Source: [`crates/hm-llm/src/outcome.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/outcome.rs).

When to use: Use `ResponseFault` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub struct ResponseFault {
    pub version: u16,
    pub outcome: ResponseOutcome,
    pub model_id: String,
    pub detail: String,
    pub requested_output_tokens: u32,
    pub usage: Usage,
}
```

## hm-llm::Prompt

<a id="rust-crates-hm-llm-src-registry-rs-prompt"></a>

Source: [`crates/hm-llm/src/registry.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/registry.rs).

When to use: Use `Prompt` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub struct Prompt {
    pub id: String,
    pub version: u32,
    pub content: String,
    pub digest: [u8; 32],
}
```

## hm-llm::PromptRegistry

<a id="rust-crates-hm-llm-src-registry-rs-promptregistry"></a>

Source: [`crates/hm-llm/src/registry.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-llm/src/registry.rs).

When to use: Use `PromptRegistry` for budgeted provider requests, structured responses, prompt identities, and measured usage.

Do not use: Do not treat model text as observed evidence or make unbounded provider calls from foreground recall/activation.


```rust
pub struct PromptRegistry {
    prompts: BTreeMap<(String, u32), Prompt>,
}
```

## hm-mcp::McpToolDispatcher

<a id="rust-crates-hm-mcp-src-dispatcher-rs-mcptooldispatcher"></a>

Source: [`crates/hm-mcp/src/dispatcher.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/dispatcher.rs).

When to use: Use `McpToolDispatcher` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub struct McpToolDispatcher {
    pub embedding_runtime: Option<EmbeddingRuntime>,
    pub reconstruction_runtime: Option<ReconstructionRuntime>,
    pub consolidation_runtime: Option<ConsolidationRuntime>,
}
```

## hm-mcp::Envelope

<a id="rust-crates-hm-mcp-src-lib-rs-envelope"></a>

Source: [`crates/hm-mcp/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/lib.rs).

When to use: Use `Envelope` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub struct Envelope {
    pub ok: bool,
    pub items: Vec<Value>,
    pub provenance: Vec<String>,
    pub budget: Option<Value>,
    pub gaps: Vec<Value>,
    pub health: Value,
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<Value>,
}
```

## hm-mcp::ActivateInput

<a id="rust-crates-hm-mcp-src-lib-rs-activateinput"></a>

Source: [`crates/hm-mcp/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/lib.rs).

When to use: Use `ActivateInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct ActivateInput {
    pub conversation: String,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub turn_text: String,
    pub budget_tokens: usize,
}
```

## hm-mcp::McpServer

<a id="rust-crates-hm-mcp-src-lib-rs-mcpserver"></a>

Source: [`crates/hm-mcp/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/lib.rs).

When to use: Use `McpServer` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub struct McpServer {
    actor: ActorEngine,
    admin_token: Option<hm_serve::config::CapabilityToken>,
    dispute_runtime: Option<DisputeRuntime>,
    consolidation_runtime: Option<ConsolidationRuntime>,
    embedding_runtime: Option<EmbeddingRuntime>,
    reconstruction_runtime: Option<ReconstructionRuntime>,
}
```

## hm-mcp::AttestDisposition

<a id="rust-crates-hm-mcp-src-tools-attest-rs-attestdisposition"></a>

Source: [`crates/hm-mcp/src/tools/attest.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/attest.rs).

When to use: Use `AttestDisposition` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub enum AttestDisposition {
    Used,
    Ignored,
    Helpful,
    Harmful,
}
```

## hm-mcp::AttestInput

<a id="rust-crates-hm-mcp-src-tools-attest-rs-attestinput"></a>

Source: [`crates/hm-mcp/src/tools/attest.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/attest.rs).

When to use: Use `AttestInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct AttestInput {
    pub provenance: Vec<String>,
    pub disposition: AttestDisposition,
    pub idempotency_key: String,
}
```

## hm-mcp::BeliefTypeInput

<a id="rust-crates-hm-mcp-src-tools-believe-rs-belieftypeinput"></a>

Source: [`crates/hm-mcp/src/tools/believe.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/believe.rs).

When to use: Use `BeliefTypeInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub enum BeliefTypeInput {
    Fact,
    Preference,
    Constraint,
    Goal,
    Identity,
}
```

## hm-mcp::ClaimInput

<a id="rust-crates-hm-mcp-src-tools-believe-rs-claiminput"></a>

Source: [`crates/hm-mcp/src/tools/believe.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/believe.rs).

When to use: Use `ClaimInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub enum ClaimInput {
    #[default]
    Affirmative,
    NegativeExistence,
}
```

## hm-mcp::ProvenanceInput

<a id="rust-crates-hm-mcp-src-tools-believe-rs-provenanceinput"></a>

Source: [`crates/hm-mcp/src/tools/believe.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/believe.rs).

When to use: Use `ProvenanceInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct ProvenanceInput {
    pub first_lsn: u64,
    pub last_lsn: u64,
    #[serde(default)]
    pub byte_start: u32,
    pub byte_end: u32,
}
```

## hm-mcp::BeliefClaimInput

<a id="rust-crates-hm-mcp-src-tools-believe-rs-beliefclaiminput"></a>

Source: [`crates/hm-mcp/src/tools/believe.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/believe.rs).

When to use: Use `BeliefClaimInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct BeliefClaimInput {
    pub belief_id: String,
    pub belief_type: BeliefTypeInput,
    pub canonical_identity: String,
    pub value: String,
    #[serde(default)]
    pub valid_from_ns: i64,
    #[serde(default)]
    pub valid_to_ns: i64,
    pub provenance: Vec<ProvenanceInput>,
    #[serde(default)]
    pub conflict_domain: Option<String>,
    #[serde(default)]
    pub claim: ClaimInput,
}
```

## hm-mcp::BelieveInput

<a id="rust-crates-hm-mcp-src-tools-believe-rs-believeinput"></a>

Source: [`crates/hm-mcp/src/tools/believe.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/believe.rs).

When to use: Use `BelieveInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct BelieveInput {
    pub conversation: String,
    #[serde(flatten)]
    pub belief: BeliefClaimInput,
    #[serde(default)]
    pub run_id: Option<String>,
}
```

## hm-mcp::BindInput

<a id="rust-crates-hm-mcp-src-tools-bind-rs-bindinput"></a>

Source: [`crates/hm-mcp/src/tools/bind.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/bind.rs).

When to use: Use `BindInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct BindInput {
    pub conversation: String,
    #[serde(default)]
    pub task: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    pub canonical_entity: String,
    pub property: String,
    pub evidence_lsn: u64,
    pub revision: String,
    pub freshness_requirement_ns: u64,
}
```

## hm-mcp::ConsolidationRuntime

<a id="rust-crates-hm-mcp-src-tools-consolidate-rs-consolidationruntime"></a>

Source: [`crates/hm-mcp/src/tools/consolidate.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/consolidate.rs).

When to use: Use `ConsolidationRuntime` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub struct ConsolidationRuntime {
    provider: Arc<dyn LlmProvider>,
    pub admission: Arc<CallAdmission>,
}
```

## hm-mcp::ConsolidateAction

<a id="rust-crates-hm-mcp-src-tools-consolidate-rs-consolidateaction"></a>

Source: [`crates/hm-mcp/src/tools/consolidate.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/consolidate.rs).

When to use: Use `ConsolidateAction` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub enum ConsolidateAction {
    Run,
    List,
    Retract,
}
```

## hm-mcp::ConsolidateMode

<a id="rust-crates-hm-mcp-src-tools-consolidate-rs-consolidatemode"></a>

Source: [`crates/hm-mcp/src/tools/consolidate.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/consolidate.rs).

When to use: Use `ConsolidateMode` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub enum ConsolidateMode {
    Nrem,
    Rem,
    Both,
}
```

## hm-mcp::ConsolidateBudget

<a id="rust-crates-hm-mcp-src-tools-consolidate-rs-consolidatebudget"></a>

Source: [`crates/hm-mcp/src/tools/consolidate.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/consolidate.rs).

When to use: Use `ConsolidateBudget` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Set explicit deployment limits before opening the associated resource.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not log secret fields or substitute defaults for an explicitly authorized budget.


```rust
pub struct ConsolidateBudget {
    pub max_llm_calls: u64,
    pub max_tokens: u64,
    pub max_microusd: u64,
    pub max_wall_ms: u64,
}
```

## hm-mcp::ConsolidateInput

<a id="rust-crates-hm-mcp-src-tools-consolidate-rs-consolidateinput"></a>

Source: [`crates/hm-mcp/src/tools/consolidate.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/consolidate.rs).

When to use: Use `ConsolidateInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct ConsolidateInput {
    pub action: ConsolidateAction,
    #[serde(default)]
    pub mode: Option<ConsolidateMode>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub cadence_key: Option<String>,
    #[serde(default)]
    pub budget: Option<ConsolidateBudget>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}
```

## hm-mcp::DisputeRuntime

<a id="rust-crates-hm-mcp-src-tools-dispute-rs-disputeruntime"></a>

Source: [`crates/hm-mcp/src/tools/dispute.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/dispute.rs).

When to use: Use `DisputeRuntime` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub struct DisputeRuntime {
    nli: Arc<NliModel>,
    provider: Option<Arc<dyn LlmProvider>>,
    minimum_tier: ModelTier,
}
```

## hm-mcp::DisputeInput

<a id="rust-crates-hm-mcp-src-tools-dispute-rs-disputeinput"></a>

Source: [`crates/hm-mcp/src/tools/dispute.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/dispute.rs).

When to use: Use `DisputeInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct DisputeInput {
    pub conversation: String,
    pub existing: BeliefClaimInput,
    pub incoming: BeliefClaimInput,
}
```

## hm-mcp::ForgetAction

<a id="rust-crates-hm-mcp-src-tools-forget-rs-forgetaction"></a>

Source: [`crates/hm-mcp/src/tools/forget.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/forget.rs).

When to use: Use `ForgetAction` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub enum ForgetAction {
    Fade,
    RetractRun,
    CryptoShred,
}
```

## hm-mcp::ForgetInput

<a id="rust-crates-hm-mcp-src-tools-forget-rs-forgetinput"></a>

Source: [`crates/hm-mcp/src/tools/forget.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/forget.rs).

When to use: Use `ForgetInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct ForgetInput {
    pub action: ForgetAction,
    #[serde(default)]
    pub lsn: Option<u64>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub admin_token: Option<String>,
}
```

## hm-mcp::InspectMode

<a id="rust-crates-hm-mcp-src-tools-inspect-rs-inspectmode"></a>

Source: [`crates/hm-mcp/src/tools/inspect.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/inspect.rs).

When to use: Use `InspectMode` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub enum InspectMode {
    #[default]
    Status,
    Discover,
}
```

## hm-mcp::InspectInput

<a id="rust-crates-hm-mcp-src-tools-inspect-rs-inspectinput"></a>

Source: [`crates/hm-mcp/src/tools/inspect.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/inspect.rs).

When to use: Use `InspectInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct InspectInput {
    #[serde(default)]
    pub uri: Option<String>,
    #[serde(default)]
    pub mode: InspectMode,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}
```

## hm-mcp::IntendCloseReason

<a id="rust-crates-hm-mcp-src-tools-intend-rs-intendclosereason"></a>

Source: [`crates/hm-mcp/src/tools/intend.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/intend.rs).

When to use: Use `IntendCloseReason` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub enum IntendCloseReason {
    Done,
    Abandoned,
    HandedOff,
    Superseded,
}
```

## hm-mcp::IntendAction

<a id="rust-crates-hm-mcp-src-tools-intend-rs-intendaction"></a>

Source: [`crates/hm-mcp/src/tools/intend.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/intend.rs).

When to use: Use `IntendAction` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub enum IntendAction {
    SetIntention {
        intention_id: String,
        objective: String,
        trigger: WakeTriggerInput,
        expires_at_ns: i64,
        reply_route: String,
    },
    CancelIntention {
        intention_id: String,
        reason: String,
    },
    EvaluateWake {
        observation_lsn: u64,
        factors: AttentionFactorsInput,
        #[serde(default)]
        rearm_at_ns: Option<i64>,
    },
    AdoptProcedure {
        procedure_id: String,
        procedure_lsn: u64,
    },
    SetObjective {
        objective: String,
    },
    OpenLoop {
        loop_id: String,
        objective: String,
    },
    CloseLoop {
        loop_id: String,
        reason: IntendCloseReason,
        #[serde(default)]
        cause: String,
        #[serde(default)]
        evidence_lsns: Vec<u64>,
    },
}
```

## hm-mcp::WakeTriggerInput

<a id="rust-crates-hm-mcp-src-tools-intend-rs-waketriggerinput"></a>

Source: [`crates/hm-mcp/src/tools/intend.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/intend.rs).

When to use: Use `WakeTriggerInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub enum WakeTriggerInput {
    At { at_ns: i64 },
    Schedule { schedule: String },
    ChildTerminal { child_id: String },
    ProcessExit { process_id: String },
    FileChanged { path: String },
    RepositoryChanged { repository: String },
    ChannelMessage { channel: String },
    ExternalCondition { condition: String },
    UserResponse { reply_to: String },
    EntityMentioned { entity_id: String },
    LoopClosed { loop_id: String },
    PredictionResolved { prediction_id: String },
    BeliefChanged { canonical_identity: String },
}
```

## hm-mcp::AttentionFactorsInput

<a id="rust-crates-hm-mcp-src-tools-intend-rs-attentionfactorsinput"></a>

Source: [`crates/hm-mcp/src/tools/intend.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/intend.rs).

When to use: Use `AttentionFactorsInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct AttentionFactorsInput {
    pub urgency: u32,
    pub expected_value: u32,
    pub confidence: u32,
    pub interruption_cost: u32,
    pub resource_cost: u32,
    pub duplication_penalty: u32,
    pub quiet_hours: bool,
    pub notifications_remaining: u32,
    pub workload: u32,
}
```

## hm-mcp::IntendInput

<a id="rust-crates-hm-mcp-src-tools-intend-rs-intendinput"></a>

Source: [`crates/hm-mcp/src/tools/intend.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/intend.rs).

When to use: Use `IntendInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct IntendInput {
    pub conversation: String,
    pub action: IntendAction,
}
```

## hm-mcp::OutcomeInput

<a id="rust-crates-hm-mcp-src-tools-outcome-rs-outcomeinput"></a>

Source: [`crates/hm-mcp/src/tools/outcome.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/outcome.rs).

When to use: Use `OutcomeInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct OutcomeInput {
    pub conversation: String,
    pub prediction_id: String,
    pub revision: u32,
    pub observation_lsns: Vec<u64>,
}
```

## hm-mcp::PredicateKindInput

<a id="rust-crates-hm-mcp-src-tools-predict-rs-predicatekindinput"></a>

Source: [`crates/hm-mcp/src/tools/predict.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/predict.rs).

When to use: Use `PredicateKindInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub enum PredicateKindInput {
    ObjectExists,
    RevisionEquals,
    DigestEquals,
    ReceiptMatches,
    PropertySatisfies,
    ProcessTerminated,
    AnswerCommitted,
}
```

## hm-mcp::ExpectedPredicateInput

<a id="rust-crates-hm-mcp-src-tools-predict-rs-expectedpredicateinput"></a>

Source: [`crates/hm-mcp/src/tools/predict.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/predict.rs).

When to use: Use `ExpectedPredicateInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct ExpectedPredicateInput {
    pub kind: PredicateKindInput,
    pub scope: String,
    #[serde(default)]
    pub property: Option<String>,
    #[serde(default)]
    pub expected: Option<Value>,
}
```

## hm-mcp::PredictInput

<a id="rust-crates-hm-mcp-src-tools-predict-rs-predictinput"></a>

Source: [`crates/hm-mcp/src/tools/predict.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/predict.rs).

When to use: Use `PredictInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct PredictInput {
    pub conversation: String,
    pub prediction_id: String,
    pub revision: u32,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub attempt_id: Option<String>,
    #[serde(default)]
    pub operation_id: Option<String>,
    pub mechanism: String,
    pub predicates: Vec<ExpectedPredicateInput>,
    pub deadline_ns: i64,
    pub uncertainty: String,
}
```

## hm-mcp::RecallMode

<a id="rust-crates-hm-mcp-src-tools-recall-rs-recallmode"></a>

Source: [`crates/hm-mcp/src/tools/recall.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/recall.rs).

When to use: Use `RecallMode` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub enum RecallMode {
    Semantic,
    Lexical,
    Entity,
    Temporal,
    Near,
    Timeline,
    Reconstruct,
}
```

## hm-mcp::RecallFilters

<a id="rust-crates-hm-mcp-src-tools-recall-rs-recallfilters"></a>

Source: [`crates/hm-mcp/src/tools/recall.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/recall.rs).

When to use: Use `RecallFilters` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub struct RecallFilters {
    #[serde(default)]
    pub anchor_lsns: Vec<u64>,
    #[serde(default)]
    pub maximum_output_tokens: Option<u32>,
    #[serde(default)]
    pub conversation: Option<String>,
    #[serde(default)]
    pub since_lsn: Option<u64>,
    #[serde(default)]
    pub until_lsn: Option<u64>,
    #[serde(default)]
    pub temporal_from_ns: Option<i64>,
    #[serde(default)]
    pub temporal_to_ns: Option<i64>,
    #[serde(default)]
    pub anchor: Option<String>,
    #[serde(default)]
    pub turn_text: String,
}
```

## hm-mcp::RecallInput

<a id="rust-crates-hm-mcp-src-tools-recall-rs-recallinput"></a>

Source: [`crates/hm-mcp/src/tools/recall.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/recall.rs).

When to use: Use `RecallInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct RecallInput {
    pub mode: RecallMode,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub conversation: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub since_lsn: u64,
    #[serde(default)]
    pub filters: RecallFilters,
}
```

## hm-mcp::ReconstructionRuntime

<a id="rust-crates-hm-mcp-src-tools-reconstruct-rs-reconstructionruntime"></a>

Source: [`crates/hm-mcp/src/tools/reconstruct.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/reconstruct.rs).

When to use: Use `ReconstructionRuntime` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub struct ReconstructionRuntime {
    pub provider: Arc<dyn LlmProvider>,
    pub admission: Arc<CallAdmission>,
}
```

## hm-mcp::RelationBuildReport

<a id="rust-crates-hm-mcp-src-tools-relation-rs-relationbuildreport"></a>

Source: [`crates/hm-mcp/src/tools/relation.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/relation.rs).

When to use: Use `RelationBuildReport` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct RelationBuildReport {
    pub scanned: u64,
    pub embedded: u64,
    pub skipped: u64,
    pub space_id: String,
    pub first_lsn: u64,
    pub last_lsn: u64,
}
```

## hm-mcp::EmbeddingRuntime

<a id="rust-crates-hm-mcp-src-tools-remember-rs-embeddingruntime"></a>

Source: [`crates/hm-mcp/src/tools/remember.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/remember.rs).

When to use: Use `EmbeddingRuntime` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub struct EmbeddingRuntime {
    embedder: Arc<dyn Embedder>,
}
```

## hm-mcp::RememberKind

<a id="rust-crates-hm-mcp-src-tools-remember-rs-rememberkind"></a>

Source: [`crates/hm-mcp/src/tools/remember.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/remember.rs).

When to use: Use `RememberKind` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub enum RememberKind {
    User,
    Assistant,
    Document,
    Vocabulary,
}
```

## hm-mcp::AnchorFacet

<a id="rust-crates-hm-mcp-src-tools-remember-rs-anchorfacet"></a>

Source: [`crates/hm-mcp/src/tools/remember.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/remember.rs).

When to use: Use `AnchorFacet` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub enum AnchorFacet {
    Path,
    Symbol,
    Url,
    Entity,
}
```

## hm-mcp::RememberAnchor

<a id="rust-crates-hm-mcp-src-tools-remember-rs-rememberanchor"></a>

Source: [`crates/hm-mcp/src/tools/remember.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/remember.rs).

When to use: Use `RememberAnchor` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub struct RememberAnchor {
    pub facet: AnchorFacet,
    pub value: String,
}
```

## hm-mcp::RetentionInput

<a id="rust-crates-hm-mcp-src-tools-remember-rs-retentioninput"></a>

Source: [`crates/hm-mcp/src/tools/remember.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/remember.rs).

When to use: Use `RetentionInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub enum RetentionInput {
    CurrentState,
    Daily,
    Durable,
    DoNotStore,
}
```

## hm-mcp::SensitivityInput

<a id="rust-crates-hm-mcp-src-tools-remember-rs-sensitivityinput"></a>

Source: [`crates/hm-mcp/src/tools/remember.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/remember.rs).

When to use: Use `SensitivityInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub enum SensitivityInput {
    Public,
    Personal,
    Secret,
}
```

## hm-mcp::VocabularyInput

<a id="rust-crates-hm-mcp-src-tools-remember-rs-vocabularyinput"></a>

Source: [`crates/hm-mcp/src/tools/remember.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/remember.rs).

When to use: Use `VocabularyInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct VocabularyInput {
    pub vocabulary_id: String,
    pub version: u16,
    pub source_uri: String,
}
```

## hm-mcp::RememberInput

<a id="rust-crates-hm-mcp-src-tools-remember-rs-rememberinput"></a>

Source: [`crates/hm-mcp/src/tools/remember.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/remember.rs).

When to use: Use `RememberInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct RememberInput {
    pub conversation: String,
    pub content: String,
    pub kind: RememberKind,
    #[serde(default)]
    pub chunk_bytes: Option<usize>,
    #[serde(default)]
    pub anchor: Option<RememberAnchor>,
    #[serde(default)]
    pub retention: Option<RetentionInput>,
    #[serde(default)]
    pub sensitivity: Option<SensitivityInput>,
    #[serde(default)]
    pub vocabulary: Option<VocabularyInput>,
}
```

## hm-mcp::RetractInput

<a id="rust-crates-hm-mcp-src-tools-retract-rs-retractinput"></a>

Source: [`crates/hm-mcp/src/tools/retract.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/retract.rs).

When to use: Use `RetractInput` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct RetractInput {
    pub conversation: String,
    pub belief_id: String,
    pub provenance: Vec<ProvenanceInput>,
}
```

## hm-mcp::Requirement

<a id="rust-crates-hm-mcp-src-tools-surfaces-rs-requirement"></a>

Source: [`crates/hm-mcp/src/tools/surfaces.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/surfaces.rs).

When to use: Use `Requirement` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub enum Requirement {
    None,
    AdminToken,
    EmbeddingRuntime,
    ConsolidationRuntime,
    ReconstructionRuntime,
    DisputeRuntime,
}
```

## hm-mcp::Availability

<a id="rust-crates-hm-mcp-src-tools-surfaces-rs-availability"></a>

Source: [`crates/hm-mcp/src/tools/surfaces.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/surfaces.rs).

When to use: Use `Availability` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub struct Availability {
    pub admin_token: bool,
    pub embedding: bool,
    pub consolidation: bool,
    pub reconstruction: bool,
    pub dispute: bool,
}
```

## hm-mcp::Surface

<a id="rust-crates-hm-mcp-src-tools-surfaces-rs-surface"></a>

Source: [`crates/hm-mcp/src/tools/surfaces.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/surfaces.rs).

When to use: Use `Surface` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub struct Surface {
    pub verb: &'static str,
    pub surface: &'static str,
    pub summary: &'static str,
    pub arguments: &'static str,
    pub mutation: bool,
    pub requirement: Requirement,
    pub keywords: &'static [&'static str],
}
```

## hm-mcp::CrawlPolicy

<a id="rust-crates-hm-mcp-src-tools-websource-rs-crawlpolicy"></a>

Source: [`crates/hm-mcp/src/tools/websource.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/websource.rs).

When to use: Use `CrawlPolicy` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub struct CrawlPolicy {
    pub allowed_hosts: Vec<String>,
    pub maximum_bytes: usize,
    pub maximum_redirects: u8,
    pub minimum_interval_ms: u64,
    pub request_timeout_ms: u64,
    pub allow_cross_host_redirect: bool,
}
```

## hm-mcp::FetchResponse

<a id="rust-crates-hm-mcp-src-tools-websource-rs-fetchresponse"></a>

Source: [`crates/hm-mcp/src/tools/websource.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/websource.rs).

When to use: Use `FetchResponse` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct FetchResponse {
    pub status: u16,
    pub content_type: Option<String>,
    pub content_length: Option<u64>,
    pub location: Option<String>,
    pub body: Vec<u8>,
}
```

## hm-mcp::FetchedSource

<a id="rust-crates-hm-mcp-src-tools-websource-rs-fetchedsource"></a>

Source: [`crates/hm-mcp/src/tools/websource.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/websource.rs).

When to use: Use `FetchedSource` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub struct FetchedSource {
    pub requested_url: String,
    pub final_url: String,
    pub media_type: String,
    pub status: u16,
    pub redirects: Vec<String>,
    pub bytes: Vec<u8>,
    pub digest: [u8; 32],
}
```

## hm-mcp::FetchTransport

<a id="rust-crates-hm-mcp-src-tools-websource-rs-fetchtransport"></a>

Source: [`crates/hm-mcp/src/tools/websource.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/websource.rs).

When to use: Use `FetchTransport` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub trait FetchTransport: Send + Sync {
    fn fetch(
        &self,
        url: &str,
        timeout_ms: u64,
        maximum_bytes: usize,
    ) -> Result<FetchResponse, Error>;
}
```

## hm-mcp::HttpFetchTransport

<a id="rust-crates-hm-mcp-src-tools-websource-rs-httpfetchtransport"></a>

Source: [`crates/hm-mcp/src/tools/websource.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/websource.rs).

When to use: Use `HttpFetchTransport` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub struct HttpFetchTransport;
```

## hm-mcp::RecordedFetchTransport

<a id="rust-crates-hm-mcp-src-tools-websource-rs-recordedfetchtransport"></a>

Source: [`crates/hm-mcp/src/tools/websource.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/websource.rs).

When to use: Use `RecordedFetchTransport` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub struct RecordedFetchTransport {
    responses: Mutex<VecDeque<(String, FetchResponse)>>,
}
```

## hm-mcp::WebSourceRuntime

<a id="rust-crates-hm-mcp-src-tools-websource-rs-websourceruntime"></a>

Source: [`crates/hm-mcp/src/tools/websource.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/websource.rs).

When to use: Use `WebSourceRuntime` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub struct WebSourceRuntime {
    policy: CrawlPolicy,
    transport: Arc<dyn FetchTransport>,
    last_request: Mutex<HashMap<String, Instant>>,
}
```

## hm-mcp::ExtractedText

<a id="rust-crates-hm-mcp-src-tools-webtext-rs-extractedtext"></a>

Source: [`crates/hm-mcp/src/tools/webtext.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-mcp/src/tools/webtext.rs).

When to use: Use `ExtractedText` for typed MCP tool arguments, common envelopes, and explicitly configured provider-backed operations.

Do not use: Do not ignore ok/effect_state, manufacture observed evidence through remember, or bypass destructive-operation authority.


```rust
pub struct ExtractedText {
    pub media_type: String,
    pub title: Option<String>,
    pub text: String,
}
```

## hm-proj::AttentionRecord

<a id="rust-crates-hm-proj-src-attention-rs-attentionrecord"></a>

Source: [`crates/hm-proj/src/attention.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/attention.rs).

When to use: Use `AttentionRecord` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct AttentionRecord {
    pub lsn: u64,
    pub intention_id: Vec<u8>,
    pub wake_id: Vec<u8>,
    pub decision: AttentionDecision,
    pub reason: String,
}
```

## hm-proj::AttentionProjection

<a id="rust-crates-hm-proj-src-attention-rs-attentionprojection"></a>

Source: [`crates/hm-proj/src/attention.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/attention.rs).

When to use: Use `AttentionProjection` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct AttentionProjection;
```

## hm-proj::AttestationRecord

<a id="rust-crates-hm-proj-src-attestations-rs-attestationrecord"></a>

Source: [`crates/hm-proj/src/attestations.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/attestations.rs).

When to use: Use `AttestationRecord` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct AttestationRecord {
    pub target_lsn: u64,
    pub used: u64,
    pub ignored: u64,
    pub helpful: u64,
    pub harmful: u64,
    pub last_attestation_lsn: u64,
}
```

## hm-proj::PreferenceWeight

<a id="rust-crates-hm-proj-src-attestations-rs-preferenceweight"></a>

Source: [`crates/hm-proj/src/attestations.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/attestations.rs).

When to use: Use `PreferenceWeight` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct PreferenceWeight {
    pub target_lsn: u64,
    pub weight_q16: u32,
    pub observations: u64,
    pub last_attestation_lsn: u64,
}
```

## hm-proj::AttestationsProjection

<a id="rust-crates-hm-proj-src-attestations-rs-attestationsprojection"></a>

Source: [`crates/hm-proj/src/attestations.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/attestations.rs).

When to use: Use `AttestationsProjection` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct AttestationsProjection;
```

## hm-proj::BeliefProvenance

<a id="rust-crates-hm-proj-src-beliefs-rs-beliefprovenance"></a>

Source: [`crates/hm-proj/src/beliefs.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/beliefs.rs).

When to use: Use `BeliefProvenance` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct BeliefProvenance {
    pub first_lsn: u64,
    pub last_lsn: u64,
    pub byte_start: u32,
    pub byte_end: u32,
}
```

## hm-proj::BeliefConflictEdge

<a id="rust-crates-hm-proj-src-beliefs-rs-beliefconflictedge"></a>

Source: [`crates/hm-proj/src/beliefs.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/beliefs.rs).

When to use: Use `BeliefConflictEdge` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct BeliefConflictEdge {
    pub other_type: BeliefType,
    pub other_canonical_identity: String,
    pub created_lsn: u64,
    pub resolved_lsn: u64,
    pub obligated_surfacing: bool,
}
```

## hm-proj::BeliefRecord

<a id="rust-crates-hm-proj-src-beliefs-rs-beliefrecord"></a>

Source: [`crates/hm-proj/src/beliefs.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/beliefs.rs).

When to use: Use `BeliefRecord` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct BeliefRecord {
    pub belief_type: BeliefType,
    pub belief_id: Vec<u8>,
    pub canonical_identity: String,
    pub conflict_domain: String,
    pub value: Vec<u8>,
    pub claim: AssertionClaim,
    pub event_time_ns: i64,
    pub observation_lsn: u64,
    pub valid_from_ns: i64,
    pub valid_to_ns: i64,
    pub version: u64,
    pub supersedes_version: u64,
    pub provenance: Vec<BeliefProvenance>,
    pub conflict_edges: Vec<BeliefConflictEdge>,
    pub authority: Authority,
    pub tombstoned: bool,
}
```

## hm-proj::PendingProposal

<a id="rust-crates-hm-proj-src-beliefs-rs-pendingproposal"></a>

Source: [`crates/hm-proj/src/beliefs.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/beliefs.rs).

When to use: Use `PendingProposal` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct PendingProposal {
    pub belief_id: Vec<u8>,
    pub belief_type: BeliefType,
    pub canonical_identity: String,
    pub conflict_domain: String,
    pub value: Vec<u8>,
    pub claim: AssertionClaim,
    pub event_time_ns: i64,
    pub observation_lsn: u64,
    pub valid_from_ns: i64,
    pub valid_to_ns: i64,
    pub provenance: Vec<BeliefProvenance>,
    pub authority: Authority,
    pub run_id: Option<Vec<u8>>,
}
```

## hm-proj::BeliefAsOf

<a id="rust-crates-hm-proj-src-beliefs-rs-beliefasof"></a>

Source: [`crates/hm-proj/src/beliefs.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/beliefs.rs).

When to use: Use `BeliefAsOf` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub enum BeliefAsOf {
    ValidAt(i64),
    KnownAt(LSN),
}
```

## hm-proj::BeliefAsOfAxis

<a id="rust-crates-hm-proj-src-beliefs-rs-beliefasofaxis"></a>

Source: [`crates/hm-proj/src/beliefs.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/beliefs.rs).

When to use: Use `BeliefAsOfAxis` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub enum BeliefAsOfAxis {
    ValidTime,
    KnownLsn,
}
```

## hm-proj::BeliefAsOfResult

<a id="rust-crates-hm-proj-src-beliefs-rs-beliefasofresult"></a>

Source: [`crates/hm-proj/src/beliefs.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/beliefs.rs).

When to use: Use `BeliefAsOfResult` for snapshot reads and deterministic materialization of already-committed ledger events. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct BeliefAsOfResult {
    pub axis: BeliefAsOfAxis,
    pub record: Option<BeliefRecord>,
}
```

## hm-proj::BeliefRebuildProgress

<a id="rust-crates-hm-proj-src-beliefs-rs-beliefrebuildprogress"></a>

Source: [`crates/hm-proj/src/beliefs.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/beliefs.rs).

When to use: Use `BeliefRebuildProgress` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct BeliefRebuildProgress {
    pub applied_lsn: LSN,
    pub applied_frames: usize,
    pub complete: bool,
}
```

## hm-proj::BeliefProjection

<a id="rust-crates-hm-proj-src-beliefs-rs-beliefprojection"></a>

Source: [`crates/hm-proj/src/beliefs.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/beliefs.rs).

When to use: Use `BeliefProjection` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct BeliefProjection;
```

## hm-proj::BindingTarget

<a id="rust-crates-hm-proj-src-bindings-rs-bindingtarget"></a>

Source: [`crates/hm-proj/src/bindings.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/bindings.rs).

When to use: Use `BindingTarget` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub enum BindingTarget {
    Task(Vec<u8>),
    Scope(Vec<u8>),
}
```

## hm-proj::BindingRecord

<a id="rust-crates-hm-proj-src-bindings-rs-bindingrecord"></a>

Source: [`crates/hm-proj/src/bindings.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/bindings.rs).

When to use: Use `BindingRecord` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct BindingRecord {
    pub target: BindingTarget,
    pub canonical_entity: String,
    pub property: String,
    pub evidence_lsn: LSN,
    pub revision: Vec<u8>,
    pub freshness_requirement_ns: u64,
    pub effective_time_ns: UtcNanos,
    pub binding_lsn: LSN,
}
```

## hm-proj::BindingRequirement

<a id="rust-crates-hm-proj-src-bindings-rs-bindingrequirement"></a>

Source: [`crates/hm-proj/src/bindings.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/bindings.rs).

When to use: Use `BindingRequirement` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct BindingRequirement {
    pub canonical_entity: String,
    pub property: String,
    pub revision: Option<Vec<u8>>,
    pub freshness_requirement_ns: Option<u64>,
}
```

## hm-proj::BindingStatus

<a id="rust-crates-hm-proj-src-bindings-rs-bindingstatus"></a>

Source: [`crates/hm-proj/src/bindings.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/bindings.rs).

When to use: Use `BindingStatus` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub enum BindingStatus {
    Resolved,
    Missing,
    Stale,
    Conflicting,
}
```

## hm-proj::BindingResolution

<a id="rust-crates-hm-proj-src-bindings-rs-bindingresolution"></a>

Source: [`crates/hm-proj/src/bindings.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/bindings.rs).

When to use: Use `BindingResolution` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct BindingResolution {
    pub requirement: BindingRequirement,
    pub status: BindingStatus,
    pub binding: Option<BindingRecord>,
}
```

## hm-proj::BindingsRebuildProgress

<a id="rust-crates-hm-proj-src-bindings-rs-bindingsrebuildprogress"></a>

Source: [`crates/hm-proj/src/bindings.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/bindings.rs).

When to use: Use `BindingsRebuildProgress` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct BindingsRebuildProgress {
    pub applied_lsn: LSN,
    pub applied_frames: usize,
    pub complete: bool,
}
```

## hm-proj::BindingsProjection

<a id="rust-crates-hm-proj-src-bindings-rs-bindingsprojection"></a>

Source: [`crates/hm-proj/src/bindings.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/bindings.rs).

When to use: Use `BindingsProjection` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct BindingsProjection;
```

## hm-proj::CheckpointRead

<a id="rust-crates-hm-proj-src-checkpoint-rs-checkpointread"></a>

Source: [`crates/hm-proj/src/checkpoint.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/checkpoint.rs).

When to use: Use `CheckpointRead` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct CheckpointRead {
    pub lsn: LSN,
    pub blob: Vec<u8>,
}
```

## hm-proj::DocumentRecord

<a id="rust-crates-hm-proj-src-documents-rs-documentrecord"></a>

Source: [`crates/hm-proj/src/documents.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/documents.rs).

When to use: Use `DocumentRecord` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct DocumentRecord {
    pub document_id: Vec<u8>,
    pub name: String,
    pub media_type: String,
    pub content_digest: Vec<u8>,
    pub byte_length: u64,
    pub ingest_lsn: u64,
}
```

## hm-proj::ExtractionRecord

<a id="rust-crates-hm-proj-src-documents-rs-extractionrecord"></a>

Source: [`crates/hm-proj/src/documents.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/documents.rs).

When to use: Use `ExtractionRecord` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct ExtractionRecord {
    pub document_id: Vec<u8>,
    pub source_lsn: u64,
    pub loader_id: String,
    pub extraction_version: u16,
    pub text: Vec<u8>,
    pub text_digest: Vec<u8>,
    pub partial_reason: String,
    pub failed_units: Vec<u32>,
    pub run_id: Vec<u8>,
    pub generation: u64,
    pub event_lsn: u64,
}
```

## hm-proj::ChunkRecord

<a id="rust-crates-hm-proj-src-documents-rs-chunkrecord"></a>

Source: [`crates/hm-proj/src/documents.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/documents.rs).

When to use: Use `ChunkRecord` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct ChunkRecord {
    pub document_id: Vec<u8>,
    pub chunk_id: Vec<u8>,
    pub content_hash: Vec<u8>,
    pub occurrence: u32,
    pub ordinal: u32,
    pub byte_start: u32,
    pub byte_end: u32,
    pub cut: u8,
    pub change: u8,
    pub page_number: u32,
    pub row_index: u32,
    pub column_start: u32,
    pub column_end: u32,
    pub token_estimate: u32,
    pub run_id: Vec<u8>,
    pub generation: u64,
    pub event_lsn: u64,
}
```

## hm-proj::DocumentsProjection

<a id="rust-crates-hm-proj-src-documents-rs-documentsprojection"></a>

Source: [`crates/hm-proj/src/documents.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/documents.rs).

When to use: Use `DocumentsProjection` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct DocumentsProjection;
```

## hm-proj::EntityHit

<a id="rust-crates-hm-proj-src-entities-rs-entityhit"></a>

Source: [`crates/hm-proj/src/entities.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/entities.rs).

When to use: Use `EntityHit` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct EntityHit {
    pub lsn: LSN,
    pub matched_aliases: Vec<String>,
}
```

## hm-proj::EntityProjection

<a id="rust-crates-hm-proj-src-entities-rs-entityprojection"></a>

Source: [`crates/hm-proj/src/entities.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/entities.rs).

When to use: Use `EntityProjection` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct EntityProjection;
```

## hm-proj::FsrsState

<a id="rust-crates-hm-proj-src-fsrs-rs-fsrsstate"></a>

Source: [`crates/hm-proj/src/fsrs.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/fsrs.rs).

When to use: Use `FsrsState` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct FsrsState {
    pub memory_id: Vec<u8>,
    pub rating: ReviewRating,
    pub source_lsn: u64,
    pub reviewed_at_ns: i64,
    pub stability_millis: u64,
    pub difficulty_micros: u32,
    pub due_at_ns: i64,
    pub run_id: Vec<u8>,
    pub generation: u64,
    pub event_lsn: u64,
}
```

## hm-proj::FsrsProjection

<a id="rust-crates-hm-proj-src-fsrs-rs-fsrsprojection"></a>

Source: [`crates/hm-proj/src/fsrs.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/fsrs.rs).

When to use: Use `FsrsProjection` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct FsrsProjection;
```

## hm-proj::GenerationProjection

<a id="rust-crates-hm-proj-src-generation-rs-generationprojection"></a>

Source: [`crates/hm-proj/src/generation.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/generation.rs).

When to use: Use `GenerationProjection` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct GenerationProjection;
```

## hm-proj::EdgeCitation

<a id="rust-crates-hm-proj-src-graph-rs-edgecitation"></a>

Source: [`crates/hm-proj/src/graph.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/graph.rs).

When to use: Use `EdgeCitation` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct EdgeCitation {
    pub first_lsn: u64,
    pub last_lsn: u64,
    pub byte_start: u32,
    pub byte_end: u32,
}
```

## hm-proj::EdgeRecord

<a id="rust-crates-hm-proj-src-graph-rs-edgerecord"></a>

Source: [`crates/hm-proj/src/graph.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/graph.rs).

When to use: Use `EdgeRecord` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct EdgeRecord {
    pub edge_id: Vec<u8>,
    pub source_id: Vec<u8>,
    pub target_id: Vec<u8>,
    pub relation: String,
    pub weight_micros: u32,
    pub valid_from_ns: i64,
    pub valid_to_ns: i64,
    pub citations: Vec<EdgeCitation>,
    pub run_id: Vec<u8>,
    pub generation: u64,
    pub event_lsn: u64,
    pub model_id: String,
    pub prompt_id: String,
    pub prompt_version: u16,
    pub call_id: Vec<u8>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_microusd: u64,
}
```

## hm-proj::GraphProjection

<a id="rust-crates-hm-proj-src-graph-rs-graphprojection"></a>

Source: [`crates/hm-proj/src/graph.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/graph.rs).

When to use: Use `GraphProjection` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct GraphProjection;
```

## hm-proj::LoopClosure

<a id="rust-crates-hm-proj-src-intent-rs-loopclosure"></a>

Source: [`crates/hm-proj/src/intent.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/intent.rs).

When to use: Use `LoopClosure` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub enum LoopClosure {
    Done = 0,
    Abandoned = 1,
    HandedOff = 2,
    Superseded = 3,
}
```

## hm-proj::IntentObjective

<a id="rust-crates-hm-proj-src-intent-rs-intentobjective"></a>

Source: [`crates/hm-proj/src/intent.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/intent.rs).

When to use: Use `IntentObjective` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct IntentObjective {
    pub set_lsn: LSN,
    pub content: Vec<u8>,
}
```

## hm-proj::OpenLoop

<a id="rust-crates-hm-proj-src-intent-rs-openloop"></a>

Source: [`crates/hm-proj/src/intent.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/intent.rs).

When to use: Use `OpenLoop` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct OpenLoop {
    pub opened_lsn: LSN,
    pub loop_id: Vec<u8>,
    pub objective: Vec<u8>,
}
```

## hm-proj::ClosedLoop

<a id="rust-crates-hm-proj-src-intent-rs-closedloop"></a>

Source: [`crates/hm-proj/src/intent.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/intent.rs).

When to use: Use `ClosedLoop` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct ClosedLoop {
    pub opened_lsn: LSN,
    pub closed_lsn: LSN,
    pub reason: LoopClosure,
    pub loop_id: Vec<u8>,
    pub objective: Vec<u8>,
    pub cause: Vec<u8>,
}
```

## hm-proj::IntentFrameView

<a id="rust-crates-hm-proj-src-intent-rs-intentframeview"></a>

Source: [`crates/hm-proj/src/intent.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/intent.rs).

When to use: Use `IntentFrameView` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct IntentFrameView {
    pub objective: Option<IntentObjective>,
    pub open_loops: Vec<OpenLoop>,
}
```

## hm-proj::IntentRebuildProgress

<a id="rust-crates-hm-proj-src-intent-rs-intentrebuildprogress"></a>

Source: [`crates/hm-proj/src/intent.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/intent.rs).

When to use: Use `IntentRebuildProgress` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct IntentRebuildProgress {
    pub applied_lsn: LSN,
    pub applied_frames: usize,
    pub complete: bool,
}
```

## hm-proj::IntentFrameProjection

<a id="rust-crates-hm-proj-src-intent-rs-intentframeprojection"></a>

Source: [`crates/hm-proj/src/intent.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/intent.rs).

When to use: Use `IntentFrameProjection` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct IntentFrameProjection;
```

## hm-proj::TriggerKind

<a id="rust-crates-hm-proj-src-intentions-rs-triggerkind"></a>

Source: [`crates/hm-proj/src/intentions.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/intentions.rs).

When to use: Use `TriggerKind` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub enum TriggerKind {
    At,
    Schedule,
    ChildTerminal,
    ProcessExit,
    FileChanged,
    RepositoryChanged,
    ChannelMessage,
    ExternalCondition,
    UserResponse,
    EntityMentioned,
    LoopClosed,
    PredictionResolved,
    BeliefChanged,
}
```

## hm-proj::IntentionStatus

<a id="rust-crates-hm-proj-src-intentions-rs-intentionstatus"></a>

Source: [`crates/hm-proj/src/intentions.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/intentions.rs).

When to use: Use `IntentionStatus` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub enum IntentionStatus {
    Pending,
    Fired,
    Cancelled,
}
```

## hm-proj::IntentionRecord

<a id="rust-crates-hm-proj-src-intentions-rs-intentionrecord"></a>

Source: [`crates/hm-proj/src/intentions.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/intentions.rs).

When to use: Use `IntentionRecord` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct IntentionRecord {
    pub intention_id: Vec<u8>,
    pub objective: Vec<u8>,
    pub trigger: WakeTrigger,
    pub trigger_kind: TriggerKind,
    pub expires_at_ns: i64,
    pub reply_route: String,
    pub set_lsn: u64,
    pub status_lsn: u64,
    pub status: IntentionStatus,
    pub wake_id: Option<Vec<u8>>,
    pub trigger_lsn: u64,
    pub cancellation_reason: Option<String>,
}
```

## hm-proj::IntentionsProjection

<a id="rust-crates-hm-proj-src-intentions-rs-intentionsprojection"></a>

Source: [`crates/hm-proj/src/intentions.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/intentions.rs).

When to use: Use `IntentionsProjection` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct IntentionsProjection;
```

## hm-proj::TemporalLevel

<a id="rust-crates-hm-proj-src-ladder-rs-temporallevel"></a>

Source: [`crates/hm-proj/src/ladder.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/ladder.rs).

When to use: Use `TemporalLevel` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub enum TemporalLevel {
    Minute,
    Hour,
    Day,
    Week,
}
```

## hm-proj::TemporalWindow

<a id="rust-crates-hm-proj-src-ladder-rs-temporalwindow"></a>

Source: [`crates/hm-proj/src/ladder.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/ladder.rs).

When to use: Use `TemporalWindow` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct TemporalWindow {
    pub level: TemporalLevel,
    pub start_ns: i64,
    pub end_ns: i64,
    pub member_count: u64,
}
```

## hm-proj::TemporalRebuildProgress

<a id="rust-crates-hm-proj-src-ladder-rs-temporalrebuildprogress"></a>

Source: [`crates/hm-proj/src/ladder.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/ladder.rs).

When to use: Use `TemporalRebuildProgress` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct TemporalRebuildProgress {
    pub applied_lsn: LSN,
    pub applied_frames: usize,
    pub complete: bool,
}
```

## hm-proj::TemporalLadder

<a id="rust-crates-hm-proj-src-ladder-rs-temporalladder"></a>

Source: [`crates/hm-proj/src/ladder.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/ladder.rs).

When to use: Use `TemporalLadder` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct TemporalLadder;
```

## hm-proj::GenerationLease

<a id="rust-crates-hm-proj-src-lease-rs-generationlease"></a>

Source: [`crates/hm-proj/src/lease.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/lease.rs).

When to use: Use `GenerationLease` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct GenerationLease {
    pub lease_id: u64,
    pub generation: u64,
    pub expires_at_ns: i64,
}
```

## hm-proj::LeaseManager

<a id="rust-crates-hm-proj-src-lease-rs-leasemanager"></a>

Source: [`crates/hm-proj/src/lease.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/lease.rs).

When to use: Use `LeaseManager` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct LeaseManager {
    next_id: AtomicU64,
    leases: Mutex<BTreeMap<u64, GenerationLease>>,
}
```

## hm-proj::WorkKind

<a id="rust-crates-hm-proj-src-ledger-rs-workkind"></a>

Source: [`crates/hm-proj/src/ledger.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/ledger.rs).

When to use: Use `WorkKind` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub enum WorkKind {
    ToolCall = 0,
    Effect = 1,
}
```

## hm-proj::WorkState

<a id="rust-crates-hm-proj-src-ledger-rs-workstate"></a>

Source: [`crates/hm-proj/src/ledger.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/ledger.rs).

When to use: Use `WorkState` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub enum WorkState {
    Dispatched = 0,
    Committed = 1,
    Returned = 2,
    OutcomeUnknown = 3,
}
```

## hm-proj::WorkItem

<a id="rust-crates-hm-proj-src-ledger-rs-workitem"></a>

Source: [`crates/hm-proj/src/ledger.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/ledger.rs).

When to use: Use `WorkItem` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct WorkItem {
    pub kind: WorkKind,
    pub state: WorkState,
    pub tool_call_lsn: LSN,
    pub state_lsn: LSN,
    pub call_id: Vec<u8>,
    pub tool_name: Vec<u8>,
    pub arguments: Vec<u8>,
    pub effect_id: Vec<u8>,
    pub detail: Vec<u8>,
    pub requires_reconciliation: bool,
}
```

## hm-proj::LedgerRebuildProgress

<a id="rust-crates-hm-proj-src-ledger-rs-ledgerrebuildprogress"></a>

Source: [`crates/hm-proj/src/ledger.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/ledger.rs).

When to use: Use `LedgerRebuildProgress` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct LedgerRebuildProgress {
    pub applied_lsn: LSN,
    pub applied_frames: usize,
    pub complete: bool,
}
```

## hm-proj::WorkLedgerProjection

<a id="rust-crates-hm-proj-src-ledger-rs-workledgerprojection"></a>

Source: [`crates/hm-proj/src/ledger.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/ledger.rs).

When to use: Use `WorkLedgerProjection` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct WorkLedgerProjection;
```

## hm-proj::LexicalHit

<a id="rust-crates-hm-proj-src-lexical-rs-lexicalhit"></a>

Source: [`crates/hm-proj/src/lexical.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/lexical.rs).

When to use: Use `LexicalHit` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct LexicalHit {
    pub lsn: LSN,
    pub score_q32: u64,
}
```

## hm-proj::LexicalProjection

<a id="rust-crates-hm-proj-src-lexical-rs-lexicalprojection"></a>

Source: [`crates/hm-proj/src/lexical.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/lexical.rs).

When to use: Use `LexicalProjection` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct LexicalProjection;
```

## hm-proj::MemoryCitation

<a id="rust-crates-hm-proj-src-memories-rs-memorycitation"></a>

Source: [`crates/hm-proj/src/memories.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/memories.rs).

When to use: Use `MemoryCitation` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct MemoryCitation {
    pub first_lsn: u64,
    pub last_lsn: u64,
    pub byte_start: u32,
    pub byte_end: u32,
}
```

## hm-proj::MemoryModelProvenance

<a id="rust-crates-hm-proj-src-memories-rs-memorymodelprovenance"></a>

Source: [`crates/hm-proj/src/memories.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/memories.rs).

When to use: Use `MemoryModelProvenance` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct MemoryModelProvenance {
    pub model_id: String,
    pub prompt_id: String,
    pub prompt_version: u16,
    pub call_id: Vec<u8>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub cost_microusd: u64,
}
```

## hm-proj::MemoryRecord

<a id="rust-crates-hm-proj-src-memories-rs-memoryrecord"></a>

Source: [`crates/hm-proj/src/memories.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/memories.rs).

When to use: Use `MemoryRecord` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct MemoryRecord {
    pub memory_id: Vec<u8>,
    pub name: String,
    pub definition: Vec<u8>,
    pub tags: Vec<String>,
    pub salience_micros: u32,
    pub citations: Vec<MemoryCitation>,
    pub authority: Authority,
    pub run_id: Vec<u8>,
    pub generation: u64,
    pub version_lsn: u64,
    pub previous_lsn: u64,
    pub merged_from: Vec<Vec<u8>>,
    pub model_provenance: MemoryModelProvenance,
}
```

## hm-proj::MemoryProjection

<a id="rust-crates-hm-proj-src-memories-rs-memoryprojection"></a>

Source: [`crates/hm-proj/src/memories.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/memories.rs).

When to use: Use `MemoryProjection` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct MemoryProjection;
```

## hm-proj::PredictionRecord

<a id="rust-crates-hm-proj-src-predictions-rs-predictionrecord"></a>

Source: [`crates/hm-proj/src/predictions.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/predictions.rs).

When to use: Use `PredictionRecord` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct PredictionRecord {
    pub prediction_id: Vec<u8>,
    pub revision: u32,
    pub task_id: Option<Vec<u8>>,
    pub attempt_id: Option<Vec<u8>>,
    pub operation_id: Option<Vec<u8>>,
    pub mechanism: String,
    pub predicates: Vec<ExpectedPredicate>,
    pub deadline_ns: i64,
    pub uncertainty: String,
    pub predicted_lsn: u64,
    pub assessment: Option<OutcomeAssessment>,
    pub observation_lsns: Vec<u64>,
    pub evaluator_version: Option<String>,
    pub outcome_lsn: u64,
}
```

## hm-proj::CalibrationCounters

<a id="rust-crates-hm-proj-src-predictions-rs-calibrationcounters"></a>

Source: [`crates/hm-proj/src/predictions.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/predictions.rs).

When to use: Use `CalibrationCounters` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct CalibrationCounters {
    pub supported: u64,
    pub contradicted: u64,
    pub pending: u64,
    pub unresolvable: u64,
    pub not_executed: u64,
}
```

## hm-proj::MechanismFailures

<a id="rust-crates-hm-proj-src-predictions-rs-mechanismfailures"></a>

Source: [`crates/hm-proj/src/predictions.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/predictions.rs).

When to use: Use `MechanismFailures` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct MechanismFailures {
    pub consecutive: u32,
    pub revision_required: bool,
}
```

## hm-proj::PredictionsProjection

<a id="rust-crates-hm-proj-src-predictions-rs-predictionsprojection"></a>

Source: [`crates/hm-proj/src/predictions.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/predictions.rs).

When to use: Use `PredictionsProjection` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct PredictionsProjection;
```

## hm-proj::ProcedureState

<a id="rust-crates-hm-proj-src-procedures-rs-procedurestate"></a>

Source: [`crates/hm-proj/src/procedures.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/procedures.rs).

When to use: Use `ProcedureState` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub enum ProcedureState {
    Tentative,
    Supported,
    Adopted,
}
```

## hm-proj::ProcedureRecord

<a id="rust-crates-hm-proj-src-procedures-rs-procedurerecord"></a>

Source: [`crates/hm-proj/src/procedures.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/procedures.rs).

When to use: Use `ProcedureRecord` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct ProcedureRecord {
    pub procedure_id: Vec<u8>,
    pub strategy: String,
    pub expected_outcomes: Vec<String>,
    pub preconditions: Vec<String>,
    pub supports: Vec<ProcedureSupport>,
    pub failures: Vec<u64>,
    pub counterexamples: Vec<u64>,
    pub state: ProcedureState,
    pub version_lsn: u64,
    pub previous_lsn: u64,
    pub adopted_lsn: u64,
}
```

## hm-proj::ProceduresProjection

<a id="rust-crates-hm-proj-src-procedures-rs-proceduresprojection"></a>

Source: [`crates/hm-proj/src/procedures.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/procedures.rs).

When to use: Use `ProceduresProjection` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct ProceduresProjection;
```

## hm-proj::SecurityEvent

<a id="rust-crates-hm-proj-src-protected-rs-securityevent"></a>

Source: [`crates/hm-proj/src/protected.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/protected.rs).

When to use: Use `SecurityEvent` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct SecurityEvent {
    pub observation_lsn: u64,
    pub code: String,
    pub belief_type: BeliefType,
    pub belief_id: Vec<u8>,
    pub authority: Authority,
    pub run_id: Option<Vec<u8>>,
}
```

## hm-proj::RebuildProgress

<a id="rust-crates-hm-proj-src-rebuild-rs-rebuildprogress"></a>

Source: [`crates/hm-proj/src/rebuild.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/rebuild.rs).

When to use: Use `RebuildProgress` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct RebuildProgress {
    pub applied_lsn: LSN,
    pub applied_frames: usize,
    pub complete: bool,
}
```

## hm-proj::RunStatus

<a id="rust-crates-hm-proj-src-runs-rs-runstatus"></a>

Source: [`crates/hm-proj/src/runs.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/runs.rs).

When to use: Use `RunStatus` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub enum RunStatus {
    Open,
    Published,
    Retracted,
}
```

## hm-proj::StagedProjection

<a id="rust-crates-hm-proj-src-runs-rs-stagedprojection"></a>

Source: [`crates/hm-proj/src/runs.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/runs.rs).

When to use: Use `StagedProjection` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub enum StagedProjection {
    Memories,
    Graph,
    Fsrs,
    Documents,
}
```

## hm-proj::PromptRecord

<a id="rust-crates-hm-proj-src-runs-rs-promptrecord"></a>

Source: [`crates/hm-proj/src/runs.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/runs.rs).

When to use: Use `PromptRecord` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct PromptRecord {
    pub prompt_id: String,
    pub version: u16,
    pub model_id: String,
}
```

## hm-proj::BudgetRecord

<a id="rust-crates-hm-proj-src-runs-rs-budgetrecord"></a>

Source: [`crates/hm-proj/src/runs.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/runs.rs).

When to use: Use `BudgetRecord` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct BudgetRecord {
    pub max_llm_calls: u64,
    pub max_tokens: u64,
    pub max_microusd: u64,
    pub max_wall_ms: u64,
}
```

## hm-proj::PhaseRecord

<a id="rust-crates-hm-proj-src-runs-rs-phaserecord"></a>

Source: [`crates/hm-proj/src/runs.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/runs.rs).

When to use: Use `PhaseRecord` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct PhaseRecord {
    pub phase: u8,
    pub state: u8,
    pub attempt_prefix: Vec<u8>,
    pub cursor: Option<Vec<u8>>,
    pub llm_calls: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_microusd: u64,
    pub dropped_candidates: u64,
    pub event_lsn: u64,
}
```

## hm-proj::RunRecord

<a id="rust-crates-hm-proj-src-runs-rs-runrecord"></a>

Source: [`crates/hm-proj/src/runs.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/runs.rs).

When to use: Use `RunRecord` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct RunRecord {
    pub run_id: Vec<u8>,
    pub generation: u64,
    pub parent_generation: u64,
    pub scope_digest: Vec<u8>,
    pub cadence_key: String,
    pub phases: Vec<u8>,
    pub prompts: Vec<PromptRecord>,
    pub budget: BudgetRecord,
    pub status: RunStatus,
    pub opened_lsn: u64,
    pub published_lsn: u64,
    pub retracted_lsn: u64,
    pub staged: BTreeMap<u64, StagedProjection>,
    pub progress: BTreeMap<u8, PhaseRecord>,
    pub derived_records: u64,
    pub dropped_candidates: u64,
    pub llm_calls: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_microusd: u64,
    pub cleanup_scheduled: bool,
}
```

## hm-proj::RunsProjection

<a id="rust-crates-hm-proj-src-runs-rs-runsprojection"></a>

Source: [`crates/hm-proj/src/runs.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/runs.rs).

When to use: Use `RunsProjection` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct RunsProjection;
```

## hm-proj::SpaceDefinition

<a id="rust-crates-hm-proj-src-spaces-rs-spacedefinition"></a>

Source: [`crates/hm-proj/src/spaces.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/spaces.rs).

When to use: Use `SpaceDefinition` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct SpaceDefinition {
    pub generation_id: String,
    pub space_id: String,
    pub encoder_id: String,
    pub revision: String,
    pub dimensions: usize,
    pub distance: String,
    pub normalization: String,
    pub input_role: String,
}
```

## hm-proj::SpaceCatalog

<a id="rust-crates-hm-proj-src-spaces-rs-spacecatalog"></a>

Source: [`crates/hm-proj/src/spaces.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/spaces.rs).

When to use: Use `SpaceCatalog` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct SpaceCatalog {
    root: PathBuf,
}
```

## hm-proj::ProjectionId

<a id="rust-crates-hm-proj-src-store-rs-projectionid"></a>

Source: [`crates/hm-proj/src/store.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/store.rs).

When to use: Use `ProjectionId` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub enum ProjectionId {
    BeliefStore,
    EntityIndex,
    VectorLane,
    Bm25,
    TemporalLadder,
    IntentFrame,
    WorkLedger,
    ConversationHeads,
    Bindings,
    Memories,
    Graph,
    Fsrs,
    Runs,
    Intentions,
    AttentionHistory,
    Predictions,
    Procedures,
    Attestations,
    Vocabulary,
    Documents,
}
```

## hm-proj::MutationKind

<a id="rust-crates-hm-proj-src-store-rs-mutationkind"></a>

Source: [`crates/hm-proj/src/store.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/store.rs).

When to use: Use `MutationKind` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub enum MutationKind {
    Put,
    Delete,
}
```

## hm-proj::Mutation

<a id="rust-crates-hm-proj-src-store-rs-mutation"></a>

Source: [`crates/hm-proj/src/store.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/store.rs).

When to use: Use `Mutation` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct Mutation {
    pub kind: MutationKind,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}
```

## hm-proj::KeyValue

<a id="rust-crates-hm-proj-src-store-rs-keyvalue"></a>

Source: [`crates/hm-proj/src/store.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/store.rs).

When to use: Use `KeyValue` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct KeyValue {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}
```

## hm-proj::ProjectionStore

<a id="rust-crates-hm-proj-src-store-rs-projectionstore"></a>

Source: [`crates/hm-proj/src/store.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/store.rs).

When to use: Use `ProjectionStore` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct ProjectionStore {
    environment: Env<WithoutTls>,
    databases: [ByteDatabase; ProjectionId::COUNT],
    metadata_database: ByteDatabase,
    writer_thread: ThreadId,
}
```

## hm-proj::ReadSnapshot

<a id="rust-crates-hm-proj-src-store-rs-readsnapshot"></a>

Source: [`crates/hm-proj/src/store.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/store.rs).

When to use: Use `ReadSnapshot` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct ReadSnapshot<'environment> {
    transaction: RoTxn<'environment, WithoutTls>,
    databases: [ByteDatabase; ProjectionId::COUNT],
    metadata_database: ByteDatabase,
    epoch: usize,
}
```

## hm-proj::ConversationRecord

<a id="rust-crates-hm-proj-src-timeline-rs-conversationrecord"></a>

Source: [`crates/hm-proj/src/timeline.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/timeline.rs).

When to use: Use `ConversationRecord` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct ConversationRecord {
    pub lsn: LSN,
    pub kind: EventKind,
    pub wall_timestamp_ns: UtcNanos,
    pub conversation: ConversationId,
    pub payload: Vec<u8>,
}
```

## hm-proj::LatestRecordScan

<a id="rust-crates-hm-proj-src-timeline-rs-latestrecordscan"></a>

Source: [`crates/hm-proj/src/timeline.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/timeline.rs).

When to use: Use `LatestRecordScan` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct LatestRecordScan {
    pub record: Option<ConversationRecord>,
    pub next_before_lsn: LSN,
    pub scanned: usize,
}
```

## hm-proj::HnswProjection

<a id="rust-crates-hm-proj-src-vectors-hnsw-rs-hnswprojection"></a>

Source: [`crates/hm-proj/src/vectors_hnsw.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/vectors_hnsw.rs).

When to use: Use `HnswProjection` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct HnswProjection {
    path: PathBuf,
    index: HnswIndex,
    records: BTreeMap<u64, HnswVector>,
    source: blake3::Hasher,
    checkpoint_count: usize,
}
```

## hm-proj::VectorHit

<a id="rust-crates-hm-proj-src-vectors-rs-vectorhit"></a>

Source: [`crates/hm-proj/src/vectors.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/vectors.rs).

When to use: Use `VectorHit` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct VectorHit {
    pub target_lsn: LSN,
    pub score: i64,
    pub hamming_distance: u32,
}
```

## hm-proj::VectorEntry

<a id="rust-crates-hm-proj-src-vectors-rs-vectorentry"></a>

Source: [`crates/hm-proj/src/vectors.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/vectors.rs).

When to use: Use `VectorEntry` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct VectorEntry {
    pub target_lsn: LSN,
    pub quantized: Vec<i8>,
    pub binary_prefilter: Vec<u8>,
}
```

## hm-proj::VectorLane

<a id="rust-crates-hm-proj-src-vectors-rs-vectorlane"></a>

Source: [`crates/hm-proj/src/vectors.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/vectors.rs).

When to use: Use `VectorLane` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct VectorLane {
    path: PathBuf,
    generation_id: String,
    space_id: String,
    dimensions: usize,
    #[cfg(feature = "hnsw")]
    count: Arc<AtomicUsize>,
    #[cfg(feature = "hnsw")]
    hnsw: Arc<Mutex<Option<HnswProjection>>>,
}
```

## hm-proj::VocabularyRecord

<a id="rust-crates-hm-proj-src-vocabulary-rs-vocabularyrecord"></a>

Source: [`crates/hm-proj/src/vocabulary.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/vocabulary.rs).

When to use: Use `VocabularyRecord` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct VocabularyRecord {
    pub vocabulary_id: Vec<u8>,
    pub version: u16,
    pub source_uri: String,
    pub source_media_type: String,
    pub source_digest: Vec<u8>,
    pub term_count: u32,
    pub ignored_triples: u32,
    pub event_lsn: u64,
}
```

## hm-proj::TermRecord

<a id="rust-crates-hm-proj-src-vocabulary-rs-termrecord"></a>

Source: [`crates/hm-proj/src/vocabulary.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/vocabulary.rs).

When to use: Use `TermRecord` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct TermRecord {
    pub vocabulary_id: Vec<u8>,
    pub version: u16,
    pub term_id: String,
    pub canonical_name: String,
    pub category: VocabularyCategory,
    pub parent_term_id: Option<String>,
    pub aliases: Vec<String>,
    pub event_lsn: u64,
}
```

## hm-proj::AliasEntry

<a id="rust-crates-hm-proj-src-vocabulary-rs-aliasentry"></a>

Source: [`crates/hm-proj/src/vocabulary.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/vocabulary.rs).

When to use: Use `AliasEntry` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct AliasEntry {
    pub vocabulary_id: Vec<u8>,
    pub version: u16,
    pub term_id: String,
}
```

## hm-proj::VocabularyProjection

<a id="rust-crates-hm-proj-src-vocabulary-rs-vocabularyprojection"></a>

Source: [`crates/hm-proj/src/vocabulary.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-proj/src/vocabulary.rs).

When to use: Use `VocabularyProjection` for snapshot reads and deterministic materialization of already-committed ledger events.

Do not use: Do not make projections a second source of truth or update data without its checkpoint in the same transaction.


```rust
pub struct VocabularyProjection;
```

## hm-schema::Boundary

<a id="rust-crates-hm-schema-src-event-rs-boundary"></a>

Source: [`crates/hm-schema/src/event.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-schema/src/event.rs).

When to use: Use `Boundary` for validated encoding and decoding at the versioned event or protocol boundary.

Do not use: Do not admit unchecked buffers, renumber wire discriminants, or bypass authority and ordering validation.


```rust
pub enum Boundary {
    Disk,
    Socket,
    Import,
}
```

## hm-schema::EventKind

<a id="rust-crates-hm-schema-src-event-rs-eventkind"></a>

Source: [`crates/hm-schema/src/event.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-schema/src/event.rs).

When to use: Use `EventKind` for validated encoding and decoding at the versioned event or protocol boundary.

Do not use: Do not admit unchecked buffers, renumber wire discriminants, or bypass authority and ordering validation.


```rust
pub enum EventKind {
    UserMsg = 1,
    DeliveredMsg = 2,
    ToolCall = 3,
    ToolResult = 4,
    Reasoning = 5,
    ProviderFrame = 6,
    MediaRef = 7,
    Effect = 8,
    Approval = 9,
    Outcome = 10,
    Checkpoint = 11,
    Supervisor = 12,
    Recovery = 13,
    IntentSet = 14,
    LoopOpened = 15,
    LoopClosed = 16,
    Assertion = 17,
    Consolidation = 18,
    Embedding = 19,
    Retract = 20,
    Attestation = 21,
    Binding = 22,
    ProposedAssertion = 23,
    MemoryMinted = 24,
    MemoryRevised = 25,
    MemoryMerged = 26,
    MemoryFaded = 27,
    EdgeAsserted = 28,
    EdgeRetracted = 29,
    ConsolidationOpened = 30,
    ConsolidationPhase = 31,
    ConsolidationClosed = 32,
    ConsolidationRetracted = 33,
    Reviewed = 34,
    IntentionSet = 35,
    IntentionFired = 36,
    AttentionDecided = 37,
    IntentionCancelled = 38,
    Predicted = 39,
    OutcomeObserved = 40,
    ProcedureMined = 41,
    ProcedureRevised = 42,
    ProcedureAdopted = 43,
    VocabularyImported = 44,
    DocumentIngested = 45,
    DocumentExtracted = 46,
    DocumentChunked = 47,
    SourceConnectorBound = 48,
    SourceDeliveryAccepted = 49,
    SourceDeliverySettled = 50,
    SourceRevisionObserved = 51,
    ProcedureImported = 52,
    ProcedureImprovementProposed = 53,
}
```

## hm-schema::HistorySource

<a id="rust-crates-hm-schema-src-event-rs-historysource"></a>

Source: [`crates/hm-schema/src/event.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-schema/src/event.rs).

When to use: Use `HistorySource` for validated encoding and decoding at the versioned event or protocol boundary.

Do not use: Do not admit unchecked buffers, renumber wire discriminants, or bypass authority and ordering validation.


```rust
pub enum HistorySource {
    LedgerEvent,
    Memory,
    Summary,
    Reconstruction,
}
```

## hm-schema::EventHistory

<a id="rust-crates-hm-schema-src-event-rs-eventhistory"></a>

Source: [`crates/hm-schema/src/event.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-schema/src/event.rs).

When to use: Use `EventHistory` for validated encoding and decoding at the versioned event or protocol boundary.

Do not use: Do not admit unchecked buffers, renumber wire discriminants, or bypass authority and ordering validation.


```rust
pub trait EventHistory {
    fn kind_at(&self, lsn: LSN) -> Option<EventKind>;

    fn authority_at(&self, _lsn: LSN) -> Option<Authority> {
        None
    }

    fn source_at(&self, _lsn: LSN) -> HistorySource {
        HistorySource::LedgerEvent
    }
}
```

## hm-schema::VerifiedEvent

<a id="rust-crates-hm-schema-src-event-rs-verifiedevent"></a>

Source: [`crates/hm-schema/src/event.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-schema/src/event.rs).

When to use: Use `VerifiedEvent` for validated encoding and decoding at the versioned event or protocol boundary.

Do not use: Do not admit unchecked buffers, renumber wire discriminants, or bypass authority and ordering validation.


```rust
pub struct VerifiedEvent {
    pub envelope: EventEnvelope,
    pub kind: EventKind,
    pub boundary: Boundary,
}
```

## hm-schema::VerifiedRequest

<a id="rust-crates-hm-schema-src-protocol-rs-verifiedrequest"></a>

Source: [`crates/hm-schema/src/protocol.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-schema/src/protocol.rs).

When to use: Use `VerifiedRequest` for validated encoding and decoding at the versioned event or protocol boundary. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not admit unchecked buffers, renumber wire discriminants, or bypass authority and ordering validation. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct VerifiedRequest {
    pub proto_version: u16,
    pub request: Request,
}
```

## hm-serve::ActorConfig

<a id="rust-crates-hm-serve-src-actor-rs-actorconfig"></a>

Source: [`crates/hm-serve/src/actor.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/actor.rs).

When to use: Use `ActorConfig` for actor ownership, embedded sessions, authenticated transports, and daemon request execution. Set explicit deployment limits before opening the associated resource.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication. Do not log secret fields or substitute defaults for an explicitly authorized budget.


```rust
pub struct ActorConfig {
    pub actor_directory: PathBuf,
    pub actor: ActorId,
    pub user: UserId,
    pub kek: KeyEncryptionKey,
    pub projection_map_bytes: usize,
}
```

## hm-serve::IncomingEvent

<a id="rust-crates-hm-serve-src-actor-rs-incomingevent"></a>

Source: [`crates/hm-serve/src/actor.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/actor.rs).

When to use: Use `IncomingEvent` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct IncomingEvent {
    pub kind: EventKind,
    pub conversation: ConversationId,
    pub payload: Vec<u8>,
}
```

## hm-serve::AppendOutcome

<a id="rust-crates-hm-serve-src-actor-rs-appendoutcome"></a>

Source: [`crates/hm-serve/src/actor.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/actor.rs).

When to use: Use `AppendOutcome` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct AppendOutcome {
    pub first_lsn: LSN,
    pub last_lsn: LSN,
    pub duplicate: bool,
    pub leaf_count: u64,
    pub last_leaf_hash: MmrHash,
    pub mmr_root: MmrHash,
}
```

## hm-serve::CheckpointOutcome

<a id="rust-crates-hm-serve-src-actor-rs-checkpointoutcome"></a>

Source: [`crates/hm-serve/src/actor.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/actor.rs).

When to use: Use `CheckpointOutcome` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct CheckpointOutcome {
    pub lsn: LSN,
    pub duplicate: bool,
}
```

## hm-serve::IntegrityReceipt

<a id="rust-crates-hm-serve-src-actor-rs-integrityreceipt"></a>

Source: [`crates/hm-serve/src/actor.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/actor.rs).

When to use: Use `IntegrityReceipt` for actor ownership, embedded sessions, authenticated transports, and daemon request execution. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub struct IntegrityReceipt {
    pub lsn: LSN,
    pub leaf_hash: MmrHash,
    pub root: MmrHash,
    pub checkpoint_lsn: LSN,
}
```

## hm-serve::RecallItem

<a id="rust-crates-hm-serve-src-actor-rs-recallitem"></a>

Source: [`crates/hm-serve/src/actor.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/actor.rs).

When to use: Use `RecallItem` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct RecallItem {
    pub lsn: LSN,
    pub kind: EventKind,
    pub conversation: ConversationId,
    pub wall_timestamp_ns: UtcNanos,
    pub payload: Vec<u8>,
    pub score_q32: u64,
}
```

## hm-serve::GraphNeighbour

<a id="rust-crates-hm-serve-src-actor-rs-graphneighbour"></a>

Source: [`crates/hm-serve/src/actor.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/actor.rs).

When to use: Use `GraphNeighbour` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct GraphNeighbour {
    pub edge: EdgeRecord,
    pub outgoing: bool,
    pub endpoint: Option<MemoryRecord>,
}
```

## hm-serve::GraphNeighbourhood

<a id="rust-crates-hm-serve-src-actor-rs-graphneighbourhood"></a>

Source: [`crates/hm-serve/src/actor.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/actor.rs).

When to use: Use `GraphNeighbourhood` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct GraphNeighbourhood {
    pub generation: u64,
    pub node: Option<MemoryRecord>,
    pub neighbours: Vec<GraphNeighbour>,
}
```

## hm-serve::RecallRequest

<a id="rust-crates-hm-serve-src-actor-rs-recallrequest"></a>

Source: [`crates/hm-serve/src/actor.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/actor.rs).

When to use: Use `RecallRequest` for actor ownership, embedded sessions, authenticated transports, and daemon request execution. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub enum RecallRequest {
    Vector {
        space_id: String,
        query: Vec<i8>,
        binary_prefilter: Vec<u8>,
        limit: usize,
    },
    Semantic {
        query: String,
        limit: usize,
    },
    Lexical {
        query: String,
        limit: usize,
    },
    Entity {
        query: String,
        turn_text: String,
        limit: usize,
    },
    Near {
        anchor: String,
        query: String,
        turn_text: String,
        limit: usize,
    },
    Temporal {
        start_ns: i64,
        end_ns: i64,
        limit: usize,
    },
    Timeline {
        conversation: ConversationId,
        since_lsn: LSN,
        limit: usize,
    },
}
```

## hm-serve::ActivateRequest

<a id="rust-crates-hm-serve-src-actor-rs-activaterequest"></a>

Source: [`crates/hm-serve/src/actor.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/actor.rs).

When to use: Use `ActivateRequest` for actor ownership, embedded sessions, authenticated transports, and daemon request execution. Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication. Do not treat constructing or serializing a request as evidence that it was accepted or executed.


```rust
pub struct ActivateRequest {
    pub conversation: ConversationId,
    pub query: String,
    pub turn_text: String,
    pub budget_tokens: usize,
    pub token_weights: FallbackWeights,
}
```

## hm-serve::AppliedState

<a id="rust-crates-hm-serve-src-actor-rs-appliedstate"></a>

Source: [`crates/hm-serve/src/actor.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/actor.rs).

When to use: Use `AppliedState` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct AppliedState {
    pub last_lsn: LSN,
    pub event_count: u64,
    pub rolling_digest: [u8; 32],
}
```

## hm-serve::ProjectionStat

<a id="rust-crates-hm-serve-src-actor-rs-projectionstat"></a>

Source: [`crates/hm-serve/src/actor.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/actor.rs).

When to use: Use `ProjectionStat` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct ProjectionStat {
    pub name: &'static str,
    pub applied_lsn: LSN,
}
```

## hm-serve::ActorStats

<a id="rust-crates-hm-serve-src-actor-rs-actorstats"></a>

Source: [`crates/hm-serve/src/actor.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/actor.rs).

When to use: Use `ActorStats` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct ActorStats {
    pub actor: ActorId,
    pub log_events: u64,
    pub log_bytes: u64,
    pub projections: Vec<ProjectionStat>,
    pub applied: AppliedState,
}
```

## hm-serve::ActorEngine

<a id="rust-crates-hm-serve-src-actor-rs-actorengine"></a>

Source: [`crates/hm-serve/src/actor.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/actor.rs).

When to use: Use `ActorEngine` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct ActorEngine {
    actor: ActorId,
    commands: mpsc::Sender<Command>,
    events: broadcast::Sender<Frame>,
}
```

## hm-serve::LatencyHistograms

<a id="rust-crates-hm-serve-src-admin-rs-latencyhistograms"></a>

Source: [`crates/hm-serve/src/admin.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/admin.rs).

When to use: Use `LatencyHistograms` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct LatencyHistograms {
    counts: Arc<Mutex<BTreeMap<String, [u64; LATENCY_BOUNDS_NS.len()]>>>,
}
```

## hm-serve::WakeEvaluation

<a id="rust-crates-hm-serve-src-anticipation-rs-wakeevaluation"></a>

Source: [`crates/hm-serve/src/anticipation.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/anticipation.rs).

When to use: Use `WakeEvaluation` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct WakeEvaluation {
    pub observation_lsn: u64,
    pub fired: Vec<WakeDecision>,
}
```

## hm-serve::WakeDecision

<a id="rust-crates-hm-serve-src-anticipation-rs-wakedecision"></a>

Source: [`crates/hm-serve/src/anticipation.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/anticipation.rs).

When to use: Use `WakeDecision` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct WakeDecision {
    pub intention_id: Vec<u8>,
    pub wake_id: Vec<u8>,
    pub decision: AttentionDecision,
    pub reason: String,
    pub fired_lsn: u64,
    pub decision_lsn: u64,
    pub rearmed_intention_id: Option<Vec<u8>>,
}
```

## hm-serve::Principal

<a id="rust-crates-hm-serve-src-auth-rs-principal"></a>

Source: [`crates/hm-serve/src/auth.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/auth.rs).

When to use: Use `Principal` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub enum Principal {
    Actor(u16),
    Admin,
}
```

## hm-serve::CapabilityToken

<a id="rust-crates-hm-serve-src-config-rs-capabilitytoken"></a>

Source: [`crates/hm-serve/src/config.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/config.rs).

When to use: Use `CapabilityToken` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub type CapabilityToken = [u8;
```

## hm-serve::ActorCapability

<a id="rust-crates-hm-serve-src-config-rs-actorcapability"></a>

Source: [`crates/hm-serve/src/config.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/config.rs).

When to use: Use `ActorCapability` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct ActorCapability {
    pub actor: u16,
    pub token: CapabilityToken,
}
```

## hm-serve::ServerConfig

<a id="rust-crates-hm-serve-src-config-rs-serverconfig"></a>

Source: [`crates/hm-serve/src/config.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/config.rs).

When to use: Use `ServerConfig` for actor ownership, embedded sessions, authenticated transports, and daemon request execution. Set explicit deployment limits before opening the associated resource.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication. Do not log secret fields or substitute defaults for an explicitly authorized budget.


```rust
pub struct ServerConfig {
    pub socket_path: PathBuf,
    pub data_directory: PathBuf,
    pub user: UserId,
    pub kek: KeyEncryptionKey,
    pub admin_token: CapabilityToken,
    pub actors: Vec<ActorCapability>,
    pub maximum_connections: usize,
    pub maximum_output_frames: usize,
    pub maximum_output_bytes: usize,
    pub projection_map_bytes: usize,
    pub maximum_active_actors: usize,
    pub maximum_heavy_jobs: usize,
    pub lease_wait_ms: u64,
}
```

## hm-serve::EmbeddedConfig

<a id="rust-crates-hm-serve-src-embedded-rs-embeddedconfig"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use `EmbeddedConfig` for actor ownership, embedded sessions, authenticated transports, and daemon request execution. Set explicit deployment limits before opening the associated resource.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication. Do not log secret fields or substitute defaults for an explicitly authorized budget.


```rust
pub struct EmbeddedConfig {
    pub actor: ActorId,
    pub user: UserId,
    pub kek: KeyEncryptionKey,
    pub projection_map_bytes: usize,
}
```

## hm-serve::HyperMind

<a id="rust-crates-hm-serve-src-embedded-rs-hypermind"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use `HyperMind` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct HyperMind {
    actor: Actor,
}
```

## hm-serve::Actor

<a id="rust-crates-hm-serve-src-embedded-rs-actor"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use `Actor` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct Actor {
    engine: ActorEngine,
}
```

## hm-serve::Session

<a id="rust-crates-hm-serve-src-embedded-rs-session"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use `Session` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct Session {
    actor: Actor,
    conversation: ConversationId,
}
```

## hm-serve::MemoryKind

<a id="rust-crates-hm-serve-src-embedded-rs-memorykind"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use `MemoryKind` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub enum MemoryKind {
    User,
    Assistant,
}
```

## hm-serve::RenderModel

<a id="rust-crates-hm-serve-src-embedded-rs-rendermodel"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use `RenderModel` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub enum RenderModel {
    OpenAi,
    Anthropic,
    PlainText,
}
```

## hm-serve::RenderAuthority

<a id="rust-crates-hm-serve-src-embedded-rs-renderauthority"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use `RenderAuthority` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub enum RenderAuthority {
    UntrustedMemory,
}
```

## hm-serve::RenderedItem

<a id="rust-crates-hm-serve-src-embedded-rs-rendereditem"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use `RenderedItem` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct RenderedItem {
    pub tier: Tier,
    pub role: &'static str,
    pub authority: RenderAuthority,
    pub source_authority: Authority,
    pub provenance_uri: String,
    pub provenance: Vec<LSN>,
    pub content: String,
}
```

## hm-serve::RenderedSection

<a id="rust-crates-hm-serve-src-embedded-rs-renderedsection"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use `RenderedSection` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct RenderedSection {
    pub tier: Tier,
    pub items: Vec<RenderedItem>,
}
```

## hm-serve::RenderedBundle

<a id="rust-crates-hm-serve-src-embedded-rs-renderedbundle"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use `RenderedBundle` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct RenderedBundle {
    pub model: RenderModel,
    pub sections: Vec<RenderedSection>,
    pub bundle_hash: [u8; 32],
}
```

## hm-serve::MutationEffectState

<a id="rust-crates-hm-serve-src-errors-rs-mutationeffectstate"></a>

Source: [`crates/hm-serve/src/errors.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/errors.rs).

When to use: Use `MutationEffectState` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub enum MutationEffectState {
    NotDispatched = 1,
    Unknown = 2,
    Rejected = 3,
}
```

## hm-serve::ListenerRole

<a id="rust-crates-hm-serve-src-grpc-bridge-rs-listenerrole"></a>

Source: [`crates/hm-serve/src/grpc/bridge.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/grpc/bridge.rs).

When to use: Use `ListenerRole` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub enum ListenerRole {
    Actor,
    Admin,
}
```

## hm-serve::Gateway

<a id="rust-crates-hm-serve-src-grpc-bridge-rs-gateway"></a>

Source: [`crates/hm-serve/src/grpc/bridge.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/grpc/bridge.rs).

When to use: Use `Gateway` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct Gateway {
    pub(crate) config: Arc<ServerConfig>,
    pub(crate) role: ListenerRole,
    permits: Arc<Semaphore>,
}
```

## hm-serve::GrpcServer

<a id="rust-crates-hm-serve-src-grpc-mod-rs-grpcserver"></a>

Source: [`crates/hm-serve/src/grpc/mod.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/grpc/mod.rs).

When to use: Use `GrpcServer` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct GrpcServer {
    listener: TcpListener,
    gateway: Gateway,
    tls: TlsIdentity,
}
```

## hm-serve::BoundedStream

<a id="rust-crates-hm-serve-src-grpc-mod-rs-boundedstream"></a>

Source: [`crates/hm-serve/src/grpc/mod.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/grpc/mod.rs).

When to use: Use `BoundedStream` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct BoundedStream {
    receiver: mpsc::Receiver<Envelope>,
    bytes: Arc<AtomicUsize>,
    terminal: Arc<Mutex<Option<Status>>>,
    _permit: OwnedSemaphorePermit,
}
```

## hm-serve::TlsIdentity

<a id="rust-crates-hm-serve-src-grpc-tls-rs-tlsidentity"></a>

Source: [`crates/hm-serve/src/grpc/tls.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/grpc/tls.rs).

When to use: Use `TlsIdentity` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct TlsIdentity {
    pub certificate_pem: Vec<u8>,
    pub private_key_pem: Vec<u8>,
    pub client_ca_pem: Vec<u8>,
}
```

## hm-serve::LeaseLimits

<a id="rust-crates-hm-serve-src-leases-rs-leaselimits"></a>

Source: [`crates/hm-serve/src/leases.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/leases.rs).

When to use: Use `LeaseLimits` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct LeaseLimits {
    pub maximum_active_actors: usize,
    pub maximum_heavy_jobs: usize,
    pub wait_ms: u64,
}
```

## hm-serve::LeaseClass

<a id="rust-crates-hm-serve-src-leases-rs-leaseclass"></a>

Source: [`crates/hm-serve/src/leases.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/leases.rs).

When to use: Use `LeaseClass` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub enum LeaseClass {
    Actor,
    HeavyJob,
}
```

## hm-serve::LeaseSnapshot

<a id="rust-crates-hm-serve-src-leases-rs-leasesnapshot"></a>

Source: [`crates/hm-serve/src/leases.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/leases.rs).

When to use: Use `LeaseSnapshot` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct LeaseSnapshot {
    pub active_actors: usize,
    pub active_leases: usize,
    pub heavy_jobs: usize,
    pub refusals: u64,
}
```

## hm-serve::LeaseRegistry

<a id="rust-crates-hm-serve-src-leases-rs-leaseregistry"></a>

Source: [`crates/hm-serve/src/leases.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/leases.rs).

When to use: Use `LeaseRegistry` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct LeaseRegistry {
    limits: LeaseLimits,
    shared: Arc<Shared>,
}
```

## hm-serve::ActorLease

<a id="rust-crates-hm-serve-src-leases-rs-actorlease"></a>

Source: [`crates/hm-serve/src/leases.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/leases.rs).

When to use: Use `ActorLease` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct ActorLease {
    shared: Arc<Shared>,
    actor: u16,
    class: LeaseClass,
    _heavy: Option<OwnedSemaphorePermit>,
}
```

## hm-serve::FrameParser

<a id="rust-crates-hm-serve-src-protocol-rs-frameparser"></a>

Source: [`crates/hm-serve/src/protocol.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/protocol.rs).

When to use: Use `FrameParser` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct FrameParser {
    pending: Vec<u8>,
    maximum_payload_bytes: usize,
}
```

## hm-serve::QueryEmbeddingSource

<a id="rust-crates-hm-serve-src-requests-activate-rs-queryembeddingsource"></a>

Source: [`crates/hm-serve/src/requests/activate.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/requests/activate.rs).

When to use: Use `QueryEmbeddingSource` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub enum QueryEmbeddingSource {
    ConfiguredEmbedder,
    Precomputed,
}
```

## hm-serve::PreparedQueryEmbedding

<a id="rust-crates-hm-serve-src-requests-activate-rs-preparedqueryembedding"></a>

Source: [`crates/hm-serve/src/requests/activate.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/requests/activate.rs).

When to use: Use `PreparedQueryEmbedding` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct PreparedQueryEmbedding {
    pub space: SpaceIdentity,
    pub quantized: Vec<i8>,
    pub binary_prefilter: Vec<u8>,
    pub inverse_scale: f32,
    pub source: QueryEmbeddingSource,
}
```

## hm-serve::SubscriptionStart

<a id="rust-crates-hm-serve-src-requests-subscribe-rs-subscriptionstart"></a>

Source: [`crates/hm-serve/src/requests/subscribe.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/requests/subscribe.rs).

When to use: Use `SubscriptionStart` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct SubscriptionStart {
    pub receiver: broadcast::Receiver<Frame>,
    pub replay: Vec<Frame>,
    pub replay_tail: LSN,
}
```

## hm-serve::ToolCall

<a id="rust-crates-hm-serve-src-rest-mod-rs-toolcall"></a>

Source: [`crates/hm-serve/src/rest/mod.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/rest/mod.rs).

When to use: Use `ToolCall` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct ToolCall {
    /// Stable 16-byte connection identity encoded as 32 hexadecimal characters.
    pub connection_id: String,
    pub request_id: u64,
    /// Unchanged arguments accepted by the corresponding MCP verb.
    pub arguments: Value,
}
```

## hm-serve::AdminCall

<a id="rust-crates-hm-serve-src-rest-mod-rs-admincall"></a>

Source: [`crates/hm-serve/src/rest/mod.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/rest/mod.rs).

When to use: Use `AdminCall` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct AdminCall {
    pub connection_id: String,
    pub request_id: u64,
    #[serde(default)]
    pub actor: u16,
    #[serde(default)]
    pub arguments: Value,
}
```

## hm-serve::ToolEnvelope

<a id="rust-crates-hm-serve-src-rest-mod-rs-toolenvelope"></a>

Source: [`crates/hm-serve/src/rest/mod.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/rest/mod.rs).

When to use: Use `ToolEnvelope` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct ToolEnvelope {
    pub ok: bool,
    pub items: Vec<Value>,
    pub provenance: Vec<String>,
    pub budget: Option<Value>,
    pub gaps: Vec<Value>,
    pub health: Value,
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_state: Option<String>,
    /// Retrieval manifest returned by activate: retrieved, selected, included, and used LSNs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<Value>,
}
```

## hm-serve::RestServer

<a id="rust-crates-hm-serve-src-rest-mod-rs-restserver"></a>

Source: [`crates/hm-serve/src/rest/mod.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/rest/mod.rs).

When to use: Use `RestServer` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct RestServer {
    listener: TcpListener,
    gateway: Gateway,
    acceptor: TlsAcceptor,
    console_directory: Option<PathBuf>,
}
```

## hm-serve::TelemetryMode

<a id="rust-crates-hm-serve-src-telemetry-rs-telemetrymode"></a>

Source: [`crates/hm-serve/src/telemetry.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/telemetry.rs).

When to use: Use `TelemetryMode` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub enum TelemetryMode {
    #[default]
    Off,
    File,
}
```

## hm-serve::FileSpanSink

<a id="rust-crates-hm-serve-src-telemetry-rs-filespansink"></a>

Source: [`crates/hm-serve/src/telemetry.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/telemetry.rs).

When to use: Use `FileSpanSink` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct FileSpanSink {
    state: Mutex<SinkState>,
}
```

## hm-serve::ToolDispatcher

<a id="rust-crates-hm-serve-src-uds-rs-tooldispatcher"></a>

Source: [`crates/hm-serve/src/uds.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/uds.rs).

When to use: Use `ToolDispatcher` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub trait ToolDispatcher: Send + Sync {
    fn dispatch(
        &self,
        actor: ActorEngine,
        verb: String,
        arguments_json: Vec<u8>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<Vec<u8>, Error>> + Send + '_>>;
}
```

## hm-serve::UdsServer

<a id="rust-crates-hm-serve-src-uds-rs-udsserver"></a>

Source: [`crates/hm-serve/src/uds.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/uds.rs).

When to use: Use `UdsServer` for actor ownership, embedded sessions, authenticated transports, and daemon request execution.

Do not use: Do not open one actor directory in competing processes, mix admin and actor capabilities, or weaken remote TLS authentication.


```rust
pub struct UdsServer {
    config: Arc<ServerConfig>,
    actors: Arc<BTreeMap<u16, ActorEngine>>,
    listener: UnixListener,
    active_connections: Arc<AtomicUsize>,
    latencies: LatencyHistograms,
    tool_dispatcher: Option<Arc<dyn ToolDispatcher>>,
    leases: LeaseRegistry,
}
```

## hm-sim::SimClock

<a id="rust-crates-hm-sim-src-env-rs-simclock"></a>

Source: [`crates/hm-sim/src/env.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-sim/src/env.rs).

When to use: Use `SimClock` for deterministic fault schedules and real-daemon journey verification.

Do not use: Do not mistake a simulator result for proof that an unrelated deployment or provider executed an external action.


```rust
pub struct SimClock {
    next_ns: i64,
}
```

## hm-sim::SimEntropy

<a id="rust-crates-hm-sim-src-env-rs-simentropy"></a>

Source: [`crates/hm-sim/src/env.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-sim/src/env.rs).

When to use: Use `SimEntropy` for deterministic fault schedules and real-daemon journey verification.

Do not use: Do not mistake a simulator result for proof that an unrelated deployment or provider executed an external action.


```rust
pub struct SimEntropy {
    state: u64,
}
```

## hm-sim::SimulatedStorage

<a id="rust-crates-hm-sim-src-env-rs-simulatedstorage"></a>

Source: [`crates/hm-sim/src/env.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-sim/src/env.rs).

When to use: Use `SimulatedStorage` for deterministic fault schedules and real-daemon journey verification.

Do not use: Do not mistake a simulator result for proof that an unrelated deployment or provider executed an external action.


```rust
pub struct SimulatedStorage {
    plan: FaultPlan,
    durable_bytes: Vec<u8>,
    volatile_bytes: Vec<u8>,
    next_lsn: u64,
}
```

## hm-sim::FaultPlan

<a id="rust-crates-hm-sim-src-fault-rs-faultplan"></a>

Source: [`crates/hm-sim/src/fault.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-sim/src/fault.rs).

When to use: Use `FaultPlan` for deterministic fault schedules and real-daemon journey verification.

Do not use: Do not mistake a simulator result for proof that an unrelated deployment or provider executed an external action.


```rust
pub struct FaultPlan {
    pub maximum_write_bytes: usize,
    pub maximum_read_bytes: usize,
    pub torn_after_bytes: usize,
    pub kill_after_durable_lsn: u64,
    pub reverse_write_completion: bool,
    pub fsync_lies: bool,
}
```

## hm-sim::HarnessResult

<a id="rust-crates-hm-sim-src-harness-rs-harnessresult"></a>

Source: [`crates/hm-sim/src/harness.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-sim/src/harness.rs).

When to use: Use `HarnessResult` for deterministic fault schedules and real-daemon journey verification. Inspect its status, coverage, identifiers, and evidence before reporting success.

Do not use: Do not mistake a simulator result for proof that an unrelated deployment or provider executed an external action. Do not discard gaps, partial coverage, or unknown effect state.


```rust
pub type HarnessResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
```

## hm-sim::JourneyHarness

<a id="rust-crates-hm-sim-src-harness-rs-journeyharness"></a>

Source: [`crates/hm-sim/src/harness.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-sim/src/harness.rs).

When to use: Use `JourneyHarness` for deterministic fault schedules and real-daemon journey verification.

Do not use: Do not mistake a simulator result for proof that an unrelated deployment or provider executed an external action.


```rust
pub struct JourneyHarness {
    config_path: PathBuf,
    socket_path: PathBuf,
    executable: PathBuf,
}
```

## hm-sim::McpClient

<a id="rust-crates-hm-sim-src-harness-rs-mcpclient"></a>

Source: [`crates/hm-sim/src/harness.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-sim/src/harness.rs).

When to use: Use `McpClient` for deterministic fault schedules and real-daemon journey verification.

Do not use: Do not mistake a simulator result for proof that an unrelated deployment or provider executed an external action.


```rust
pub struct McpClient {
    service: RunningService<RoleClient, ()>,
    process_id: u32,
}
```

## hm-sim::UdsDaemon

<a id="rust-crates-hm-sim-src-harness-rs-udsdaemon"></a>

Source: [`crates/hm-sim/src/harness.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-sim/src/harness.rs).

When to use: Use `UdsDaemon` for deterministic fault schedules and real-daemon journey verification.

Do not use: Do not mistake a simulator result for proof that an unrelated deployment or provider executed an external action.


```rust
pub struct UdsDaemon {
    child: Child,
}
```
