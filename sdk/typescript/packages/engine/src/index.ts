import path from "node:path";
import { assertRememberable, Bundle } from "@hypermind/render";
import {
  AsOfOptions,
  BeliefProvenance,
  BeliefRecord,
  BeliefType,
  BelieveInput,
  parseBundle,
} from "@hypermind/client";

export type { AsOfOptions, BeliefProvenance, BeliefRecord, BeliefType, BelieveInput };

interface NativeEngineHandle {
  session(conversation: string): NativeSessionHandle;
}

interface NativeSessionHandle {
  remember(content: string, kind: string, optionsJson: string): Promise<string>;
  recall(query: string, limit: number, mode: string, filtersJson: string): Promise<string>;
  activate(query: string, budgetTokens: number): Promise<Uint8Array>;
  checkpoint(turnId: string, blob: Uint8Array): Promise<string>;
  intend(input: string): Promise<string>;
  bind(input: string): Promise<string>;
  attest(input: string): Promise<string>;
  consolidate(input: string): Promise<string>;
  believe(input: string): Promise<string>;
  retract(beliefId: string, provenance: string): Promise<string>;
  asOf(
    beliefType: string,
    canonicalIdentity: string,
    validAtNs?: string,
    knownAtLsn?: string,
  ): Promise<string>;
}

interface NativeModule {
  NativeEngine: {
    open(path: string, config: string): Promise<NativeEngineHandle>;
  };
}

export interface EngineConfig {
  actor: number;
  userHex: string;
  kekHex: string;
  projectionMapBytes?: number;
}

export type HealthStatus = "semantic_ready" | "semantic_lagging" | "lexical_only" | "unavailable";
export type RecallMode = "semantic" | "lexical" | "entity" | "temporal" | "near";
export type Retention = "current_state" | "daily" | "durable" | "do_not_store";
export type Sensitivity = "public" | "personal" | "secret";
export type AnchorFacet = "path" | "symbol" | "url" | "entity";

export interface Health {
  encoder?: HealthStatus;
  backlog?: HealthStatus;
  projection: HealthStatus | "ready";
  inclusion?: HealthStatus;
  bundle_hash?: string;
}

export interface Gap {
  kind: string;
  detail: string;
}

export interface Envelope {
  ok: boolean;
  items: unknown[];
  provenance: string[];
  gaps: Gap[];
  health: Health;
  warnings: string[];
  effect_state?: "not_dispatched" | "unknown" | "rejected";
}

export interface RememberOptions {
  kind?: "user" | "assistant" | "document";
  anchor?: { facet: AnchorFacet; value: string };
  retention?: Retention;
  sensitivity?: Sensitivity;
}

export interface RecallOptions {
  mode?: RecallMode;
  limit?: number;
  filters?: {
    conversation?: string;
    since_lsn?: number;
    until_lsn?: number;
    temporal_from_ns?: number;
    temporal_to_ns?: number;
    anchor?: string;
    turn_text?: string;
  };
}

export interface BindInput {
  task?: string;
  scope?: string;
  canonicalEntity: string;
  property: string;
  evidenceLsn: bigint;
  revision: string;
  freshnessRequirementNs: bigint;
}

export type AttestDisposition = "used" | "ignored" | "helpful" | "harmful";

export interface AttestInput {
  provenance: string[];
  disposition: AttestDisposition;
  idempotencyKey: string;
}

export interface ConsolidateBudget {
  maxLlmCalls: number;
  maxTokens: number;
  maxMicrousd: number;
  maxWallMs: number;
}

export type ConsolidateInput =
  | {
      action: "run";
      mode: "nrem" | "rem" | "both";
      scope?: string;
      cadenceKey: string;
      budget: ConsolidateBudget;
    }
  | { action: "list" }
  | { action: "retract"; runId: string; reason: string };

export class HyperMind {
  private constructor(private readonly native: NativeEngineHandle) {}

  static async open(directory: string, config: EngineConfig): Promise<HyperMind> {
    const native = loadNative();
    const handle = await native.NativeEngine.open(
      directory,
      JSON.stringify({
        ...config,
        projectionMapBytes: config.projectionMapBytes ?? 64 * 1024 * 1024,
      }),
    );
    return new HyperMind(handle);
  }

  session(conversation: string): Session {
    return new Session(this.native.session(conversation), conversation);
  }
}

export class Session {
  constructor(
    private readonly native: NativeSessionHandle,
    readonly conversation: string,
  ) {}

  async remember(
    content: string,
    options: "user" | "assistant" | RememberOptions = {},
  ): Promise<Envelope> {
    assertRememberable(content);
    const normalized = typeof options === "string" ? { kind: options } : options;
    return decodeEnvelope(
      await this.native.remember(
        content,
        normalized.kind ?? "user",
        JSON.stringify({
          anchor: normalized.anchor,
          retention: normalized.retention,
          sensitivity: normalized.sensitivity,
        }),
      ),
    );
  }

  async recall(query: string, options: number | RecallOptions = {}): Promise<Envelope> {
    const normalized = typeof options === "number" ? { limit: options } : options;
    return decodeEnvelope(
      await this.native.recall(
        query,
        normalized.limit ?? 32,
        normalized.mode ?? "lexical",
        JSON.stringify(normalized.filters ?? {}),
      ),
    );
  }

  async activate(query: string, budgetTokens: number): Promise<Bundle> {
    return parseBundle(await this.native.activate(query, budgetTokens));
  }

