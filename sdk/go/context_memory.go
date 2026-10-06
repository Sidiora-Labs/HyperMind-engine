package cortexclient

import (
	"bytes"
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"unicode/utf8"
)

const maximumContextMemoryArtifactBytes = 64 * 1024 * 1024

type ContextMemoryExport struct {
	Version uint32            `json:"version"`
	Scope   ContextScope      `json:"scope"`
	Cursor  uint64            `json:"cursor"`
	Events  []json.RawMessage `json:"events"`
	Digest  string            `json:"digest"`
}
type ContextMemoryArtifact struct {
	Version         uint32       `json:"version"`
	Scope           ContextScope `json:"scope"`
	Cursor          uint64       `json:"cursor"`
	MediaType       string       `json:"media_type"`
	Bytes           ContextBytes `json:"bytes"`
	ByteCount       uint64       `json:"byte_count"`
	ArtifactDigest  string       `json:"artifact_digest"`
	ExportDigest    string       `json:"export_digest"`
	RestoreMaxBytes uint64       `json:"restore_max_bytes"`
}
type ContextMemoryReceipt struct {
	Version  uint32       `json:"version"`
	Scope    ContextScope `json:"scope"`
	Cursor   uint64       `json:"cursor"`
	LastLSN  uint64       `json:"last_lsn"`
	Replayed bool         `json:"replayed"`
}
type contextMemoryManifest struct {
	Kind       string       `json:"kind"`
	Version    uint32       `json:"version"`
	Scope      ContextScope `json:"scope"`
	Cursor     uint64       `json:"cursor"`
	EventCount uint64       `json:"event_count"`
	Digest     string       `json:"digest"`
}

