package cortexclient

import (
	"context"
	"encoding/hex"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"syscall"
	"testing"
	"time"
)

func TestContextContinuationRealDaemon(t *testing.T) {
	binary := os.Getenv("HM_DAEMON_BIN")
	if binary == "" {
		binary = filepath.Join("..", "..", "target", "debug", "hm")
	}
	if _, err := os.Stat(binary); err != nil {
		t.Fatal(err)
	}
	directory := t.TempDir()
	scope := ContextScope{OwnerID: "continuity-owner", ProjectID: "continuity-project"}
	socket := filepath.Join(directory, "hm.sock")
	config := filepath.Join(directory, "hm.conf")
	binding := filepath.Join(directory, "scope.json")
	var token [32]byte
	for i := range token {
		token[i] = 0x44
	}
	conf := fmt.Sprintf("socket=%s\ndata=%s\nuser=%s\nkek=%s\nadmin_token=%s\nactor=7:%s\nprojection_map_bytes=67108864\n", socket, filepath.Join(directory, "data"), strings.Repeat("11", 16), strings.Repeat("22", 32), strings.Repeat("33", 32), hex.EncodeToString(token[:]))
	if err := os.WriteFile(config, []byte(conf), 0600); err != nil {
		t.Fatal(err)
	}
	encoded, _ := contextJSON(map[string]any{"version": 1, "actor": 7, "scope": scope})
	if err := os.WriteFile(binding, encoded, 0600); err != nil {
		t.Fatal(err)
	}
	daemon := &contextDaemon{config: config, binding: binding, socket: socket, binary: binary}
	daemon.start(t)
	defer daemon.stop()
	cfg := ContextConfig{Scope: scope, SessionID: "parent/%?", Conversation: "parent", Actor: 7, ContextOwner: "hypermind"}
	dial := func(token [32]byte) (*ContextClient, error) {
		client, err := Dial(Config{SocketPath: socket, CapabilityToken: token, DialTimeout: time.Second, RequestTimeout: 5 * time.Second})
		if err != nil {
			return nil, err
		}
		consumer, err := NewContextClient(client, cfg)
		if err != nil {
			client.Close()
		}
		return consumer, err
	}
	consumer, err := dial(token)
	if err != nil {
		t.Fatal(err)
	}
	continuation, err := NewContextContinuation(consumer, nil)
	if err != nil {
		t.Fatal(err)
	}
	defer func() { continuation.Close() }()
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	occurred := Nanoseconds(1791287999123456789)
	source := ContextSource{ID: "original", Ordinal: 0, Role: "user", Parts: []ContextPart{{Kind: "text", Text: "retained original"}}, OccurredAtNS: &occurred, RecordedAtNS: 1791288000123456789, Authority: "user_asserted"}
	frozen, err := source.Freeze()
	if err != nil {
		t.Fatal(err)
	}
	if _, err = continuation.IngestSource(ctx, frozen, []byte("retained original")); err != nil {
		t.Fatal(err)
	}
	budget := ContextBudget{ContextTokens: 8192, ReservedOutputTokens: 512}
	profile := &ContinuationProfile{User: true, Assistant: true, Tool: true, Text: true, ToolCalls: true, ToolResults: true, Opaque: true}
	if _, err = continuation.Activate(ctx, budget, ContinuationActivation{ContextActivation: ContextActivation{Query: "retained", ModelID: "gpt-4o"}, Profile: profile}); err != nil {
		t.Fatal(err)
	}
	accepted := continuation.Checkpoint().Accepted
	if _, err = continuation.Fork(ctx, "child/%?", "child"); err != nil {
		t.Fatal(err)
	}
	if err = continuation.Close(); err != nil {
		t.Fatal(err)
	}
	daemon.stop()
	daemon.start(t)
	var wrong [32]byte
	wrong[0] = 0xff
	if _, err = continuation.Reconnect(ctx, func(context.Context) (*ContextClient, error) { return dial(wrong) }); err == nil {
		t.Fatal("reconnect accepted wrong actor credential")
	}
	if continuation.Checkpoint().Accepted.Generation != accepted.Generation {
		t.Fatal("failed reauthentication changed accepted state")
	}
	if _, err = continuation.Reconnect(ctx, func(context.Context) (*ContextClient, error) { return dial(token) }); err != nil {
		t.Fatal(err)
	}
	current := continuation.Checkpoint()
	if current.Accepted.Cursor != accepted.Cursor || current.Accepted.Generation != accepted.Generation || continuation.GenerationCompatible() {
		t.Fatal("accepted fence not preserved")
	}
	childConfig := cfg
	childConfig.SessionID = "child/%?"
	childConfig.Conversation = "child"
	child, err := NewContextClient(continuation.consumer.client, childConfig)
	if err != nil {
		t.Fatal(err)
	}
	if _, err = child.Activate(ctx, budget, ContextActivation{Query: "retained"}); err != nil {
		t.Fatal(err)
	}
	recovery, err := child.Recover(ctx, "original")
	if err != nil {
		t.Fatal(err)
	}
	if recovery.Source.SourceDigest != frozen.SourceDigest || recovery.Source.RecordedAtNS != frozen.RecordedAtNS || string(recovery.OriginalBytes) != "retained original" {
		t.Fatal("fork lost source identity or exact time/bytes")
	}
	if _, err = continuation.Activate(ctx, budget, ContinuationActivation{ContextActivation: ContextActivation{Query: "retained", ModelID: "gpt-4o-mini"}, Profile: profile}); err != nil {
		t.Fatal(err)
	}
	if continuation.Checkpoint().Accepted.Generation <= accepted.Generation {
		t.Fatal("model generation not invalidated")
	}
	if err = daemon.command.Process.Signal(syscall.SIGSTOP); err != nil {
		t.Fatal(err)
	}
	stopped := true
	defer func() {
		if stopped {
			daemon.command.Process.Signal(syscall.SIGCONT)
		}
	}()
	next := source
	next.ID = "cancelled"
	next.Ordinal = 1
	next.RecordedAtNS = 1791288000123456790
	mutationCtx, mutationCancel := context.WithTimeout(ctx, 100*time.Millisecond)
	defer mutationCancel()
	_, err = continuation.IngestSource(mutationCtx, next, []byte("retained original"))
	var uncertain *ContextClientError
	if !errors.As(err, &uncertain) || uncertain.EffectState != "unknown" {
		t.Fatalf("actual blocked transport cancellation must remain unknown: %v", err)
	}
	if _, err = continuation.IngestSource(ctx, next, []byte("retained original")); !errors.As(err, &uncertain) || uncertain.EffectState != "not_dispatched" {
		t.Fatal("possibly accepted source replay was permitted")
	}
	if err = daemon.command.Process.Signal(syscall.SIGCONT); err != nil {
		t.Fatal(err)
	}
	stopped = false
	if _, err = continuation.Inspect(ctx); err != nil {
		t.Fatal(err)
	}
	if continuation.Checkpoint().Uncertain == nil {
		t.Fatal("inspect cleared unknown")
	}
	if _, err = continuation.AbandonUncertain(ctx); err != nil {
		t.Fatal(err)
	}
	checkpoint := continuation.Checkpoint()
	checkpoint.Accepted.ModelID = "external mutation"
	if continuation.Checkpoint().Accepted.ModelID == "external mutation" {
		t.Fatal("checkpoint leaked mutable internal state")
	}
	checkpoint = continuation.Checkpoint()
	continuation, err = NewContextContinuation(continuation.consumer, &checkpoint)
	if err != nil {
		t.Fatal(err)
	}
	if len(continuation.Checkpoint().Unresolved) != 1 {
		t.Fatal("unknown outcome lost across caller checkpoint")
	}
	if _, err = continuation.IngestSource(ctx, next, []byte("retained original")); !errors.As(err, &uncertain) || uncertain.EffectState != "not_dispatched" {
		t.Fatal("unresolved identity replay was permitted")
	}
}
