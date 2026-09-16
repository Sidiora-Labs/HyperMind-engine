package cortexclient

import (
	"context"
	"crypto/rand"
	"encoding/json"
	"errors"
	"fmt"
	"net"
	"strings"
	"sync"

	flatbuffers "centra/core/cortexclient/internal/flatbuffers"
	"centra/core/cortexclient/wire/hypermind/protocol"
)

type ToolEnvelope map[string]any
type RecallOptions struct {
	Mode               string
	Limit              uint32
	Anchor             string
	FromNS, ToNS       int64
	SinceLSN, UntilLSN uint64
}

func (c *Client) Recall(ctx context.Context, query string, options RecallOptions) ([]uint64, error) {
	level := uint8(1)
	mode := protocol.RecallModelist_windows
	switch options.Mode {
	case "", "lexical":
	case "semantic":
		level = 0
	case "entity":
		level = 2
	case "temporal":
		level = 3
	case "near":
		mode = protocol.RecallModeopen_window
		level = 0
	default:
		return nil, ErrProtocol
	}
	if options.Limit == 0 {
		options.Limit = 20
	}
	if options.Anchor != "" {
		query = options.Anchor
	}
	table, _, err := c.request(ctx, func(b *flatbuffers.Builder, id uint64) []byte {
		q := b.CreateByteVector([]byte(query))
		protocol.RecallStart(b)
		protocol.RecallAddQuery(b, q)
		protocol.RecallAddLimit(b, options.Limit)
		protocol.RecallAddMode(b, mode)
		protocol.RecallAddLevel(b, level)
		protocol.RecallAddStartNs(b, options.FromNS)
		protocol.RecallAddEndNs(b, options.ToNS)
		return finishRequest(b, id, protocol.RequestPayloadRecall, protocol.RecallEnd(b))
	}, protocol.ResponsePayloadRecallResult)
	if err != nil {
		return nil, err
	}
	var result protocol.RecallResult
	result.Init(table.Bytes, table.Pos)
	out := make([]uint64, 0, result.MembersLength())
	for i := 0; i < result.MembersLength(); i++ {
		v := result.Members(i)
		if (options.SinceLSN == 0 || v > options.SinceLSN) && (options.UntilLSN == 0 || v <= options.UntilLSN) {
			out = append(out, v)
		}
	}
	return out, nil
}

type AsOfOptions struct {
	ValidAtNS  int64
	KnownAtLSN uint64
}

func (c *Client) AsOf(ctx context.Context, beliefType uint8, identity string, options AsOfOptions) (*protocol.BeliefResult, error) {
	if options.ValidAtNS != 0 && options.KnownAtLSN != 0 {
		return nil, ErrProtocol
	}
	table, _, err := c.request(ctx, func(b *flatbuffers.Builder, id uint64) []byte {
		key := b.CreateString(identity)
		protocol.AsOfStart(b)
		protocol.AsOfAddBeliefType(b, beliefType)
		protocol.AsOfAddCanonicalIdentity(b, key)
		protocol.AsOfAddValidTimeNs(b, options.ValidAtNS)
		protocol.AsOfAddKnownLsn(b, options.KnownAtLSN)
		return finishRequest(b, id, protocol.RequestPayloadAsOf, protocol.AsOfEnd(b))
	}, protocol.ResponsePayloadBeliefResult)
	if err != nil {
		return nil, err
	}
	result := &protocol.BeliefResult{}
	result.Init(table.Bytes, table.Pos)
	if !result.Present() {
		return nil, ErrAbsent
	}
	return result, nil
}

