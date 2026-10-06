package cortexclient

import (
	"bytes"
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"strconv"
	"strings"
	"sync"
	"unicode/utf8"
)

const ContextVersion uint32 = 1
const contextSafeInteger uint64 = 1<<53 - 1

type ContextScope struct {
	OwnerID     string  `json:"owner_id"`
	ProjectID   string  `json:"project_id"`
	WorkspaceID *string `json:"workspace_id"`
}

func contextIdentifier(value string) error {
	if len(value) == 0 || len(value) > 256 {
		return errors.New("invalid context identifier")
	}
	for _, b := range []byte(value) {
		if b < 33 || b > 126 {
			return errors.New("invalid context identifier")
		}
	}
	return nil
}
func (s ContextScope) Validate() error {
	for _, v := range []string{s.OwnerID, s.ProjectID} {
		if err := contextIdentifier(v); err != nil {
			return err
		}
	}
	if s.WorkspaceID != nil {
		return contextIdentifier(*s.WorkspaceID)
	}
	return nil
}
func (s ContextScope) equal(other ContextScope) bool {
	return s.OwnerID == other.OwnerID && s.ProjectID == other.ProjectID && ((s.WorkspaceID == nil && other.WorkspaceID == nil) || (s.WorkspaceID != nil && other.WorkspaceID != nil && *s.WorkspaceID == *other.WorkspaceID))
}

type ContextCursor struct {
	Epoch    uint64 `json:"epoch"`
	Sequence uint64 `json:"sequence"`
}

func (c ContextCursor) validate() error {
	if c.Epoch == 0 || c.Epoch > contextSafeInteger || c.Sequence > contextSafeInteger {
		return errors.New("invalid context cursor")
	}
	return nil
}
func (c ContextCursor) before(other ContextCursor) bool {
	return c.Epoch < other.Epoch || (c.Epoch == other.Epoch && c.Sequence < other.Sequence)
}

type ContextBudget struct {
	ContextTokens        uint64 `json:"context_tokens"`
	ReservedOutputTokens uint64 `json:"reserved_output_tokens"`
	RequiredTokens       uint64 `json:"required_tokens"`
}

func (b ContextBudget) Available() (uint64, error) {
	if b.ContextTokens == 0 || b.ContextTokens > contextSafeInteger || b.ReservedOutputTokens == 0 || b.ReservedOutputTokens > b.ContextTokens || b.RequiredTokens > b.ContextTokens-b.ReservedOutputTokens {
		return 0, errors.New("invalid context budget")
	}
	return b.ContextTokens - b.ReservedOutputTokens - b.RequiredTokens, nil
}

type Nanoseconds int64

func (n Nanoseconds) MarshalJSON() ([]byte, error) {
	return json.Marshal(strconv.FormatInt(int64(n), 10))
}
func (n *Nanoseconds) UnmarshalJSON(data []byte) error {
	var s string
	if len(data) > 0 && data[0] == '"' {
		if err := json.Unmarshal(data, &s); err != nil {
			return err
		}
	} else {
		s = string(data)
	}
	value, err := strconv.ParseInt(s, 10, 64)
	if err != nil || strconv.FormatInt(value, 10) != s {
		return errors.New("invalid canonical nanoseconds")
	}
	if len(data) > 0 && data[0] != '"' && (value > int64(contextSafeInteger) || value < -int64(contextSafeInteger)) {
		return errors.New("unsafe numeric nanoseconds")
	}
	*n = Nanoseconds(value)
	return nil
}
func ParseNanoseconds(value string) (Nanoseconds, error) {
	var n Nanoseconds
	data, _ := json.Marshal(value)
	err := json.Unmarshal(data, &n)
	return n, err
}

type ContextBytes []byte