  async checkpoint(turnId: string, blob: Uint8Array): Promise<bigint> {
    return BigInt(await this.native.checkpoint(turnId, blob));
  }

  async intend(action: unknown): Promise<Envelope> {
    return decodeEnvelope(
      await this.native.intend(JSON.stringify({ conversation: this.conversation, action })),
    );
  }

  async bind(input: BindInput): Promise<Envelope> {
    return decodeEnvelope(
      await this.native.bind(
        JSON.stringify({
          conversation: this.conversation,
          task: input.task,
          scope: input.scope,
          canonical_entity: input.canonicalEntity,
          property: input.property,
          evidence_lsn: input.evidenceLsn.toString(),
          revision: input.revision,
          freshness_requirement_ns: input.freshnessRequirementNs.toString(),
        }),
      ),
    );
  }

  async attest(input: AttestInput): Promise<Envelope> {
    return decodeEnvelope(
      await this.native.attest(
        JSON.stringify({
          provenance: input.provenance,
          disposition: input.disposition,
          idempotency_key: input.idempotencyKey,
        }),
      ),
    );
  }

  async consolidate(input: ConsolidateInput): Promise<Envelope> {
    const encoded = input.action === "run"
      ? {
          action: input.action,
          mode: input.mode,
          scope: input.scope,
          cadence_key: input.cadenceKey,
          budget: {
            max_llm_calls: input.budget.maxLlmCalls,
            max_tokens: input.budget.maxTokens,
            max_microusd: input.budget.maxMicrousd,
            max_wall_ms: input.budget.maxWallMs,
          },
        }
      : input.action === "retract"
        ? { action: input.action, run_id: input.runId, reason: input.reason }
        : { action: input.action };
    return decodeEnvelope(await this.native.consolidate(JSON.stringify(encoded)));
  }

  async believe(input: BelieveInput & { runId?: string }): Promise<Envelope> {
    return decodeEnvelope(
      await this.native.believe(JSON.stringify({
        belief_id: input.beliefId,
        belief_type: input.beliefType,
        canonical_identity: input.canonicalIdentity,
        value: input.value,
        valid_from_ns: (input.validFromNs ?? 0n).toString(),
        valid_to_ns: (input.validToNs ?? 0n).toString(),
        provenance: encodeProvenance(input.provenance),
        conflict_domain: input.conflictDomain,
        claim: input.claim,
        run_id: input.runId,
      })),
    );
  }

  async retract(beliefId: string, provenance: BeliefProvenance[]): Promise<Envelope> {
    return decodeEnvelope(
      await this.native.retract(beliefId, JSON.stringify(encodeProvenance(provenance))),
    );
  }

  async asOf(
    beliefType: BeliefType,
    canonicalIdentity: string,
    options: AsOfOptions,
  ): Promise<BeliefRecord | undefined> {
    const encoded = await this.native.asOf(
      beliefType,
      canonicalIdentity,
      options.validAtNs?.toString(),
      options.knownAtLsn?.toString(),
    );
    const raw = JSON.parse(encoded) as EncodedBeliefRecord | null;
    return raw === null ? undefined : decodeBelief(raw);
  }
}

interface EncodedBeliefRecord extends Omit<BeliefRecord,
  | "validFromNs"
  | "validToNs"
  | "transactionLsn"
  | "version"
  | "supersedesVersion"
  | "provenance"
  | "conflicts"
> {
  validFromNs: string;
  validToNs: string;
  transactionLsn: string;
  version: string;
  supersedesVersion: string;
  provenance: Array<Omit<BeliefProvenance, "firstLsn" | "lastLsn"> & {
    firstLsn: string;
    lastLsn: string;
  }>;
  conflicts: Array<Omit<BeliefRecord["conflicts"][number],
    "createdLsn" | "resolvedLsn"
  > & {
    createdLsn: string;
    resolvedLsn: string;
  }>;
}

function encodeProvenance(provenance: BeliefProvenance[]): unknown[] {
  return provenance.map((range) => ({
    first_lsn: range.firstLsn.toString(),
    last_lsn: range.lastLsn.toString(),
    byte_start: range.byteStart,
    byte_end: range.byteEnd,
  }));
}

function decodeBelief(raw: EncodedBeliefRecord): BeliefRecord {
  return {
    ...raw,
    validFromNs: BigInt(raw.validFromNs),
    validToNs: BigInt(raw.validToNs),
    transactionLsn: BigInt(raw.transactionLsn),
    version: BigInt(raw.version),
    supersedesVersion: BigInt(raw.supersedesVersion),
    provenance: raw.provenance.map((range) => ({
      ...range,
      firstLsn: BigInt(range.firstLsn),
      lastLsn: BigInt(range.lastLsn),
    })),
    conflicts: raw.conflicts.map((edge) => ({
      ...edge,
      createdLsn: BigInt(edge.createdLsn),
      resolvedLsn: BigInt(edge.resolvedLsn),
    })),
  };
}

function decodeEnvelope(value: string): Envelope {
  return JSON.parse(value) as Envelope;
}

function loadNative(): NativeModule {
  const platform = process.platform;
  const arch = process.arch;
  const suffix = platform === "linux" ? `${platform}-${arch}-gnu` : `${platform}-${arch}`;
  const local = path.resolve(__dirname, `../hypermind_engine.${suffix}.node`);
  try {
    return require(local) as NativeModule;
  } catch (localError) {
    try {
      return require(`@hypermind/engine-${suffix}`) as NativeModule;
    } catch {
      throw localError;
    }
  }
}
