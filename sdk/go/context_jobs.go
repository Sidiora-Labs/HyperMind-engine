package cortexclient

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
)

type ContextJobAction interface{ contextAction() string }
type ContextNote struct {
	ID             string          `json:"id"`
	Kind           string          `json:"kind"`
	Revision       uint64          `json:"revision"`
	Text           string          `json:"text"`
	Parents        []string        `json:"parents"`
	Contradictions []string        `json:"contradictions"`
	ExpiresAtNS    *Nanoseconds    `json:"expires_at_ns"`
	Predicate      json.RawMessage `json:"predicate"`
	Tombstoned     bool            `json:"tombstoned"`
}
type ContextNotesCommand interface{ contextCommand() string }
type CreateContextNote struct{ Note ContextNote }

func (CreateContextNote) contextCommand() string { return "Create" }
func (c CreateContextNote) MarshalJSON() ([]byte, error) {
	return contextJSON(map[string]any{"Create": c.Note})
}

type ReviseContextNote struct {
	Note             ContextNote `json:"note"`
	ExpectedRevision uint64      `json:"expected_revision"`
}

func (ReviseContextNote) contextCommand() string { return "Revise" }
func (c ReviseContextNote) MarshalJSON() ([]byte, error) {
	type plain ReviseContextNote
	return contextJSON(map[string]any{"Revise": plain(c)})
}

type TombstoneContextNote struct {
	ID               string `json:"id"`
	ExpectedRevision uint64 `json:"expected_revision"`
}

func (TombstoneContextNote) contextCommand() string { return "Tombstone" }
func (c TombstoneContextNote) MarshalJSON() ([]byte, error) {
	type plain TombstoneContextNote
	return contextJSON(map[string]any{"Tombstone": plain(c)})
}

type ContextNoteGrant struct {
	Principal string `json:"principal"`
	NoteID    string `json:"note_id"`
	Read      bool   `json:"read"`
	Write     bool   `json:"write"`
}

func (ContextNoteGrant) contextCommand() string { return "SetGrant" }
func (c ContextNoteGrant) MarshalJSON() ([]byte, error) {
	type plain ContextNoteGrant
	return contextJSON(map[string]any{"SetGrant": plain(c)})
}

type ContextAttributeProposal struct {
	ID           string `json:"id"`
	Key          string `json:"key"`
	Value        string `json:"value"`
	BaseRevision uint64 `json:"base_revision"`
	Proposer     string `json:"proposer"`
}

func (ContextAttributeProposal) contextCommand() string { return "ProposeAttribute" }
func (c ContextAttributeProposal) MarshalJSON() ([]byte, error) {
	type plain ContextAttributeProposal
	return contextJSON(map[string]any{"ProposeAttribute": plain(c)})
}

type ContextAttributeDecision struct {
	ProposalID string
	Accept     bool
}

func (c ContextAttributeDecision) contextCommand() string {
	if c.Accept {
		return "AcceptAttribute"
	}
	return "RejectAttribute"
}
func (c ContextAttributeDecision) MarshalJSON() ([]byte, error) {
	return contextJSON(map[string]any{c.contextCommand(): map[string]string{"proposal_id": c.ProposalID}})
}

type ContextNotesAction struct {
	Command ContextNotesCommand `json:"command"`
}

func (ContextNotesAction) contextAction() string { return "notes" }

type ContextReadNoteAction struct {
	ID    string            `json:"id"`
	NowNS Nanoseconds       `json:"now_ns"`
	Facts map[string]string `json:"facts"`
}

func (ContextReadNoteAction) contextAction() string { return "read_note" }

type ContextReadAttributeAction struct {
	Key string `json:"key"`
}

func (ContextReadAttributeAction) contextAction() string { return "read_attribute" }

type ContextSetPolicyAction struct {
	SessionID        string `json:"session_id"`
	ExpectedRevision uint64 `json:"expected_revision"`
	Revision         uint64 `json:"revision"`
}

func (ContextSetPolicyAction) contextAction() string { return "set_policy" }

