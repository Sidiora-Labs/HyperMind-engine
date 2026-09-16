import type { Client } from "@hypermind/client";
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
