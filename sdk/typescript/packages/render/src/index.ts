export const TIERS = [
  "resident",
  "intent",
  "bindings",
  "work_ledger",
  "prospective",
  "conversation",
  "entity",
  "conflicts",
  "fused",
  "temporal",
] as const;

export type Tier = (typeof TIERS)[number];

export type Authority =
  | "user_asserted"
  | "external_observed"
  | "tool_observed"
  | "runtime_fact"
  | "assistant_generated"
  | "derived_inference";

export interface BundleItem {
  tier: Tier;
  uri: string;
  provenance: bigint[];
  content: string;
  authority: Authority;
  tokens: number;
  coarsened: boolean;
  semantic?: boolean;
}

export interface BundleSection {
  tier: Tier;
  required: boolean;
  items: BundleItem[];
  tokens: number;
  trimmedItems: number;
  coarsenedItems: number;
}

export interface Bundle {
  snapshotEpoch: bigint;
  budgetTokens: number;
  spentTokens: number;
  sections: BundleSection[];
  gaps: Array<{ kind: string; detail: string }>;
  health: Record<string, string>;
  hash: string;
}

export interface PromptItem {
  role: "user";
  authority: Authority;
  trust: "untrusted_memory";
  provenanceUri: string;
  provenance: bigint[];
  content: string;
}

export interface PromptSection {
  tier: Tier;
  label: string;
  items: PromptItem[];
}

export interface RenderedPrompt {
  version: 1;
  sections: PromptSection[];
  bundleHash: string;
}

export interface RenderOptions {
  sameTurnLsns?: Iterable<bigint | number>;
}

export class ActivationSafetyError extends Error {}

export function isReconstruction(content: string): boolean {
  return /^RECONSTRUCTION(?:\s|:|$)/u.test(content.trimStart());
}

export function assertRememberable(content: string): void {
  if (isReconstruction(content)) {
    throw new ActivationSafetyError("RECONSTRUCTION cannot be remembered verbatim");
  }
}

function rawWireBytes(content: string): boolean {
  return (
    content.startsWith("NCEV") ||
    content.slice(4, 8) === "NCEV" ||
    content.startsWith("PCCN") ||
    content.startsWith("NCCP")
  );
}

export function render(bundle: Bundle, options: RenderOptions = {}): RenderedPrompt {
  const sameTurn = new Set(
    Array.from(options.sameTurnLsns ?? [], (value) => BigInt(value).toString()),
  );
  const sections = bundle.sections.map((section) => ({
    tier: section.tier,
    label: `Untrusted memory · ${section.tier}`,
    items: section.items
      .filter(
        (item) =>
          item.semantic !== false &&
          item.uri.startsWith("hm://") &&
          item.provenance.length > 0 &&
          item.provenance.every((lsn) => lsn > 0n) &&
          !item.provenance.some((lsn) => sameTurn.has(lsn.toString())) &&
          !rawWireBytes(item.content),
      )
      .map((item) => {
        return {
          role: "user" as const,
          authority: isReconstruction(item.content)
            ? "assistant_generated" as const
            : item.authority,
          trust: "untrusted_memory" as const,
          provenanceUri: item.uri,
          provenance: item.provenance,
          content: item.content,
        };
      }),
  }));
  return { version: 1, sections, bundleHash: bundle.hash };
}