type ContextSourceChunk struct {
	Sources []ContextSource `json:"sources"`
	Spans   []ContextSpan   `json:"spans"`
	Digest  string          `json:"digest"`
}
type ContextSummaryTier struct {
	Text     string        `json:"text"`
	Coverage []ContextSpan `json:"coverage"`
}
type ContextHistorianResult struct {
	SourceDigest string                `json:"source_digest"`
	Tiers        [4]ContextSummaryTier `json:"tiers"`
}
type ContextHistorianJob struct {
	ID             string                  `json:"id"`
	SessionID      string                  `json:"session_id"`
	Cursor         ContextCursor           `json:"cursor"`
	PolicyRevision uint64                  `json:"policy_revision"`
	Chunk          ContextSourceChunk      `json:"chunk"`
	Attempt        uint64                  `json:"attempt"`
	State          json.RawMessage         `json:"state"`
	Result         *ContextHistorianResult `json:"result"`
}
type ContextHistorianClaim struct {
	Job     ContextHistorianJob `json:"job"`
	Worker  string              `json:"worker"`
	Attempt uint64              `json:"attempt"`
}
type ContextUsage struct{ Known *uint64 }

func (u ContextUsage) MarshalJSON() ([]byte, error) {
	if u.Known == nil {
		return []byte(`"Unknown"`), nil
	}
	return contextJSON(map[string]uint64{"Known": *u.Known})
}

type ContextHistorianEnqueue struct {
	SessionID      string             `json:"session_id"`
	Cursor         ContextCursor      `json:"cursor"`
	PolicyRevision uint64             `json:"policy_revision"`
	Chunk          ContextSourceChunk `json:"chunk"`
	Reservation    uint64             `json:"reservation"`
	NowMS          uint64             `json:"now_ms"`
}

func (ContextHistorianEnqueue) contextAction() string { return "historian_enqueue" }

type ContextHistorianClaimAction struct {
	Worker  string `json:"worker"`
	NowMS   uint64 `json:"now_ms"`
	LeaseMS uint64 `json:"lease_ms"`
}

func (ContextHistorianClaimAction) contextAction() string { return "historian_claim" }

type ContextHistorianHeartbeat struct {
	Claim   ContextHistorianClaim `json:"claim"`
	NowMS   uint64                `json:"now_ms"`
	LeaseMS uint64                `json:"lease_ms"`
}

func (ContextHistorianHeartbeat) contextAction() string { return "historian_heartbeat" }

type ContextHistorianComplete struct {
	Claim  ContextHistorianClaim  `json:"claim"`
	Result ContextHistorianResult `json:"result"`
	Usage  ContextUsage           `json:"usage"`
	NowMS  uint64                 `json:"now_ms"`
}

func (ContextHistorianComplete) contextAction() string { return "historian_complete" }

type ContextHistorianFail struct {
	Claim      ContextHistorianClaim `json:"claim"`
	Usage      ContextUsage          `json:"usage"`
	NowMS      uint64                `json:"now_ms"`
	CooldownMS uint64                `json:"cooldown_ms"`
}

func (ContextHistorianFail) contextAction() string { return "historian_fail" }

type ContextHistorianCancel struct {
	ID string `json:"id"`
}

func (ContextHistorianCancel) contextAction() string { return "historian_cancel" }

type ContextHistorianExpire struct {
	NowMS      uint64 `json:"now_ms"`
	CooldownMS uint64 `json:"cooldown_ms"`
}

func (ContextHistorianExpire) contextAction() string { return "historian_expire" }

type ContextMaintenanceRequest struct {
	Kind           string          `json:"kind"`
	Sources        []ContextSource `json:"sources"`
	Cursor         ContextCursor   `json:"cursor"`
	SourceRevision uint64          `json:"source_revision"`
	PolicyRevision uint64          `json:"policy_revision"`
	Reservation    uint64          `json:"reservation"`
}
type ContextPublicationFence struct {
	InputDigest    string `json:"input_digest"`
	SourceRevision uint64 `json:"source_revision"`
	PolicyRevision uint64 `json:"policy_revision"`
}
type ContextJobLease struct {
	JobID     string                  `json:"job_id"`
	Attempt   uint64                  `json:"attempt"`
	ExpiresMS uint64                  `json:"expires_ms"`
	Fence     ContextPublicationFence `json:"fence"`
}
type ContextMaintenanceEnqueue struct {
	SessionID string                    `json:"session_id"`
	Request   ContextMaintenanceRequest `json:"request"`
}

