import { createHash, randomBytes } from "node:crypto";
import net from "node:net";
import * as flatbuffers from "flatbuffers";
import { Bundle } from "@hypermind/render";
import { parseBundle } from "./canonical";
import { Activate } from "./wire/hypermind/protocol/activate";
import { AppendEventT } from "./wire/hypermind/protocol/append-event";
import { AppendT } from "./wire/hypermind/protocol/append";
import { BytesResultT } from "./wire/hypermind/protocol/bytes-result";
import { CheckpointAckT } from "./wire/hypermind/protocol/checkpoint-ack";
import { CheckpointResultT } from "./wire/hypermind/protocol/checkpoint-result";
import { CheckpointT } from "./wire/hypermind/protocol/checkpoint";
import { ErrorDetailT } from "./wire/hypermind/protocol/error-detail";
import { HelloT } from "./wire/hypermind/protocol/hello";
import { LatestCheckpointT } from "./wire/hypermind/protocol/latest-checkpoint";
import { MutationEffectState } from "./wire/hypermind/protocol/mutation-effect-state";
import { RecallMode as WireRecallMode } from "./wire/hypermind/protocol/recall-mode";
import { RecallResultT } from "./wire/hypermind/protocol/recall-result";
import { RecallT } from "./wire/hypermind/protocol/recall";
import { RequestPayload } from "./wire/hypermind/protocol/request-payload";
import { RequestT } from "./wire/hypermind/protocol/request";
import { ResponsePayload, unionToResponsePayload } from "./wire/hypermind/protocol/response-payload";
import { ResponseStatus } from "./wire/hypermind/protocol/response-status";
import { Response } from "./wire/hypermind/protocol/response";
import { Welcome } from "./wire/hypermind/protocol/welcome";
import { WireEnvelope, WireEnvelopeT } from "./wire/hypermind/protocol/wire-envelope";
import { WirePayload } from "./wire/hypermind/protocol/wire-payload";
import { Authority } from "./wire/hypermind/schema/authority";
import { Binding } from "./wire/hypermind/schema/binding";
import { DeliveredMsgT } from "./wire/hypermind/schema/delivered-msg";
import { EventEnvelope } from "./wire/hypermind/schema/event-envelope";
import { EventPayload } from "./wire/hypermind/schema/event-payload";
import { IntentSetT } from "./wire/hypermind/schema/intent-set";
import { LoopCloseReason } from "./wire/hypermind/schema/loop-close-reason";
import { LoopClosedT } from "./wire/hypermind/schema/loop-closed";
import { LoopOpenedT } from "./wire/hypermind/schema/loop-opened";
import { Retention } from "./wire/hypermind/schema/retention";
import { Sensitivity } from "./wire/hypermind/schema/sensitivity";
import { UserMsgT } from "./wire/hypermind/schema/user-msg";

const PROTOCOL_VERSION = 3;
const MAXIMUM_FRAME_BYTES = 17 * 1024 * 1024;
const text = new TextEncoder();

export type EffectState = "not_dispatched" | "unknown" | "rejected";
export type MemoryKind = "user" | "assistant";
export type CloseReason = "done" | "abandoned" | "handed_off" | "superseded";

export interface ClientConfig {
  socketPath: string;
  capabilityToken: Uint8Array;
  connectionId?: Uint8Array;
  requestTimeoutMs?: number;
  pendingLimit?: number;
  sequenceRecoveryLimit?: bigint;
}

export interface WelcomeInfo {
  actorNamespace: number;
  nextClientSeq: bigint;
  maximumBatchEvents: number;
}

export class EngineError extends Error {
  constructor(
    public readonly code: number,
    public readonly effectState: EffectState,
    public readonly lsn: bigint,
    public readonly offset: bigint,
    public readonly systemError: number,
  ) {
    super(`hypermind error code=${code} effect_state=${effectState}`);
  }
}

