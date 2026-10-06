package cortexclient

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"sync"
)

type ContinuationProfile struct {
	User        bool `json:"user"`
	Assistant   bool `json:"assistant"`
	Tool        bool `json:"tool"`
	Text        bool `json:"text"`
	ToolCalls   bool `json:"tool_calls"`
	ToolResults bool `json:"tool_results"`
	Opaque      bool `json:"opaque"`
}
type ContinuationActivation struct {
	ContextActivation
	Profile *ContinuationProfile
}
type ContinuationState struct {
	Cursor     ContextCursor
	Generation uint64
	ModelID    string
	Profile    *ContinuationProfile
}
type MutationOutcome struct {
	Operation   string
	EffectState string
	Identity    string
	Detail      string
}
type ContinuationCheckpoint struct {
	Config     ContextConfig
	Accepted   *ContinuationState
	Uncertain  *MutationOutcome
	Unresolved []MutationOutcome
}
type ContextContinuation struct {
	mu         sync.Mutex
	consumer   *ContextClient
	accepted   *ContinuationState
	uncertain  *MutationOutcome
	unresolved []MutationOutcome
	compatible bool
}

func copyContinuationState(s *ContinuationState) *ContinuationState {
	if s == nil {
		return nil
	}
	v := *s
	if s.Profile != nil {
		p := *s.Profile
		v.Profile = &p
	}
	return &v
}
func copyContinuationConfig(c ContextConfig) ContextConfig {
	if c.Scope.WorkspaceID != nil {
		w := *c.Scope.WorkspaceID
		c.Scope.WorkspaceID = &w
	}
	return c
}
func sameContinuationConfig(a, b ContextConfig) bool {
	return a.Scope.equal(b.Scope) && a.Actor == b.Actor && a.SessionID == b.SessionID && a.Conversation == b.Conversation && a.ContextOwner == b.ContextOwner
}
func NewContextContinuation(consumer *ContextClient, checkpoint *ContinuationCheckpoint) (*ContextContinuation, error) {
	if consumer == nil {
		return nil, errors.New("context consumer required")
	}
	c := &ContextContinuation{consumer: consumer}
	if checkpoint != nil {
		if !sameContinuationConfig(checkpoint.Config, consumer.config) {
			return nil, &ContextClientError{Code: "checkpoint_identity_mismatch", EffectState: "not_dispatched"}
		}
		if s := checkpoint.Accepted; s != nil {
			if s.Generation > contextSafeInteger || s.Cursor.validate() != nil {
				return nil, errors.New("invalid checkpoint state")
			}
		}
		c.accepted = copyContinuationState(checkpoint.Accepted)
		if checkpoint.Uncertain != nil {
			u := *checkpoint.Uncertain
			c.uncertain = &u
		}
		c.unresolved = append([]MutationOutcome(nil), checkpoint.Unresolved...)
	}
	return c, nil
}
func (c *ContextContinuation) Checkpoint() ContinuationCheckpoint {
	c.mu.Lock()
	defer c.mu.Unlock()
	p := ContinuationCheckpoint{Config: copyContinuationConfig(c.consumer.config), Accepted: copyContinuationState(c.accepted), Unresolved: append([]MutationOutcome(nil), c.unresolved...)}
	if c.uncertain != nil {
		v := *c.uncertain
		p.Uncertain = &v
	}
	return p
}
func (c *ContextContinuation) GenerationCompatible() bool {
	c.mu.Lock()
	defer c.mu.Unlock()
	return c.compatible
}
func (c *ContextContinuation) accept(d ContextDiagnostics, model string, profile *ContinuationProfile) error {
	if d.Report.Generation > contextSafeInteger || d.Report.Cursor.validate() != nil {
		return &ContextClientError{Code: "invalid_generation", EffectState: "unknown"}
	}
	if c.accepted != nil && d.Report.Cursor.before(c.accepted.Cursor) {
		return &ContextClientError{Code: "stale_cursor", EffectState: "rejected"}
	}
	c.accepted = copyContinuationState(&ContinuationState{Cursor: d.Report.Cursor, Generation: d.Report.Generation, ModelID: model, Profile: profile})
	c.compatible = true
	return nil
}
func continuationEffect(err error) string {
	var native *ContextClientError
	if errors.As(err, &native) && (native.EffectState == "not_dispatched" || native.EffectState == "rejected") {
		return native.EffectState
	}
	var transport *TransportError
	if errors.As(err, &transport) && (transport.EffectState == "not_dispatched" || transport.EffectState == "rejected") {
		return transport.EffectState
	}
	var engine *EngineError
	if errors.As(err, &engine) && (engine.EffectState == "not_dispatched" || engine.EffectState == "rejected") {
		return engine.EffectState
	}
	return "unknown"
}

