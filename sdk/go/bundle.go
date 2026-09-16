package cortexclient

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"net/url"
	"strconv"
	"strings"
	"time"
	"unicode/utf8"

	"github.com/zeebo/blake3"
)

var modernTiers = []string{"resident", "intent", "bindings", "work_ledger", "prospective", "conversation", "entity", "conflicts", "fused", "temporal"}
var authorities = []string{"user_asserted", "external_observed", "tool_observed", "runtime_fact", "assistant_generated", "derived_inference"}

type Gap struct{ Kind, Detail string }

type ModernItem struct {
	Tier       string
	URI        string
	Provenance []uint64
	Content    string
	Authority  string
	Tokens     uint64
	Coarsened  bool
}
type ModernSection struct {
	Tier                                 string
	Required                             bool
	Tokens, TrimmedItems, CoarsenedItems uint64
	Items                                []ModernItem
}
type PromptItem struct {
	Role, Trust, Authority, ProvenanceURI, Content string
	Provenance                                     []uint64
}
type PromptSection struct {
	Tier, Label string
	Items       []PromptItem
}
type RenderedPrompt struct {
	Version    int
	BundleHash string
	Sections   []PromptSection
}
type RenderOptions struct{ SameTurnLSNs []uint64 }

type modernReader struct {
	bundleCursor
	err error
}

func (r *modernReader) byte() byte {
	if r.err != nil {
		return 0
	}
	v, e := r.u8()
	r.err = e
	return v
}
func (r *modernReader) integer() uint64 {
	if r.err != nil {
		return 0
	}
	v, e := r.u64()
	r.err = e
	return v
}
func (r *modernReader) count() int {
	v := r.integer()
	if v > uint64(len(r.bytes)-r.offset) {
		r.err = ErrProtocol
		return 0
	}
	return int(v)
}
func (r *modernReader) raw(n int) []byte {
	if r.err != nil {
		return nil
	}
	if n < 0 || n > len(r.bytes)-r.offset {
		r.err = ErrProtocol
		return nil
	}
	v := r.bytes[r.offset : r.offset+n]
	r.offset += n
	return v
}
func (r *modernReader) bytesValue() []byte { n := r.count(); return r.raw(n) }
func (r *modernReader) lsns() []uint64 {
	n := r.count()
	out := make([]uint64, n)
	for i := range out {
		out[i] = r.integer()
	}
	return out
}

func parseModernBundle(encoded []byte) (*Bundle, error) {
	r := &modernReader{bundleCursor: bundleCursor{bytes: encoded, offset: 4}}
	b := &Bundle{Canonical: append([]byte(nil), encoded...)}
	b.SnapshotEpoch = r.integer()
	b.BudgetTokens = r.integer()
	b.SpentTokens = r.integer()
	n := r.count()
	if n != len(modernTiers) {
		return nil, ErrProtocol
	}
	b.ModernSections = make([]ModernSection, n)
	legacy := map[int]int{0: 0, 1: 1, 3: 2, 5: 3, 6: 4, 7: 5, 8: 6, 9: 7}
	for i := range b.ModernSections {
		tier := int(r.byte())
		if tier != i {
			return nil, ErrProtocol
		}
		s := &b.ModernSections[i]
		s.Tier = modernTiers[tier]
		s.Required = r.byte() != 0
		s.Tokens = r.integer()
		s.TrimmedItems = r.integer()
		s.CoarsenedItems = r.integer()
		count := r.count()
		s.Items = make([]ModernItem, 0, count)
		for j := 0; j < count; j++ {
			if int(r.byte()) != tier {
				return nil, ErrProtocol
			}
			item := ModernItem{Tier: s.Tier, Coarsened: r.byte() != 0}
			r.byte()
			authority := int(r.byte())
			if authority >= len(authorities) {
				return nil, ErrProtocol
			}
			item.Authority = authorities[authority]
			r.raw(8)
			item.Tokens = r.integer()
			item.URI = string(r.bytesValue())
			item.Provenance = r.lsns()
			item.Content = string(r.bytesValue())
			s.Items = append(s.Items, item)
			if old, ok := legacy[i]; ok {
				b.Sections[old].Tier = uint8(old)
				b.Sections[old].Items = append(b.Sections[old].Items, BundleItem{Tier: uint8(old), URI: item.URI, Provenance: item.Provenance, Content: []byte(item.Content), Tokens: item.Tokens, Coarsened: item.Coarsened})
			}
		}
	}
	r.raw(64)
	r.integer()
	r.bytesValue()
	r.integer()
	r.raw(r.count())
	for i := 0; i < 4; i++ {
		r.lsns()
	}
	gaps := r.count()
	gapNames := []string{"truncated_lane", "dropped_tier", "narrowed_subtask", "missing_binding", "stale_binding", "conflicting_binding", "index_lag", "pending_protected_proposal"}
	for i := 0; i < gaps; i++ {
		kind := int(r.byte())
		r.raw(2)
		name := "unknown"
		if kind < len(gapNames) {
			name = gapNames[kind]
		}
		b.Gaps = append(b.Gaps, Gap{Kind: name, Detail: string(r.bytesValue())})
	}
	b.Health = make(map[string]string)
	healthNames := []string{"semantic_ready", "semantic_lagging", "lexical_only", "unavailable"}
	for _, key := range []string{"encoder", "backlog", "projection", "inclusion"} {
		value := int(r.byte())
		name := "unknown"
		if value < len(healthNames) {
			name = healthNames[value]
		}
		b.Health[key] = name
	}
	if r.err != nil {
		return nil, r.err
	}
	if r.offset != len(encoded) {
		return nil, fmt.Errorf("%w: activation trailing bytes", ErrProtocol)
	}
	return b, nil
}