export class PendingWriteError extends Error {
  readonly effectState = "unknown" as const;

  constructor(public readonly clientSeq: bigint, public readonly cause: unknown) {
    super(`write pending reconnect at client_seq=${clientSeq}`);
  }
}

interface PendingWrite {
  sequence: bigint;
  send: (sequence: bigint) => Promise<unknown>;
}

class SocketFrames {
  private buffer = Buffer.alloc(0);
  private readonly frames: Uint8Array[] = [];
  private readonly waiters: Array<{
    resolve: (value: Uint8Array) => void;
    reject: (error: Error) => void;
  }> = [];
  private ended: Error | undefined;

  constructor(readonly socket: net.Socket) {
    socket.on("data", (chunk) => this.push(chunk));
    socket.on("error", (error) => this.end(error));
    socket.on("close", () => this.end(new Error("socket closed")));
  }

  next(timeoutMs: number): Promise<Uint8Array> {
    const ready = this.frames.shift();
    if (ready !== undefined) return Promise.resolve(ready);
    if (this.ended !== undefined) return Promise.reject(this.ended);
    return new Promise((resolve, reject) => {
      const waiter = { resolve, reject };
      this.waiters.push(waiter);
      const timer = setTimeout(() => {
        const index = this.waiters.indexOf(waiter);
        if (index >= 0) this.waiters.splice(index, 1);
        reject(new Error("request timeout"));
      }, timeoutMs);
      waiter.resolve = (value) => {
        clearTimeout(timer);
        resolve(value);
      };
      waiter.reject = (error) => {
        clearTimeout(timer);
        reject(error);
      };
    });
  }

  private push(chunk: Buffer): void {
    this.buffer = Buffer.concat([this.buffer, chunk]);
    while (this.buffer.length >= 8) {
      const length = this.buffer.readUInt32LE(0);
      if (length === 0 || length > MAXIMUM_FRAME_BYTES) {
        this.end(new Error("invalid frame length"));
        return;
      }
      if (this.buffer.length < length + 8) return;
      const expected = this.buffer.readUInt32LE(4);
      const payload = this.buffer.subarray(8, length + 8);
      this.buffer = this.buffer.subarray(length + 8);
      if (crc32c(payload) !== expected) {
        this.end(new Error("frame checksum mismatch"));
        return;
      }
      const waiter = this.waiters.shift();
      if (waiter === undefined) this.frames.push(payload);
      else waiter.resolve(payload);
    }
  }

  private end(error: Error): void {
    if (this.ended !== undefined) return;
    this.ended = error;
    for (const waiter of this.waiters.splice(0)) waiter.reject(error);
  }
}

export class Client {
  private readonly connectionId: Uint8Array;
  private readonly timeoutMs: number;
  private readonly pendingLimit: number;
  private readonly recoveryLimit: bigint;
  private frames: SocketFrames | undefined;
  private welcomeValue: WelcomeInfo | undefined;
  private requestId = 0n;
  private clientSeq = 0n;
  private recovered = 0n;
  private readonly pending: PendingWrite[] = [];
  private serialized: Promise<void> = Promise.resolve();
  private closed = false;

  private constructor(private readonly config: ClientConfig) {
    if (config.capabilityToken.length !== 32) throw new Error("capability token must be 32 bytes");
    this.connectionId = config.connectionId ?? randomBytes(16);
    if (this.connectionId.length !== 16) throw new Error("connection id must be 16 bytes");
    this.timeoutMs = config.requestTimeoutMs ?? 30_000;
    this.pendingLimit = config.pendingLimit ?? 256;
    this.recoveryLimit = config.sequenceRecoveryLimit ?? 65_536n;
  }

  static async connect(config: ClientConfig): Promise<Client> {
    const client = new Client(config);
    await client.exclusive(async () => {
      await client.connectTransport();
      client.clientSeq = client.welcomeValue!.nextClientSeq - 1n;
      client.consumeRecovery(client.clientSeq);
    });
    return client;
  }