func (ContextMaintenanceEnqueue) contextAction() string { return "maintenance_enqueue" }

type ContextMaintenanceClaim struct {
	NowMS uint64 `json:"now_ms"`
}

func (ContextMaintenanceClaim) contextAction() string { return "maintenance_claim" }

type ContextMaintenanceComplete struct {
	Lease        ContextJobLease `json:"lease"`
	Usage        ContextUsage    `json:"usage"`
	OutputDigest string          `json:"output_digest"`
	NowMS        uint64          `json:"now_ms"`
}

func (ContextMaintenanceComplete) contextAction() string { return "maintenance_complete" }

type ContextMaintenanceFail struct {
	Lease ContextJobLease `json:"lease"`
	Usage ContextUsage    `json:"usage"`
	NowMS uint64          `json:"now_ms"`
}

func (ContextMaintenanceFail) contextAction() string { return "maintenance_fail" }

type ContextMaintenanceCancel struct {
	ID    string `json:"id"`
	NowMS uint64 `json:"now_ms"`
}

func (ContextMaintenanceCancel) contextAction() string { return "maintenance_cancel" }

type ContextMaintenanceSettle struct {
	ID      string `json:"id"`
	Attempt uint64 `json:"attempt"`
	Actual  uint64 `json:"actual"`
}

func (ContextMaintenanceSettle) contextAction() string { return "maintenance_settle" }

type ContextInspectJobs struct{}

func (ContextInspectJobs) contextAction() string { return "inspect" }
func (c *ContextClient) Job(ctx context.Context, requestID string, action ContextJobAction) (json.RawMessage, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	if err := contextIdentifier(requestID); err != nil {
		return nil, err
	}
	if action == nil {
		return nil, errors.New("typed job action required")
	}
	data, err := contextJSON(action)
	if err != nil {
		return nil, err
	}
	var fields map[string]any
	decoder := json.NewDecoder(bytes.NewReader(data))
	decoder.UseNumber()
	if err = decoder.Decode(&fields); err != nil {
		return nil, err
	}
	fields["action"] = action.contextAction()
	request := map[string]any{"version": ContextVersion, "scope": c.config.Scope, "request_id": requestID, "action": fields}
	var result json.RawMessage
	err = c.remember(ctx, c.config.Conversation, "job", request, &result)
	return result, err
}

type ContextImportEntry struct {
	SourceID string          `json:"source_id"`
	Kind     string          `json:"kind"`
	Digest   string          `json:"digest"`
	Payload  json.RawMessage `json:"payload"`
}
type ContextImportBundle struct {
	Version  uint32               `json:"version"`
	ImportID string               `json:"import_id"`
	Scope    ContextScope         `json:"scope"`
	Entries  []ContextImportEntry `json:"entries"`
	Digest   string               `json:"digest"`
}
type ContextImportReceipt struct {
	Version      uint32       `json:"version"`
	Scope        ContextScope `json:"scope"`
	ImportID     string       `json:"import_id"`
	BundleDigest string       `json:"bundle_digest"`
	Accepted     uint64       `json:"accepted"`
	Total        uint64       `json:"total"`
	Complete     bool         `json:"complete"`
	LastLSN      uint64       `json:"last_lsn"`
}

