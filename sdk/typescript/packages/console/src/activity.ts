import { escapeText, section } from "./html.js";

export type ActivityLane = "ingestion" | "recall" | "consolidation" | "other";

export interface ActivityEvent {
  lsn: bigint;
  kind: number;
  lane: ActivityLane;
  conversation: string;
  wallTimestampNs: bigint;
}

export interface ActivitySubscribeFailure {
  disposition: "reset" | "retry";
  reason: string;
  cursorAtFailure: bigint;
}

export interface ActivityFeed {
  actorKey: string;
  cursor: bigint;
  events: ActivityEvent[];
  lastSubscribeFailure: ActivitySubscribeFailure | undefined;
  note(event: { lsn: bigint; kind: number; conversation: string; wallTimestampNs: bigint }): void;
  switchActor(actorKey: string): void;
  recordSubscribeFailure(error: unknown): "reset" | "retry";
}

const SEQUENCE_VIOLATION_CODE = 3;
const CAPACITY_EXCEEDED_CODE = 48;

const INGESTION_EVENT_KINDS = {
  userMsg: 1,
  deliveredMsg: 2,
  mediaRef: 7,
  embedding: 19,
  vocabularyImported: 44,
  documentIngested: 45,
  documentExtracted: 46,
  documentChunked: 47,
  sourceConnectorBound: 48,
  sourceDeliveryAccepted: 49,
  sourceDeliverySettled: 50,
  sourceRevisionObserved: 51,
} as const;

const RECALL_EVENT_KINDS = {
  attestation: 21,
  reviewed: 34,
} as const;

const CONSOLIDATION_EVENT_KINDS = {
  consolidation: 18,
  memoryMinted: 24,
  memoryRevised: 25,
  memoryMerged: 26,
  memoryFaded: 27,
  edgeAsserted: 28,
  edgeRetracted: 29,
  consolidationOpened: 30,
  consolidationPhase: 31,
  consolidationClosed: 32,
  consolidationRetracted: 33,
} as const;

const LANE_MEMBERSHIP: ReadonlyArray<readonly [ActivityLane, Readonly<Record<string, number>>]> = [
  ["ingestion", INGESTION_EVENT_KINDS],
  ["recall", RECALL_EVENT_KINDS],
  ["consolidation", CONSOLIDATION_EVENT_KINDS],
];

const ACTIVITY_LANE_ORDER: readonly ActivityLane[] = ["ingestion", "recall", "consolidation", "other"];

export const ACTIVITY_LANES: Readonly<Record<number, ActivityLane>> = Object.freeze(
  Object.fromEntries(
    LANE_MEMBERSHIP.flatMap(([lane, kinds]) => Object.values(kinds).map((kind) => [kind, lane] as const)),
  ) as Record<number, ActivityLane>,
);

export const MAXIMUM_ACTIVITY_EVENTS = 512;

export function classifyActivity(kind: number): ActivityLane {
  return ACTIVITY_LANES[kind] ?? "other";
}

function subscribeErrorCode(error: unknown): number | undefined {
  if (typeof error !== "object" || error === null) return undefined;
  const code = (error as { code?: unknown }).code;
  return typeof code === "number" && Number.isInteger(code) ? code : undefined;
}

function failureReason(error: unknown, code: number | undefined): string {
  if (code !== undefined) return `the daemon rejected the subscription with engine error code ${code}`;
  if (error instanceof Error && error.message.length > 0) return error.message;
  return "the subscription failed without an engine error code";
}

class ConsoleActivityFeed implements ActivityFeed {
  actorKey: string;
  cursor: bigint;
  readonly events: ActivityEvent[] = [];
  lastSubscribeFailure: ActivitySubscribeFailure | undefined = undefined;
  private readonly maximumEvents: number;
  private readonly seen = new Set<string>();
  private resetWithoutProgress = false;

  constructor(actorKey: string, sinceLsn: bigint, maximumEvents: number) {
    this.actorKey = actorKey;
    this.cursor = sinceLsn;
    this.maximumEvents = maximumEvents;
  }