func Render(bundle *Bundle, options RenderOptions) RenderedPrompt {
	out := RenderedPrompt{Version: 1}
	if bundle == nil {
		return out
	}
	hash := blake3.Sum256(bundle.Canonical)
	out.BundleHash = hex.EncodeToString(hash[:])
	excluded := map[uint64]bool{}
	for _, lsn := range options.SameTurnLSNs {
		excluded[lsn] = true
	}
	for _, section := range bundle.ModernSections {
		target := PromptSection{Tier: section.Tier, Label: "Untrusted memory · " + section.Tier}
		for _, item := range section.Items {
			if !strings.HasPrefix(item.URI, "hm://") || len(item.Provenance) == 0 || !utf8.ValidString(item.Content) || internalActivationContent([]byte(item.Content)) {
				continue
			}
			valid := true
			for _, lsn := range item.Provenance {
				if lsn == 0 || excluded[lsn] {
					valid = false
				}
			}
			if !valid {
				continue
			}
			authority := item.Authority
			if strings.HasPrefix(strings.TrimSpace(item.Content), "RECONSTRUCTION") {
				authority = "assistant_generated"
			}
			target.Items = append(target.Items, PromptItem{Role: "user", Trust: "untrusted_memory", Authority: authority, ProvenanceURI: item.URI, Provenance: append([]uint64(nil), item.Provenance...), Content: item.Content})
		}
		out.Sections = append(out.Sections, target)
	}
	return out
}

func renderModernText(bundle *Bundle, premises []string) string {
	var out strings.Builder
	for _, section := range Render(bundle, RenderOptions{}).Sections {
		if section.Tier == "conversation" || section.Tier == "work_ledger" {
			continue
		}
		if len(section.Items) == 0 {
			continue
		}
		out.WriteString("[" + section.Label + "]\n")
		for _, item := range section.Items {
			out.WriteString(item.Content + "\n")
		}
	}
	if len(premises) > 0 {
		out.WriteString("[premises]\n" + strings.Join(premises, "\n") + "\n")
	}
	return strings.TrimSpace(out.String())
}

func projectModern(bundle *Bundle, exclusions ProjectionExclusions) []MemoryProjection {
	var out []MemoryProjection
	seen := map[[32]byte]bool{}
	for _, section := range Render(bundle, RenderOptions{}).Sections {
		if section.Tier == "conversation" || section.Tier == "work_ledger" || section.Tier == "prospective" {
			continue
		}
		for _, item := range section.Items {
			if _, ok := exclusions.SourceIdentities[item.ProvenanceURI]; ok {
				continue
			}
			digest := sha256.Sum256([]byte(strings.TrimSpace(item.Content)))
			if _, ok := exclusions.ContentHashes[digest]; ok || seen[digest] {
				continue
			}
			seen[digest] = true
			u, err := url.Parse(item.ProvenanceURI)
			if err != nil {
				continue
			}
			parts := strings.Split(strings.Trim(u.Path, "/"), "/")
			q := u.Query()
			entry := MemoryProjection{Tier: section.Tier, Text: item.Content, URI: item.ProvenanceURI, SourceIdentity: item.ProvenanceURI, Provenance: item.Provenance, SourceType: q.Get("src"), SelectionReason: q.Get("why"), EpistemicStatus: item.Authority, Confidence: 1}
			if section.Tier == "fused" {
				entry.Tier = "recall"
			}
			if len(parts) > 0 {
				entry.ConversationID = parts[0]
			}
			if ns, err := strconv.ParseInt(q.Get("at"), 10, 64); err == nil && ns > 0 {
				entry.Date = time.Unix(0, ns).UTC().Format("2006-01-02")
			}
			entry.RelevanceScore, _ = strconv.ParseFloat(q.Get("score"), 64)
			out = append(out, entry)
		}
	}
	return out
}