func (c *Client) requestOnce(ctx context.Context, build func(*flatbuffers.Builder, uint64) []byte, want protocol.ResponsePayload) (flatbuffers.Table, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	if err := ctx.Err(); err != nil {
		return flatbuffers.Table{}, &TransportError{EffectState: "not_dispatched", Cause: err}
	}
	if err := c.ensureLocked(); err != nil {
		return flatbuffers.Table{}, &TransportError{EffectState: "not_dispatched", Cause: err}
	}
	id := c.nextRequestIDLocked()
	encoded := build(flatbuffers.NewBuilder(512), id)
	response, _, err := c.roundTripLocked(id, encoded)
	if err != nil {
		var transport *TransportError
		if errors.As(err, &transport) {
			return flatbuffers.Table{}, transport
		}
		return flatbuffers.Table{}, &TransportError{EffectState: "unknown", Cause: err}
	}
	return responseTable(response, want)
}
func (c *Client) CallTool(ctx context.Context, verb string, input any) (ToolEnvelope, error) {
	valid := map[string]bool{"remember": true, "recall": true, "activate": true, "attest": true, "believe": true, "retract": true, "dispute": true, "intend": true, "bind": true, "predict": true, "outcome": true, "consolidate": true, "forget": true, "inspect": true}
	if !valid[verb] {
		return nil, &TransportError{EffectState: "not_dispatched", Cause: ErrProtocol}
	}
	arguments, err := json.Marshal(input)
	if err != nil {
		return nil, &TransportError{EffectState: "not_dispatched", Cause: err}
	}
	table, err := c.requestOnce(ctx, func(b *flatbuffers.Builder, id uint64) []byte {
		name := b.CreateString(verb)
		args := b.CreateByteVector(arguments)
		protocol.ToolRequestStart(b)
		protocol.ToolRequestAddVerb(b, name)
		protocol.ToolRequestAddArgumentsJson(b, args)
		return finishRequest(b, id, protocol.RequestPayloadToolRequest, protocol.ToolRequestEnd(b))
	}, protocol.ResponsePayloadBytesResult)
	if err != nil {
		return nil, err
	}
	var result protocol.BytesResult
	result.Init(table.Bytes, table.Pos)
	var envelope ToolEnvelope
	decoder := json.NewDecoder(strings.NewReader(string(result.BytesBytes())))
	decoder.UseNumber()
	if err = decoder.Decode(&envelope); err != nil {
		return nil, &TransportError{EffectState: "unknown", Cause: err}
	}
	return envelope, nil
}
func (c *Client) CryptoDelete(ctx context.Context, actor uint16) ([]byte, error) {
	table, err := c.requestOnce(ctx, func(b *flatbuffers.Builder, id uint64) []byte {
		protocol.CryptoDeleteStart(b)
		protocol.CryptoDeleteAddActor(b, actor)
		return finishRequest(b, id, protocol.RequestPayloadCryptoDelete, protocol.CryptoDeleteEnd(b))
	}, protocol.ResponsePayloadDeleteResult)
	if err != nil {
		return nil, err
	}
	var result protocol.DeleteResult
	result.Init(table.Bytes, table.Pos)
	return append([]byte(nil), result.ReceiptBytes()...), nil
}

type Subscription struct {
	client *Client
	reader *frameReader
	conn   net.Conn
	mu     sync.Mutex
	ID     uint64
	stop   func() bool
}
type SubscribedEvent struct {
	LSN             uint64
	Kind            EventKind
	Actor           uint16
	Conversation    [16]byte
	WallTimestampNS int64
	Payload         []byte
}

func (c *Client) Subscribe(ctx context.Context, conversation *[16]byte, sinceLSN uint64) (*Subscription, error) {
	config := c.cfg
	if _, err := rand.Read(config.ConnectionID[:]); err != nil {
		return nil, err
	}
	client, err := Dial(config)
	if err != nil {
		return nil, err
	}
	s := &Subscription{client: client, reader: client.reader, conn: client.conn}
	s.stop = context.AfterFunc(ctx, func() { s.conn.Close() })
	table, _, err := client.request(ctx, func(b *flatbuffers.Builder, id uint64) []byte {
		var conv flatbuffers.UOffsetT
		if conversation != nil {
			conv = b.CreateByteVector(conversation[:])
		}
		protocol.SubscribeStart(b)
		if conv != 0 {
			protocol.SubscribeAddConversation(b, conv)
		}
		protocol.SubscribeAddSinceLsn(b, sinceLSN)
		return finishRequest(b, id, protocol.RequestPayloadSubscribe, protocol.SubscribeEnd(b))
	}, protocol.ResponsePayloadSubscriptionAck)
	if err != nil {
		s.Close()
		return nil, err
	}
	var ack protocol.SubscriptionAck
	ack.Init(table.Bytes, table.Pos)
	s.ID = ack.SubscriptionId()
	return s, nil
}
func (s *Subscription) Next(ctx context.Context) (SubscribedEvent, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	if err := ctx.Err(); err != nil {
		return SubscribedEvent{}, err
	}
	cancel := context.AfterFunc(ctx, func() { s.conn.Close() })
	defer cancel()
	bytes, err := s.reader.next(s.client.cfg.RequestTimeout)
	if err != nil {
		return SubscribedEvent{}, err
	}
	envelope := protocol.GetRootAsWireEnvelope(bytes, 0)
	var table flatbuffers.Table
	if envelope.PayloadType() != protocol.WirePayloadEvent || !envelope.Payload(&table) {
		return SubscribedEvent{}, ErrProtocol
	}
	var event protocol.Event
	event.Init(table.Bytes, table.Pos)
	if event.SubscriptionId() != s.ID {
		return SubscribedEvent{}, ErrProtocol
	}
	out := SubscribedEvent{LSN: event.Lsn(), Kind: EventKind(event.Kind()), Actor: event.Actor(), WallTimestampNS: event.WallTimestampNs(), Payload: append([]byte(nil), event.PayloadBytes()...)}
	copy(out.Conversation[:], event.ConversationBytes())
	return out, nil
}
func (s *Subscription) Close() error {
	if s.stop != nil {
		s.stop()
	}
	s.conn.Close()
	return s.client.Close()
}

