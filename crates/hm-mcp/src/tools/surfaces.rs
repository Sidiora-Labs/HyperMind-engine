#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Requirement {
    None,
    AdminToken,
    EmbeddingRuntime,
    ConsolidationRuntime,
    ReconstructionRuntime,
    DisputeRuntime,
}

impl Requirement {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::AdminToken => "admin_token",
            Self::EmbeddingRuntime => "embedding_runtime",
            Self::ConsolidationRuntime => "consolidation_runtime",
            Self::ReconstructionRuntime => "reconstruction_runtime",
            Self::DisputeRuntime => "dispute_runtime",
        }
    }

    #[must_use]
    pub const fn unavailable_reason(self) -> &'static str {
        match self {
            Self::None => "none_required",
            Self::AdminToken => "admin_token_absent",
            Self::EmbeddingRuntime => "embedding_runtime_absent",
            Self::ConsolidationRuntime => "consolidation_runtime_absent",
            Self::ReconstructionRuntime => "reconstruction_runtime_absent",
            Self::DisputeRuntime => "dispute_runtime_absent",
        }
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Availability {
    pub admin_token: bool,
    pub embedding: bool,
    pub consolidation: bool,
    pub reconstruction: bool,
    pub dispute: bool,
}

impl Availability {
    #[must_use]
    pub const fn satisfies(&self, requirement: Requirement) -> bool {
        match requirement {
            Requirement::None => true,
            Requirement::AdminToken => self.admin_token,
            Requirement::EmbeddingRuntime => self.embedding,
            Requirement::ConsolidationRuntime => self.consolidation,
            Requirement::ReconstructionRuntime => self.reconstruction,
            Requirement::DisputeRuntime => self.dispute,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Surface {
    pub verb: &'static str,
    pub surface: &'static str,
    pub summary: &'static str,
    pub arguments: &'static str,
    pub mutation: bool,
    pub requirement: Requirement,
    pub keywords: &'static [&'static str],
}

pub const DEFAULT_DISCOVERY_LIMIT: usize = 12;
pub const MAXIMUM_DISCOVERY_LIMIT: usize = 64;
pub(crate) const ADVERTISED_TOOLS: usize = 14;

pub static SURFACES: &[Surface] = &[
    Surface {
        verb: "activate",
        surface: "bundle",
        summary: "Compose a deterministic, token-budgeted activation bundle for one conversation turn.",
        arguments: r#"{"conversation":"<id>","query":"<text>","turn_text":"<text>","budget_tokens":2048}"#,
        mutation: false,
        requirement: Requirement::None,
        keywords: &[
            "activate", "bundle", "context", "budget", "tokens", "compose",
        ],
    },
    Surface {
        verb: "attest",
        surface: "feedback",
        summary: "Record used, ignored, helpful or harmful feedback against cited provenance URIs.",
        arguments: r#"{"provenance":["hm://<actor>/lsn/<n>"],"disposition":"used","idempotency_key":"<key>"}"#,
        mutation: true,
        requirement: Requirement::None,
        keywords: &[
            "attest",
            "feedback",
            "used",
            "ignored",
            "helpful",
            "harmful",
            "disposition",
        ],
    },
    Surface {
        verb: "believe",
        surface: "assert",
        summary: "Write a typed bitemporal belief with its validity window and provenance ranges.",
        arguments: r#"{"conversation":"<id>","belief_id":"<id>","belief_type":"fact","canonical_identity":"<identity>","value":"<text>","provenance":[]}"#,
        mutation: true,
        requirement: Requirement::None,
        keywords: &[
            "believe",
            "belief",
            "assertion",
            "fact",
            "preference",
            "constraint",
            "bitemporal",
        ],
    },
    Surface {
        verb: "bind",
        surface: "revision",
        summary: "Bind a task or scope to a canonical entity revision with a freshness requirement.",
        arguments: r#"{"conversation":"<id>","task":"<id>","canonical_entity":"<entity>","property":"<name>","evidence_lsn":1,"revision":"<revision>","freshness_requirement_ns":1}"#,
        mutation: true,
        requirement: Requirement::None,
        keywords: &[
            "bind",
            "binding",
            "revision",
            "freshness",
            "entity",
            "scope",
            "task",
        ],
    },
    Surface {
        verb: "consolidate",
        surface: "list",
        summary: "List recorded consolidation generations with their budget accounting.",
        arguments: r#"{"action":"list"}"#,
        mutation: false,
        requirement: Requirement::None,
        keywords: &[
            "list",
            "generations",
            "runs",
            "accounting",
            "report",
            "inventory",
        ],
    },
    Surface {
        verb: "consolidate",
        surface: "retract",
        summary: "Withdraw a previously written consolidation generation.",
        arguments: r#"{"action":"retract","run_id":"<id>","reason":"<text>"}"#,
        mutation: true,
        requirement: Requirement::None,
        keywords: &[
            "retract",
            "withdraw",
            "generation",
            "undo",
            "revert",
            "cancel",
        ],
    },
    Surface {
        verb: "consolidate",
        surface: "run",
        summary: "Execute a budgeted NREM or REM consolidation generation through a configured provider.",
        arguments: r#"{"action":"run","mode":"both","budget":{"max_llm_calls":4,"max_tokens":8000,"max_microusd":50000,"max_wall_ms":60000}}"#,
        mutation: true,
        requirement: Requirement::ConsolidationRuntime,
        keywords: &[
            "consolidate",
            "sleep",
            "nrem",
            "rem",
            "generation",
            "budget",
        ],
    },
    Surface {
        verb: "dispute",
        surface: "adjudicate",
        summary: "Adjudicate an existing and an incoming belief claim with bidirectional natural-language inference.",
        arguments: r#"{"conversation":"<id>","existing":{"belief_id":"<id>","belief_type":"fact","canonical_identity":"<identity>","value":"<text>","provenance":[]},"incoming":{"belief_id":"<id>","belief_type":"fact","canonical_identity":"<identity>","value":"<text>","provenance":[]}}"#,
        mutation: true,
        requirement: Requirement::DisputeRuntime,
        keywords: &[
            "dispute",
            "adjudicate",
            "conflict",
            "contradiction",
            "nli",
            "verdict",
        ],
    },
    Surface {
        verb: "forget",
        surface: "crypto_shred",
        summary: "Destroy the actor key material so sealed payloads become permanently unreadable.",
        arguments: r#"{"action":"crypto_shred","admin_token":"<token>"}"#,
        mutation: true,
        requirement: Requirement::AdminToken,
        keywords: &[
            "crypto_shred",
            "destroy",
            "erase",
            "key",
            "irreversible",
            "deletion",
        ],
    },
    Surface {
        verb: "forget",
        surface: "fade",
        summary: "Attenuate one admitted event without removing it from the ledger.",
        arguments: r#"{"action":"fade","lsn":1}"#,
        mutation: true,
        requirement: Requirement::None,
        keywords: &["fade", "attenuate", "demote", "suppress", "forget", "decay"],
    },
    Surface {
        verb: "forget",
        surface: "retract_run",
        summary: "Withdraw every event produced by one identified run.",
        arguments: r#"{"action":"retract_run","run_id":"<id>"}"#,
        mutation: true,
        requirement: Requirement::None,
        keywords: &["retract", "run", "withdraw", "undo", "rollback", "reverse"],
    },
    Surface {
        verb: "inspect",
        surface: "attention",
        summary: "List the recorded attention decisions and wake identifiers for this actor.",
        arguments: r#"{"uri":"hm://<actor>/attention"}"#,
        mutation: false,
        requirement: Requirement::None,
        keywords: &[
            "attention",
            "wake",
            "decision",
            "intention",
            "notify",
            "ignore",
        ],
    },
    Surface {
        verb: "inspect",
        surface: "calibration",
        summary: "Report per-predicate supported, contradicted, pending and unresolvable counts.",
        arguments: r#"{"uri":"hm://<actor>/calibration"}"#,
        mutation: false,
        requirement: Requirement::None,
        keywords: &[
            "calibration",
            "predicate",
            "supported",
            "contradicted",
            "pending",
            "accuracy",
        ],
    },
    Surface {
        verb: "inspect",
        surface: "discover",
        summary: "Enumerate the capability surfaces behind the advertised verbs with an availability annotation.",
        arguments: r#"{"mode":"discover","query":"<text>","limit":12}"#,
        mutation: false,
        requirement: Requirement::None,
        keywords: &[
            "discover",
            "capability",
            "catalog",
            "surfaces",
            "search",
            "directory",
        ],
    },
    Surface {
        verb: "inspect",
        surface: "provenance_chain",
        summary: "Walk the referenced-event chain behind one sequence number and return its integrity proofs.",
        arguments: r#"{"uri":"hm://<actor>/lsn/1"}"#,
        mutation: false,
        requirement: Requirement::None,
        keywords: &[
            "provenance",
            "chain",
            "evidence",
            "integrity",
            "proof",
            "lsn",
        ],
    },
    Surface {
        verb: "inspect",
        surface: "status",
        summary: "Report ledger size, applied projection positions, the applied-state digest and checkpoint verification.",
        arguments: r"{}",
        mutation: false,
        requirement: Requirement::None,
        keywords: &[
            "status",
            "health",
            "stats",
            "digest",
            "checkpoint",
            "projection",
        ],
    },
    Surface {
        verb: "intend",
        surface: "plan",
        summary: "Set an objective, register or cancel an intention with a wake trigger, evaluate a wake, adopt a supported procedure, or open and close a work loop.",
        arguments: r#"{"conversation":"<id>","action":{"kind":"open_loop","loop_id":"<id>","objective":"<text>"}}"#,
        mutation: true,
        requirement: Requirement::None,
        keywords: &[
            "intend",
            "intention",
            "objective",
            "loop",
            "wake",
            "procedure",
            "trigger",
            "schedule",
        ],
    },
    Surface {
        verb: "outcome",
        surface: "assess",
        summary: "Assess a registered prediction from cited observed ledger events.",
        arguments: r#"{"conversation":"<id>","prediction_id":"<id>","revision":1,"observation_lsns":[1]}"#,
        mutation: true,
        requirement: Requirement::None,
        keywords: &[
            "outcome",
            "assess",
            "evaluate",
            "observation",
            "evidence",
            "result",
        ],
    },
    Surface {
        verb: "predict",
        surface: "register",
        summary: "Register immutable bounded expectations before the work that would satisfy them.",
        arguments: r#"{"conversation":"<id>","prediction_id":"<id>","revision":1,"mechanism":"<text>","predicates":[{"kind":"object_exists","scope":"<scope>"}],"deadline_ns":1,"uncertainty":"<text>"}"#,
        mutation: true,
        requirement: Requirement::None,
        keywords: &[
            "predict",
            "prediction",
            "expectation",
            "predicate",
            "deadline",
            "commitment",
        ],
    },
    Surface {
        verb: "recall",
        surface: "entity",
        summary: "Retrieve events attached to a canonical entity named in the query or the current turn.",
        arguments: r#"{"mode":"entity","query":"<entity>","filters":{"turn_text":"<turn>"},"limit":32}"#,
        mutation: false,
        requirement: Requirement::None,
        keywords: &["entity", "canonical", "name", "subject", "mention", "who"],
    },
    Surface {
        verb: "recall",
        surface: "lexical",
        summary: "Rank admitted events by literal term overlap without any encoder.",
        arguments: r#"{"mode":"lexical","query":"<text>","limit":32}"#,
        mutation: false,
        requirement: Requirement::None,
        keywords: &["lexical", "keyword", "literal", "term", "exact", "text"],
    },
    Surface {
        verb: "recall",
        surface: "near",
        summary: "Retrieve events adjacent to a declared anchor such as a path or symbol.",
        arguments: r#"{"mode":"near","query":"<text>","filters":{"anchor":"<anchor>"},"limit":32}"#,
        mutation: false,
        requirement: Requirement::None,
        keywords: &["near", "adjacent", "anchor", "around", "locality", "path"],
    },
    Surface {
        verb: "recall",
        surface: "reconstruct",
        summary: "Compose a cited narrative over two or more anchor events through a configured provider.",
        arguments: r#"{"mode":"reconstruct","filters":{"anchor_lsns":[1,2]}}"#,
        mutation: false,
        requirement: Requirement::ReconstructionRuntime,
        keywords: &[
            "reconstruct",
            "narrative",
            "synthesis",
            "cited",
            "anchors",
            "retell",
        ],
    },
    Surface {
        verb: "recall",
        surface: "semantic",
        summary: "Rank admitted events by quantized embedding similarity for a natural-language query.",
        arguments: r#"{"mode":"semantic","query":"<text>","limit":32}"#,
        mutation: false,
        requirement: Requirement::EmbeddingRuntime,
        keywords: &[
            "semantic",
            "vector",
            "embedding",
            "similarity",
            "meaning",
            "nearest",
        ],
    },
    Surface {
        verb: "recall",
        surface: "temporal",
        summary: "Retrieve events whose wall timestamps fall inside an explicit nanosecond window.",
        arguments: r#"{"mode":"temporal","filters":{"temporal_from_ns":0,"temporal_to_ns":1},"limit":32}"#,
        mutation: false,
        requirement: Requirement::None,
        keywords: &["temporal", "time", "window", "range", "when", "clock"],
    },
    Surface {
        verb: "recall",
        surface: "timeline",
        summary: "Replay one conversation in ledger order from a starting sequence number.",
        arguments: r#"{"mode":"timeline","conversation":"<id>","since_lsn":0,"limit":32}"#,
        mutation: false,
        requirement: Requirement::None,
        keywords: &[
            "timeline",
            "conversation",
            "order",
            "replay",
            "sequence",
            "history",
        ],
    },
    Surface {
        verb: "remember",
        surface: "write",
        summary: "Persist a user message, delivered assistant message or chunked document with optional anchor, retention and sensitivity.",
        arguments: r#"{"conversation":"<id>","content":"<text>","kind":"user"}"#,
        mutation: true,
        requirement: Requirement::None,
        keywords: &[
            "remember", "store", "write", "persist", "document", "message", "chunk",
        ],
    },
    Surface {
        verb: "retract",
        surface: "tombstone",
        summary: "Tombstone a belief while preserving its recorded history.",
        arguments: r#"{"conversation":"<id>","belief_id":"<id>","provenance":[]}"#,
        mutation: true,
        requirement: Requirement::None,
        keywords: &[
            "retract",
            "tombstone",
            "belief",
            "withdraw",
            "supersede",
            "remove",
        ],
    },
];

#[must_use]
pub fn search(query: Option<&str>, limit: usize) -> Vec<&'static Surface> {
    let limit = limit.clamp(1, MAXIMUM_DISCOVERY_LIMIT);
    let tokens = tokens(query);
    let mut ranked = SURFACES
        .iter()
        .map(|surface| (score(surface, &tokens), surface))
        .filter(|(score, _)| tokens.is_empty() || *score > 0)
        .collect::<Vec<_>>();
    ranked.sort_unstable_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.verb.cmp(right.1.verb))
            .then_with(|| left.1.surface.cmp(right.1.surface))
    });
    ranked
        .into_iter()
        .take(limit)
        .map(|(_, surface)| surface)
        .collect()
}

fn tokens(query: Option<&str>) -> Vec<String> {
    query
        .unwrap_or_default()
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

fn score(surface: &Surface, tokens: &[String]) -> u32 {
    let summary = surface.summary.to_ascii_lowercase();
    let mut total = 0;
    for token in tokens {
        let token = token.as_str();
        if surface.verb == token {
            total += 8;
        } else if surface.verb.contains(token) {
            total += 3;
        }
        if surface.surface == token {
            total += 8;
        } else if surface.surface.contains(token) {
            total += 5;
        }
        if surface.keywords.contains(&token) {
            total += 4;
        } else if surface
            .keywords
            .iter()
            .any(|keyword| keyword.contains(token))
        {
            total += 2;
        }
        if summary.contains(token) {
            total += 1;
        }
    }
    total
}
