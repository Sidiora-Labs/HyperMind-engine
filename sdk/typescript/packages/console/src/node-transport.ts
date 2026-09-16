import type { Client } from "@hypermind/client";
import type { ActivityFeed } from "./activity.js";
import type { ConsoleEnvelope, ConsoleManifest, ConsoleTransport } from "./transport.js";

type ClientVerb = Parameters<Client["callTool"]>[0];

export function clientTransport(client: Client): ConsoleTransport {
  return {
    async callTool(verb: string, args: unknown): Promise<ConsoleEnvelope> {
      const envelope = await client.callTool(verb as ClientVerb, args);
      const manifest = (envelope as { manifest?: ConsoleManifest }).manifest;
      return {
        ok: envelope.ok,
        items: envelope.items,
        provenance: envelope.provenance,
        budget: envelope.budget,
        gaps: envelope.gaps,
        health: envelope.health,
        warnings: envelope.warnings,
        ...(envelope.effect_state === undefined ? {} : { effect_state: envelope.effect_state }),
        ...(manifest === undefined ? {} : { manifest }),
      };
    },
  };
}

function conversationHex(conversation: readonly number[]): string {
  return conversation.map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

export async function subscribeActivity(
  client: Client,
  feed: ActivityFeed,
  options: { signal?: AbortSignal } = {},
): Promise<void> {
  try {
    for await (const event of client.subscribe({ sinceLsn: feed.cursor, signal: options.signal })) {
      feed.note({
        lsn: event.lsn,
        kind: event.kind,
        conversation: conversationHex(event.conversation),
        wallTimestampNs: event.wallTimestampNs,
      });
    }
  } catch (error) {
    if (options.signal?.aborted === true) return;
    feed.recordSubscribeFailure(error);
    throw error;
  }
}
