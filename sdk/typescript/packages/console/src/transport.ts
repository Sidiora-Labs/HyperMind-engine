import { ConsoleDataError } from "./errors.js";

export interface ConsoleManifest {
  manifest_id: string;
  query_digest: string;
  snapshot_epoch: number;
  encoder: string;
  index_generation: number;
  candidate_lanes: unknown[];
  retrieved: unknown[];
  selected: unknown[];
  included: unknown[];
  used: unknown[];
}

export interface ConsoleEnvelope {
  ok: boolean;
  items: unknown[];
  provenance: string[];
  budget: unknown;
  gaps: unknown[];
  health: Record<string, unknown>;
  warnings: string[];
  effect_state?: string;
  manifest?: ConsoleManifest;
}

export interface ConsoleTransport {
  callTool(verb: string, args: unknown): Promise<ConsoleEnvelope>;
}

export interface RestTransportOptions {
  baseUrl: string;
  token: string;
  connectionId: string;
}

export function restTransport(options: RestTransportOptions): ConsoleTransport {
  const base = options.baseUrl.replace(/\/+$/, "");
  let requestId = 0;
  return {
    async callTool(verb: string, args: unknown): Promise<ConsoleEnvelope> {
      requestId += 1;
      const response = await fetch(`${base}/v1/${verb}`, {
        method: "POST",
        headers: {
          "content-type": "application/json",
          authorization: `Bearer ${options.token}`,
        },
        body: JSON.stringify({
          connection_id: options.connectionId,
          request_id: requestId,
          arguments: args ?? {},
        }),
      });
      if (!response.ok) {
        throw new ConsoleDataError(verb, "envelope", `the listener answered HTTP ${response.status}`);
      }
      return (await response.json()) as ConsoleEnvelope;
    },
  };
}

export function requireItem<T>(envelope: ConsoleEnvelope, surface: string, index = 0): T {
  if (!envelope.ok) throw new ConsoleDataError(surface, "ok", "the envelope reports a failed call");
  if (!Array.isArray(envelope.items)) throw new ConsoleDataError(surface, "items");
  const item = envelope.items[index];
  if (item === undefined || item === null) throw new ConsoleDataError(surface, `items[${index}]`);
  return item as T;
}