func strictContextMemoryJSON(data []byte, out any) error {
	decoder := json.NewDecoder(bytes.NewReader(data))
	decoder.DisallowUnknownFields()
	decoder.UseNumber()
	if err := decoder.Decode(out); err != nil {
		return err
	}
	var trailing any
	if err := decoder.Decode(&trailing); err != io.EOF {
		return errors.New("trailing memory export content")
	}
	return nil
}
func ParseContextMemoryJSONL(data []byte, scope ContextScope) (ContextMemoryExport, error) {
	var result ContextMemoryExport
	if err := scope.Validate(); err != nil {
		return result, err
	}
	if len(data) == 0 || len(data) > maximumContextMemoryArtifactBytes || !utf8.Valid(data) || data[len(data)-1] != '\n' {
		return result, errors.New("invalid memory export bytes")
	}
	lines := bytes.Split(data[:len(data)-1], []byte{'\n'})
	var header contextMemoryManifest
	if err := strictContextMemoryJSON(lines[0], &header); err != nil {
		return result, err
	}
	if header.Kind != "manifest" || header.Version != ContextVersion || !header.Scope.equal(scope) || header.EventCount != uint64(len(lines)-1) || header.Cursor != header.EventCount {
		return result, errors.New("memory export manifest mismatch")
	}
	result = ContextMemoryExport{Version: header.Version, Scope: header.Scope, Cursor: header.Cursor, Events: make([]json.RawMessage, 0, len(lines)-1), Digest: header.Digest}
	for _, line := range lines[1:] {
		var row struct {
			Kind  string          `json:"kind"`
			Event json.RawMessage `json:"event"`
		}
		if err := strictContextMemoryJSON(line, &row); err != nil {
			return result, err
		}
		if row.Kind != "event" || len(row.Event) == 0 || row.Event[0] != '{' {
			return result, errors.New("invalid native memory event")
		}
		result.Events = append(result.Events, append(json.RawMessage(nil), row.Event...))
	}
	if err := result.Validate(scope); err != nil {
		return result, err
	}
	canonical, err := result.JSONL()
	if err != nil {
		return result, err
	}
	if !bytes.Equal(canonical, data) {
		return result, errors.New("noncanonical memory artifact")
	}
	return result, nil
}
func (e ContextMemoryExport) Validate(scope ContextScope) error {
	if e.Version != ContextVersion || !e.Scope.equal(scope) || e.Cursor != uint64(len(e.Events)) {
		return errors.New("memory export identity mismatch")
	}
	copy := e
	copy.Digest = ""
	digest, err := contextDigest(copy)
	if err != nil {
		return err
	}
	if digest != e.Digest {
		return errors.New("memory export digest mismatch")
	}
	for index, event := range e.Events {
		var fence struct {
			Version   uint32       `json:"version"`
			Scope     ContextScope `json:"scope"`
			Principal ContextScope `json:"principal"`
			Cursor    uint64       `json:"cursor"`
		}
		if err := json.Unmarshal(event, &fence); err != nil {
			return err
		}
		if fence.Version != ContextVersion || !fence.Scope.equal(scope) || !fence.Principal.equal(scope) || fence.Cursor != uint64(index)+1 {
			return errors.New("memory event scope or cursor mismatch")
		}
	}
	return nil
}
func (e ContextMemoryExport) JSONL() ([]byte, error) {
	if err := e.Validate(e.Scope); err != nil {
		return nil, err
	}
	manifest := map[string]any{"kind": "manifest", "version": e.Version, "scope": e.Scope, "cursor": e.Cursor, "event_count": len(e.Events), "digest": e.Digest}
	header, err := contextJSON(manifest)
	if err != nil {
		return nil, err
	}
	var output bytes.Buffer
	output.Write(header)
	output.WriteByte('\n')
	for _, event := range e.Events {
		row, err := contextJSON(map[string]any{"kind": "event", "event": event})
		if err != nil {
			return nil, err
		}
		output.Write(row)
		output.WriteByte('\n')
		if output.Len() > maximumContextMemoryArtifactBytes {
			return nil, errors.New("memory artifact capacity exceeded")
		}
	}
	return output.Bytes(), nil
}
func (a ContextMemoryArtifact) Validate(scope ContextScope) (ContextMemoryExport, error) {
	var result ContextMemoryExport
	if a.Version != ContextVersion || !a.Scope.equal(scope) || a.MediaType != "application/x-ndjson" || a.RestoreMaxBytes == 0 || a.RestoreMaxBytes > maximumContextMemoryArtifactBytes || a.ByteCount != uint64(len(a.Bytes)) || len(a.Bytes) > maximumContextMemoryArtifactBytes {
		return result, errors.New("memory artifact metadata mismatch")
	}
	digest := sha256.Sum256(a.Bytes)
	if a.ArtifactDigest != hex.EncodeToString(digest[:]) {
		return result, errors.New("memory artifact digest mismatch")
	}
	result, err := ParseContextMemoryJSONL(a.Bytes, scope)
	if err != nil {
		return result, err
	}
	if result.Digest != a.ExportDigest || result.Cursor != a.Cursor {
		return result, errors.New("memory artifact manifest mismatch")
	}
	return result, nil
}
func (c *ContextClient) MemoryExportURI() string {
	return fmt.Sprintf("hm://%d/context-memory-export", c.config.Actor)
}
func (c *ContextClient) ExportMemory(ctx context.Context) (ContextMemoryArtifact, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	var result ContextMemoryArtifact
	if err := c.invoke(ctx, "inspect", map[string]any{"uri": c.MemoryExportURI()}, &result); err != nil {
		return result, err
	}
	if _, err := result.Validate(c.config.Scope); err != nil {
		return ContextMemoryArtifact{}, &ContextClientError{Code: "invalid_memory_artifact", EffectState: "unknown", Cause: err}
	}
	return result, nil
}
func (c *ContextClient) RestoreMemory(ctx context.Context, requestID string, artifact ContextMemoryArtifact) (ContextMemoryReceipt, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	var result ContextMemoryReceipt
	if err := contextIdentifier(requestID); err != nil {
		return result, err
	}
	if artifact.ByteCount > artifact.RestoreMaxBytes {
		return result, &ContextClientError{Code: "memory_restore_capacity", EffectState: "not_dispatched", Cause: errors.New("artifact exceeds native restore byte limit")}
	}
	artifact.Bytes = append(ContextBytes(nil), artifact.Bytes...)
	_, err := artifact.Validate(c.config.Scope)
	if err != nil {
		return result, &ContextClientError{Code: "invalid_memory_artifact", EffectState: "not_dispatched", Cause: err}
	}
	request := struct {
		Version   uint32       `json:"version"`
		Scope     ContextScope `json:"scope"`
		RequestID string       `json:"request_id"`
		Command   struct {
			Kind           string       `json:"kind"`
			JSONL          ContextBytes `json:"jsonl"`
			ArtifactDigest string       `json:"artifact_digest"`
		} `json:"command"`
	}{Version: ContextVersion, Scope: c.config.Scope, RequestID: requestID}
	request.Command.Kind = "restore_jsonl"
	request.Command.JSONL = artifact.Bytes
	request.Command.ArtifactDigest = artifact.ArtifactDigest
	if err = c.remember(ctx, c.config.Conversation, "memory", request, &result); err != nil {
		return result, err
	}
	if result.Version != ContextVersion || !result.Scope.equal(c.config.Scope) {
		return ContextMemoryReceipt{}, &ContextClientError{Code: "invalid_memory_receipt", EffectState: "unknown"}
	}
	return result, nil
}