func (b ContextImportBundle) Freeze() (ContextImportBundle, error) {
	if b.Version != ContextVersion {
		return b, errors.New("unsupported import version")
	}
	if err := b.Scope.Validate(); err != nil {
		return b, err
	}
	if err := contextIdentifier(b.ImportID); err != nil {
		return b, err
	}
	if len(b.Entries) > 100000 {
		return b, errors.New("import too large")
	}
	b.Entries = append([]ContextImportEntry(nil), b.Entries...)
	ids := map[string]bool{}
	for i, entry := range b.Entries {
		if entry.SourceID == "" || ids[entry.SourceID] {
			return b, errors.New("duplicate import source")
		}
		ids[entry.SourceID] = true
		var payload any
		decoder := json.NewDecoder(bytes.NewReader(entry.Payload))
		decoder.UseNumber()
		if err := decoder.Decode(&payload); err != nil {
			return b, err
		}
		canonical, err := contextJSON(payload)
		if err != nil {
			return b, err
		}
		digest, err := contextDigest(payload)
		if err != nil {
			return b, err
		}
		if entry.Digest != "" && entry.Digest != digest {
			return b, errors.New("import entry identity changed")
		}
		entry.Payload = canonical
		entry.Digest = digest
		b.Entries[i] = entry
	}
	expected := b.Digest
	b.Digest = ""
	digest, err := contextDigest(b)
	if err != nil {
		return b, err
	}
	if expected != "" && expected != digest {
		return b, errors.New("import bundle identity changed")
	}
	b.Digest = digest
	return b, nil
}
func (c *ContextClient) Import(ctx context.Context, bundle ContextImportBundle, maxEntries uint64) (ContextImportReceipt, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	var result ContextImportReceipt
	if !bundle.Scope.equal(c.config.Scope) || maxEntries == 0 || maxEntries > 256 {
		return result, errors.New("invalid scoped import")
	}
	frozen, err := bundle.Freeze()
	if err != nil {
		return result, err
	}
	err = c.invoke(ctx, "remember", map[string]any{"conversation": c.config.Conversation, "content": "", "kind": "user", "context": map[string]any{"operation": "import", "request": frozen, "max_entries": maxEntries}}, &result)
	if err == nil && (result.Version != ContextVersion || !result.Scope.equal(c.config.Scope) || result.ImportID != frozen.ImportID || result.BundleDigest != frozen.Digest) {
		err = &ContextClientError{Code: "invalid_import_receipt", EffectState: "unknown"}
	}
	return result, err
}

type ContextAttributeGrant struct {
	Principal string `json:"principal"`
	Key       string `json:"key"`
	Write     bool   `json:"write"`
}

func (ContextAttributeGrant) contextCommand() string { return "SetAttributeGrant" }
func (c ContextAttributeGrant) MarshalJSON() ([]byte, error) {
	type plain ContextAttributeGrant
	return contextJSON(map[string]any{"SetAttributeGrant": plain(c)})
}

type RevokeContextNoteGrant struct {
	Principal string `json:"principal"`
	NoteID    string `json:"note_id"`
}

func (RevokeContextNoteGrant) contextCommand() string { return "RevokeGrant" }
func (c RevokeContextNoteGrant) MarshalJSON() ([]byte, error) {
	type plain RevokeContextNoteGrant
	return contextJSON(map[string]any{"RevokeGrant": plain(c)})
}
func (chunk ContextSourceChunk) Freeze() (ContextSourceChunk, error) {
	if len(chunk.Sources) == 0 || len(chunk.Sources) != len(chunk.Spans) {
		return chunk, errors.New("invalid chunk coverage")
	}
	chunk.Sources = append([]ContextSource(nil), chunk.Sources...)
	chunk.Spans = append([]ContextSpan(nil), chunk.Spans...)
	for i, source := range chunk.Sources {
		frozen, err := source.Freeze()
		if err != nil {
			return chunk, err
		}
		chunk.Sources[i] = frozen
		span := chunk.Spans[i]
		if span.SourceID != frozen.ID || span.SourceDigest != frozen.SourceDigest || span.ByteStart != 0 || span.ByteEnd == 0 || (i > 0 && chunk.Sources[i-1].Ordinal >= source.Ordinal) {
			return chunk, errors.New("invalid chunk identity")
		}
	}
	digest, err := contextDigest([2]any{chunk.Sources, chunk.Spans})
	if err != nil {
		return chunk, err
	}
	if chunk.Digest != "" && chunk.Digest != digest {
		return chunk, errors.New("chunk identity changed")
	}
	chunk.Digest = digest
	return chunk, nil
}
