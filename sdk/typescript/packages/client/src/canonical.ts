import { blake3 } from "@noble/hashes/blake3";
import { Bundle, BundleItem, BundleSection, TIERS } from "@hypermind/render";

const HEALTH = ["semantic_ready", "semantic_lagging", "lexical_only", "unavailable"];
const GAP = [
  "truncated_lane",
  "dropped_tier",
  "narrowed_subtask",
  "missing_binding",
  "stale_binding",
  "conflicting_binding",
  "index_lag",
  "pending_protected_proposal",
];

class Reader {
  private offset = 0;

  constructor(private readonly bytes: Uint8Array) {}

  u8(): number {
    if (this.offset >= this.bytes.length) throw new Error("truncated activation bundle");
    return this.bytes[this.offset++]!;
  }

  u32(): number {
    const value = this.view(4).getUint32(0, true);
    this.offset += 4;
    return value;
  }

  u64(): bigint {
    const value = this.view(8).getBigUint64(0, true);
    this.offset += 8;
    return value;
  }

  count(): number {
    const value = this.u64();
    if (value > BigInt(Number.MAX_SAFE_INTEGER)) throw new Error("bundle count overflow");
    return Number(value);
  }

  raw(length: number): Uint8Array {
    if (length < 0 || this.offset + length > this.bytes.length) {
      throw new Error("truncated activation bundle");
    }
    const value = this.bytes.slice(this.offset, this.offset + length);
    this.offset += length;
    return value;
  }

  blob(): Uint8Array {
    return this.raw(this.count());
  }

  lsns(): bigint[] {
    return Array.from({ length: this.count() }, () => this.u64());
  }

  done(): boolean {
    return this.offset === this.bytes.length;
  }

  private view(length: number): DataView {
    if (this.offset + length > this.bytes.length) throw new Error("truncated activation bundle");
    return new DataView(this.bytes.buffer, this.bytes.byteOffset + this.offset, length);
  }
}

export function parseBundle(bytes: Uint8Array): Bundle {
  const reader = new Reader(bytes);
  if (Buffer.from(reader.raw(4)).toString("ascii") !== "HMA1") {
    throw new Error("invalid activation bundle magic");
  }
  const snapshotEpoch = reader.u64();
  const budgetTokens = reader.count();
  const spentTokens = reader.count();
  const sections: BundleSection[] = [];
  const sectionCount = reader.count();
  for (let sectionIndex = 0; sectionIndex < sectionCount; sectionIndex += 1) {
    const tier = TIERS[reader.u8()];
    if (tier === undefined) throw new Error("invalid activation tier");
    const required = reader.u8() !== 0;
    const tokens = reader.count();
    const trimmedItems = reader.count();
    const coarsenedItems = reader.count();
    const items: BundleItem[] = [];
    const itemCount = reader.count();
    for (let itemIndex = 0; itemIndex < itemCount; itemIndex += 1) {
      const itemTier = TIERS[reader.u8()];
      if (itemTier !== tier) throw new Error("activation item tier mismatch");
      const coarsened = reader.u8() !== 0;
      reader.u8();
      reader.u32();
      reader.u32();
      const itemTokens = reader.count();
      const uri = Buffer.from(reader.blob()).toString("utf8");
      const provenance = reader.lsns();
      const content = Buffer.from(reader.blob()).toString("utf8");
      items.push({
        tier,
        uri,
        provenance,
        content,
        tokens: itemTokens,
        coarsened,
      });
    }
    sections.push({ tier, required, items, tokens, trimmedItems, coarsenedItems });
  }
  reader.raw(32);
  reader.u64();
  reader.blob();
  reader.u64();
  reader.raw(reader.count());
  reader.lsns();
  reader.lsns();
  reader.lsns();
  reader.lsns();
  const gaps = Array.from({ length: reader.count() }, () => {
    const kind = GAP[reader.u8()] ?? "unknown";
    reader.u8();
    reader.u8();
    return { kind, detail: Buffer.from(reader.blob()).toString("utf8") };
  });
  const health = {
    encoder: HEALTH[reader.u8()] ?? "unknown",
    backlog: HEALTH[reader.u8()] ?? "unknown",
    projection: HEALTH[reader.u8()] ?? "unknown",
    inclusion: HEALTH[reader.u8()] ?? "unknown",
  };
  if (!reader.done()) throw new Error("activation bundle trailing bytes");
  return {
    snapshotEpoch,
    budgetTokens,
    spentTokens,
    sections,
    gaps,
    health,
    hash: Buffer.from(blake3(bytes)).toString("hex"),
  };
}