type Session struct {
	client       *Client
	Conversation string
	conversation [16]byte
}

func (c *Client) Session(conversation string) (*Session, error) {
	if conversation == "" {
		return nil, fmt.Errorf("%w: conversation required", ErrProtocol)
	}
	return &Session{client: c, Conversation: conversation, conversation: ConversationBytes(conversation)}, nil
}
func (s *Session) Remember(ctx context.Context, content string) (uint64, error) {
	if strings.HasPrefix(strings.TrimSpace(content), "RECONSTRUCTION") {
		return 0, fmt.Errorf("%w: RECONSTRUCTION cannot be remembered verbatim", ErrProtocol)
	}
	ack, err := s.client.Append(ctx, []AppendEvent{UserMsgEvent(s.conversation, content)})
	return ack.FirstLsn, err
}
func (s *Session) Recall(ctx context.Context, query string, options RecallOptions) ([]uint64, error) {
	return s.client.Recall(ctx, query, options)
}
func (s *Session) Activate(ctx context.Context, query string, budget uint64) (*Bundle, error) {
	seam, err := NewLoopSeam(s.client, SeamConfig{ConversationID: s.Conversation, BudgetTokens: budget})
	if err != nil {
		return nil, err
	}
	return seam.ActivateBundle(ctx, ActivationQuery{Query: query})
}
func (s *Session) Render(bundle *Bundle, options RenderOptions) RenderedPrompt {
	return Render(bundle, options)
}
func (s *Session) Attest(ctx context.Context, used, ignored []uint64) (uint32, error) {
	return s.client.Attest(ctx, used, ignored)
}
func (s *Session) AsOf(ctx context.Context, kind uint8, identity string, options AsOfOptions) (*protocol.BeliefResult, error) {
	return s.client.AsOf(ctx, kind, identity, options)
}
func (s *Session) call(ctx context.Context, verb string, input map[string]any) (ToolEnvelope, error) {
	args := map[string]any{}
	for k, v := range input {
		args[k] = v
	}
	args["conversation"] = s.Conversation
	return s.client.CallTool(ctx, verb, args)
}
func (s *Session) Believe(ctx context.Context, input map[string]any) (ToolEnvelope, error) {
	return s.call(ctx, "believe", input)
}
func (s *Session) Intend(ctx context.Context, input map[string]any) (ToolEnvelope, error) {
	return s.call(ctx, "intend", input)
}
func (s *Session) Bind(ctx context.Context, input map[string]any) (ToolEnvelope, error) {
	return s.call(ctx, "bind", input)
}
func (s *Session) Predict(ctx context.Context, input map[string]any) (ToolEnvelope, error) {
	return s.call(ctx, "predict", input)
}
func (s *Session) Outcome(ctx context.Context, input map[string]any) (ToolEnvelope, error) {
	return s.call(ctx, "outcome", input)
}
func (s *Session) Consolidate(ctx context.Context, input map[string]any) (ToolEnvelope, error) {
	return s.call(ctx, "consolidate", input)
}
func (s *Session) Retract(ctx context.Context, input map[string]any) (ToolEnvelope, error) {
	return s.call(ctx, "retract", input)
}
func (s *Session) Dispute(ctx context.Context, input map[string]any) (ToolEnvelope, error) {
	return s.call(ctx, "dispute", input)
}
func (s *Session) Forget(ctx context.Context, input map[string]any) (ToolEnvelope, error) {
	return s.call(ctx, "forget", input)
}
func (s *Session) Inspect(ctx context.Context, input map[string]any) (ToolEnvelope, error) {
	return s.call(ctx, "inspect", input)
}
