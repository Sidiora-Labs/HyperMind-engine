import assert from "node:assert/strict";
import test from "node:test";
import { Client, EngineError } from "@hypermind/client";
import {
  ACTIVITY_LANES,
  ActivityFeed,
  classifyActivity,
  createActivityFeed,
  MAXIMUM_ACTIVITY_EVENTS,
  renderActivity,
} from "./activity.js";
import { clientTransport, subscribeActivity } from "./node-transport.js";
import { daemonFixture } from "./test/daemon.js";

interface LsnRange {
  first_lsn: number;
  last_lsn: number;
}

let streamed: ActivityFeed | undefined;
let streamedLsns: bigint[] = [];

test("console follows live activity from the daemon", async (context) => {
  const fixture = await daemonFixture();
  context.after(fixture.stop);
  const client = await Client.connect({
    socketPath: fixture.socket,
    capabilityToken: fixture.token,
    connectionId: fixture.connectionId,
  });
  context.after(() => client.close());
  const transport = clientTransport(client);
  const feed = createActivityFeed({ actorKey: "7" });
  assert.equal(feed.cursor, 0n);
  assert.equal(feed.events.length, 0);
  const abort = new AbortController();
  const stream = subscribeActivity(client, feed, { signal: abort.signal });
  const ingested: bigint[] = [];
  for (const content of ["console activity one", "console activity two", "console activity three"]) {
    const envelope = await transport.callTool("remember", {
      conversation: "console-activity",
      content,
      kind: "document",
    });
    assert.equal(envelope.ok, true);
    const range = envelope.items[0] as LsnRange;
    assert.equal(range.first_lsn, range.last_lsn);
    ingested.push(BigInt(range.last_lsn));
  }
  const attested = await transport.callTool("attest", {
    provenance: [`hm://7/lsn/${ingested[0]}`],
    disposition: "used",
    idempotency_key: "console-activity-feed",
  });
  assert.equal(attested.ok, true);
  const attestation = attested.items[0] as LsnRange;
  assert.equal(attestation.first_lsn, attestation.last_lsn);
  const attestationLsn = BigInt(attestation.last_lsn);
  const expected = [...ingested, attestationLsn];
  for (let attempt = 0; attempt < 600; attempt += 1) {
    if (expected.every((lsn) => feed.events.some((event) => event.lsn === lsn))) break;
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
  abort.abort();
  await stream;
  assert.equal(feed.events.length, 4, `feed holds ${feed.events.map((event) => event.lsn).join(",")}`);
  for (let index = 1; index < feed.events.length; index += 1) {
    assert.ok(feed.events[index]!.lsn > feed.events[index - 1]!.lsn, "lsns ascend");
  }
  for (const lsn of ingested) {
    const entry = feed.events.find((event) => event.lsn === lsn);
    assert.ok(entry !== undefined, `lsn ${lsn} is missing`);
    assert.equal(entry.lane, "ingestion");
  }
  const recalled = feed.events.find((event) => event.lsn === attestationLsn);
  assert.ok(recalled !== undefined, "the attestation is missing");
  assert.equal(recalled.lane, "recall");
  assert.equal(feed.cursor, expected.reduce((highest, lsn) => (lsn > highest ? lsn : highest), 0n));
  assert.equal(ACTIVITY_LANES[recalled.kind], "recall");
  assert.equal(classifyActivity(5), "other");
  const html = renderActivity(feed);
  assert.ok(html.includes("<h3>ingestion</h3>"));
  assert.ok(html.includes("<h3>recall</h3>"));
  assert.ok(html.includes(`lsn ${attestationLsn}`));
  streamed = feed;
  streamedLsns = expected;
});

test("console resets a stale cursor when the actor changes", () => {
  const feed = streamed;
  assert.ok(feed !== undefined, "the streaming test must run first");
  const seenUnderSeven = streamedLsns[0]!;
  assert.ok(feed.cursor > 0n);
  feed.switchActor("8");
  assert.equal(feed.actorKey, "8");
  assert.equal(feed.cursor, 0n);
  assert.equal(feed.events.length, 0);
  feed.note({ lsn: seenUnderSeven, kind: 1, conversation: "0a", wallTimestampNs: 1n });
  assert.equal(feed.events.length, 1);
  assert.equal(feed.events[0]!.lsn, seenUnderSeven);
  assert.equal(feed.cursor, seenUnderSeven);
});

test("console classifies a subscribe rejection", () => {
  const feed = createActivityFeed({ actorKey: "7" });
  feed.note({ lsn: 12n, kind: 1, conversation: "0b", wallTimestampNs: 1n });
  assert.equal(feed.cursor, 12n);
  assert.equal(feed.recordSubscribeFailure(new EngineError(48, "unknown", 0n, 0n, 0)), "reset");
  assert.equal(feed.cursor, 0n);
  assert.equal(feed.recordSubscribeFailure(new EngineError(48, "unknown", 0n, 0n, 0)), "retry");
  assert.equal(feed.cursor, 0n);
  feed.note({ lsn: 3n, kind: 1, conversation: "0b", wallTimestampNs: 2n });
  assert.equal(feed.recordSubscribeFailure(new EngineError(3, "unknown", 0n, 0n, 0)), "reset");
  assert.equal(feed.cursor, 0n);
  feed.note({ lsn: 4n, kind: 21, conversation: "0b", wallTimestampNs: 3n });
  assert.equal(feed.cursor, 4n);
  assert.equal(feed.recordSubscribeFailure(new Error("the socket closed")), "retry");
  assert.equal(feed.cursor, 4n);
  assert.ok(renderActivity(feed).includes("the cursor was kept at lsn 4"));
});

test("console caps the feed", () => {
  const feed = createActivityFeed({ actorKey: "7" });
  const total = MAXIMUM_ACTIVITY_EVENTS + 5;
  for (let index = 1; index <= total; index += 1) {
    feed.note({ lsn: BigInt(index), kind: 1, conversation: "0c", wallTimestampNs: BigInt(index) });
  }
  assert.equal(feed.events.length, MAXIMUM_ACTIVITY_EVENTS);
  assert.equal(feed.events[0]!.lsn, 6n);
  assert.equal(feed.events[MAXIMUM_ACTIVITY_EVENTS - 1]!.lsn, BigInt(total));
  assert.equal(feed.cursor, BigInt(total));
});
