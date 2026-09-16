import * as grpc from "@grpc/grpc-js";

export interface GrpcConfig {
  target: string;
  ca: Uint8Array;
  certificate: Uint8Array;
  privateKey: Uint8Array;
}

export class GrpcTransportError extends Error {
  readonly effectState: "not_dispatched" | "unknown";
  constructor(readonly cause: grpc.ServiceError) {
    super(cause.message);
    this.effectState = cause.metadata.get("effect-state")[0] === "not_dispatched"
      ? "not_dispatched" : "unknown";
  }
}

function field(number: number, value: Uint8Array): Buffer {
  const length: number[] = [];
  let remaining = value.length;
  do { length.push((remaining & 127) | (remaining > 127 ? 128 : 0)); remaining >>>= 7; } while (remaining);
  return Buffer.concat([Buffer.from([number * 8 + 2, ...length]), Buffer.from(value)]);
}

function envelope(bytes: Buffer): Uint8Array {
  let offset = 0;
  let found = new Uint8Array();
  const variable = (): number => {
    let value = 0;
    for (let shift = 0; shift < 35 && offset < bytes.length; shift += 7) {
      const byte = bytes[offset++]!;
      value += (byte & 127) * 2 ** shift;
      if (!(byte & 128)) return value;
    }
    throw new Error("invalid protobuf varint");
  };
  while (offset < bytes.length) {
    const key = variable();
    if ((key & 7) !== 2) throw new Error("invalid protobuf envelope field");
    const length = variable();
    if (length > bytes.length - offset) throw new Error("truncated protobuf envelope");
    if (key >>> 3 === 1) found = Uint8Array.from(bytes.subarray(offset, offset + length));
    offset += length;
  }
  return found;
}

export class GrpcFrames {
  private readonly client: grpc.Client;
  private hello?: Uint8Array;
  private ready?: Uint8Array;
  constructor(config: GrpcConfig, private readonly timeoutMs: number) {
    if (!config.ca.length || !config.certificate.length || !config.privateKey.length) {
      throw new Error("gRPC requires CA, client certificate, and private key");
    }
    this.client = new grpc.Client(config.target, grpc.credentials.createSsl(
      Buffer.from(config.ca), Buffer.from(config.privateKey), Buffer.from(config.certificate),
    ), { "grpc.max_receive_message_length": 17 * 1024 * 1024 + 4096,
      "grpc.max_send_message_length": 17 * 1024 * 1024 + 4096 });
  }
  async send(bytes: Uint8Array): Promise<void> {
    const connecting = this.hello === undefined;
    const encoded = connecting ? field(1, bytes)
      : Buffer.concat([field(1, this.hello!), field(2, bytes)]);
    this.ready = await new Promise<Uint8Array>((resolve, reject) => {
      this.client.makeUnaryRequest(`/hypermind.v3.HyperMind/${connecting ? "Connect" : "Exchange"}`,
        (value: Buffer) => value, envelope, encoded,
        { deadline: Date.now() + this.timeoutMs },
        (error, value) => error ? reject(new GrpcTransportError(error)) : resolve(value!));
    });
    if (connecting) this.hello = bytes.slice();
  }
  next(_timeoutMs: number): Promise<Uint8Array> {
    if (!this.ready) return Promise.reject(new Error("no gRPC response pending"));
    const value = this.ready;
    this.ready = undefined;
    return Promise.resolve(value);
  }
  async *subscribe(request: Uint8Array, signal?: AbortSignal): AsyncIterable<Uint8Array> {
    if (!this.hello) throw new Error("gRPC not connected");
    const stream = this.client.makeServerStreamRequest("/hypermind.v3.HyperMind/Subscribe",
      (value: Buffer) => value, envelope,
      Buffer.concat([field(1, this.hello), field(2, request)]));
    const cancel = (): void => stream.cancel();
    signal?.addEventListener("abort", cancel, { once: true });
    if (signal?.aborted) cancel();
    try {
      for await (const bytes of stream) yield bytes as Uint8Array;
    } catch (error) {
      if (!signal?.aborted) throw new GrpcTransportError(error as grpc.ServiceError);
    } finally {
      signal?.removeEventListener("abort", cancel);
      stream.cancel();
    }
  }
  close(): void { this.client.close(); }
}