func (b ContextBytes) MarshalJSON() ([]byte, error) {
	values := make([]uint16, len(b))
	for i, v := range b {
		values[i] = uint16(v)
	}
	return json.Marshal(values)
}
func (b *ContextBytes) UnmarshalJSON(data []byte) error {
	var values []uint16
	if err := json.Unmarshal(data, &values); err != nil {
		return err
	}
	if bytes.Equal(data, []byte("null")) {
		return errors.New("null source bytes")
	}
	out := make([]byte, len(values))
	for i, v := range values {
		if v > 255 {
			return errors.New("invalid source byte")
		}
		out[i] = byte(v)
	}
	*b = out
	return nil
}
func contextJSON(value any) ([]byte, error) {
	var buffer bytes.Buffer
	encoder := json.NewEncoder(&buffer)
	encoder.SetEscapeHTML(false)
	if err := encoder.Encode(value); err != nil {
		return nil, err
	}
	result := bytes.TrimSuffix(buffer.Bytes(), []byte("\n"))
	var canonical bytes.Buffer
	for i := 0; i < len(result); i++ {
		if result[i] == '\\' && i+5 < len(result) && (string(result[i:i+6]) == `\u2028` || string(result[i:i+6]) == `\u2029`) {
			preceding := 0
			for j := i - 1; j >= 0 && result[j] == '\\'; j-- {
				preceding++
			}
			if preceding%2 == 0 {
				if result[i+5] == '8' {
					canonical.WriteString("\u2028")
				} else {
					canonical.WriteString("\u2029")
				}
				i += 5
				continue
			}
		}
		canonical.WriteByte(result[i])
	}
	result = canonical.Bytes()
	return result, nil
}
func contextDigest(value any) (string, error) {
	data, err := contextJSON(value)
	if err != nil {
		return "", err
	}
	sum := sha256.Sum256(data)
	return hex.EncodeToString(sum[:]), nil
}

type ContextPart struct {
	Kind      string
	Text      string
	CallID    string
	Name      string
	Arguments string
	Content   string
	Failed    bool
	MediaType string
	Reference string
	Digest    string
}

func (p ContextPart) MarshalJSON() ([]byte, error) {
	for _, v := range []string{p.Kind, p.Text, p.CallID, p.Name, p.Arguments, p.Content, p.MediaType, p.Reference, p.Digest} {
		if !utf8.ValidString(v) {
			return nil, errors.New("invalid source UTF-8")
		}
	}
	switch p.Kind {
	case "text":
		return contextJSON(struct {
			Kind string `json:"kind"`
			Text string `json:"text"`
		}{p.Kind, p.Text})
	case "tool_call":
		return contextJSON(struct {
			Kind      string `json:"kind"`
			CallID    string `json:"call_id"`
			Name      string `json:"name"`
			Arguments string `json:"arguments"`
		}{p.Kind, p.CallID, p.Name, p.Arguments})
	case "tool_result":
		return contextJSON(struct {
			Kind    string `json:"kind"`
			CallID  string `json:"call_id"`
			Content string `json:"content"`
			Failed  bool   `json:"failed"`
		}{p.Kind, p.CallID, p.Content, p.Failed})
	case "opaque":
		return contextJSON(struct {
			Kind      string `json:"kind"`
			MediaType string `json:"media_type"`
			Reference string `json:"reference"`
			Digest    string `json:"digest"`
		}{p.Kind, p.MediaType, p.Reference, p.Digest})
	default:
		return nil, errors.New("unsupported source part")
	}
}
func (p *ContextPart) UnmarshalJSON(data []byte) error {
	var fields struct {
		Kind      string `json:"kind"`
		Text      string `json:"text"`
		CallID    string `json:"call_id"`
		Name      string `json:"name"`
		Arguments string `json:"arguments"`
		Content   string `json:"content"`
		Failed    bool   `json:"failed"`
		MediaType string `json:"media_type"`
		Reference string `json:"reference"`
		Digest    string `json:"digest"`
	}
	if err := json.Unmarshal(data, &fields); err != nil {
		return err
	}
	*p = ContextPart{fields.Kind, fields.Text, fields.CallID, fields.Name, fields.Arguments, fields.Content, fields.Failed, fields.MediaType, fields.Reference, fields.Digest}
	_, err := p.MarshalJSON()
	return err
}

