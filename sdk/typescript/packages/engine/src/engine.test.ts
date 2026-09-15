import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { render } from "@hypermind/render";
import { HyperMind } from "./index";

test("napi engine exposes one embedded continuity session", async (context) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "hypermind-napi-"));
  context.after(() => rm(directory, { recursive: true, force: true }));
  const engine = await HyperMind.open(directory, {
    actor: 7,
    userHex: "11".repeat(16),
    kekHex: "22".repeat(32),
    projectionMapBytes: 64 * 1024 * 1024,
  });
  const session = engine.session("napi-continuity");
  const remembered = await session.remember("heliotrope native evidence");
  assert.equal(remembered.ok, true);
  assert.equal(remembered.provenance.length, 1);
  const evidenceLsn = BigInt((remembered.items[0] as { first_lsn: number }).first_lsn);
  assert.equal(await session.checkpoint("native-turn", Buffer.from("native-state")), 2n);
  const intended = await session.intend({
    kind: "open_loop",
    loop_id: "native-task",
    objective: "exercise napi",
  });
  assert.equal(intended.ok, true);
  const bound = await session.bind({
    task: "native-task",
    canonicalEntity: "repository",
    property: "revision",
    evidenceLsn,
    revision: "native-rev",
    freshnessRequirementNs: 60_000_000_000n,
  });
  assert.equal(bound.ok, true);
  assert.equal((bound.items[0] as { status: string }).status, "resolved");
  const recalled = await session.recall("heliotrope");
  assert.equal(recalled.ok, true);
  assert.equal(recalled.items.length, 1);
  const bundle = await session.activate("heliotrope", 2048);
  assert.equal(bundle.sections.length, 10);
  const prompt = render(bundle);
  assert.equal(prompt.sections.flatMap((section) => section.items).every((item) => item.role === "user"), true);
});
