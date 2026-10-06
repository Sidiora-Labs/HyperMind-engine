package cortexclient

import (
	"bytes"
	"context"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

type contextDaemon struct {
	command *exec.Cmd
	log     *os.File
	config  string
	binding string
	socket  string
	binary  string
}

func (d *contextDaemon) start(t *testing.T) {
	t.Helper()
	log, err := os.OpenFile(d.config+".log", os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0600)
	if err != nil {
		t.Fatal(err)
	}
	d.log = log
	d.command = exec.Command(d.binary, "--json", "serve", "--config", d.config, "--context-scope", d.binding)
	d.command.Stdout = log
	d.command.Stderr = log
	if err = d.command.Start(); err != nil {
		t.Fatal(err)
	}
	deadline := time.Now().Add(15 * time.Second)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("unix", d.socket, 100*time.Millisecond)
		if err == nil {
			conn.Close()
			return
		}
		time.Sleep(20 * time.Millisecond)
	}
	data, _ := os.ReadFile(d.config + ".log")
	t.Fatalf("actual daemon readiness failed: %s", data)
}
func (d *contextDaemon) stop() {
	if d.command != nil && d.command.Process != nil {
		_ = d.command.Process.Kill()
		_ = d.command.Wait()
		d.command = nil
	}
	if d.log != nil {
		_ = d.log.Close()
		d.log = nil
	}
}
func TestContextJourney(t *testing.T) {
	t.Run("canonical_contract", func(t *testing.T) {
		occurred := Nanoseconds(1791287999123456789)
		source := ContextSource{ID: "m1", Ordinal: 0, Role: "user", Parts: []ContextPart{{Kind: "text", Text: "hello λ"}}, OccurredAtNS: &occurred, RecordedAtNS: 1791288000123456789, Authority: "user_asserted"}
		frozen, err := source.Freeze()
		if err != nil {
			t.Fatal(err)
		}
		if frozen.SourceDigest != "78af0610090d418ca2335976c873cfcc7647f9ce431f98696040d8fa1660500c" {
			t.Fatalf("cross-language digest mismatch: %s", frozen.SourceDigest)
		}
		for _, bad := range []string{`"01"`, `"-0"`, `"+1"`, `"9223372036854775808"`, `1791288000123456789`, `1.0`, `1e3`} {
			var n Nanoseconds
			if json.Unmarshal([]byte(bad), &n) == nil {
				t.Fatalf("accepted invalid ns %s", bad)
			}
		}
		for _, good := range []string{`"-9223372036854775808"`, `"9223372036854775807"`, `"1791288000123456790"`, `42`} {
			var n Nanoseconds
			if err := json.Unmarshal([]byte(good), &n); err != nil {
				t.Fatal(err)
			}
		}
		encoded, err := EncodeContextIdentifier("session/a?b#c%'")
		if err != nil || encoded != "session%2Fa%3Fb%23c%25%27" {
			t.Fatalf("opaque URI: %s %v", encoded, err)
		}
	})
	binary := os.Getenv("HM_DAEMON_BIN")
	if binary == "" {
		binary = filepath.Join("..", "..", "target", "debug", "hm")
	}
	if _, err := os.Stat(binary); err != nil {
		t.Fatalf("compiled HyperMind daemon required; set HM_DAEMON_BIN: %v", err)
	}
	directory := t.TempDir()
	data := filepath.Join(directory, "data")
	if err := os.Mkdir(data, 0700); err != nil {
		t.Fatal(err)
	}
	var token [32]byte
	var connection [16]byte
	if _, err := rand.Read(token[:]); err != nil {
		t.Fatal(err)
	}
	if _, err := rand.Read(connection[:]); err != nil {
		t.Fatal(err)
	}
	scope := ContextScope{OwnerID: "go-owner", ProjectID: "go-project"}
	config := filepath.Join(directory, "hm.conf")
	binding := filepath.Join(directory, "scope.json")
	socket := filepath.Join(directory, "hm.sock")
	conf := fmt.Sprintf("socket=%s\ndata=%s\nuser=%s\nkek=%s\nadmin_token=%s\nactor=7:%s\nprojection_map_bytes=%d\n", socket, data, strings.Repeat("19", 16), strings.Repeat("2a", 32), strings.Repeat("3b", 32), hex.EncodeToString(token[:]), 16*1024*1024)
	if err := os.WriteFile(config, []byte(conf), 0600); err != nil {
		t.Fatal(err)
	}
	scopeConfig, _ := contextJSON(map[string]any{"version": 1, "actor": 7, "scope": scope})
	if err := os.WriteFile(binding, scopeConfig, 0600); err != nil {
		t.Fatal(err)
	}
	daemon := &contextDaemon{config: config, binding: binding, socket: socket, binary: binary}
	daemon.start(t)
	defer daemon.stop()
	ctx, cancel := context.WithTimeout(context.Background(), 45*time.Second)
	defer cancel()
	dial := func() *Client {
		client, err := Dial(Config{SocketPath: socket, CapabilityToken: token, ConnectionID: connection, DialTimeout: 3 * time.Second, RequestTimeout: 10 * time.Second})
		if err != nil {
			t.Fatal(err)
		}
		return client
	}
	client := dial()
	defer func() { client.Close() }()
	cfg := ContextConfig{Scope: scope, SessionID: "session/a?b#c%'", Conversation: "go-conversation", Actor: 7, ContextOwner: "hypermind"}
	consumer, err := NewContextClient(client, cfg)
	if err != nil {
		t.Fatal(err)
	}
	budget := ContextBudget{ContextTokens: 4096, ReservedOutputTokens: 512, RequiredTokens: 8}
	diagnostics, err := consumer.Activate(ctx, budget, ContextActivation{Query: "begin"})
	if err != nil {
		t.Fatalf("activate: %v", err)
	}
	if diagnostics.Version != 1 || consumer.Version() != 1 {
		t.Fatal("version negotiation missing")
	}
	source := ContextSource{ID: "source/a?%#", Ordinal: 1, Role: "user", Parts: []ContextPart{{Kind: "text", Text: "evidence λ <&>\u2028\u2029 literal \\u2028"}}, RecordedAtNS: 1791288000123456789, Authority: "user_asserted"}
	original := []byte("  exact host bytes\r\nλ\x00  ")
	receipt, err := consumer.IngestSource(ctx, source, original)
	if err != nil {
		t.Fatalf("ingest: %v", err)
	}
	if receipt.SourceSpan == nil {
		t.Fatal("missing recovery reference")
	}
	replay, err := consumer.IngestSource(ctx, source, original)
	if err != nil || !replay.Replayed || replay.LastLSN != receipt.LastLSN {
		t.Fatalf("idempotent source: %+v %v", replay, err)
	}
	recovered, err := consumer.Recover(ctx, source.ID)
	if err != nil || !bytes.Equal(recovered.OriginalBytes, original) {
		t.Fatalf("exact recovery: %+v %v", recovered, err)
	}
	if recovered.Source.RecordedAtNS != source.RecordedAtNS {
		t.Fatal("nanoseconds truncated")
	}
	replacement := source
	replacement.ID = "replacement"
	replacement.Ordinal = 2
	replacement.Parts = []ContextPart{{Kind: "text", Text: "corrected evidence"}}
	replacement.RecordedAtNS = 1791288000123456790
	if _, err = consumer.IngestSource(ctx, replacement, []byte("corrected original")); err != nil {
		t.Fatal(err)
	}
	if _, err = consumer.Relate(ctx, ContextRelation{Kind: "edit", ID: "edit-1", OriginalID: source.ID, ReplacementID: replacement.ID}); err != nil {
		t.Fatal(err)
	}
	if _, err = consumer.Activate(ctx, budget, ContextActivation{Query: "corrected"}); err != nil {
		t.Fatal(err)
	}
	history, err := consumer.History(ctx)
	if err != nil || len(history.Relations) != 1 {
		t.Fatalf("edit history: %+v %v", history, err)
	}
	if _, err = consumer.Fork(ctx, "child/session", "child-conversation"); err != nil {
		t.Fatal(err)
	}
	childConfig := cfg
	childConfig.SessionID = "child/session"
	childConfig.Conversation = "child-conversation"
	child, err := NewContextClient(client, childConfig)
	if err != nil {
		t.Fatal(err)
	}
	if _, err = child.Activate(ctx, budget, ContextActivation{}); err != nil {
		t.Fatal(err)
	}
	if _, err = child.Recover(ctx, source.ID); err != nil {
		t.Fatal(err)
	}
	jobs, err := consumer.Job(ctx, "inspect-jobs", ContextInspectJobs{})
	if err != nil || len(jobs) == 0 {
		t.Fatalf("typed jobs: %s %v", jobs, err)
	}
	note := ContextNote{ID: "go-note", Kind: "Note", Revision: 1, Text: "retained fact", Parents: []string{}, Contradictions: []string{}, Predicate: json.RawMessage(`"True"`)}
	if _, err = consumer.Job(ctx, "create-note", ContextNotesAction{Command: CreateContextNote{Note: note}}); err != nil {
		t.Fatalf("typed knowledge: %v", err)
	}
	invalid := map[string]any{"conversation": cfg.Conversation, "query": "", "turn_text": "", "budget_tokens": 3000, "context": map[string]any{"version": 99, "scope": scope, "session_id": cfg.SessionID, "budget": budget, "generation": 0}}
	if envelope, err := client.CallTool(ctx, "activate", invalid); err != nil || envelope["ok"] != false {
		t.Fatalf("expected explicit server version rejection: %v %v", envelope, err)
	} else {
		values := envelope["items"].([]any)
		item := values[0].(map[string]any)
		if item["error"] != "kInvalidArgument" {
			t.Fatalf("unexpected version rejection: %v", item)
		}
	}
	foreign := cfg
	foreign.Scope.OwnerID = "foreign"
	foreignClient, err := NewContextClient(client, foreign)
	if err != nil {
		t.Fatal(err)
	}
	if _, err = foreignClient.Activate(ctx, budget, ContextActivation{}); err == nil {
		t.Fatal("server accepted foreign scope")
	} else {
		var refusal *ContextClientError
		if !errors.As(err, &refusal) || refusal.Code != "kInvalidArgument" || refusal.EffectState != "unknown" {
			t.Fatalf("expected explicit server scope refusal: %v", err)
		}
	}
	cursor := consumer.Cursor()
	_ = client.Close()
	daemon.stop()
	daemon.start(t)
	client = dial()
	consumer, err = NewContextClient(client, cfg)
	if err != nil {
		t.Fatal(err)
	}
	diagnostics, err = consumer.Activate(ctx, budget, ContextActivation{Query: "resume"})
	if err != nil {
		t.Fatalf("restart context: %v", err)
	}
	if diagnostics.Report.Cursor.before(cursor) {
		t.Fatal("restart lost cursor")
	}
	recovered, err = consumer.Recover(ctx, source.ID)
	if err != nil || !bytes.Equal(recovered.OriginalBytes, original) {
		t.Fatalf("restart exact original recovery: %v", err)
	}
	history, err = consumer.History(ctx)
	if err != nil || len(history.Relations) != 1 {
		t.Fatalf("restart edits: %v", err)
	}
	if _, err = consumer.Job(ctx, "inspect-after-restart", ContextInspectJobs{}); err != nil {
		t.Fatal(err)
	}
}