  get welcome(): WelcomeInfo {
    if (this.welcomeValue === undefined) throw new Error("not connected");
    return this.welcomeValue;
  }

  get pendingLength(): number {
    return this.pending.length;
  }

  session(conversation: string): Session {
    if (conversation.length === 0) throw new Error("conversation is required");
    return new Session(this, conversation, conversationId(conversation));
  }

  close(): void {
    this.closed = true;
    this.dropTransport();
  }

  async checkpoint(turnId: string, blob: Uint8Array): Promise<bigint> {
    return this.sequenced(async (sequence) => {
      const result = await this.request(
        RequestPayload.Checkpoint,
        new CheckpointT([...text.encode(turnId)], [...blob], sequence),
        ResponsePayload.CheckpointAck,
      );
      return (result as CheckpointAckT).lsn;
    });
  }

  async latestCheckpoint(turnId: string): Promise<{ lsn: bigint; blob: Uint8Array } | undefined> {
    return this.exclusive(async () => {
      await this.ensure();
      const result = (await this.request(
        RequestPayload.LatestCheckpoint,
        new LatestCheckpointT([...text.encode(turnId)]),
        ResponsePayload.CheckpointResult,
      )) as CheckpointResultT;
      return result.present ? { lsn: result.lsn, blob: Uint8Array.from(result.blob) } : undefined;
    });
  }

  async append(kind: number, conversation: Uint8Array, payload: Uint8Array): Promise<bigint> {
    return this.sequenced(async (sequence) => {
      const result = await this.request(
        RequestPayload.Append,
        new AppendT(sequence, [new AppendEventT(kind, [...conversation], [...payload])]),
        ResponsePayload.AppendAck,
      );
      return (result as { firstLsn: bigint }).firstLsn;
    });
  }

  async recall(query: string, limit = 32): Promise<bigint[]> {
    return this.exclusive(async () => {
      await this.ensure();
      const result = (await this.request(
        RequestPayload.Recall,
        new RecallT([...text.encode(query)], limit, WireRecallMode.list_windows, 0, 0n, 0n),
        ResponsePayload.RecallResult,
      )) as RecallResultT;
      return result.members;
    });
  }

  async activate(conversation: Uint8Array, query: string, budgetTokens: number): Promise<Bundle> {
    return this.exclusive(async () => {
      await this.ensure();
      const result = (await this.request(
        RequestPayload.Activate,
        {
          pack(builder: flatbuffers.Builder): flatbuffers.Offset {
            const conversationOffset = Activate.createConversationVector(builder, conversation);
            const queryOffset = Activate.createQueryVector(builder, text.encode(query));
            const weights = Activate.createTokenWeightsVector(builder, Array(256).fill(256));
            Activate.startActivate(builder);
            Activate.addConversation(builder, conversationOffset);
            Activate.addQuery(builder, queryOffset);
            Activate.addBudgetTokens(builder, BigInt(budgetTokens));
            Activate.addTokenWeights(builder, weights);
            return Activate.endActivate(builder);
          },
        },
        ResponsePayload.BytesResult,
      )) as BytesResultT;
      return parseBundle(Uint8Array.from(result.bytes));
    });
  }

  private async sequenced<T>(send: (sequence: bigint) => Promise<T>): Promise<T> {
    return this.exclusive(async () => {
      await this.ensure();
      this.clientSeq += 1n;
      let sequence = this.clientSeq;
      for (;;) {
        try {
          const value = await send(sequence);
          this.welcomeValue!.nextClientSeq = sequence + 1n;
          return value;
        } catch (error) {
          if (error instanceof EngineError && (error.code === 3 || error.code === 50)) {
            this.dropTransport();
            await this.connectTransport();
            const next = this.welcomeValue!.nextClientSeq;
            if (next <= sequence) {
              this.clientSeq -= 1n;
              throw new Error("sequence recovery failed");
            }
            this.consumeRecovery(next - sequence);
            sequence = next;
            this.clientSeq = next;
            continue;
          }
          if (error instanceof EngineError && error.effectState !== "unknown") {
            this.clientSeq -= 1n;
            throw error;
          }
          if (this.pending.length >= this.pendingLimit) {
            this.clientSeq -= 1n;
            throw new Error("pending queue full");
          }
          this.dropTransport();
          this.pending.push({ sequence, send });
          throw new PendingWriteError(sequence, error);
        }
      }
    });
  }