  note(event: { lsn: bigint; kind: number; conversation: string; wallTimestampNs: bigint }): void {
    const key = event.lsn.toString();
    if (this.seen.has(key)) return;
    this.seen.add(key);
    this.events.push({
      lsn: event.lsn,
      kind: event.kind,
      lane: classifyActivity(event.kind),
      conversation: event.conversation,
      wallTimestampNs: event.wallTimestampNs,
    });
    while (this.events.length > this.maximumEvents) this.events.shift();
    if (event.lsn > this.cursor) this.cursor = event.lsn;
    this.resetWithoutProgress = false;
  }

  switchActor(actorKey: string): void {
    this.actorKey = actorKey;
    this.cursor = 0n;
    this.events.length = 0;
    this.seen.clear();
    this.resetWithoutProgress = false;
    this.lastSubscribeFailure = undefined;
  }

  recordSubscribeFailure(error: unknown): "reset" | "retry" {
    const code = subscribeErrorCode(error);
    const stale = code === SEQUENCE_VIOLATION_CODE || code === CAPACITY_EXCEEDED_CODE;
    const cursorAtFailure = this.cursor;
    const reason = failureReason(error, code);
    if (stale && !this.resetWithoutProgress) {
      this.cursor = 0n;
      this.resetWithoutProgress = true;
      this.lastSubscribeFailure = { disposition: "reset", reason, cursorAtFailure };
      return "reset";
    }
    this.lastSubscribeFailure = { disposition: "retry", reason, cursorAtFailure };
    return "retry";
  }
}

export function createActivityFeed(options: {
  actorKey: string;
  sinceLsn?: bigint;
  maximumEvents?: number;
}): ActivityFeed {
  const maximumEvents = options.maximumEvents ?? MAXIMUM_ACTIVITY_EVENTS;
  if (!Number.isInteger(maximumEvents) || maximumEvents < 1) {
    throw new RangeError("maximumEvents must be a positive integer");
  }
  return new ConsoleActivityFeed(options.actorKey, options.sinceLsn ?? 0n, maximumEvents);
}

function timestampText(wallTimestampNs: bigint): string {
  const milliseconds = Number(wallTimestampNs / 1000000n);
  return Number.isFinite(milliseconds) && Math.abs(milliseconds) <= 8640000000000000
    ? new Date(milliseconds).toISOString()
    : wallTimestampNs.toString();
}

function laneBlock(feed: ActivityFeed, lane: ActivityLane): string {
  const entries = feed.events.filter((event) => event.lane === lane);
  const items = entries
    .map((event) => [
      `<li><span class="lsn">lsn ${escapeText(event.lsn.toString())}</span>`,
      `<span class="kind">kind ${escapeText(String(event.kind))}</span>`,
      `<span class="conversation">${escapeText(event.conversation)}</span>`,
      `<time>${escapeText(timestampText(event.wallTimestampNs))}</time></li>`,
    ].join(" "))
    .join("");
  return [
    `<div class="lane lane-${escapeText(lane)}"><h3>${escapeText(lane)}</h3>`,
    `<p>${escapeText(String(entries.length))} events</p><ul>${items}</ul></div>`,
  ].join("");
}

export function renderActivity(feed: ActivityFeed): string {
  const cursor = `<p class="cursor">actor ${escapeText(feed.actorKey)} cursor at lsn ${escapeText(feed.cursor.toString())}</p>`;
  const failure = feed.lastSubscribeFailure;
  const status = failure === undefined
    ? ""
    : failure.disposition === "reset"
      ? `<p class="cursor-reset">the cursor was reset to lsn 0 from lsn ${escapeText(failure.cursorAtFailure.toString())} because ${escapeText(failure.reason)}</p>`
      : `<p class="cursor-retry">the cursor was kept at lsn ${escapeText(failure.cursorAtFailure.toString())} and the subscription will be retried because ${escapeText(failure.reason)}</p>`;
  const lanes = ACTIVITY_LANE_ORDER.map((lane) => laneBlock(feed, lane)).join("");
  return section("Live activity", `${cursor}${status}<div class="lanes">${lanes}</div>`);
}