type ContextSource struct {
	ID           string        `json:"id"`
	Ordinal      uint64        `json:"ordinal"`
	Role         string        `json:"role"`
	Parts        []ContextPart `json:"parts"`
	OccurredAtNS *Nanoseconds  `json:"occurred_at_ns"`
	RecordedAtNS Nanoseconds   `json:"recorded_at_ns"`
	Authority    string        `json:"authority"`
	SourceDigest string        `json:"source_digest"`
}

func (s ContextSource) Freeze() (ContextSource, error) {
	if err := contextIdentifier(s.ID); err != nil {
		return s, err
	}
	if s.Ordinal > contextSafeInteger || len(s.Parts) == 0 || len(s.Parts) > 4096 {
		return s, errors.New("invalid source bounds")
	}
	if s.Role != "user" && s.Role != "assistant" && s.Role != "tool" {
		return s, errors.New("invalid source role")
	}
	switch s.Authority {
	case "user_asserted", "external_observed", "tool_observed", "runtime_fact", "assistant_generated", "derived_inference":
	default:
		return s, errors.New("invalid source authority")
	}
	s.Parts = append([]ContextPart(nil), s.Parts...)
	if s.OccurredAtNS != nil {
		n := *s.OccurredAtNS
		s.OccurredAtNS = &n
	}
	expected := s.SourceDigest
	s.SourceDigest = ""
	digest, err := contextDigest(s)
	if err != nil {
		return s, err
	}
	if expected != "" && expected != digest {
		return s, errors.New("source identity changed")
	}
	s.SourceDigest = digest
	return s, nil
}

type ContextSpan struct {
	SourceID     string `json:"source_id"`
	SourceDigest string `json:"source_digest"`
	ByteStart    uint64 `json:"byte_start"`
	ByteEnd      uint64 `json:"byte_end"`
}
type ContextBlock struct {
	ID         string        `json:"id"`
	Text       string        `json:"text"`
	Authority  string        `json:"authority"`
	Provenance []ContextSpan `json:"provenance"`
	Tokens     uint64        `json:"tokens"`
	Required   bool          `json:"required"`
}
type ContextReport struct {
	Version    uint32         `json:"version"`
	Scope      ContextScope   `json:"scope"`
	SessionID  string         `json:"session_id"`
	Cursor     ContextCursor  `json:"cursor"`
	Generation uint64         `json:"generation"`
	Blocks     []ContextBlock `json:"blocks"`
	Included   []string       `json:"included"`
	Omitted    []struct {
		ID     string `json:"id"`
		Reason string `json:"reason"`
	} `json:"omitted"`
	Gaps       []string `json:"gaps"`
	TokenCount uint64   `json:"token_count"`
	Digest     string   `json:"digest"`
}
type ContextDiagnostics struct {
	Version    uint32          `json:"version"`
	SessionID  string          `json:"session_id"`
	Report     ContextReport   `json:"report"`
	Messages   json.RawMessage `json:"messages"`
	History    json.RawMessage `json:"history"`
	Coverage   json.RawMessage `json:"coverage"`
	Cache      json.RawMessage `json:"cache"`
	Jobs       json.RawMessage `json:"jobs"`
	Provenance []string        `json:"provenance"`
	Migrations json.RawMessage `json:"migrations"`
	Gaps       json.RawMessage `json:"gaps"`
}
type ContextHistoryReceipt struct {
	Version    uint32        `json:"version"`
	Scope      ContextScope  `json:"scope"`
	SessionID  string        `json:"session_id"`
	Cursor     ContextCursor `json:"cursor"`
	LastLSN    uint64        `json:"last_lsn"`
	Replayed   bool          `json:"replayed"`
	SourceSpan *ContextSpan  `json:"source_span"`
}
type ContextRelation struct {
	Kind          string `json:"kind"`
	ID            string `json:"id"`
	OriginalID    string `json:"original_id,omitempty"`
	ReplacementID string `json:"replacement_id,omitempty"`
	SourceID      string `json:"source_id,omitempty"`
}