  private async ensure(): Promise<void> {
    if (this.closed) throw new Error("client closed");
    if (this.frames === undefined) await this.connectTransport();
    await this.flushPending();
    if (this.pending.length === 0 && this.welcomeValue!.nextClientSeq - 1n > this.clientSeq) {
      const tail = this.welcomeValue!.nextClientSeq - 1n;
      this.consumeRecovery(tail - this.clientSeq);
      this.clientSeq = tail;
    }
  }

  private async flushPending(): Promise<void> {
    while (this.pending.length > 0) {
      const head = this.pending[0]!;
      const next = this.welcomeValue!.nextClientSeq;
      if (next !== head.sequence && next !== head.sequence + 1n) {
        throw new Error(`unknown write outcome at client_seq=${head.sequence}`);
      }
      try {
        await head.send(head.sequence);
        this.welcomeValue!.nextClientSeq = head.sequence + 1n;
        this.pending.shift();
      } catch (error) {
        if (error instanceof EngineError && error.effectState !== "unknown" && next === head.sequence) {
          this.pending.shift();
          this.clientSeq -= 1n;
          for (const item of this.pending) item.sequence -= 1n;
          throw error;
        }
        this.dropTransport();
        throw error;
      }
    }
  }

  private async connectTransport(): Promise<void> {
    const socket = net.createConnection(this.config.socketPath);
    await new Promise<void>((resolve, reject) => {
      socket.once("connect", resolve);
      socket.once("error", reject);
    });
    const frames = new SocketFrames(socket);
    this.frames = frames;
    await this.writeEnvelope(WirePayload.Hello, new HelloT(
      PROTOCOL_VERSION,
      [...this.connectionId],
      [...this.config.capabilityToken],
    ));
    const envelope = decodeEnvelope(await frames.next(this.timeoutMs));
    if (envelope.payloadType() !== WirePayload.Welcome) throw new Error("handshake rejected");
    const welcome = envelope.payload(new Welcome())!.unpack();
    if (welcome.protoVersion !== PROTOCOL_VERSION || welcome.nextClientSeq === 0n) {
      throw new Error("invalid welcome");
    }
    this.welcomeValue = {
      actorNamespace: welcome.actorNs,
      nextClientSeq: welcome.nextClientSeq,
      maximumBatchEvents: welcome.maximumBatchEvents,
    };
  }

  private dropTransport(): void {
    this.frames?.socket.destroy();
    this.frames = undefined;
  }

  private consumeRecovery(delta: bigint): void {
    if (delta < 0n || this.recovered + delta > this.recoveryLimit) {
      throw new Error("sequence recovery limit reached");
    }
    this.recovered += delta;
  }

  private async request(
    payloadType: RequestPayload,
    payload: { pack(builder: flatbuffers.Builder): flatbuffers.Offset },
    wanted: ResponsePayload,
  ): Promise<unknown> {
    this.requestId += 1n;
    const requestId = this.requestId;
    await this.writeEnvelope(
      WirePayload.Request,
      new RequestT(requestId, payloadType, payload as never),
    );
    for (;;) {
      const envelope = decodeEnvelope(await this.frames!.next(this.timeoutMs));
      if (envelope.payloadType() === WirePayload.Event) continue;
      if (envelope.payloadType() !== WirePayload.Response) throw new Error("protocol violation");
      const response = envelope.payload(new Response())!;
      if (response.requestId() !== requestId) throw new Error("request id mismatch");
      const unpacked = unionToResponsePayload(response.payloadType(), response.payload.bind(response));
      if (response.status() !== ResponseStatus.ok) {
        if (unpacked === null || response.payloadType() !== ResponsePayload.ErrorDetail) {
          throw new Error("error response lacks detail");
        }
        throw engineError(unpacked.unpack() as ErrorDetailT);
      }
      if (response.payloadType() !== wanted || unpacked === null) throw new Error("response type mismatch");
      return unpacked.unpack();
    }
  }

