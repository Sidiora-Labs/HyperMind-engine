import assert from "node:assert/strict";
import test, { TestContext } from "node:test";
import { Client } from "@hypermind/client";
import { AccessView, buildAccessView, renderAccess } from "./access.js";
import { ConsoleDataError } from "./errors.js";
import { clientTransport } from "./node-transport.js";
import { ConsoleEnvelope, ConsoleTransport } from "./transport.js";
import { daemonFixture } from "./test/daemon.js";

interface ServerVerb {
  verb: string;
  mutating: boolean;
  admin_token_required_actions: string[];
}

interface ServerAccess {
  surface: string;
  source: string;
  actor: number;
  isolation: string;
  admin_token_configured: boolean;
  admin_operations: string[];
  verbs: ServerVerb[];
  protected_belief_types: string[];
}

function envelope(item: unknown): ConsoleEnvelope {
  return {
    ok: true,
    items: [item],
    provenance: [],
    budget: null,
    gaps: [],
    health: {},
    warnings: [],
  };
}

async function connect(context: TestContext): Promise<ConsoleTransport> {
  const fixture = await daemonFixture();
  context.after(fixture.stop);
  const client = await Client.connect({
    socketPath: fixture.socket,
    capabilityToken: fixture.token,
    connectionId: fixture.connectionId,
  });
  context.after(() => client.close());
  return clientTransport(client);
}

test("console renders server permissions only", async (context) => {
  const transport = await connect(context);
  const answered = await transport.callTool("inspect", { uri: "hm://7/access" });
  assert.equal(answered.ok, true);
  const item = answered.items.find(
    (entry) => typeof entry === "object" && entry !== null && (entry as ServerAccess).surface === "access",
  ) as ServerAccess | undefined;
  assert.ok(item !== undefined, "the daemon answers with an access item");
  const view = buildAccessView(answered);

  assert.equal(view.actor, 7);
  assert.equal(view.isolation, "actor_scoped");
  assert.equal(view.verbs.length, 14);
  assert.equal(view.verbs.length, item.verbs.length);
  assert.equal(view.adminTokenConfigured, item.admin_token_configured);
  assert.deepEqual(view.adminOperations, item.admin_operations);
  assert.deepEqual(view.protectedBeliefTypes, item.protected_belief_types);
  assert.deepEqual(
    view.verbs,
    item.verbs.map((row) => ({
      verb: row.verb,
      mutating: row.mutating,
      adminTokenRequiredActions: row.admin_token_required_actions,
    })),
  );

  const privileged = view.verbs.filter((row) => row.adminTokenRequiredActions.length > 0);
  assert.equal(privileged.length, 1);
  assert.equal(privileged[0].verb, "forget");
  assert.deepEqual(privileged[0].adminTokenRequiredActions, ["crypto_shred"]);

  const html = renderAccess(view);
  assert.ok(html.includes(">actor_scoped<"), "the isolation value is stated verbatim");
  assert.equal((html.match(/<td class="access-mode">/g) ?? []).length, item.verbs.length);
  for (const row of item.verbs) {
    const mode = row.mutating ? "write" : "read";
    assert.ok(
      html.includes(`<tr><td>${row.verb}</td><td class="access-mode">${mode}</td>`),
      `${row.verb} is not rendered as ${mode}`,
    );
  }
  assert.equal((html.match(/<td class="access-admin">crypto_shred<\/td>/g) ?? []).length, 1);
  assert.ok(
    html.includes(`<tr><td>forget</td><td class="access-mode">write</td><td class="access-admin">crypto_shred</td></tr>`),
  );
  for (const belief of item.protected_belief_types) {
    assert.ok(html.includes(`<tr><td>${belief}</td><td>`), `${belief} is not rendered`);
  }
  assert.ok(html.includes("surfaces as a proposal"));
});

test("console refuses a non-authoritative access envelope", () => {
  const authoritative: ServerAccess = {
    surface: "access",
    source: "server",
    actor: 7,
    isolation: "actor_scoped",
    admin_token_configured: false,
    admin_operations: ["health"],
    verbs: [{ verb: "recall", mutating: false, admin_token_required_actions: [] }],
    protected_belief_types: ["identity"],
  };
  const { source, verbs, ...withoutSource } = authoritative;
  assert.throws(
    () => buildAccessView(envelope({ ...withoutSource, verbs })),
    (error) => error instanceof ConsoleDataError && error.field === "source",
  );
  assert.throws(
    () => buildAccessView(envelope({ ...authoritative, source: "console" })),
    (error) => error instanceof ConsoleDataError && error.field === "source",
  );
  assert.throws(
    () => buildAccessView({ ...envelope(authoritative), ok: false }),
    (error) => error instanceof ConsoleDataError && error.field === "ok",
  );
  assert.throws(
    () => buildAccessView(envelope({ ...withoutSource, source })),
    (error) => error instanceof ConsoleDataError && error.field === "verbs",
  );
});

test("console escapes every governance string the server supplies", () => {
  const view: AccessView = {
    actor: 7,
    isolation: "<em>actor_scoped</em>",
    adminTokenConfigured: true,
    adminOperations: ["<img src=x>"],
    verbs: [
      { verb: "recall", mutating: false, adminTokenRequiredActions: [] },
      { verb: "<script>", mutating: true, adminTokenRequiredActions: ["<b>shred</b>"] },
    ],
    protectedBeliefTypes: ["<iframe>"],
  };
  const html = renderAccess(view);
  assert.equal(html.includes("<img"), false);
  assert.equal(html.includes("<script>"), false);
  assert.equal(html.includes("<iframe>"), false);
  assert.equal(html.includes("<em>"), false);
  assert.ok(html.includes("&lt;img src=x&gt;"));
  assert.ok(html.includes("&lt;script&gt;"));
  assert.ok(html.includes("&lt;iframe&gt;"));
  assert.ok(html.includes("&lt;em&gt;actor_scoped&lt;/em&gt;"));
  assert.ok(html.includes(`<td class="access-mode">read</td>`));
  assert.ok(html.includes(`<td class="access-mode">write</td>`));
});
