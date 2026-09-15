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

export interface BundleItem {
  tier: Tier;
  uri: string;
  provenance: bigint[];
  content: string;
  tokens: number;
  coarsened: boolean;
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
  authority: "untrusted_memory";
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

export function render(bundle: Bundle, options: RenderOptions = {}): RenderedPrompt {
  const sameTurn = new Set(
    Array.from(options.sameTurnLsns ?? [], (value) => BigInt(value).toString()),
  );
  const sections = bundle.sections.map((section) => ({
    tier: section.tier,
    label: `Untrusted memory · ${section.tier}`,
    items: section.items
      .filter(
        (item) => !item.provenance.some((lsn) => sameTurn.has(lsn.toString())),
      )
      .map((item) => {
        if (!item.uri.startsWith("hm://") || item.provenance.length === 0) {
          throw new ActivationSafetyError("memory item lacks ledger provenance");
        }
        return {
          role: "user" as const,
          authority: "untrusted_memory" as const,
          provenanceUri: item.uri,
          provenance: item.provenance,
          content: item.content,
        };
      }),
  }));
  return { version: 1, sections, bundleHash: bundle.hash };
}