  private async writeEnvelope(
    payloadType: WirePayload,
    payload: { pack(builder: flatbuffers.Builder): flatbuffers.Offset },
  ): Promise<void> {
    const builder = new flatbuffers.Builder(1024);
    const offset = new WireEnvelopeT(PROTOCOL_VERSION, payloadType, payload as never).pack(builder);
    builder.finish(offset, "NCPR");
    const encoded = builder.asUint8Array();
    const frame = Buffer.allocUnsafe(encoded.length + 8);
    frame.writeUInt32LE(encoded.length, 0);
    frame.writeUInt32LE(crc32c(encoded), 4);
    frame.set(encoded, 8);
    await new Promise<void>((resolve, reject) => {
      this.frames!.socket.write(frame, (error) => (error == null ? resolve() : reject(error)));
    });
  }

  private exclusive<T>(operation: () => Promise<T>): Promise<T> {
    const result = this.serialized.then(operation, operation);
    this.serialized = result.then(() => undefined, () => undefined);
    return result;
  }
}

export class Session {
  constructor(
    private readonly client: Client,
    readonly conversation: string,
    private readonly conversationBytes: Uint8Array,
  ) {}

  remember(content: string, kind: MemoryKind = "user"): Promise<bigint> {
    const payload = kind === "user" ? new UserMsgT([...text.encode(content)]) : new DeliveredMsgT([...text.encode(content)]);
    return this.client.append(kind === "user" ? 1 : 2, this.conversationBytes, eventEnvelope(
      kind === "user" ? EventPayload.UserMsg : EventPayload.DeliveredMsg,
      payload,
      kind === "user" ? Authority.user_asserted : Authority.assistant_generated,
    ));
  }

  recall(query: string, limit = 32): Promise<bigint[]> {
    return this.client.recall(query, limit);
  }

  activate(query: string, budgetTokens: number): Promise<Bundle> {
    return this.client.activate(this.conversationBytes, query, budgetTokens);
  }

  checkpoint(turnId: string, blob: Uint8Array): Promise<bigint> {
    return this.client.checkpoint(turnId, blob);
  }

  intend(action: "set_objective", objective: string): Promise<bigint>;
  intend(action: "open_loop", loopId: string, objective: string): Promise<bigint>;
  intend(action: "close_loop", loopId: string, reason: CloseReason, cause?: string, evidenceLsns?: bigint[]): Promise<bigint>;
  intend(action: string, first: string, second?: string, cause = "", evidenceLsns: bigint[] = []): Promise<bigint> {
    if (action === "set_objective") {
      return this.client.append(14, this.conversationBytes, eventEnvelope(EventPayload.IntentSet, new IntentSetT([...text.encode(first)]), Authority.user_asserted));
    }
    if (action === "open_loop") {
      return this.client.append(15, this.conversationBytes, eventEnvelope(EventPayload.LoopOpened, new LoopOpenedT([...text.encode(first)], [...text.encode(second ?? "")]), Authority.user_asserted));
    }
    const reason = ({ done: 0, abandoned: 1, handed_off: 2, superseded: 3 } as const)[second as CloseReason];
    return this.client.append(16, this.conversationBytes, eventEnvelope(EventPayload.LoopClosed, new LoopClosedT([...text.encode(first)], reason as LoopCloseReason, [...text.encode(cause)], evidenceLsns), Authority.user_asserted));
  }