func (r ContextRelation) validate() error {
	if err := contextIdentifier(r.ID); err != nil {
		return err
	}
	switch r.Kind {
	case "edit", "regenerate":
		if r.SourceID != "" {
			return errors.New("invalid relation fields")
		}
		if err := contextIdentifier(r.OriginalID); err != nil {
			return err
		}
		return contextIdentifier(r.ReplacementID)
	case "tombstone":
		if r.OriginalID != "" || r.ReplacementID != "" {
			return errors.New("invalid relation fields")
		}
		return contextIdentifier(r.SourceID)
	default:
		return errors.New("invalid source relation")
	}
}

type ContextRecovery struct {
	Version       uint32        `json:"version"`
	Scope         ContextScope  `json:"scope"`
	SessionID     string        `json:"session_id"`
	Source        ContextSource `json:"source"`
	Span          ContextSpan   `json:"span"`
	OriginalBytes ContextBytes  `json:"original_bytes"`
}
type ContextHistory struct {
	Version          uint32            `json:"version"`
	Scope            ContextScope      `json:"scope"`
	SessionID        string            `json:"session_id"`
	Cursor           ContextCursor     `json:"cursor"`
	Messages         []ContextSource   `json:"messages"`
	Relations        []ContextRelation `json:"relations"`
	Spans            []ContextSpan     `json:"spans"`
	Parent           json.RawMessage   `json:"parent"`
	UnsupportedParts json.RawMessage   `json:"unsupported_parts"`
}
type ContextClientError struct {
	Code        string
	EffectState string
	Cause       error
	Detail      json.RawMessage
}

func (e *ContextClientError) Error() string {
	return fmt.Sprintf("context %s (%s): %v", e.Code, e.EffectState, e.Cause)
}
func (e *ContextClientError) Unwrap() error { return e.Cause }

type ContextConfig struct {
	Scope        ContextScope
	SessionID    string
	Conversation string
	Actor        uint16
	ContextOwner string
}
type ContextClient struct {
	client     *Client
	config     ContextConfig
	mu         sync.Mutex
	cursor     ContextCursor
	negotiated uint32
}

func NewContextClient(client *Client, config ContextConfig) (*ContextClient, error) {
	if client == nil {
		return nil, errors.New("context transport required")
	}
	if err := config.Scope.Validate(); err != nil {
		return nil, err
	}
	if err := contextIdentifier(config.SessionID); err != nil {
		return nil, err
	}
	if config.Conversation == "" {
		config.Conversation = config.SessionID
	}
	if err := contextIdentifier(config.Conversation); err != nil {
		return nil, err
	}
	if config.ContextOwner != "host" && config.ContextOwner != "hypermind" {
		return nil, errors.New("explicit context owner required")
	}
	welcome := client.Welcome()
	if !welcome.Admin && welcome.ActorNamespace != config.Actor {
		return nil, errors.New("context actor does not match authenticated transport")
	}
	if config.Scope.WorkspaceID != nil {
		copy := *config.Scope.WorkspaceID
		config.Scope.WorkspaceID = &copy
	}
	return &ContextClient{client: client, config: config, cursor: ContextCursor{Epoch: 1}}, nil
}
func (c *ContextClient) Cursor() ContextCursor { c.mu.Lock(); defer c.mu.Unlock(); return c.cursor }
func (c *ContextClient) Version() uint32       { c.mu.Lock(); defer c.mu.Unlock(); return c.negotiated }
func EncodeContextIdentifier(value string) (string, error) {
	if err := contextIdentifier(value); err != nil {
		return "", err
	}
	var b strings.Builder
	for _, v := range []byte(value) {
		if v >= 'a' && v <= 'z' || v >= 'A' && v <= 'Z' || v >= '0' && v <= '9' || v == '-' || v == '_' || v == '.' || v == '~' {
			b.WriteByte(v)
		} else {
			fmt.Fprintf(&b, "%%%02X", v)
		}
	}
	return b.String(), nil
}
func (c *ContextClient) InspectURI() string {
	id, _ := EncodeContextIdentifier(c.config.SessionID)
	return fmt.Sprintf("hm://%d/context/%s", c.config.Actor, id)
}
func (c *ContextClient) invoke(ctx context.Context, verb string, input any, out any) error {
	envelope, err := c.client.CallTool(ctx, verb, input)
	if err != nil {
		return err
	}
	if envelope["ok"] != true {
		state, _ := envelope["effect_state"].(string)
		if state != "not_dispatched" && state != "rejected" {
			state = "unknown"
		}
		detail, _ := contextJSON(envelope)
		code := "operation_failed"
		if values, ok := envelope["items"].([]any); ok && len(values) > 0 {
			if first, ok := values[0].(map[string]any); ok {
				if name, ok := first["error"].(string); ok {
					code = name
				}
			}
		}
		return &ContextClientError{Code: code, EffectState: state, Detail: detail}
	}
	items, ok := envelope["items"].([]any)
	if !ok || len(items) == 0 {
		return &ContextClientError{Code: "invalid_response", EffectState: "unknown"}
	}
	encoded, err := contextJSON(items[0])
	if err != nil {
		return err
	}
	if err = json.Unmarshal(encoded, out); err != nil {
		return &ContextClientError{Code: "invalid_response", EffectState: "unknown", Cause: err}
	}
	return nil
}
func (c *ContextClient) accept(version uint32, scope ContextScope, session string, cursor *ContextCursor) error {
	if version != ContextVersion {
		return &ContextClientError{Code: "unsupported_version", EffectState: "rejected"}
	}
	if !scope.equal(c.config.Scope) || session != c.config.SessionID {
		return &ContextClientError{Code: "scope_mismatch", EffectState: "rejected"}
	}
	if cursor != nil {
		if err := cursor.validate(); err != nil {
			return err
		}
		if cursor.before(c.cursor) {
			return &ContextClientError{Code: "stale_cursor", EffectState: "rejected"}
		}
		c.cursor = *cursor
	}
	c.negotiated = ContextVersion
	return nil
}

