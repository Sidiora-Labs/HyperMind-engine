"""Canonical activation bundles and memory-safe rendering."""
from dataclasses import dataclass
import struct
from blake3 import blake3

TIERS = ("resident", "intent", "bindings", "work_ledger", "prospective", "conversation", "entity", "conflicts", "fused", "temporal")
AUTHORITIES = ("user_asserted", "external_observed", "tool_observed", "runtime_fact", "assistant_generated", "derived_inference")
HEALTH = ("semantic_ready", "semantic_lagging", "lexical_only", "unavailable")
GAPS = ("truncated_lane", "dropped_tier", "narrowed_subtask", "missing_binding", "stale_binding", "conflicting_binding", "index_lag", "pending_protected_proposal")


@dataclass(frozen=True)
class Bundle:
    snapshot_epoch: int
    budget_tokens: int
    spent_tokens: int
    sections: list[dict]
    gaps: list[dict]
    health: dict[str, str]
    hash: str


class ActivationSafetyError(ValueError):
    pass


def is_reconstruction(content: str) -> bool:
    value = content.lstrip()
    suffix = value.removeprefix("RECONSTRUCTION")
    return suffix != value and (not suffix or suffix[0].isspace() or suffix[0] == ":")


def assert_rememberable(content: str) -> None:
    if is_reconstruction(content):
        raise ActivationSafetyError("RECONSTRUCTION cannot be remembered verbatim")


class _Reader:
    def __init__(self, data: bytes):
        self.data, self.offset = data, 0

    def raw(self, size: int) -> bytes:
        if size < 0 or size > len(self.data) - self.offset:
            raise ValueError("truncated activation bundle")
        start = self.offset
        self.offset += size
        return self.data[start:self.offset]

    def u8(self): return self.raw(1)[0]
    def u32(self): return struct.unpack("<I", self.raw(4))[0]
    def u64(self): return struct.unpack("<Q", self.raw(8))[0]

    def count(self):
        count = self.u64()
        if count > len(self.data):
            raise ValueError("unbounded activation count")
        return count

    def blob(self): return self.raw(self.count())
    def text(self): return self.blob().decode("utf-8", errors="strict")
    def lsns(self): return [self.u64() for _ in range(self.count())]


def parse_bundle(data: bytes) -> Bundle:
    reader = _Reader(bytes(data))
    if reader.raw(4) != b"HMA1":
        raise ValueError("invalid activation bundle magic")
    epoch, budget, spent = reader.u64(), reader.u64(), reader.u64()
    if spent > budget:
        raise ValueError("activation exceeds budget")
    sections = []
    count = reader.count()
    if count != len(TIERS):
        raise ValueError("invalid activation tier count")
    for expected in range(count):
        tier, required = reader.u8(), reader.u8()
        if tier != expected or required != int(expected < 4):
            raise ValueError("invalid activation tier order or safety requirement")
        tokens, trimmed, coarsened = reader.u64(), reader.u64(), reader.u64()
        items = []
        for _ in range(reader.count()):
            item_tier, item_coarsened, why, authority = reader.u8(), reader.u8(), reader.u8(), reader.u8()
            if item_tier != tier or authority >= len(AUTHORITIES) or item_coarsened > 1:
                raise ValueError("invalid activation item")
            reader.u32(); reader.u32()
            item_tokens, uri, provenance, content = reader.u64(), reader.text(), reader.lsns(), reader.text()
            items.append(dict(tier=TIERS[tier], coarsened=bool(item_coarsened), authority=AUTHORITIES[authority],
                tokens=item_tokens, uri=uri, provenance=provenance, content=content, semantic=True))
        sections.append(dict(tier=TIERS[tier], required=bool(required), tokens=tokens, trimmedItems=trimmed, coarsenedItems=coarsened, items=items))
    reader.raw(32); reader.raw(32); reader.u64(); reader.blob(); reader.u64()
    reader.raw(reader.count())
    for _ in range(4): reader.lsns()
    gaps = []
    for _ in range(reader.count()):
        kind = reader.u8(); reader.u8(); reader.u8()
        gaps.append(dict(kind=GAPS[kind] if kind < len(GAPS) else "unknown", detail=reader.text()))
    health = {}
    for name in ("encoder", "backlog", "projection", "inclusion"):
        state = reader.u8()
        if state >= len(HEALTH): raise ValueError("invalid activation health")
        health[name] = HEALTH[state]
    if reader.offset != len(reader.data) or sum(section["tokens"] for section in sections) != spent:
        raise ValueError("activation trailing bytes or invalid token total")
    return Bundle(epoch, budget, spent, sections, gaps, health, blake3(data).hexdigest())


def render(bundle: Bundle, *, same_turn_lsns=()) -> dict:
    same_turn = {int(value) for value in same_turn_lsns}
    sections = []
    for section in bundle.sections:
        items = []
        for item in section["items"]:
            text, provenance = item["content"], item["provenance"]
            if (item.get("semantic") is False or not item["uri"].startswith("hm://") or not provenance
                or any(lsn <= 0 or lsn in same_turn for lsn in provenance)
                or text.startswith(("NCEV", "PCCN", "NCCP")) or text[4:8] == "NCEV"):
                continue
            items.append(dict(role="user", authority="assistant_generated" if is_reconstruction(text) else item["authority"],
                trust="untrusted_memory", provenanceUri=item["uri"], provenance=provenance, content=text))
        sections.append(dict(tier=section["tier"], label=f"Untrusted memory · {section['tier']}", items=items))
    return dict(version=1, sections=sections, bundleHash=bundle.hash)
