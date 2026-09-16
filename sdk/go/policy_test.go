package cortexclient

import (
	"context"
	"errors"
	"strings"
	"testing"
	"time"

	"centra/core/cortexclient/wire/hypermind/schema"
)

func TestCurrentProtectedConstraintPolicy(t *testing.T) {
	fixture := startDaemon(t)
	client := dialActor(t, fixture, 97)
	ctx := context.Background()
	seam, err := NewLoopSeam(client, SeamConfig{ConversationID: "protected-current", BudgetTokens: 4096})
	if err != nil {
		t.Fatal(err)
	}
	seam.RecordUser("current authoritative user transcript cobalt")
	if err := seam.RecordError(); err != nil {
		t.Fatal(err)
	}
	_, first, last := seam.ProvenanceRange()
	_, err = seam.Consolidate(ctx, []Assertion{{BeliefID: []byte("derived-constraint"), Type: 2, CanonicalIdentity: "derived-policy", Value: []byte("unapproved protected constraint"), Provenance: [][2]uint64{{first, last}}}})
	if err != nil {
		t.Fatal(err)
	}
	if _, err := client.AsOf(ctx, 2, "derived-policy", AsOfOptions{ValidAtNS: time.Now().UnixNano()}); !errors.Is(err, ErrAbsent) {
		t.Fatalf("derived protected constraint became current: %v", err)
	}
	records, _, err := client.Transcript(ctx, seam.Conversation(), 0, 20)
	if err != nil {
		t.Fatal(err)
	}
	found := false
	for _, record := range records {
		if record.Kind == KindConsolidation {
			found = true
			envelope := schema.GetRootAsEventEnvelope(record.Payload, 0)
			if envelope.Authority() != schema.Authorityderived_inference {
				t.Fatal("derived authority promoted")
			}
		}
	}
	if !found {
		t.Fatal("missing real consolidation evidence")
	}
	bundle, err := seam.ActivateBundle(ctx, ActivationQuery{Query: "cobalt derived-policy"})
	if err != nil {
		t.Fatal(err)
	}
	rendered := RenderBundle(bundle, nil)
	if strings.Contains(rendered, "current authoritative user transcript") || strings.Contains(rendered, "unapproved protected constraint") {
		t.Fatalf("unsafe legacy activation: %q", rendered)
	}
}