type ContextActivation struct {
	Query              string
	TurnText           string
	Generation         uint64
	ModelID            string
	RequiredMessageIDs []string
	Tier               string
	DeferReductions    bool
}

func (c *ContextClient) Activate(ctx context.Context, budget ContextBudget, options ContextActivation) (ContextDiagnostics, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	var result ContextDiagnostics
	available, err := budget.Available()
	if err != nil {
		return result, err
	}
	if options.Generation > contextSafeInteger {
		return result, errors.New("invalid generation")
	}
	request := map[string]any{"version": ContextVersion, "scope": c.config.Scope, "session_id": c.config.SessionID, "budget": budget, "generation": options.Generation, "query": options.Query, "defer_reductions": options.DeferReductions}
	if options.ModelID != "" {
		request["model_id"] = options.ModelID
	}
	if options.RequiredMessageIDs != nil {
		request["required_message_ids"] = options.RequiredMessageIDs
	}
	if options.Tier != "" {
		request["tier"] = options.Tier
	}
	err = c.invoke(ctx, "activate", map[string]any{"conversation": c.config.Conversation, "query": options.Query, "turn_text": options.TurnText, "budget_tokens": available, "context": request}, &result)
	if err == nil {
		if result.Report.Version != ContextVersion || result.Version != ContextVersion {
			err = &ContextClientError{Code: "unsupported_version", EffectState: "rejected"}
		} else if result.SessionID != c.config.SessionID {
			err = &ContextClientError{Code: "scope_mismatch", EffectState: "rejected"}
		} else {
			err = c.accept(result.Version, result.Report.Scope, result.Report.SessionID, &result.Report.Cursor)
		}
	}
	return result, err
}
func (c *ContextClient) Inspect(ctx context.Context) (ContextDiagnostics, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	var result ContextDiagnostics
	err := c.invoke(ctx, "inspect", map[string]any{"uri": c.InspectURI()}, &result)
	if err == nil {
		if result.Report.Version != ContextVersion || result.Version != ContextVersion {
			err = &ContextClientError{Code: "unsupported_version", EffectState: "rejected"}
		} else if result.SessionID != c.config.SessionID {
			err = &ContextClientError{Code: "scope_mismatch", EffectState: "rejected"}
		} else {
			err = c.accept(result.Version, result.Report.Scope, result.Report.SessionID, &result.Report.Cursor)
		}
	}
	return result, err
}
func (c *ContextClient) remember(ctx context.Context, conversation, operation string, request any, out any) error {
	return c.invoke(ctx, "remember", map[string]any{"conversation": conversation, "content": "", "kind": "user", "context": map[string]any{"operation": operation, "request": request}}, out)
}
func (c *ContextClient) IngestSource(ctx context.Context, source ContextSource, original []byte) (ContextHistoryReceipt, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	var result ContextHistoryReceipt
	frozen, err := source.Freeze()
	if err != nil {
		return result, err
	}
	request := struct {
		Version       uint32        `json:"version"`
		Scope         ContextScope  `json:"scope"`
		SessionID     string        `json:"session_id"`
		Conversation  string        `json:"conversation"`
		Message       ContextSource `json:"message"`
		OriginalBytes ContextBytes  `json:"original_bytes"`
	}{ContextVersion, c.config.Scope, c.config.SessionID, c.config.Conversation, frozen, ContextBytes(original)}
	err = c.remember(ctx, c.config.Conversation, "source", request, &result)
	if err == nil {
		err = c.accept(result.Version, result.Scope, result.SessionID, &result.Cursor)
	}
	return result, err
}
func (c *ContextClient) Relate(ctx context.Context, relation ContextRelation) (ContextHistoryReceipt, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	var result ContextHistoryReceipt
	if err := relation.validate(); err != nil {
		return result, err
	}
	request := map[string]any{"version": ContextVersion, "scope": c.config.Scope, "session_id": c.config.SessionID, "conversation": c.config.Conversation, "relation": relation}
	err := c.remember(ctx, c.config.Conversation, "relation", request, &result)
	if err == nil {
		err = c.accept(result.Version, result.Scope, result.SessionID, &result.Cursor)
	}
	return result, err
}
func (c *ContextClient) Fork(ctx context.Context, childSession, childConversation string) (ContextHistoryReceipt, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	var result ContextHistoryReceipt
	if err := contextIdentifier(childSession); err != nil {
		return result, err
	}
	if childConversation == "" {
		childConversation = childSession
	}
	if err := contextIdentifier(childConversation); err != nil {
		return result, err
	}
	request := map[string]any{"version": ContextVersion, "scope": c.config.Scope, "parent_session_id": c.config.SessionID, "parent_conversation": c.config.Conversation, "child_session_id": childSession, "child_conversation": childConversation}
	err := c.remember(ctx, childConversation, "fork", request, &result)
	if err == nil && (result.Version != ContextVersion || !result.Scope.equal(c.config.Scope) || result.SessionID != childSession) {
		err = &ContextClientError{Code: "invalid_receipt", EffectState: "unknown"}
	}
	return result, err
}
func (c *ContextClient) Recover(ctx context.Context, sourceID string) (ContextRecovery, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	var result ContextRecovery
	encoded, err := EncodeContextIdentifier(sourceID)
	if err != nil {
		return result, err
	}
	err = c.invoke(ctx, "inspect", map[string]any{"uri": c.InspectURI() + "/source/" + encoded}, &result)
	if err == nil {
		err = c.accept(result.Version, result.Scope, result.SessionID, nil)
	}
	if err == nil {
		frozen, freezeErr := result.Source.Freeze()
		if freezeErr != nil || frozen.ID != sourceID || result.Span.SourceID != sourceID || result.Span.SourceDigest != frozen.SourceDigest || result.Span.ByteStart != 0 || result.Span.ByteEnd != uint64(len(result.OriginalBytes)) {
			err = &ContextClientError{Code: "invalid_recovery", EffectState: "unknown", Cause: freezeErr}
		}
	}
	return result, err
}
func (c *ContextClient) History(ctx context.Context) (ContextHistory, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	var result ContextHistory
	err := c.invoke(ctx, "inspect", map[string]any{"uri": c.InspectURI() + "/history"}, &result)
	if err == nil {
		err = c.accept(result.Version, result.Scope, result.SessionID, &result.Cursor)
	}
	return result, err
}
