import path from "node:path";
import { Bundle } from "@hypermind/render";
import { parseBundle } from "@hypermind/client";

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