  bind(input: {
    task?: string;
    scope?: string;
    canonicalEntity: string;
    property: string;
    evidenceLsn: bigint;
    revision: string;
    freshnessRequirementNs: bigint;
  }): Promise<{ status: "resolved"; lsn: bigint }> {
    const binding = {
      pack(builder: flatbuffers.Builder): flatbuffers.Offset {
        const task = input.task === undefined ? 0 : Binding.createTaskVector(builder, text.encode(input.task));
        const scope = input.scope === undefined ? 0 : Binding.createScopeVector(builder, text.encode(input.scope));
        const entity = builder.createString(input.canonicalEntity);
        const property = builder.createString(input.property);
        const revision = Binding.createRevisionVector(builder, text.encode(input.revision));
        return Binding.createBinding(
          builder,
          task,
          scope,
          entity,
          property,
          input.evidenceLsn,
          revision,
          input.freshnessRequirementNs,
        );
      },
    };
    return this.client.append(22, this.conversationBytes, eventEnvelope(EventPayload.Binding, binding, Authority.user_asserted)).then((lsn) => ({ status: "resolved", lsn }));
  }
}

function eventEnvelope(payloadType: EventPayload, payload: { pack(builder: flatbuffers.Builder): flatbuffers.Offset }, authority: Authority): Uint8Array {
  const builder = new flatbuffers.Builder(512);
  const payloadOffset = payload.pack(builder);
  EventEnvelope.startEventEnvelope(builder);
  EventEnvelope.addSchemaVersion(builder, 2);
  EventEnvelope.addPayloadType(builder, payloadType);
  EventEnvelope.addPayload(builder, payloadOffset);
  EventEnvelope.addClientEventCount(builder, 1);
  EventEnvelope.addAuthority(builder, authority);
  EventEnvelope.addRetention(builder, Retention.durable);
  EventEnvelope.addSensitivity(builder, Sensitivity.personal);
  const offset = EventEnvelope.endEventEnvelope(builder);
  builder.finish(offset, "NCEV");
  return builder.asUint8Array();
}

function conversationId(value: string): Uint8Array {
  return createHash("sha256")
    .update("neocortex-conversation-v1\0")
    .update(value)
    .digest()
    .subarray(0, 16);
}

function decodeEnvelope(bytes: Uint8Array): WireEnvelope {
  const buffer = new flatbuffers.ByteBuffer(bytes);
  if (!WireEnvelope.bufferHasIdentifier(buffer)) throw new Error("wire identifier mismatch");
  const envelope = WireEnvelope.getRootAsWireEnvelope(buffer);
  if (envelope.protoVersion() !== PROTOCOL_VERSION) throw new Error("protocol version mismatch");
  return envelope;
}

function engineError(detail: ErrorDetailT): EngineError {
  const states: Record<number, EffectState> = {
    [MutationEffectState.not_dispatched]: "not_dispatched",
    [MutationEffectState.unknown]: "unknown",
    [MutationEffectState.rejected]: "rejected",
  };
  return new EngineError(
    detail.code,
    states[detail.effectState] ?? "unknown",
    detail.lsn,
    detail.offset,
    detail.systemError,
  );
}

const CRC32C_TABLE = Array.from({ length: 256 }, (_, value) => {
  let crc = value;
  for (let bit = 0; bit < 8; bit += 1) crc = (crc & 1) !== 0 ? (crc >>> 1) ^ 0x82f63b78 : crc >>> 1;
  return crc >>> 0;
});

function crc32c(bytes: Uint8Array): number {
  let crc = 0xffffffff;
  for (const byte of bytes) crc = CRC32C_TABLE[(crc ^ byte) & 0xff]! ^ (crc >>> 8);
  return (crc ^ 0xffffffff) >>> 0;
}
