package cortexclient

import (
	"bytes"
	"context"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestContextMemoryPortabilityJourney(t *testing.T) {
	binary := os.Getenv("HM_DAEMON_BIN")
	if binary == "" {
		binary = filepath.Join("..", "..", "target", "debug", "hm")
	}
	if _, err := os.Stat(binary); err != nil {
		t.Fatalf("compiled HyperMind daemon required: %v", err)
	}
	var token [32]byte
	var connection [16]byte
	if _, err := rand.Read(token[:]); err != nil {
		t.Fatal(err)
	}
	if _, err := rand.Read(connection[:]); err != nil {
		t.Fatal(err)
	}
	scope := ContextScope{OwnerID: "portability-owner", ProjectID: "portability-project"}
	cfg := ContextConfig{Scope: scope, SessionID: "native-portability", Conversation: "native-portability-conversation", Actor: 7, ContextOwner: "hypermind"}
	start := func() (*contextDaemon, *Client, *ContextClient) {
		t.Helper()
		root := t.TempDir()
		data := filepath.Join(root, "data")
		if err := os.Mkdir(data, 0700); err != nil {
			t.Fatal(err)
		}
		socket := filepath.Join(root, "hm.sock")
		config := filepath.Join(root, "hm.conf")
		binding := filepath.Join(root, "scope.json")
		content := fmt.Sprintf("socket=%s\ndata=%s\nuser=%s\nkek=%s\nadmin_token=%s\nactor=7:%s\nprojection_map_bytes=%d\n", socket, data, strings.Repeat("45", 16), strings.Repeat("56", 32), strings.Repeat("67", 32), hex.EncodeToString(token[:]), 16*1024*1024)
		if err := os.WriteFile(config, []byte(content), 0600); err != nil {
			t.Fatal(err)
		}
		bindingBytes, _ := contextJSON(map[string]any{"version": 1, "actor": 7, "scope": scope})
		if err := os.WriteFile(binding, bindingBytes, 0600); err != nil {
			t.Fatal(err)
		}
		daemon := &contextDaemon{binary: binary, config: config, binding: binding, socket: socket}
		t.Cleanup(daemon.stop)
		daemon.start(t)
		client, err := Dial(Config{SocketPath: socket, CapabilityToken: token, ConnectionID: connection, DialTimeout: 3 * time.Second, RequestTimeout: 10 * time.Second})
		if err != nil {
			t.Fatal(err)
		}
		t.Cleanup(func() { client.Close() })
		consumer, err := NewContextClient(client, cfg)
		if err != nil {
			t.Fatal(err)
		}
		return daemon, client, consumer
	}
	ctx, cancel := context.WithTimeout(context.Background(), 45*time.Second)
	defer cancel()
	daemon, client, consumer := start()
	record := map[string]any{"id": "portable-note", "kind": "note", "category": "general", "status": "active", "revision": 1, "revision_digest": "", "content": "original retained λ <&> evidence", "authority": "user_asserted", "confidence": 1000000, "importance": 500000, "occurred_at_ns": nil, "recorded_at_ns": Nanoseconds(1791288000123456789), "expires_at_ns": nil, "pinned": true, "provenance": []any{}, "lineage": []any{}, "contradictions": []string{}, "last_lsn": 0, "predicate": nil, "smart_condition": nil, "retention_until_ns": nil, "metadata": map[string]any{"exact_counter": json.Number("18446744073709551615"), "floating": json.Number("1.0"), "fraction": json.Number("1.2345678901234567"), "unknown_time": nil}}
	mutate := func(requestID string, command any) {
		t.Helper()
		var receipt ContextMemoryReceipt
		request := map[string]any{"version": 1, "scope": scope, "request_id": requestID, "command": command}
		if err := consumer.remember(ctx, cfg.Conversation, "memory", request, &receipt); err != nil {
			t.Fatal(err)
		}
	}
	mutate("native-create", map[string]any{"kind": "create", "record": record})
	record["revision"] = 2
	record["content"] = "revised retained evidence"
	record["recorded_at_ns"] = Nanoseconds(1791288000123456790)
	mutate("native-revise", map[string]any{"kind": "revise", "record": record, "expected_revision": 1})
	artifact, err := consumer.ExportMemory(ctx)
	if err != nil {
		t.Fatalf("native export: %v", err)
	}
	export, err := artifact.Validate(scope)
	if err != nil || export.Cursor != 2 || len(export.Events) != 2 {
		t.Fatalf("native manifest: %+v %v", export, err)
	}
	if artifact.ByteCount != uint64(len(artifact.Bytes)) || artifact.ArtifactDigest == "" || artifact.ExportDigest == "" || artifact.RestoreMaxBytes != 524288 {
		t.Fatal("missing artifact identity")
	}
	var revision struct {
		Command struct {
			Record struct {
				RecordedAtNS Nanoseconds  `json:"recorded_at_ns"`
				OccurredAtNS *Nanoseconds `json:"occurred_at_ns"`
				Content      string       `json:"content"`
			} `json:"record"`
		} `json:"command"`
	}
	if err := json.Unmarshal(export.Events[1], &revision); err != nil {
		t.Fatal(err)
	}
	if revision.Command.Record.RecordedAtNS != 1791288000123456790 || revision.Command.Record.OccurredAtNS != nil {
		t.Fatal("native timestamp or unknown time lost")
	}
	// Verify the exact artifact remains stable after reopening the originating ledger.
	client.Close()
	daemon.stop()
	daemon.start(t)
	client, err = Dial(Config{SocketPath: daemon.socket, CapabilityToken: token, ConnectionID: connection, DialTimeout: 3 * time.Second, RequestTimeout: 10 * time.Second})
	if err != nil {
		t.Fatal(err)
	}
	defer client.Close()
	consumer, err = NewContextClient(client, cfg)
	if err != nil {
		t.Fatal(err)
	}
	reopened, err := consumer.ExportMemory(ctx)
	if err != nil || !bytes.Equal(reopened.Bytes, artifact.Bytes) || reopened.ArtifactDigest != artifact.ArtifactDigest {
		t.Fatalf("reopened canonical artifact: %v", err)
	}
	restoreDaemon, restoreClient, restorer := start()
	tampered := artifact
	tampered.Bytes = append(ContextBytes(nil), artifact.Bytes...)
	tampered.Bytes[len(tampered.Bytes)/2] ^= 1
	if _, err := restorer.RestoreMemory(ctx, "reject-tampered", tampered); err == nil {
		t.Fatal("accepted changed artifact bytes")
	} else {
		var failure *ContextClientError
		if !errors.As(err, &failure) || failure.EffectState != "not_dispatched" {
			t.Fatalf("local integrity refusal state: %v", err)
		}
	}
	oversized := artifact
	oversized.ByteCount = artifact.RestoreMaxBytes + 1
	if _, err := restorer.RestoreMemory(ctx, "reject-restore-capacity", oversized); err == nil {
		t.Fatal("accepted artifact beyond negotiated restore cap")
	} else {
		var failure *ContextClientError
		if !errors.As(err, &failure) || failure.Code != "memory_restore_capacity" || failure.EffectState != "not_dispatched" {
			t.Fatalf("restore cap refusal: %v", err)
		}
	}
	wrongSize := artifact
	wrongSize.ByteCount++
	if _, err := restorer.RestoreMemory(ctx, "reject-size", wrongSize); err == nil {
		t.Fatal("accepted wrong artifact length")
	}
	foreign := artifact
	foreign.Scope.OwnerID = "foreign"
	if _, err := restorer.RestoreMemory(ctx, "reject-foreign", foreign); err == nil {
		t.Fatal("accepted foreign artifact")
	}
	var forgedReceipt ContextMemoryReceipt
	forgedDigest := strings.Repeat("0", 64)
	request := map[string]any{"version": 1, "scope": scope, "request_id": "reject-server-tamper", "command": map[string]any{"kind": "restore_jsonl", "jsonl": artifact.Bytes, "artifact_digest": forgedDigest}}
	if err := restorer.remember(ctx, cfg.Conversation, "memory", request, &forgedReceipt); err == nil {
		t.Fatal("actual server accepted forged export digest")
	} else {
		var failure *ContextClientError
		if !errors.As(err, &failure) || failure.Code == "operation_failed" {
			t.Fatalf("expected explicit server tamper refusal: %v", err)
		}
	}
	before, err := restorer.ExportMemory(ctx)
	if err != nil || before.Cursor != 0 {
		t.Fatalf("tamper refusal mutated empty ledger: %v", err)
	}
	receipt, err := restorer.RestoreMemory(ctx, "native-restore", artifact)
	if err != nil || receipt.Cursor != 1 {
		t.Fatalf("native restore: %+v %v", receipt, err)
	}
	replay, err := restorer.RestoreMemory(ctx, "native-restore", artifact)
	if err != nil || !replay.Replayed || replay.LastLSN != receipt.LastLSN {
		t.Fatalf("restore idempotency: %+v %v", replay, err)
	}
	var restored struct {
		Record struct {
			ID           string          `json:"id"`
			Revision     uint64          `json:"revision"`
			Content      string          `json:"content"`
			OccurredAtNS *Nanoseconds    `json:"occurred_at_ns"`
			RecordedAtNS Nanoseconds     `json:"recorded_at_ns"`
			Metadata     json.RawMessage `json:"metadata"`
		} `json:"record"`
	}
	if err := restorer.invoke(ctx, "inspect", map[string]any{"uri": "hm://7/context-memory/portable-note"}, &restored); err != nil {
		t.Fatal(err)
	}
	if restored.Record.Revision != 2 || restored.Record.Content != "revised retained evidence" || restored.Record.RecordedAtNS != 1791288000123456790 || restored.Record.OccurredAtNS != nil {
		t.Fatalf("restored revision fidelity: %+v", restored)
	}
	restoredArtifact, err := restorer.ExportMemory(ctx)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := restoredArtifact.Validate(scope); err != nil {
		t.Fatal(err)
	}
	if !bytes.Contains(restored.Record.Metadata, []byte(`18446744073709551615`)) {
		t.Fatal("restore changed exact native metadata integer")
	}
	if !bytes.Contains(restored.Record.Metadata, []byte(`"floating":1.0`)) || !bytes.Contains(restored.Record.Metadata, []byte(`"fraction":1.2345678901234567`)) {
		t.Fatalf("native metadata numeric representation changed: %s", restored.Record.Metadata)
	}
	restoreClient.Close()
	restoreDaemon.stop()
	restoreDaemon.start(t)
	destination, err := Dial(Config{SocketPath: restoreDaemon.socket, CapabilityToken: token, ConnectionID: connection, DialTimeout: 3 * time.Second, RequestTimeout: 10 * time.Second})
	if err != nil {
		t.Fatal(err)
	}
	defer destination.Close()
	restorer, err = NewContextClient(destination, cfg)
	if err != nil {
		t.Fatal(err)
	}
	afterRestart, err := restorer.ExportMemory(ctx)
	if err != nil || !bytes.Equal(afterRestart.Bytes, restoredArtifact.Bytes) {
		t.Fatalf("restored ledger reopen artifact: %v", err)
	}
}