type continuationResult[T any] struct {
	value T
	err   error
}

func continuationMutation[T any](c *ContextContinuation, ctx context.Context, operation, identity string, invoke func(context.Context) (T, error)) (T, error) {
	var zero T
	if c.uncertain != nil {
		return zero, &ContextClientError{Code: "uncertain_mutation_requires_reconciliation", EffectState: "not_dispatched"}
	}
	for _, u := range c.unresolved {
		if u.Operation == operation && u.Identity == identity {
			return zero, &ContextClientError{Code: "unresolved_mutation_identity", EffectState: "not_dispatched"}
		}
	}
	if err := ctx.Err(); err != nil {
		return zero, &ContextClientError{Code: "cancelled_before_dispatch", EffectState: "not_dispatched", Cause: err}
	}
	done := make(chan continuationResult[T], 1)
	go func() { v, err := invoke(context.WithoutCancel(ctx)); done <- continuationResult[T]{v, err} }()
	var result continuationResult[T]
	select {
	case result = <-done:
	case <-ctx.Done():
		result.err = &ContextClientError{Code: "cancelled_after_dispatch", EffectState: "unknown", Cause: ctx.Err()}
	}
	if result.err != nil {
		state := continuationEffect(result.err)
		if state == "unknown" {
			c.uncertain = &MutationOutcome{Operation: operation, EffectState: state, Identity: identity, Detail: fmt.Sprintf("%T", result.err)}
		}
		return zero, &ContextClientError{Code: operation + "_failed", EffectState: state, Cause: result.err}
	}
	return result.value, nil
}
func (c *ContextContinuation) Activate(ctx context.Context, budget ContextBudget, options ContinuationActivation) (ContextDiagnostics, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	var zero ContextDiagnostics
	if _, err := budget.Available(); err != nil {
		return zero, &ContextClientError{Code: "invalid_budget", EffectState: "not_dispatched", Cause: err}
	}
	model := options.ModelID
	if model == "" {
		model = "gpt-4o"
	}
	sameProfile := c.accepted != nil && ((c.accepted.Profile == nil && options.Profile == nil) || (c.accepted.Profile != nil && options.Profile != nil && *c.accepted.Profile == *options.Profile))
	if c.accepted != nil && (model != c.accepted.ModelID || !sameProfile) {
		c.compatible = false
	}
	options.Generation = 0
	if c.accepted != nil && c.compatible {
		options.Generation = c.accepted.Generation
	}
	consumer := c.consumer
	diagnostics, err := continuationMutation(c, ctx, "activate", "context-generation", func(call context.Context) (ContextDiagnostics, error) {
		if options.Profile == nil {
			return consumer.Activate(call, budget, options.ContextActivation)
		}
		consumer.mu.Lock()
		defer consumer.mu.Unlock()
		var result ContextDiagnostics
		available, _ := budget.Available()
		request := map[string]any{"version": ContextVersion, "scope": consumer.config.Scope, "session_id": consumer.config.SessionID, "budget": budget, "generation": options.Generation, "model_id": model, "profile": *options.Profile, "defer_reductions": options.DeferReductions}
		if options.RequiredMessageIDs != nil {
			request["required_message_ids"] = options.RequiredMessageIDs
		}
		if options.Tier != "" {
			request["tier"] = options.Tier
		}
		err := consumer.invoke(call, "activate", map[string]any{"conversation": consumer.config.Conversation, "query": options.Query, "turn_text": options.TurnText, "budget_tokens": available, "context": request}, &result)
		if err == nil {
			if result.Report.Version != ContextVersion || result.Version != ContextVersion {
				err = &ContextClientError{Code: "unsupported_version", EffectState: "rejected"}
			} else {
				err = consumer.accept(result.Version, result.Report.Scope, result.Report.SessionID, &result.Report.Cursor)
			}
		}
		return result, err
	})
	if err == nil {
		err = c.accept(diagnostics, model, options.Profile)
	}
	return diagnostics, err
}
func (c *ContextContinuation) Inspect(ctx context.Context) (ContextDiagnostics, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	d, err := c.consumer.Inspect(ctx)
	if err == nil {
		compatible := c.compatible
		model := ""
		var p *ContinuationProfile
		if c.accepted != nil {
			model = c.accepted.ModelID
			p = c.accepted.Profile
		}
		err = c.accept(d, model, p)
		c.compatible = compatible
	}
	return d, err
}
func (c *ContextContinuation) Reconnect(ctx context.Context, factory func(context.Context) (*ContextClient, error)) (ContextDiagnostics, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	var zero ContextDiagnostics
	fresh, err := factory(ctx)
	if err != nil {
		return zero, err
	}
	if fresh == nil || !sameContinuationConfig(fresh.config, c.consumer.config) {
		return zero, &ContextClientError{Code: "reconnect_identity_mismatch", EffectState: "not_dispatched"}
	}
	d, err := fresh.Inspect(ctx)
	if err != nil {
		return zero, err
	}
	model := ""
	var p *ContinuationProfile
	if c.accepted != nil {
		model = c.accepted.ModelID
		p = c.accepted.Profile
	}
	if err = c.accept(d, model, p); err != nil {
		return zero, err
	}
	c.consumer = fresh
	c.compatible = false
	return d, nil
}
func (c *ContextContinuation) Close() error {
	c.mu.Lock()
	defer c.mu.Unlock()
	err := c.consumer.client.Close()
	c.compatible = false
	return err
}
func (c *ContextContinuation) AbandonUncertain(ctx context.Context) (MutationOutcome, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.uncertain == nil {
		return MutationOutcome{}, errors.New("no uncertain mutation")
	}
	d, err := c.consumer.Inspect(ctx)
	if err != nil {
		return MutationOutcome{}, err
	}
	model := ""
	var p *ContinuationProfile
	if c.accepted != nil {
		model = c.accepted.ModelID
		p = c.accepted.Profile
	}
	if err = c.accept(d, model, p); err != nil {
		return MutationOutcome{}, err
	}
	u := *c.uncertain
	c.unresolved = append(c.unresolved, u)
	c.uncertain = nil
	c.compatible = false
	return u, nil
}
func (c *ContextContinuation) ReconcileSource(ctx context.Context, source ContextSource) (bool, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.uncertain == nil || c.uncertain.Operation != "source" || c.uncertain.Identity != source.ID {
		return false, errors.New("uncertain source identity required")
	}
	frozen, err := source.Freeze()
	if err != nil {
		return false, err
	}
	h, err := c.consumer.History(ctx)
	if err != nil {
		return false, err
	}
	found := false
	for _, m := range h.Messages {
		if m.ID == frozen.ID && m.SourceDigest == frozen.SourceDigest {
			found = true
		}
	}
	if !found {
		return false, nil
	}
	d, err := c.consumer.Inspect(ctx)
	if err != nil {
		return false, err
	}
	model := ""
	var p *ContinuationProfile
	if c.accepted != nil {
		model = c.accepted.ModelID
		p = c.accepted.Profile
	}
	if err = c.accept(d, model, p); err != nil {
		return false, err
	}
	c.uncertain = nil
	c.compatible = false
	return true, nil
}
func (c *ContextContinuation) IngestSource(ctx context.Context, source ContextSource, original []byte) (ContextHistoryReceipt, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	frozen, err := source.Freeze()
	if err != nil {
		return ContextHistoryReceipt{}, &ContextClientError{Code: "invalid_source", EffectState: "not_dispatched", Cause: err}
	}
	raw := append([]byte(nil), original...)
	consumer := c.consumer
	return continuationMutation(c, ctx, "source", source.ID, func(call context.Context) (ContextHistoryReceipt, error) {
		return consumer.IngestSource(call, frozen, raw)
	})
}
func (c *ContextContinuation) Relate(ctx context.Context, relation ContextRelation) (ContextHistoryReceipt, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	consumer := c.consumer
	return continuationMutation(c, ctx, "relation", relation.ID, func(call context.Context) (ContextHistoryReceipt, error) { return consumer.Relate(call, relation) })
}
func (c *ContextContinuation) Fork(ctx context.Context, session, conversation string) (ContextHistoryReceipt, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	consumer := c.consumer
	return continuationMutation(c, ctx, "fork", session, func(call context.Context) (ContextHistoryReceipt, error) {
		return consumer.Fork(call, session, conversation)
	})
}
func (c *ContextContinuation) RestoreMemory(ctx context.Context, requestID string, artifact ContextMemoryArtifact) (ContextMemoryReceipt, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	consumer := c.consumer
	artifact.Bytes = append(ContextBytes(nil), artifact.Bytes...)
	return continuationMutation(c, ctx, "memory", requestID, func(call context.Context) (ContextMemoryReceipt, error) {
		return consumer.RestoreMemory(call, requestID, artifact)
	})
}

func (c *ContextContinuation) Job(ctx context.Context, requestID string, action ContextJobAction) (json.RawMessage, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	consumer := c.consumer
	return continuationMutation(c, ctx, "job", requestID, func(call context.Context) (json.RawMessage, error) { return consumer.Job(call, requestID, action) })
}
