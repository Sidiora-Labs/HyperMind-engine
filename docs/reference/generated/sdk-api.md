# Authored SDK API catalog

Generated from authored Rust embedded, TypeScript, Python, and Go SDK sources currently present in the tree. Generated wire bindings are covered by the schema catalog. A declaration’s presence does not certify package publication or cross-language qualification. See the SDK guide for availability and supported execution paths.

## sdk/go/admin.go::Health

<a id="sdk-sdk-go-admin-go-health"></a>

Source: [`sdk/go/admin.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/admin.go).

When to use: Use Health through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type Health struct {
```

## sdk/go/admin.go::ProjectionStat

<a id="sdk-sdk-go-admin-go-projectionstat"></a>

Source: [`sdk/go/admin.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/admin.go).

When to use: Use Projection Stat through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type ProjectionStat struct {
```

## sdk/go/admin.go::ActorStats

<a id="sdk-sdk-go-admin-go-actorstats"></a>

Source: [`sdk/go/admin.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/admin.go).

When to use: Use Actor Stats through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type ActorStats struct {
```

## sdk/go/admin.go::VerifyStatus

<a id="sdk-sdk-go-admin-go-verifystatus"></a>

Source: [`sdk/go/admin.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/admin.go).

When to use: Use Verify Status through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type VerifyStatus struct {
```

## sdk/go/admin.go::AdminHealth

<a id="sdk-sdk-go-admin-go-adminhealth"></a>

Source: [`sdk/go/admin.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/admin.go).

When to use: Use Admin Health through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *Client) AdminHealth(ctx context.Context) (Health, error) {
```

## sdk/go/admin.go::AdminStats

<a id="sdk-sdk-go-admin-go-adminstats"></a>

Source: [`sdk/go/admin.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/admin.go).

When to use: Use Admin Stats through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *Client) AdminStats(ctx context.Context, actor uint16) (ActorStats, error) {
```

## sdk/go/admin.go::AdminVerifyStatus

<a id="sdk-sdk-go-admin-go-adminverifystatus"></a>

Source: [`sdk/go/admin.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/admin.go).

When to use: Use Admin Verify Status through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *Client) AdminVerifyStatus(ctx context.Context, actor uint16) (VerifyStatus, error) {
```

## sdk/go/admin.go::AdminRebuildProjection

<a id="sdk-sdk-go-admin-go-adminrebuildprojection"></a>

Source: [`sdk/go/admin.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/admin.go).

When to use: Use Admin Rebuild Projection through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *Client) AdminRebuildProjection(ctx context.Context, actor uint16, name string) (uint64, error) {
```

## sdk/go/bundle.go::Gap

<a id="sdk-sdk-go-bundle-go-gap"></a>

Source: [`sdk/go/bundle.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/bundle.go).

When to use: Use Gap through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type Gap struct{ Kind, Detail string }
```

## sdk/go/bundle.go::ModernItem

<a id="sdk-sdk-go-bundle-go-modernitem"></a>

Source: [`sdk/go/bundle.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/bundle.go).

When to use: Use Modern Item through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type ModernItem struct {
```

## sdk/go/bundle.go::ModernSection

<a id="sdk-sdk-go-bundle-go-modernsection"></a>

Source: [`sdk/go/bundle.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/bundle.go).

When to use: Use Modern Section through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type ModernSection struct {
```

## sdk/go/bundle.go::PromptItem

<a id="sdk-sdk-go-bundle-go-promptitem"></a>

Source: [`sdk/go/bundle.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/bundle.go).

When to use: Use Prompt Item through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type PromptItem struct {
```

## sdk/go/bundle.go::PromptSection

<a id="sdk-sdk-go-bundle-go-promptsection"></a>

Source: [`sdk/go/bundle.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/bundle.go).

When to use: Use Prompt Section through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type PromptSection struct {
```

## sdk/go/bundle.go::RenderedPrompt

<a id="sdk-sdk-go-bundle-go-renderedprompt"></a>

Source: [`sdk/go/bundle.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/bundle.go).

When to use: Use Rendered Prompt through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type RenderedPrompt struct {
```

## sdk/go/bundle.go::RenderOptions

<a id="sdk-sdk-go-bundle-go-renderoptions"></a>

Source: [`sdk/go/bundle.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/bundle.go).

When to use: Use Render Options through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type RenderOptions struct{ SameTurnLSNs []uint64 }
```

## sdk/go/bundle.go::Render

<a id="sdk-sdk-go-bundle-go-render"></a>

Source: [`sdk/go/bundle.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/bundle.go).

When to use: Use Render through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func Render(bundle *Bundle, options RenderOptions) RenderedPrompt {
```

## sdk/go/client.go::EventKind

<a id="sdk-sdk-go-client-go-eventkind"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Event Kind through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type EventKind uint8
```

## sdk/go/client.go::PendingWriteError

<a id="sdk-sdk-go-client-go-pendingwriteerror"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Pending Write Error through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type PendingWriteError struct {
```

## sdk/go/client.go::Error

<a id="sdk-sdk-go-client-go-error"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Error through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (e *PendingWriteError) Error() string {
```

## sdk/go/client.go::Unwrap

<a id="sdk-sdk-go-client-go-unwrap"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Unwrap through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (e *PendingWriteError) Unwrap() error { return e.Cause }
```

## sdk/go/client.go::Is

<a id="sdk-sdk-go-client-go-is"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Is through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (e *PendingWriteError) Is(target error) bool {
```

## sdk/go/client.go::EngineError

<a id="sdk-sdk-go-client-go-engineerror"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Engine Error through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type EngineError struct {
```

## sdk/go/client.go::Error

<a id="sdk-sdk-go-client-go-error"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Error through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (e *EngineError) Error() string {
```

## sdk/go/client.go::Config

<a id="sdk-sdk-go-client-go-config"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Config through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type Config struct {
```

## sdk/go/client.go::Welcome

<a id="sdk-sdk-go-client-go-welcome"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Welcome through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type Welcome struct {
```

## sdk/go/client.go::AppendEvent

<a id="sdk-sdk-go-client-go-appendevent"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Append Event through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type AppendEvent struct {
```

## sdk/go/client.go::AppendAck

<a id="sdk-sdk-go-client-go-appendack"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Append Ack through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type AppendAck struct {
```

## sdk/go/client.go::Error

<a id="sdk-sdk-go-client-go-error"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Error through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (e *rejectedWriteError) Error() string { return e.cause.Error() }
```

## sdk/go/client.go::Unwrap

<a id="sdk-sdk-go-client-go-unwrap"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Unwrap through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (e *rejectedWriteError) Unwrap() error { return e.cause }
```

## sdk/go/client.go::Client

<a id="sdk-sdk-go-client-go-client"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Client through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type Client struct {
```

## sdk/go/client.go::Dial

<a id="sdk-sdk-go-client-go-dial"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Dial through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func Dial(cfg Config) (*Client, error) {
```

## sdk/go/client.go::Welcome

<a id="sdk-sdk-go-client-go-welcome"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Welcome through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *Client) Welcome() Welcome {
```

## sdk/go/client.go::Close

<a id="sdk-sdk-go-client-go-close"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Close through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *Client) Close() error {
```

## sdk/go/client.go::PendingLen

<a id="sdk-sdk-go-client-go-pendinglen"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Pending Len through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *Client) PendingLen() int {
```

## sdk/go/client.go::NextClientSeq

<a id="sdk-sdk-go-client-go-nextclientseq"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Next Client Seq through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *Client) NextClientSeq() uint64 {
```

## sdk/go/client.go::Append

<a id="sdk-sdk-go-client-go-append"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Append through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *Client) Append(ctx context.Context, events []AppendEvent) (AppendAck, error) {
```

## sdk/go/client.go::QueueAppend

<a id="sdk-sdk-go-client-go-queueappend"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Queue Append through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *Client) QueueAppend(ctx context.Context, events []AppendEvent) (AppendAck, bool, error) {
```

## sdk/go/client.go::Supervisor

<a id="sdk-sdk-go-client-go-supervisor"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Supervisor through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type Supervisor struct {
```

## sdk/go/client.go::Start

<a id="sdk-sdk-go-client-go-start"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Start through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *Supervisor) Start() error {
```

## sdk/go/client.go::Pid

<a id="sdk-sdk-go-client-go-pid"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Pid through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *Supervisor) Pid() int {
```

## sdk/go/client.go::Stop

<a id="sdk-sdk-go-client-go-stop"></a>

Source: [`sdk/go/client.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/client.go).

When to use: Use Stop through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *Supervisor) Stop() {
```

## sdk/go/grpc.go::TransportError

<a id="sdk-sdk-go-grpc-go-transporterror"></a>

Source: [`sdk/go/grpc.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/grpc.go).

When to use: Use Transport Error through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type TransportError struct {
```

## sdk/go/grpc.go::Error

<a id="sdk-sdk-go-grpc-go-error"></a>

Source: [`sdk/go/grpc.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/grpc.go).

When to use: Use Error through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (e *TransportError) Error() string {
```

## sdk/go/grpc.go::Unwrap

<a id="sdk-sdk-go-grpc-go-unwrap"></a>

Source: [`sdk/go/grpc.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/grpc.go).

When to use: Use Unwrap through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (e *TransportError) Unwrap() error { return e.Cause }
```

## sdk/go/grpc.go::Name

<a id="sdk-sdk-go-grpc-go-name"></a>

Source: [`sdk/go/grpc.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/grpc.go).

When to use: Use Name through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (ncprCodec) Name() string { return "proto" }
```

## sdk/go/grpc.go::Marshal

<a id="sdk-sdk-go-grpc-go-marshal"></a>

Source: [`sdk/go/grpc.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/grpc.go).

When to use: Use Marshal through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (ncprCodec) Marshal(value any) ([]byte, error) {
```

## sdk/go/grpc.go::Unmarshal

<a id="sdk-sdk-go-grpc-go-unmarshal"></a>

Source: [`sdk/go/grpc.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/grpc.go).

When to use: Use Unmarshal through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (ncprCodec) Unmarshal(data []byte, value any) error {
```

## sdk/go/grpc.go::Write

<a id="sdk-sdk-go-grpc-go-write"></a>

Source: [`sdk/go/grpc.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/grpc.go).

When to use: Use Write through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *grpcConn) Write(frame []byte) (int, error) {
```

## sdk/go/grpc.go::Read

<a id="sdk-sdk-go-grpc-go-read"></a>

Source: [`sdk/go/grpc.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/grpc.go).

When to use: Use Read through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *grpcConn) Read(out []byte) (int, error) {
```

## sdk/go/grpc.go::Close

<a id="sdk-sdk-go-grpc-go-close"></a>

Source: [`sdk/go/grpc.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/grpc.go).

When to use: Use Close through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *grpcConn) Close() error { c.cancel(); return c.client.Close() }
```

## sdk/go/grpc.go::Network

<a id="sdk-sdk-go-grpc-go-network"></a>

Source: [`sdk/go/grpc.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/grpc.go).

When to use: Use Network through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (a grpcAddress) Network() string    { return "grpc" }
```

## sdk/go/grpc.go::String

<a id="sdk-sdk-go-grpc-go-string"></a>

Source: [`sdk/go/grpc.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/grpc.go).

When to use: Use String through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (a grpcAddress) String() string     { return string(a) }
```

## sdk/go/grpc.go::LocalAddr

<a id="sdk-sdk-go-grpc-go-localaddr"></a>

Source: [`sdk/go/grpc.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/grpc.go).

When to use: Use Local Addr through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *grpcConn) LocalAddr() net.Addr  { return grpcAddress("client") }
```

## sdk/go/grpc.go::RemoteAddr

<a id="sdk-sdk-go-grpc-go-remoteaddr"></a>

Source: [`sdk/go/grpc.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/grpc.go).

When to use: Use Remote Addr through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *grpcConn) RemoteAddr() net.Addr { return grpcAddress(c.client.Target()) }
```

## sdk/go/grpc.go::SetDeadline

<a id="sdk-sdk-go-grpc-go-setdeadline"></a>

Source: [`sdk/go/grpc.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/grpc.go).

When to use: Use Set Deadline through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *grpcConn) SetDeadline(t time.Time) error {
```

## sdk/go/grpc.go::SetReadDeadline

<a id="sdk-sdk-go-grpc-go-setreaddeadline"></a>

Source: [`sdk/go/grpc.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/grpc.go).

When to use: Use Set Read Deadline through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *grpcConn) SetReadDeadline(t time.Time) error  { c.readDeadline = t; return nil }
```

## sdk/go/grpc.go::SetWriteDeadline

<a id="sdk-sdk-go-grpc-go-setwritedeadline"></a>

Source: [`sdk/go/grpc.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/grpc.go).

When to use: Use Set Write Deadline through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *grpcConn) SetWriteDeadline(t time.Time) error { c.writeDeadline = t; return nil }
```

## sdk/go/loopseam.go::ConversationBytes

<a id="sdk-sdk-go-loopseam-go-conversationbytes"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Conversation Bytes through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func ConversationBytes(conversationID string) [16]byte {
```

## sdk/go/loopseam.go::UserMsgEvent

<a id="sdk-sdk-go-loopseam-go-usermsgevent"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use User Msg Event through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func UserMsgEvent(conversation [16]byte, content string) AppendEvent {
```

## sdk/go/loopseam.go::DeliveredMsgEvent

<a id="sdk-sdk-go-loopseam-go-deliveredmsgevent"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Delivered Msg Event through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func DeliveredMsgEvent(conversation [16]byte, content string) AppendEvent {
```

## sdk/go/loopseam.go::ReasoningEvent

<a id="sdk-sdk-go-loopseam-go-reasoningevent"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Reasoning Event through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func ReasoningEvent(conversation [16]byte, content string) AppendEvent {
```

## sdk/go/loopseam.go::ProviderFrameEvent

<a id="sdk-sdk-go-loopseam-go-providerframeevent"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Provider Frame Event through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func ProviderFrameEvent(conversation [16]byte, provider string, apiContent []byte) AppendEvent {
```

## sdk/go/loopseam.go::ToolCallEvent

<a id="sdk-sdk-go-loopseam-go-toolcallevent"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Tool Call Event through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func ToolCallEvent(conversation [16]byte, callID, toolName string, arguments []byte) AppendEvent {
```

## sdk/go/loopseam.go::ToolResultEvent

<a id="sdk-sdk-go-loopseam-go-toolresultevent"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Tool Result Event through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func ToolResultEvent(conversation [16]byte, callID string, toolCallLsn uint64,
```

## sdk/go/loopseam.go::Assertion

<a id="sdk-sdk-go-loopseam-go-assertion"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Assertion through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type Assertion struct {
```

## sdk/go/loopseam.go::AssertionEvent

<a id="sdk-sdk-go-loopseam-go-assertionevent"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Assertion Event through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func AssertionEvent(conversation [16]byte, assertion Assertion) AppendEvent {
```

## sdk/go/loopseam.go::Retraction

<a id="sdk-sdk-go-loopseam-go-retraction"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Retraction through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type Retraction struct {
```

## sdk/go/loopseam.go::RetractionEvent

<a id="sdk-sdk-go-loopseam-go-retractionevent"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Retraction Event through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func RetractionEvent(conversation [16]byte, retraction Retraction) AppendEvent {
```

## sdk/go/loopseam.go::ConsolidationEvent

<a id="sdk-sdk-go-loopseam-go-consolidationevent"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Consolidation Event through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func ConsolidationEvent(conversation [16]byte, assertions []Assertion) AppendEvent {
```

## sdk/go/loopseam.go::DefaultTokenWeights

<a id="sdk-sdk-go-loopseam-go-defaulttokenweights"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Default Token Weights through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func DefaultTokenWeights() []uint16 {
```

## sdk/go/loopseam.go::Bundle

<a id="sdk-sdk-go-loopseam-go-bundle"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Bundle through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type Bundle struct {
```

## sdk/go/loopseam.go::BundleSection

<a id="sdk-sdk-go-loopseam-go-bundlesection"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Bundle Section through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type BundleSection struct {
```

## sdk/go/loopseam.go::BundleItem

<a id="sdk-sdk-go-loopseam-go-bundleitem"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Bundle Item through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type BundleItem struct {
```

## sdk/go/loopseam.go::TrimmedItem

<a id="sdk-sdk-go-loopseam-go-trimmeditem"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Trimmed Item through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type TrimmedItem struct {
```

## sdk/go/loopseam.go::ParseBundle

<a id="sdk-sdk-go-loopseam-go-parsebundle"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Parse Bundle through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func ParseBundle(encoded []byte) (*Bundle, error) {
```

## sdk/go/loopseam.go::DecodedEvent

<a id="sdk-sdk-go-loopseam-go-decodedevent"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Decoded Event through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type DecodedEvent struct {
```

## sdk/go/loopseam.go::MemoryProjection

<a id="sdk-sdk-go-loopseam-go-memoryprojection"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Memory Projection through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type MemoryProjection struct {
```

## sdk/go/loopseam.go::ProjectionExclusions

<a id="sdk-sdk-go-loopseam-go-projectionexclusions"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Projection Exclusions through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type ProjectionExclusions struct {
```

## sdk/go/loopseam.go::DecodeEvent

<a id="sdk-sdk-go-loopseam-go-decodeevent"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Decode Event through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func DecodeEvent(kind EventKind, payload []byte) (out DecodedEvent, err error) {
```

## sdk/go/loopseam.go::RenderBundle

<a id="sdk-sdk-go-loopseam-go-renderbundle"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Render Bundle through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func RenderBundle(bundle *Bundle, premises []string) string {
```

## sdk/go/loopseam.go::ProjectBundle

<a id="sdk-sdk-go-loopseam-go-projectbundle"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Project Bundle through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func ProjectBundle(bundle *Bundle) []MemoryProjection {
```

## sdk/go/loopseam.go::ProjectBundleExcluding

<a id="sdk-sdk-go-loopseam-go-projectbundleexcluding"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Project Bundle Excluding through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func ProjectBundleExcluding(bundle *Bundle, exclusions ProjectionExclusions) []MemoryProjection {
```

## sdk/go/loopseam.go::ToolExecution

<a id="sdk-sdk-go-loopseam-go-toolexecution"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Tool Execution through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type ToolExecution struct {
```

## sdk/go/loopseam.go::ToolEventCitation

<a id="sdk-sdk-go-loopseam-go-tooleventcitation"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Tool Event Citation through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type ToolEventCitation struct {
```

## sdk/go/loopseam.go::ToolEventPayload

<a id="sdk-sdk-go-loopseam-go-tooleventpayload"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Tool Event Payload through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type ToolEventPayload struct {
```

## sdk/go/loopseam.go::ActivationQuery

<a id="sdk-sdk-go-loopseam-go-activationquery"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Activation Query through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type ActivationQuery struct {
```

## sdk/go/loopseam.go::LoopSeam

<a id="sdk-sdk-go-loopseam-go-loopseam"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Loop Seam through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type LoopSeam struct {
```

## sdk/go/loopseam.go::SeamConfig

<a id="sdk-sdk-go-loopseam-go-seamconfig"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Seam Config through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type SeamConfig struct {
```

## sdk/go/loopseam.go::NewLoopSeam

<a id="sdk-sdk-go-loopseam-go-newloopseam"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use New Loop Seam through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func NewLoopSeam(client *Client, cfg SeamConfig) (*LoopSeam, error) {
```

## sdk/go/loopseam.go::Conversation

<a id="sdk-sdk-go-loopseam-go-conversation"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Conversation through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *LoopSeam) Conversation() [16]byte { return s.conversation }
```

## sdk/go/loopseam.go::Activate

<a id="sdk-sdk-go-loopseam-go-activate"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Activate through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *LoopSeam) Activate(ctx context.Context, query ActivationQuery) (string, error) {
```

## sdk/go/loopseam.go::ActivateBundle

<a id="sdk-sdk-go-loopseam-go-activatebundle"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Activate Bundle through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *LoopSeam) ActivateBundle(ctx context.Context, query ActivationQuery) (*Bundle, error) {
```

## sdk/go/loopseam.go::RecordUser

<a id="sdk-sdk-go-loopseam-go-recorduser"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Record User through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *LoopSeam) RecordUser(content string) {
```

## sdk/go/loopseam.go::RecordAssistantWorking

<a id="sdk-sdk-go-loopseam-go-recordassistantworking"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Record Assistant Working through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *LoopSeam) RecordAssistantWorking(content string) {
```

## sdk/go/loopseam.go::RecordDelivery

<a id="sdk-sdk-go-loopseam-go-recorddelivery"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Record Delivery through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *LoopSeam) RecordDelivery(content string) {
```

## sdk/go/loopseam.go::ProvenanceRange

<a id="sdk-sdk-go-loopseam-go-provenancerange"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Provenance Range through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *LoopSeam) ProvenanceRange() (string, uint64, uint64) {
```

## sdk/go/loopseam.go::RecordError

<a id="sdk-sdk-go-loopseam-go-recorderror"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Record Error through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *LoopSeam) RecordError() error {
```

## sdk/go/loopseam.go::CommitToolExecution

<a id="sdk-sdk-go-loopseam-go-committoolexecution"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Commit Tool Execution through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *LoopSeam) CommitToolExecution(ctx context.Context, execution ToolExecution) (ToolEventCitation, error) {
```

## sdk/go/loopseam.go::Consolidate

<a id="sdk-sdk-go-loopseam-go-consolidate"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Consolidate through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *LoopSeam) Consolidate(ctx context.Context, assertions []Assertion) (AppendAck, error) {
```

## sdk/go/loopseam.go::SaveTurnCheckpoint

<a id="sdk-sdk-go-loopseam-go-saveturncheckpoint"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Save Turn Checkpoint through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *LoopSeam) SaveTurnCheckpoint(ctx context.Context, turnID string, blob []byte) (uint64, error) {
```

## sdk/go/loopseam.go::LatestTurnCheckpoint

<a id="sdk-sdk-go-loopseam-go-latestturncheckpoint"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Latest Turn Checkpoint through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *LoopSeam) LatestTurnCheckpoint(ctx context.Context, turnID string) ([]byte, uint64, error) {
```

## sdk/go/loopseam.go::WriteCheckpoint

<a id="sdk-sdk-go-loopseam-go-writecheckpoint"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Write Checkpoint through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *Client) WriteCheckpoint(ctx context.Context, turnID string, blob []byte) (uint64, error) {
```

## sdk/go/loopseam.go::LatestCheckpoint

<a id="sdk-sdk-go-loopseam-go-latestcheckpoint"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Latest Checkpoint through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *Client) LatestCheckpoint(ctx context.Context, turnID string) ([]byte, uint64, error) {
```

## sdk/go/loopseam.go::TranscriptRecord

<a id="sdk-sdk-go-loopseam-go-transcriptrecord"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Transcript Record through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type TranscriptRecord struct {
```

## sdk/go/loopseam.go::Transcript

<a id="sdk-sdk-go-loopseam-go-transcript"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Transcript through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *Client) Transcript(ctx context.Context, conversation [16]byte,
```

## sdk/go/loopseam.go::Attest

<a id="sdk-sdk-go-loopseam-go-attest"></a>

Source: [`sdk/go/loopseam.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/loopseam.go).

When to use: Use Attest through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *Client) Attest(ctx context.Context, used, ignored []uint64) (uint32, error) {
```

## sdk/go/session.go::ToolEnvelope

<a id="sdk-sdk-go-session-go-toolenvelope"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Tool Envelope through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type ToolEnvelope map[string]any
```

## sdk/go/session.go::RecallOptions

<a id="sdk-sdk-go-session-go-recalloptions"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Recall Options through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type RecallOptions struct {
```

## sdk/go/session.go::Recall

<a id="sdk-sdk-go-session-go-recall"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Recall through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *Client) Recall(ctx context.Context, query string, options RecallOptions) ([]uint64, error) {
```

## sdk/go/session.go::AsOfOptions

<a id="sdk-sdk-go-session-go-asofoptions"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use As Of Options through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type AsOfOptions struct {
```

## sdk/go/session.go::AsOf

<a id="sdk-sdk-go-session-go-asof"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use As Of through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *Client) AsOf(ctx context.Context, beliefType uint8, identity string, options AsOfOptions) (*protocol.BeliefResult, error) {
```

## sdk/go/session.go::CallTool

<a id="sdk-sdk-go-session-go-calltool"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Call Tool through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *Client) CallTool(ctx context.Context, verb string, input any) (ToolEnvelope, error) {
```

## sdk/go/session.go::CryptoDelete

<a id="sdk-sdk-go-session-go-cryptodelete"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Crypto Delete through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *Client) CryptoDelete(ctx context.Context, actor uint16) ([]byte, error) {
```

## sdk/go/session.go::Subscription

<a id="sdk-sdk-go-session-go-subscription"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Subscription through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type Subscription struct {
```

## sdk/go/session.go::SubscribedEvent

<a id="sdk-sdk-go-session-go-subscribedevent"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Subscribed Event through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type SubscribedEvent struct {
```

## sdk/go/session.go::Subscribe

<a id="sdk-sdk-go-session-go-subscribe"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Subscribe through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *Client) Subscribe(ctx context.Context, conversation *[16]byte, sinceLSN uint64) (*Subscription, error) {
```

## sdk/go/session.go::Next

<a id="sdk-sdk-go-session-go-next"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Next through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *Subscription) Next(ctx context.Context) (SubscribedEvent, error) {
```

## sdk/go/session.go::Close

<a id="sdk-sdk-go-session-go-close"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Close through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *Subscription) Close() error {
```

## sdk/go/session.go::Session

<a id="sdk-sdk-go-session-go-session"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Session through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
type Session struct {
```

## sdk/go/session.go::Session

<a id="sdk-sdk-go-session-go-session"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Session through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (c *Client) Session(conversation string) (*Session, error) {
```

## sdk/go/session.go::Remember

<a id="sdk-sdk-go-session-go-remember"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Remember through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *Session) Remember(ctx context.Context, content string) (uint64, error) {
```

## sdk/go/session.go::Recall

<a id="sdk-sdk-go-session-go-recall"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Recall through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *Session) Recall(ctx context.Context, query string, options RecallOptions) ([]uint64, error) {
```

## sdk/go/session.go::Activate

<a id="sdk-sdk-go-session-go-activate"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Activate through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *Session) Activate(ctx context.Context, query string, budget uint64) (*Bundle, error) {
```

## sdk/go/session.go::Render

<a id="sdk-sdk-go-session-go-render"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Render through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *Session) Render(bundle *Bundle, options RenderOptions) RenderedPrompt {
```

## sdk/go/session.go::Attest

<a id="sdk-sdk-go-session-go-attest"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Attest through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *Session) Attest(ctx context.Context, used, ignored []uint64) (uint32, error) {
```

## sdk/go/session.go::AsOf

<a id="sdk-sdk-go-session-go-asof"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use As Of through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *Session) AsOf(ctx context.Context, kind uint8, identity string, options AsOfOptions) (*protocol.BeliefResult, error) {
```

## sdk/go/session.go::Believe

<a id="sdk-sdk-go-session-go-believe"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Believe through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *Session) Believe(ctx context.Context, input map[string]any) (ToolEnvelope, error) {
```

## sdk/go/session.go::Intend

<a id="sdk-sdk-go-session-go-intend"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Intend through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *Session) Intend(ctx context.Context, input map[string]any) (ToolEnvelope, error) {
```

## sdk/go/session.go::Bind

<a id="sdk-sdk-go-session-go-bind"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Bind through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *Session) Bind(ctx context.Context, input map[string]any) (ToolEnvelope, error) {
```

## sdk/go/session.go::Predict

<a id="sdk-sdk-go-session-go-predict"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Predict through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *Session) Predict(ctx context.Context, input map[string]any) (ToolEnvelope, error) {
```

## sdk/go/session.go::Outcome

<a id="sdk-sdk-go-session-go-outcome"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Outcome through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *Session) Outcome(ctx context.Context, input map[string]any) (ToolEnvelope, error) {
```

## sdk/go/session.go::Consolidate

<a id="sdk-sdk-go-session-go-consolidate"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Consolidate through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *Session) Consolidate(ctx context.Context, input map[string]any) (ToolEnvelope, error) {
```

## sdk/go/session.go::Retract

<a id="sdk-sdk-go-session-go-retract"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Retract through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *Session) Retract(ctx context.Context, input map[string]any) (ToolEnvelope, error) {
```

## sdk/go/session.go::Dispute

<a id="sdk-sdk-go-session-go-dispute"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Dispute through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *Session) Dispute(ctx context.Context, input map[string]any) (ToolEnvelope, error) {
```

## sdk/go/session.go::Forget

<a id="sdk-sdk-go-session-go-forget"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Forget through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *Session) Forget(ctx context.Context, input map[string]any) (ToolEnvelope, error) {
```

## sdk/go/session.go::Inspect

<a id="sdk-sdk-go-session-go-inspect"></a>

Source: [`sdk/go/session.go`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/go/session.go).

When to use: Use Inspect through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```go
func (s *Session) Inspect(ctx context.Context, input map[string]any) (ToolEnvelope, error) {
```

## sdk/python/hypermind/bundle.py::Bundle

<a id="sdk-sdk-python-hypermind-bundle-py-bundle"></a>

Source: [`sdk/python/hypermind/bundle.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/bundle.py).

When to use: Use Bundle through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```python
class Bundle:
```

## sdk/python/hypermind/bundle.py::ActivationSafetyError

<a id="sdk-sdk-python-hypermind-bundle-py-activationsafetyerror"></a>

Source: [`sdk/python/hypermind/bundle.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/bundle.py).

When to use: Use Activation Safety Error through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/bundle.py::is_reconstruction

<a id="sdk-sdk-python-hypermind-bundle-py-is-reconstruction"></a>

Source: [`sdk/python/hypermind/bundle.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/bundle.py).

When to use: Use is_reconstruction through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/bundle.py::assert_rememberable

<a id="sdk-sdk-python-hypermind-bundle-py-assert-rememberable"></a>

Source: [`sdk/python/hypermind/bundle.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/bundle.py).

When to use: Use assert_rememberable through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/bundle.py::raw

<a id="sdk-sdk-python-hypermind-bundle-py-raw"></a>

Source: [`sdk/python/hypermind/bundle.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/bundle.py).

When to use: Use raw through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/bundle.py::u8

<a id="sdk-sdk-python-hypermind-bundle-py-u8"></a>

Source: [`sdk/python/hypermind/bundle.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/bundle.py).

When to use: Use u8 through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/bundle.py::u32

<a id="sdk-sdk-python-hypermind-bundle-py-u32"></a>

Source: [`sdk/python/hypermind/bundle.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/bundle.py).

When to use: Use u32 through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```python
def u32(self): return struct.unpack("<I", self.raw(4))[0]
```

## sdk/python/hypermind/bundle.py::u64

<a id="sdk-sdk-python-hypermind-bundle-py-u64"></a>

Source: [`sdk/python/hypermind/bundle.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/bundle.py).

When to use: Use u64 through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```python
def u64(self): return struct.unpack("<Q", self.raw(8))[0]
```

## sdk/python/hypermind/bundle.py::count

<a id="sdk-sdk-python-hypermind-bundle-py-count"></a>

Source: [`sdk/python/hypermind/bundle.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/bundle.py).

When to use: Use count through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/bundle.py::blob

<a id="sdk-sdk-python-hypermind-bundle-py-blob"></a>

Source: [`sdk/python/hypermind/bundle.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/bundle.py).

When to use: Use blob through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/bundle.py::text

<a id="sdk-sdk-python-hypermind-bundle-py-text"></a>

Source: [`sdk/python/hypermind/bundle.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/bundle.py).

When to use: Use text through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```python
def text(self): return self.blob().decode("utf-8", errors="strict")
```

## sdk/python/hypermind/bundle.py::lsns

<a id="sdk-sdk-python-hypermind-bundle-py-lsns"></a>

Source: [`sdk/python/hypermind/bundle.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/bundle.py).

When to use: Use lsns through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```python
def lsns(self): return [self.u64() for _ in range(self.count())]
```

## sdk/python/hypermind/bundle.py::parse_bundle

<a id="sdk-sdk-python-hypermind-bundle-py-parse-bundle"></a>

Source: [`sdk/python/hypermind/bundle.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/bundle.py).

When to use: Use parse_bundle through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/bundle.py::render

<a id="sdk-sdk-python-hypermind-bundle-py-render"></a>

Source: [`sdk/python/hypermind/bundle.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/bundle.py).

When to use: Use render through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/client.py::HyperMindError

<a id="sdk-sdk-python-hypermind-client-py-hyperminderror"></a>

Source: [`sdk/python/hypermind/client.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/client.py).

When to use: Use Hyper Mind Error through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/client.py::conversation_id

<a id="sdk-sdk-python-hypermind-client-py-conversation-id"></a>

Source: [`sdk/python/hypermind/client.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/client.py).

When to use: Use conversation_id through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/client.py::Event

<a id="sdk-sdk-python-hypermind-client-py-event"></a>

Source: [`sdk/python/hypermind/client.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/client.py).

When to use: Use Event through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```python
class Event:
```

## sdk/python/hypermind/client.py::Client

<a id="sdk-sdk-python-hypermind-client-py-client"></a>

Source: [`sdk/python/hypermind/client.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/client.py).

When to use: Use Client through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/client.py::connect

<a id="sdk-sdk-python-hypermind-client-py-connect"></a>

Source: [`sdk/python/hypermind/client.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/client.py).

When to use: Use connect through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/client.py::close

<a id="sdk-sdk-python-hypermind-client-py-close"></a>

Source: [`sdk/python/hypermind/client.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/client.py).

When to use: Use close through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/client.py::session

<a id="sdk-sdk-python-hypermind-client-py-session"></a>

Source: [`sdk/python/hypermind/client.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/client.py).

When to use: Use session through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/client.py::request

<a id="sdk-sdk-python-hypermind-client-py-request"></a>

Source: [`sdk/python/hypermind/client.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/client.py).

When to use: Use request through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/client.py::recover

<a id="sdk-sdk-python-hypermind-client-py-recover"></a>

Source: [`sdk/python/hypermind/client.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/client.py).

When to use: Use recover through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/client.py::tool

<a id="sdk-sdk-python-hypermind-client-py-tool"></a>

Source: [`sdk/python/hypermind/client.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/client.py).

When to use: Use tool through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/client.py::activate

<a id="sdk-sdk-python-hypermind-client-py-activate"></a>

Source: [`sdk/python/hypermind/client.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/client.py).

When to use: Use activate through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/client.py::checkpoint

<a id="sdk-sdk-python-hypermind-client-py-checkpoint"></a>

Source: [`sdk/python/hypermind/client.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/client.py).

When to use: Use checkpoint through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/client.py::latest_checkpoint

<a id="sdk-sdk-python-hypermind-client-py-latest-checkpoint"></a>

Source: [`sdk/python/hypermind/client.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/client.py).

When to use: Use latest_checkpoint through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/client.py::as_of

<a id="sdk-sdk-python-hypermind-client-py-as-of"></a>

Source: [`sdk/python/hypermind/client.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/client.py).

When to use: Use as_of through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/client.py::crypto_delete

<a id="sdk-sdk-python-hypermind-client-py-crypto-delete"></a>

Source: [`sdk/python/hypermind/client.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/client.py).

When to use: Use crypto_delete through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/client.py::subscribe

<a id="sdk-sdk-python-hypermind-client-py-subscribe"></a>

Source: [`sdk/python/hypermind/client.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/client.py).

When to use: Use subscribe through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/engine.py::Engine

<a id="sdk-sdk-python-hypermind-engine-py-engine"></a>

Source: [`sdk/python/hypermind/engine.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/engine.py).

When to use: Use Engine through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/engine.py::open

<a id="sdk-sdk-python-hypermind-engine-py-open"></a>

Source: [`sdk/python/hypermind/engine.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/engine.py).

When to use: Use open through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```python
async def open(cls, path, *, actor: int, user_hex: str, kek_hex: str, projection_map_bytes=268_435_456, enable_providers=False):
```

## sdk/python/hypermind/engine.py::session

<a id="sdk-sdk-python-hypermind-engine-py-session"></a>

Source: [`sdk/python/hypermind/engine.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/engine.py).

When to use: Use session through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/engine.py::tool

<a id="sdk-sdk-python-hypermind-engine-py-tool"></a>

Source: [`sdk/python/hypermind/engine.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/engine.py).

When to use: Use tool through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/engine.py::activate

<a id="sdk-sdk-python-hypermind-engine-py-activate"></a>

Source: [`sdk/python/hypermind/engine.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/engine.py).

When to use: Use activate through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/engine.py::close

<a id="sdk-sdk-python-hypermind-engine-py-close"></a>

Source: [`sdk/python/hypermind/engine.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/engine.py).

When to use: Use close through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/session.py::Session

<a id="sdk-sdk-python-hypermind-session-py-session"></a>

Source: [`sdk/python/hypermind/session.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/session.py).

When to use: Use Session through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/session.py::remember

<a id="sdk-sdk-python-hypermind-session-py-remember"></a>

Source: [`sdk/python/hypermind/session.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/session.py).

When to use: Use remember through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/session.py::recall

<a id="sdk-sdk-python-hypermind-session-py-recall"></a>

Source: [`sdk/python/hypermind/session.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/session.py).

When to use: Use recall through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/session.py::activate

<a id="sdk-sdk-python-hypermind-session-py-activate"></a>

Source: [`sdk/python/hypermind/session.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/session.py).

When to use: Use activate through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/session.py::attest

<a id="sdk-sdk-python-hypermind-session-py-attest"></a>

Source: [`sdk/python/hypermind/session.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/session.py).

When to use: Use attest through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.

## sdk/python/hypermind/session.py::believe

<a id="sdk-sdk-python-hypermind-session-py-believe"></a>

Source: [`sdk/python/hypermind/session.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/session.py).

When to use: Use believe through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```python
async def believe(self, **arguments): return await self.backend.tool("believe", arguments)
```

## sdk/python/hypermind/session.py::intend

<a id="sdk-sdk-python-hypermind-session-py-intend"></a>

Source: [`sdk/python/hypermind/session.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/session.py).

When to use: Use intend through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```python
async def intend(self, **arguments): return await self.backend.tool("intend", arguments)
```

## sdk/python/hypermind/session.py::bind

<a id="sdk-sdk-python-hypermind-session-py-bind"></a>

Source: [`sdk/python/hypermind/session.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/session.py).

When to use: Use bind through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```python
async def bind(self, **arguments): return await self.backend.tool("bind", arguments)
```

## sdk/python/hypermind/session.py::predict

<a id="sdk-sdk-python-hypermind-session-py-predict"></a>

Source: [`sdk/python/hypermind/session.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/session.py).

When to use: Use predict through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```python
async def predict(self, **arguments): return await self.backend.tool("predict", arguments)
```

## sdk/python/hypermind/session.py::outcome

<a id="sdk-sdk-python-hypermind-session-py-outcome"></a>

Source: [`sdk/python/hypermind/session.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/session.py).

When to use: Use outcome through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```python
async def outcome(self, **arguments): return await self.backend.tool("outcome", arguments)
```

## sdk/python/hypermind/session.py::consolidate

<a id="sdk-sdk-python-hypermind-session-py-consolidate"></a>

Source: [`sdk/python/hypermind/session.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/session.py).

When to use: Use consolidate through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```python
async def consolidate(self, **arguments): return await self.backend.tool("consolidate", arguments)
```

## sdk/python/hypermind/session.py::inspect

<a id="sdk-sdk-python-hypermind-session-py-inspect"></a>

Source: [`sdk/python/hypermind/session.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/session.py).

When to use: Use inspect through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```python
async def inspect(self, **arguments): return await self.backend.tool("inspect", arguments)
```

## sdk/python/hypermind/session.py::retract

<a id="sdk-sdk-python-hypermind-session-py-retract"></a>

Source: [`sdk/python/hypermind/session.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/session.py).

When to use: Use retract through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```python
async def retract(self, **arguments): return await self.backend.tool("retract", arguments)
```

## sdk/python/hypermind/session.py::dispute

<a id="sdk-sdk-python-hypermind-session-py-dispute"></a>

Source: [`sdk/python/hypermind/session.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/session.py).

When to use: Use dispute through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```python
async def dispute(self, **arguments): return await self.backend.tool("dispute", arguments)
```

## sdk/python/hypermind/session.py::forget

<a id="sdk-sdk-python-hypermind-session-py-forget"></a>

Source: [`sdk/python/hypermind/session.py`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/hypermind/session.py).

When to use: Use forget through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```python
async def forget(self, **arguments): return await self.backend.tool("forget", arguments)
```

## sdk/python/native/src/lib.rs::NativeEngine (Python native class)

<a id="python-native-sdk-python-native-src-lib-rs-nativeengine"></a>

Source: [`sdk/python/native/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/native/src/lib.rs).

When to use: Use this PyO3-exported class through the Python extension for its documented embedded ownership contract.

Do not use: Do not open an actor already owned by another process, copy sample keys, or bypass the Python safe-rendering layer.


```rust
#[pyclass]
struct NativeEngine
```

## NativeEngine::new (Python native)

<a id="python-native-method-sdk-python-native-src-lib-rs-nativeengine-new"></a>

Source: [`sdk/python/native/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/native/src/lib.rs).

When to use: Use this exported PyO3 method through its owning Python class; its signature is taken from the native extension.

Do not use: Do not assume native execution bypasses actor ownership, evidence admission, or mutation-error handling.


```rust
fn new(
```

## NativeEngine::call (Python native)

<a id="python-native-method-sdk-python-native-src-lib-rs-nativeengine-call"></a>

Source: [`sdk/python/native/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/native/src/lib.rs).

When to use: Use this exported PyO3 method through its owning Python class; its signature is taken from the native extension.

Do not use: Do not assume native execution bypasses actor ownership, evidence admission, or mutation-error handling.


```rust
fn call(&self, py: Python<'_>, verb: String, arguments_json: String) -> PyResult<String> {
```

## NativeEngine::activate (Python native)

<a id="python-native-method-sdk-python-native-src-lib-rs-nativeengine-activate"></a>

Source: [`sdk/python/native/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/native/src/lib.rs).

When to use: Use this exported PyO3 method through its owning Python class; its signature is taken from the native extension.

Do not use: Do not assume native execution bypasses actor ownership, evidence admission, or mutation-error handling.


```rust
fn activate(
```

## NativeEngine::close (Python native)

<a id="python-native-method-sdk-python-native-src-lib-rs-nativeengine-close"></a>

Source: [`sdk/python/native/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/python/native/src/lib.rs).

When to use: Use this exported PyO3 method through its owning Python class; its signature is taken from the native extension.

Do not use: Do not assume native execution bypasses actor ownership, evidence admission, or mutation-error handling.


```rust
fn close(&mut self, py: Python<'_>) -> PyResult<()> {
```

## sdk/typescript/packages/client/src/anticipation.ts::JsonValue

<a id="sdk-sdk-typescript-packages-client-src-anticipation-ts-jsonvalue"></a>

Source: [`sdk/typescript/packages/client/src/anticipation.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/anticipation.ts).

When to use: Use Json Value through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };
```

## sdk/typescript/packages/client/src/anticipation.ts::LedgerInteger

<a id="sdk-sdk-typescript-packages-client-src-anticipation-ts-ledgerinteger"></a>

Source: [`sdk/typescript/packages/client/src/anticipation.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/anticipation.ts).

When to use: Use Ledger Integer through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type LedgerInteger = number | bigint;
```

## sdk/typescript/packages/client/src/anticipation.ts::ToolEnvelope

<a id="sdk-sdk-typescript-packages-client-src-anticipation-ts-toolenvelope"></a>

Source: [`sdk/typescript/packages/client/src/anticipation.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/anticipation.ts).

When to use: Use Tool Envelope through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface ToolEnvelope {
```

## sdk/typescript/packages/client/src/anticipation.ts::PredicateKind

<a id="sdk-sdk-typescript-packages-client-src-anticipation-ts-predicatekind"></a>

Source: [`sdk/typescript/packages/client/src/anticipation.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/anticipation.ts).

When to use: Use Predicate Kind through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type PredicateKind =
```

## sdk/typescript/packages/client/src/anticipation.ts::ExpectedPredicateInput

<a id="sdk-sdk-typescript-packages-client-src-anticipation-ts-expectedpredicateinput"></a>

Source: [`sdk/typescript/packages/client/src/anticipation.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/anticipation.ts).

When to use: Use Expected Predicate Input through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface ExpectedPredicateInput {
```

## sdk/typescript/packages/client/src/anticipation.ts::PredictInput

<a id="sdk-sdk-typescript-packages-client-src-anticipation-ts-predictinput"></a>

Source: [`sdk/typescript/packages/client/src/anticipation.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/anticipation.ts).

When to use: Use Predict Input through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface PredictInput {
```

## sdk/typescript/packages/client/src/anticipation.ts::OutcomeInput

<a id="sdk-sdk-typescript-packages-client-src-anticipation-ts-outcomeinput"></a>

Source: [`sdk/typescript/packages/client/src/anticipation.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/anticipation.ts).

When to use: Use Outcome Input through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface OutcomeInput {
```

## sdk/typescript/packages/client/src/anticipation.ts::InspectInput

<a id="sdk-sdk-typescript-packages-client-src-anticipation-ts-inspectinput"></a>

Source: [`sdk/typescript/packages/client/src/anticipation.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/anticipation.ts).

When to use: Use Inspect Input through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface InspectInput {
```

## sdk/typescript/packages/client/src/anticipation.ts::WakeTrigger

<a id="sdk-sdk-typescript-packages-client-src-anticipation-ts-waketrigger"></a>

Source: [`sdk/typescript/packages/client/src/anticipation.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/anticipation.ts).

When to use: Use Wake Trigger through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type WakeTrigger =
```

## sdk/typescript/packages/client/src/anticipation.ts::AttentionFactors

<a id="sdk-sdk-typescript-packages-client-src-anticipation-ts-attentionfactors"></a>

Source: [`sdk/typescript/packages/client/src/anticipation.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/anticipation.ts).

When to use: Use Attention Factors through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface AttentionFactors {
```

## sdk/typescript/packages/client/src/anticipation.ts::IntendAction

<a id="sdk-sdk-typescript-packages-client-src-anticipation-ts-intendaction"></a>

Source: [`sdk/typescript/packages/client/src/anticipation.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/anticipation.ts).

When to use: Use Intend Action through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type IntendAction =
```

## sdk/typescript/packages/client/src/anticipation.ts::ReconstructionRecallOptions

<a id="sdk-sdk-typescript-packages-client-src-anticipation-ts-reconstructionrecalloptions"></a>

Source: [`sdk/typescript/packages/client/src/anticipation.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/anticipation.ts).

When to use: Use Reconstruction Recall Options through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface ReconstructionRecallOptions {
```

## sdk/typescript/packages/client/src/anticipation.ts::encodeToolArguments

<a id="sdk-sdk-typescript-packages-client-src-anticipation-ts-encodetoolarguments"></a>

Source: [`sdk/typescript/packages/client/src/anticipation.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/anticipation.ts).

When to use: Use encode Tool Arguments through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function encodeToolArguments(value: unknown): string {
```

## sdk/typescript/packages/client/src/canonical.ts::parseBundle

<a id="sdk-sdk-typescript-packages-client-src-canonical-ts-parsebundle"></a>

Source: [`sdk/typescript/packages/client/src/canonical.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/canonical.ts).

When to use: Use parse Bundle through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function parseBundle(bytes: Uint8Array): Bundle {
```

## sdk/typescript/packages/client/src/canonical.ts::constructor

<a id="sdk-method-sdk-typescript-packages-client-src-canonical-ts-614-constructor"></a>

Source: [`sdk/typescript/packages/client/src/canonical.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/canonical.ts).

When to use: Use this constructor method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
constructor(private readonly bytes: Uint8Array) {}
```

## sdk/typescript/packages/client/src/canonical.ts::u8

<a id="sdk-method-sdk-typescript-packages-client-src-canonical-ts-668-u8"></a>

Source: [`sdk/typescript/packages/client/src/canonical.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/canonical.ts).

When to use: Use this u8 method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
u8(): number {
```

## sdk/typescript/packages/client/src/canonical.ts::u32

<a id="sdk-method-sdk-typescript-packages-client-src-canonical-ts-819-u32"></a>

Source: [`sdk/typescript/packages/client/src/canonical.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/canonical.ts).

When to use: Use this u32 method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
u32(): number {
```

## sdk/typescript/packages/client/src/canonical.ts::u64

<a id="sdk-method-sdk-typescript-packages-client-src-canonical-ts-933-u64"></a>

Source: [`sdk/typescript/packages/client/src/canonical.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/canonical.ts).

When to use: Use this u64 method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
u64(): bigint {
```

## sdk/typescript/packages/client/src/canonical.ts::count

<a id="sdk-method-sdk-typescript-packages-client-src-canonical-ts-1050-count"></a>

Source: [`sdk/typescript/packages/client/src/canonical.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/canonical.ts).

When to use: Use this count method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
count(): number {
```

## sdk/typescript/packages/client/src/canonical.ts::raw

<a id="sdk-method-sdk-typescript-packages-client-src-canonical-ts-1222-raw"></a>

Source: [`sdk/typescript/packages/client/src/canonical.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/canonical.ts).

When to use: Use this raw method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
raw(length: number): Uint8Array {
```

## sdk/typescript/packages/client/src/canonical.ts::blob

<a id="sdk-method-sdk-typescript-packages-client-src-canonical-ts-1505-blob"></a>

Source: [`sdk/typescript/packages/client/src/canonical.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/canonical.ts).

When to use: Use this blob method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
blob(): Uint8Array {
```

## sdk/typescript/packages/client/src/canonical.ts::lsns

<a id="sdk-method-sdk-typescript-packages-client-src-canonical-ts-1568-lsns"></a>

Source: [`sdk/typescript/packages/client/src/canonical.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/canonical.ts).

When to use: Use this lsns method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
lsns(): bigint[] {
```

## sdk/typescript/packages/client/src/canonical.ts::done

<a id="sdk-method-sdk-typescript-packages-client-src-canonical-ts-1661-done"></a>

Source: [`sdk/typescript/packages/client/src/canonical.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/canonical.ts).

When to use: Use this done method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
done(): boolean {
```

## sdk/typescript/packages/client/src/client.ts::EffectState

<a id="sdk-sdk-typescript-packages-client-src-client-ts-effectstate"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use Effect State through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type EffectState = "not_dispatched" | "unknown" | "rejected";
```

## sdk/typescript/packages/client/src/client.ts::MemoryKind

<a id="sdk-sdk-typescript-packages-client-src-client-ts-memorykind"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use Memory Kind through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type MemoryKind = "user" | "assistant" | "document";
```

## sdk/typescript/packages/client/src/client.ts::CloseReason

<a id="sdk-sdk-typescript-packages-client-src-client-ts-closereason"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use Close Reason through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type CloseReason = "done" | "abandoned" | "handed_off" | "superseded";
```

## sdk/typescript/packages/client/src/client.ts::RecallMode

<a id="sdk-sdk-typescript-packages-client-src-client-ts-recallmode"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use Recall Mode through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type RecallMode = "semantic" | "lexical" | "entity" | "temporal" | "near";
```

## sdk/typescript/packages/client/src/client.ts::RetentionPolicy

<a id="sdk-sdk-typescript-packages-client-src-client-ts-retentionpolicy"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use Retention Policy through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type RetentionPolicy = "current_state" | "daily" | "durable" | "do_not_store";
```

## sdk/typescript/packages/client/src/client.ts::SensitivityPolicy

<a id="sdk-sdk-typescript-packages-client-src-client-ts-sensitivitypolicy"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use Sensitivity Policy through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type SensitivityPolicy = "public" | "personal" | "secret";
```

## sdk/typescript/packages/client/src/client.ts::AnchorFacet

<a id="sdk-sdk-typescript-packages-client-src-client-ts-anchorfacet"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use Anchor Facet through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type AnchorFacet = "path" | "symbol" | "url" | "entity";
```

## sdk/typescript/packages/client/src/client.ts::BeliefType

<a id="sdk-sdk-typescript-packages-client-src-client-ts-belieftype"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use Belief Type through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type BeliefType = "fact" | "preference" | "constraint" | "goal" | "identity";
```

## sdk/typescript/packages/client/src/client.ts::BeliefClaim

<a id="sdk-sdk-typescript-packages-client-src-client-ts-beliefclaim"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use Belief Claim through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type BeliefClaim = "affirmative" | "negative_existence";
```

## sdk/typescript/packages/client/src/client.ts::BeliefProvenance

<a id="sdk-sdk-typescript-packages-client-src-client-ts-beliefprovenance"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use Belief Provenance through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface BeliefProvenance {
```

## sdk/typescript/packages/client/src/client.ts::BelieveInput

<a id="sdk-sdk-typescript-packages-client-src-client-ts-believeinput"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use Believe Input through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface BelieveInput {
```

## sdk/typescript/packages/client/src/client.ts::AsOfOptions

<a id="sdk-sdk-typescript-packages-client-src-client-ts-asofoptions"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use As Of Options through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type AsOfOptions =
```

## sdk/typescript/packages/client/src/client.ts::BeliefRecord

<a id="sdk-sdk-typescript-packages-client-src-client-ts-beliefrecord"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use Belief Record through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface BeliefRecord {
```

## sdk/typescript/packages/client/src/client.ts::RememberOptions

<a id="sdk-sdk-typescript-packages-client-src-client-ts-rememberoptions"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use Remember Options through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface RememberOptions {
```

## sdk/typescript/packages/client/src/client.ts::DoNotStoreReceipt

<a id="sdk-sdk-typescript-packages-client-src-client-ts-donotstorereceipt"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use Do Not Store Receipt through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface DoNotStoreReceipt {
```

## sdk/typescript/packages/client/src/client.ts::RecallOptions

<a id="sdk-sdk-typescript-packages-client-src-client-ts-recalloptions"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use Recall Options through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface RecallOptions {
```

## sdk/typescript/packages/client/src/client.ts::ActivateOptions

<a id="sdk-sdk-typescript-packages-client-src-client-ts-activateoptions"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use Activate Options through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface ActivateOptions {
```

## sdk/typescript/packages/client/src/client.ts::ClientConfig

<a id="sdk-sdk-typescript-packages-client-src-client-ts-clientconfig"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use Client Config through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface ClientConfig {
```

## sdk/typescript/packages/client/src/client.ts::WelcomeInfo

<a id="sdk-sdk-typescript-packages-client-src-client-ts-welcomeinfo"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use Welcome Info through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface WelcomeInfo {
```

## sdk/typescript/packages/client/src/client.ts::EngineError

<a id="sdk-sdk-typescript-packages-client-src-client-ts-engineerror"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use Engine Error through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export class EngineError extends Error {
```

## sdk/typescript/packages/client/src/client.ts::PendingWriteError

<a id="sdk-sdk-typescript-packages-client-src-client-ts-pendingwriteerror"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use Pending Write Error through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export class PendingWriteError extends Error {
```

## sdk/typescript/packages/client/src/client.ts::ToolTransportError

<a id="sdk-sdk-typescript-packages-client-src-client-ts-tooltransporterror"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use Tool Transport Error through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export class ToolTransportError extends Error {
```

## sdk/typescript/packages/client/src/client.ts::Client

<a id="sdk-sdk-typescript-packages-client-src-client-ts-client"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use Client through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export class Client {
```

## sdk/typescript/packages/client/src/client.ts::Session

<a id="sdk-sdk-typescript-packages-client-src-client-ts-session"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use Session through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export class Session {
```

## sdk/typescript/packages/client/src/client.ts::constructor

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-6581-constructor"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this constructor method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
constructor(
```

## sdk/typescript/packages/client/src/client.ts::constructor

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-6963-constructor"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this constructor method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
constructor(public readonly clientSeq: bigint, public readonly cause: unknown) {
```

## sdk/typescript/packages/client/src/client.ts::constructor

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-7166-constructor"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this constructor method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
constructor(public readonly effectState: EffectState, public readonly cause: unknown) {
```

## sdk/typescript/packages/client/src/client.ts::constructor

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-7686-constructor"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this constructor method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
constructor(readonly socket: net.Socket) {
```

## sdk/typescript/packages/client/src/client.ts::send

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-7908-send"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this send method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async send(encoded: Uint8Array): Promise<void> {
```

## sdk/typescript/packages/client/src/client.ts::close

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-8283-close"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this close method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
close(): void { this.socket.destroy(); }
```

## sdk/typescript/packages/client/src/client.ts::next

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-8327-next"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this next method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
next(timeoutMs: number): Promise<Uint8Array> {
```

## sdk/typescript/packages/client/src/client.ts::connect

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-11200-connect"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this connect method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
static async connect(config: ClientConfig): Promise<Client> {
```

## sdk/typescript/packages/client/src/client.ts::session

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-11812-session"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this session method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
session(conversation: string): Session {
```

## sdk/typescript/packages/client/src/client.ts::close

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-12014-close"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this close method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
close(): void {
```

## sdk/typescript/packages/client/src/client.ts::attest

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-12087-attest"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this attest method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async attest(input: { used?: bigint[]; ignored?: bigint[]; helpful?: bigint[]; harmful?: bigint[] }): Promise<number> {
```

## sdk/typescript/packages/client/src/client.ts::checkpoint

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-15275-checkpoint"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this checkpoint method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async checkpoint(turnId: string, blob: Uint8Array): Promise<bigint> {
```

## sdk/typescript/packages/client/src/client.ts::latestCheckpoint

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-15649-latestcheckpoint"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this latestCheckpoint method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async latestCheckpoint(turnId: string): Promise<{ lsn: bigint; blob: Uint8Array } | undefined> {
```

## sdk/typescript/packages/client/src/client.ts::append

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-16140-append"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this append method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async append(kind: number, conversation: Uint8Array, payload: Uint8Array): Promise<bigint> {
```

## sdk/typescript/packages/client/src/client.ts::recall

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-16558-recall"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this recall method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
recall(query: string, options: ReconstructionRecallOptions): Promise<ToolEnvelope>;
```

## sdk/typescript/packages/client/src/client.ts::recall

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-16644-recall"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this recall method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
recall(query: string, options?: number | RecallOptions): Promise<bigint[]>;
```

## sdk/typescript/packages/client/src/client.ts::recall

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-16722-recall"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this recall method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async recall(query: string, options: number | RecallOptions | ReconstructionRecallOptions = {}): Promise<bigint[] | ToolEnvelope> {
```

## sdk/typescript/packages/client/src/client.ts::callTool

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-18520-calltool"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this callTool method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async callTool(
```

## sdk/typescript/packages/client/src/client.ts::asOf

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-19963-asof"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this asOf method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async asOf(
```

## sdk/typescript/packages/client/src/client.ts::activate

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-20566-activate"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this activate method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async activate(
```

## sdk/typescript/packages/client/src/client.ts::constructor

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-29225-constructor"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this constructor method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
constructor(
```

## sdk/typescript/packages/client/src/client.ts::remember

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-29372-remember"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this remember method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
remember(content: string): Promise<bigint>;
```

## sdk/typescript/packages/client/src/client.ts::remember

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-29418-remember"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this remember method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
remember(content: string, options: MemoryKind): Promise<bigint>;
```

## sdk/typescript/packages/client/src/client.ts::remember

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-29485-remember"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this remember method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
remember(
```

## sdk/typescript/packages/client/src/client.ts::remember

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-29613-remember"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this remember method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
remember(content: string, options: RememberOptions): Promise<bigint>;
```

## sdk/typescript/packages/client/src/client.ts::remember

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-29685-remember"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this remember method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
remember(
```

## sdk/typescript/packages/client/src/client.ts::recall

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-30801-recall"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this recall method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
recall(query: string, options: ReconstructionRecallOptions): Promise<ToolEnvelope>;
```

## sdk/typescript/packages/client/src/client.ts::recall

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-30887-recall"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this recall method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
recall(query: string, options?: number | RecallOptions): Promise<bigint[]>;
```

## sdk/typescript/packages/client/src/client.ts::recall

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-30965-recall"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this recall method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
recall(query: string, options: number | RecallOptions | ReconstructionRecallOptions = {}): Promise<bigint[] | ToolEnvelope> {
```

## sdk/typescript/packages/client/src/client.ts::activate

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-31471-activate"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this activate method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
activate(query: string, options: number | ActivateOptions): Promise<Bundle> {
```

## sdk/typescript/packages/client/src/client.ts::render

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-31629-render"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this render method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
render(bundle: Bundle, options: RenderOptions = {}): RenderedPrompt {
```

## sdk/typescript/packages/client/src/client.ts::believe

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-31742-believe"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this believe method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
believe(input: BelieveInput): Promise<bigint> {
```

## sdk/typescript/packages/client/src/client.ts::retract

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-32536-retract"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this retract method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
retract(beliefId: string, provenance: BeliefProvenance[]): Promise<bigint> {
```

## sdk/typescript/packages/client/src/client.ts::asOf

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-33009-asof"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this asOf method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
asOf(
```

## sdk/typescript/packages/client/src/client.ts::checkpoint

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-33217-checkpoint"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this checkpoint method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
checkpoint(turnId: string, blob: Uint8Array): Promise<bigint> {
```

## sdk/typescript/packages/client/src/client.ts::intend

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-33337-intend"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this intend method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
intend(action: "set_objective", objective: string): Promise<bigint>;
```

## sdk/typescript/packages/client/src/client.ts::intend

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-33408-intend"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this intend method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
intend(action: "open_loop", loopId: string, objective: string): Promise<bigint>;
```

## sdk/typescript/packages/client/src/client.ts::intend

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-33491-intend"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this intend method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
intend(action: "close_loop", loopId: string, reason: CloseReason, cause?: string, evidenceLsns?: bigint[]): Promise<bigint>;
```

## sdk/typescript/packages/client/src/client.ts::intend

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-33618-intend"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this intend method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
intend(action: IntendAction): Promise<ToolEnvelope>;
```

## sdk/typescript/packages/client/src/client.ts::intend

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-33673-intend"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this intend method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
intend(action: string | IntendAction, first?: string, second?: string, cause = "", evidenceLsns: bigint[] = []): Promise<bigint | ToolEnvelope> {
```

## sdk/typescript/packages/client/src/client.ts::predict

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-34752-predict"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this predict method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
predict(input: PredictInput): Promise<ToolEnvelope> {
```

## sdk/typescript/packages/client/src/client.ts::outcome

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-34904-outcome"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this outcome method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
outcome(input: OutcomeInput): Promise<ToolEnvelope> {
```

## sdk/typescript/packages/client/src/client.ts::inspect

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-35056-inspect"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this inspect method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
inspect(input: InspectInput = {}): Promise<ToolEnvelope> {
```

## sdk/typescript/packages/client/src/client.ts::attest

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-35173-attest"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this attest method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
attest(input: { used?: bigint[]; ignored?: bigint[]; helpful?: bigint[]; harmful?: bigint[] }): Promise<number> {
```

## sdk/typescript/packages/client/src/client.ts::consolidate

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-35332-consolidate"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this consolidate method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
consolidate(input: Record<string, unknown> = {}): Promise<ToolEnvelope> {
```

## sdk/typescript/packages/client/src/client.ts::dispute

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-35508-dispute"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this dispute method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
dispute(input: Record<string, unknown>): Promise<ToolEnvelope> {
```

## sdk/typescript/packages/client/src/client.ts::forget

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-35671-forget"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this forget method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
forget(input: Record<string, unknown>): Promise<ToolEnvelope> {
```

## sdk/typescript/packages/client/src/client.ts::bind

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-35832-bind"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this bind method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
bind(input: {
```

## sdk/typescript/packages/client/src/client.ts::return

<a id="sdk-method-sdk-typescript-packages-client-src-client-ts-41625-return"></a>

Source: [`sdk/typescript/packages/client/src/client.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/client.ts).

When to use: Use this return method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
return (crc ^ 0xffffffff) >>> 0;
```

## sdk/typescript/packages/client/src/grpc.ts::GrpcConfig

<a id="sdk-sdk-typescript-packages-client-src-grpc-ts-grpcconfig"></a>

Source: [`sdk/typescript/packages/client/src/grpc.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/grpc.ts).

When to use: Use Grpc Config through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface GrpcConfig {
```

## sdk/typescript/packages/client/src/grpc.ts::GrpcTransportError

<a id="sdk-sdk-typescript-packages-client-src-grpc-ts-grpctransporterror"></a>

Source: [`sdk/typescript/packages/client/src/grpc.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/grpc.ts).

When to use: Use Grpc Transport Error through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export class GrpcTransportError extends Error {
```

## sdk/typescript/packages/client/src/grpc.ts::GrpcFrames

<a id="sdk-sdk-typescript-packages-client-src-grpc-ts-grpcframes"></a>

Source: [`sdk/typescript/packages/client/src/grpc.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/grpc.ts).

When to use: Use Grpc Frames through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export class GrpcFrames {
```

## sdk/typescript/packages/client/src/grpc.ts::constructor

<a id="sdk-method-sdk-typescript-packages-client-src-grpc-ts-264-constructor"></a>

Source: [`sdk/typescript/packages/client/src/grpc.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/grpc.ts).

When to use: Use this constructor method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
constructor(readonly cause: grpc.ServiceError) {
```

## sdk/typescript/packages/client/src/grpc.ts::constructor

<a id="sdk-method-sdk-typescript-packages-client-src-grpc-ts-1713-constructor"></a>

Source: [`sdk/typescript/packages/client/src/grpc.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/grpc.ts).

When to use: Use this constructor method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
constructor(config: GrpcConfig, private readonly timeoutMs: number) {
```

## sdk/typescript/packages/client/src/grpc.ts::send

<a id="sdk-method-sdk-typescript-packages-client-src-grpc-ts-2270-send"></a>

Source: [`sdk/typescript/packages/client/src/grpc.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/grpc.ts).

When to use: Use this send method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async send(bytes: Uint8Array): Promise<void> {
```

## sdk/typescript/packages/client/src/grpc.ts::next

<a id="sdk-method-sdk-typescript-packages-client-src-grpc-ts-2909-next"></a>

Source: [`sdk/typescript/packages/client/src/grpc.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/grpc.ts).

When to use: Use this next method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
next(_timeoutMs: number): Promise<Uint8Array> {
```

## sdk/typescript/packages/client/src/grpc.ts::close

<a id="sdk-method-sdk-typescript-packages-client-src-grpc-ts-3921-close"></a>

Source: [`sdk/typescript/packages/client/src/grpc.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/client/src/grpc.ts).

When to use: Use this close method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
close(): void { this.client.close(); }
```

## sdk/typescript/packages/console/src/app.ts::paintDomainProfile

<a id="sdk-method-sdk-typescript-packages-console-src-app-ts-6496-paintdomainprofile"></a>

Source: [`sdk/typescript/packages/console/src/app.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/app.ts).

When to use: Use this paintDomainProfile method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
paintDomainProfile();
```

## sdk/typescript/packages/console/src/app.ts::paintDomainProfile

<a id="sdk-method-sdk-typescript-packages-console-src-app-ts-8766-paintdomainprofile"></a>

Source: [`sdk/typescript/packages/console/src/app.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/app.ts).

When to use: Use this paintDomainProfile method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
paintDomainProfile();
```

## sdk/typescript/packages/console/src/app.ts::showDomainProfile

<a id="sdk-method-sdk-typescript-packages-console-src-app-ts-9756-showdomainprofile"></a>

Source: [`sdk/typescript/packages/console/src/app.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/app.ts).

When to use: Use this showDomainProfile method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
showDomainProfile(mount);
```

## sdk/typescript/packages/console/src/domain-profile.ts::RESERVED_FIELD_NAMES

<a id="sdk-sdk-typescript-packages-console-src-domain-profile-ts-reserved-field-names"></a>

Source: [`sdk/typescript/packages/console/src/domain-profile.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/domain-profile.ts).

When to use: Use RESERVED_FIELD_NAMES through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export const RESERVED_FIELD_NAMES: readonly string[] = [
```

## sdk/typescript/packages/console/src/domain-profile.ts::DomainField

<a id="sdk-sdk-typescript-packages-console-src-domain-profile-ts-domainfield"></a>

Source: [`sdk/typescript/packages/console/src/domain-profile.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/domain-profile.ts).

When to use: Use Domain Field through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface DomainField {
```

## sdk/typescript/packages/console/src/domain-profile.ts::DomainRelation

<a id="sdk-sdk-typescript-packages-console-src-domain-profile-ts-domainrelation"></a>

Source: [`sdk/typescript/packages/console/src/domain-profile.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/domain-profile.ts).

When to use: Use Domain Relation through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface DomainRelation {
```

## sdk/typescript/packages/console/src/domain-profile.ts::DomainType

<a id="sdk-sdk-typescript-packages-console-src-domain-profile-ts-domaintype"></a>

Source: [`sdk/typescript/packages/console/src/domain-profile.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/domain-profile.ts).

When to use: Use Domain Type through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface DomainType {
```

## sdk/typescript/packages/console/src/domain-profile.ts::DomainProfile

<a id="sdk-sdk-typescript-packages-console-src-domain-profile-ts-domainprofile"></a>

Source: [`sdk/typescript/packages/console/src/domain-profile.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/domain-profile.ts).

When to use: Use Domain Profile through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface DomainProfile {
```

## sdk/typescript/packages/console/src/domain-profile.ts::DomainAction

<a id="sdk-sdk-typescript-packages-console-src-domain-profile-ts-domainaction"></a>

Source: [`sdk/typescript/packages/console/src/domain-profile.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/domain-profile.ts).

When to use: Use Domain Action through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type DomainAction =
```

## sdk/typescript/packages/console/src/domain-profile.ts::CompiledDomainProfile

<a id="sdk-sdk-typescript-packages-console-src-domain-profile-ts-compileddomainprofile"></a>

Source: [`sdk/typescript/packages/console/src/domain-profile.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/domain-profile.ts).

When to use: Use Compiled Domain Profile through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface CompiledDomainProfile {
```

## sdk/typescript/packages/console/src/domain-profile.ts::emptyDomainProfile

<a id="sdk-sdk-typescript-packages-console-src-domain-profile-ts-emptydomainprofile"></a>

Source: [`sdk/typescript/packages/console/src/domain-profile.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/domain-profile.ts).

When to use: Use empty Domain Profile through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function emptyDomainProfile(): DomainProfile {
```

## sdk/typescript/packages/console/src/domain-profile.ts::applyDomainAction

<a id="sdk-sdk-typescript-packages-console-src-domain-profile-ts-applydomainaction"></a>

Source: [`sdk/typescript/packages/console/src/domain-profile.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/domain-profile.ts).

When to use: Use apply Domain Action through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function applyDomainAction(profile: DomainProfile, action: DomainAction): DomainProfile {
```

## sdk/typescript/packages/console/src/domain-profile.ts::compileDomainProfile

<a id="sdk-sdk-typescript-packages-console-src-domain-profile-ts-compiledomainprofile"></a>

Source: [`sdk/typescript/packages/console/src/domain-profile.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/domain-profile.ts).

When to use: Use compile Domain Profile through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function compileDomainProfile(profile: DomainProfile): CompiledDomainProfile {
```

## sdk/typescript/packages/console/src/domain-profile.ts::persistDomainProfile

<a id="sdk-sdk-typescript-packages-console-src-domain-profile-ts-persistdomainprofile"></a>

Source: [`sdk/typescript/packages/console/src/domain-profile.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/domain-profile.ts).

When to use: Use persist Domain Profile through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export async function persistDomainProfile(
```

## sdk/typescript/packages/console/src/domain-profile.ts::loadDomainProfile

<a id="sdk-sdk-typescript-packages-console-src-domain-profile-ts-loaddomainprofile"></a>

Source: [`sdk/typescript/packages/console/src/domain-profile.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/domain-profile.ts).

When to use: Use load Domain Profile through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export async function loadDomainProfile(
```

## sdk/typescript/packages/console/src/domain-profile.ts::renderDomainProfile

<a id="sdk-sdk-typescript-packages-console-src-domain-profile-ts-renderdomainprofile"></a>

Source: [`sdk/typescript/packages/console/src/domain-profile.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/domain-profile.ts).

When to use: Use render Domain Profile through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function renderDomainProfile(
```

## sdk/typescript/packages/console/src/errors.ts::ConsoleDataError

<a id="sdk-sdk-typescript-packages-console-src-errors-ts-consoledataerror"></a>

Source: [`sdk/typescript/packages/console/src/errors.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/errors.ts).

When to use: Use Console Data Error through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export class ConsoleDataError extends Error {
```

## sdk/typescript/packages/console/src/errors.ts::constructor

<a id="sdk-method-sdk-typescript-packages-console-src-errors-ts-101-constructor"></a>

Source: [`sdk/typescript/packages/console/src/errors.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/errors.ts).

When to use: Use this constructor method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
constructor(surface: string, field: string, detail?: string) {
```

## sdk/typescript/packages/console/src/evidence.ts::EvidenceIntegrity

<a id="sdk-sdk-typescript-packages-console-src-evidence-ts-evidenceintegrity"></a>

Source: [`sdk/typescript/packages/console/src/evidence.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/evidence.ts).

When to use: Use Evidence Integrity through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface EvidenceIntegrity {
```

## sdk/typescript/packages/console/src/evidence.ts::EvidenceStep

<a id="sdk-sdk-typescript-packages-console-src-evidence-ts-evidencestep"></a>

Source: [`sdk/typescript/packages/console/src/evidence.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/evidence.ts).

When to use: Use Evidence Step through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface EvidenceStep {
```

## sdk/typescript/packages/console/src/evidence.ts::EvidencePathView

<a id="sdk-sdk-typescript-packages-console-src-evidence-ts-evidencepathview"></a>

Source: [`sdk/typescript/packages/console/src/evidence.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/evidence.ts).

When to use: Use Evidence Path View through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface EvidencePathView {
```

## sdk/typescript/packages/console/src/evidence.ts::EvidenceClass

<a id="sdk-sdk-typescript-packages-console-src-evidence-ts-evidenceclass"></a>

Source: [`sdk/typescript/packages/console/src/evidence.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/evidence.ts).

When to use: Use Evidence Class through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface EvidenceClass {
```

## sdk/typescript/packages/console/src/evidence.ts::AnswerLookupCounts

<a id="sdk-sdk-typescript-packages-console-src-evidence-ts-answerlookupcounts"></a>

Source: [`sdk/typescript/packages/console/src/evidence.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/evidence.ts).

When to use: Use Answer Lookup Counts through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface AnswerLookupCounts {
```

## sdk/typescript/packages/console/src/evidence.ts::AnswerLookupAnswer

<a id="sdk-sdk-typescript-packages-console-src-evidence-ts-answerlookupanswer"></a>

Source: [`sdk/typescript/packages/console/src/evidence.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/evidence.ts).

When to use: Use Answer Lookup Answer through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface AnswerLookupAnswer {
```

## sdk/typescript/packages/console/src/evidence.ts::AnswerLookupView

<a id="sdk-sdk-typescript-packages-console-src-evidence-ts-answerlookupview"></a>

Source: [`sdk/typescript/packages/console/src/evidence.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/evidence.ts).

When to use: Use Answer Lookup View through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface AnswerLookupView {
```

## sdk/typescript/packages/console/src/evidence.ts::buildEvidencePath

<a id="sdk-sdk-typescript-packages-console-src-evidence-ts-buildevidencepath"></a>

Source: [`sdk/typescript/packages/console/src/evidence.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/evidence.ts).

When to use: Use build Evidence Path through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function buildEvidencePath(envelope: ConsoleEnvelope): EvidencePathView {
```

## sdk/typescript/packages/console/src/evidence.ts::classifyEvidence

<a id="sdk-sdk-typescript-packages-console-src-evidence-ts-classifyevidence"></a>

Source: [`sdk/typescript/packages/console/src/evidence.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/evidence.ts).

When to use: Use classify Evidence through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function classifyEvidence(manifest: ConsoleManifest, evidence: AnswerLookupView[]): EvidenceClass[] {
```

## sdk/typescript/packages/console/src/evidence.ts::buildAnswerLookup

<a id="sdk-sdk-typescript-packages-console-src-evidence-ts-buildanswerlookup"></a>

Source: [`sdk/typescript/packages/console/src/evidence.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/evidence.ts).

When to use: Use build Answer Lookup through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function buildAnswerLookup(envelope: ConsoleEnvelope): AnswerLookupView {
```

## sdk/typescript/packages/console/src/evidence.ts::renderEvidencePath

<a id="sdk-sdk-typescript-packages-console-src-evidence-ts-renderevidencepath"></a>

Source: [`sdk/typescript/packages/console/src/evidence.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/evidence.ts).

When to use: Use render Evidence Path through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function renderEvidencePath(view: EvidencePathView): string {
```

## sdk/typescript/packages/console/src/evidence.ts::renderAnswerLookup

<a id="sdk-sdk-typescript-packages-console-src-evidence-ts-renderanswerlookup"></a>

Source: [`sdk/typescript/packages/console/src/evidence.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/evidence.ts).

When to use: Use render Answer Lookup through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function renderAnswerLookup(view: AnswerLookupView): string {
```

## sdk/typescript/packages/console/src/html.ts::escapeText

<a id="sdk-sdk-typescript-packages-console-src-html-ts-escapetext"></a>

Source: [`sdk/typescript/packages/console/src/html.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/html.ts).

When to use: Use escape Text through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function escapeText(value: string): string {
```

## sdk/typescript/packages/console/src/html.ts::uriLink

<a id="sdk-sdk-typescript-packages-console-src-html-ts-urilink"></a>

Source: [`sdk/typescript/packages/console/src/html.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/html.ts).

When to use: Use uri Link through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function uriLink(uri: string): string {
```

## sdk/typescript/packages/console/src/html.ts::section

<a id="sdk-sdk-typescript-packages-console-src-html-ts-section"></a>

Source: [`sdk/typescript/packages/console/src/html.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/html.ts).

When to use: Use section through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function section(title: string, body: string): string {
```

## sdk/typescript/packages/console/src/node-transport.ts::clientTransport

<a id="sdk-sdk-typescript-packages-console-src-node-transport-ts-clienttransport"></a>

Source: [`sdk/typescript/packages/console/src/node-transport.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/node-transport.ts).

When to use: Use client Transport through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function clientTransport(client: Client): ConsoleTransport {
```

## sdk/typescript/packages/console/src/overview.ts::OverviewProjection

<a id="sdk-sdk-typescript-packages-console-src-overview-ts-overviewprojection"></a>

Source: [`sdk/typescript/packages/console/src/overview.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/overview.ts).

When to use: Use Overview Projection through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface OverviewProjection {
```

## sdk/typescript/packages/console/src/overview.ts::OverviewView

<a id="sdk-sdk-typescript-packages-console-src-overview-ts-overviewview"></a>

Source: [`sdk/typescript/packages/console/src/overview.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/overview.ts).

When to use: Use Overview View through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface OverviewView {
```

## sdk/typescript/packages/console/src/overview.ts::buildOverview

<a id="sdk-sdk-typescript-packages-console-src-overview-ts-buildoverview"></a>

Source: [`sdk/typescript/packages/console/src/overview.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/overview.ts).

When to use: Use build Overview through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function buildOverview(envelope: ConsoleEnvelope): OverviewView {
```

## sdk/typescript/packages/console/src/overview.ts::renderOverview

<a id="sdk-sdk-typescript-packages-console-src-overview-ts-renderoverview"></a>

Source: [`sdk/typescript/packages/console/src/overview.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/overview.ts).

When to use: Use render Overview through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function renderOverview(view: OverviewView): string {
```

## sdk/typescript/packages/console/src/sources.ts::SourceSummary

<a id="sdk-sdk-typescript-packages-console-src-sources-ts-sourcesummary"></a>

Source: [`sdk/typescript/packages/console/src/sources.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/sources.ts).

When to use: Use Source Summary through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface SourceSummary {
```

## sdk/typescript/packages/console/src/sources.ts::SourceIndexView

<a id="sdk-sdk-typescript-packages-console-src-sources-ts-sourceindexview"></a>

Source: [`sdk/typescript/packages/console/src/sources.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/sources.ts).

When to use: Use Source Index View through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface SourceIndexView {
```

## sdk/typescript/packages/console/src/sources.ts::SourceDetailView

<a id="sdk-sdk-typescript-packages-console-src-sources-ts-sourcedetailview"></a>

Source: [`sdk/typescript/packages/console/src/sources.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/sources.ts).

When to use: Use Source Detail View through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface SourceDetailView {
```

## sdk/typescript/packages/console/src/sources.ts::buildSourceIndex

<a id="sdk-sdk-typescript-packages-console-src-sources-ts-buildsourceindex"></a>

Source: [`sdk/typescript/packages/console/src/sources.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/sources.ts).

When to use: Use build Source Index through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function buildSourceIndex(envelope: ConsoleEnvelope): SourceIndexView {
```

## sdk/typescript/packages/console/src/sources.ts::buildSourceDetail

<a id="sdk-sdk-typescript-packages-console-src-sources-ts-buildsourcedetail"></a>

Source: [`sdk/typescript/packages/console/src/sources.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/sources.ts).

When to use: Use build Source Detail through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function buildSourceDetail(envelope: ConsoleEnvelope): SourceDetailView {
```

## sdk/typescript/packages/console/src/sources.ts::renderSourceIndex

<a id="sdk-sdk-typescript-packages-console-src-sources-ts-rendersourceindex"></a>

Source: [`sdk/typescript/packages/console/src/sources.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/sources.ts).

When to use: Use render Source Index through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function renderSourceIndex(view: SourceIndexView): string {
```

## sdk/typescript/packages/console/src/sources.ts::renderSourceDetail

<a id="sdk-sdk-typescript-packages-console-src-sources-ts-rendersourcedetail"></a>

Source: [`sdk/typescript/packages/console/src/sources.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/sources.ts).

When to use: Use render Source Detail through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function renderSourceDetail(view: SourceDetailView): string {
```

## sdk/typescript/packages/console/src/transport.ts::ConsoleManifest

<a id="sdk-sdk-typescript-packages-console-src-transport-ts-consolemanifest"></a>

Source: [`sdk/typescript/packages/console/src/transport.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/transport.ts).

When to use: Use Console Manifest through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface ConsoleManifest {
```

## sdk/typescript/packages/console/src/transport.ts::ConsoleEnvelope

<a id="sdk-sdk-typescript-packages-console-src-transport-ts-consoleenvelope"></a>

Source: [`sdk/typescript/packages/console/src/transport.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/transport.ts).

When to use: Use Console Envelope through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface ConsoleEnvelope {
```

## sdk/typescript/packages/console/src/transport.ts::ConsoleTransport

<a id="sdk-sdk-typescript-packages-console-src-transport-ts-consoletransport"></a>

Source: [`sdk/typescript/packages/console/src/transport.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/transport.ts).

When to use: Use Console Transport through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface ConsoleTransport {
```

## sdk/typescript/packages/console/src/transport.ts::RestTransportOptions

<a id="sdk-sdk-typescript-packages-console-src-transport-ts-resttransportoptions"></a>

Source: [`sdk/typescript/packages/console/src/transport.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/transport.ts).

When to use: Use Rest Transport Options through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface RestTransportOptions {
```

## sdk/typescript/packages/console/src/transport.ts::restTransport

<a id="sdk-sdk-typescript-packages-console-src-transport-ts-resttransport"></a>

Source: [`sdk/typescript/packages/console/src/transport.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/transport.ts).

When to use: Use rest Transport through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function restTransport(options: RestTransportOptions): ConsoleTransport {
```

## sdk/typescript/packages/console/src/transport.ts::requireItem

<a id="sdk-sdk-typescript-packages-console-src-transport-ts-requireitem"></a>

Source: [`sdk/typescript/packages/console/src/transport.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/transport.ts).

When to use: Use require Item through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function requireItem<T>(envelope: ConsoleEnvelope, surface: string, index = 0): T {
```

## sdk/typescript/packages/console/src/transport.ts::callTool

<a id="sdk-method-sdk-typescript-packages-console-src-transport-ts-609-calltool"></a>

Source: [`sdk/typescript/packages/console/src/transport.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/console/src/transport.ts).

When to use: Use this callTool method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
callTool(verb: string, args: unknown): Promise<ConsoleEnvelope>;
```

## sdk/typescript/packages/engine/index.d.ts::NativeEngine

<a id="sdk-sdk-typescript-packages-engine-index-d-ts-nativeengine"></a>

Source: [`sdk/typescript/packages/engine/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/index.d.ts).

When to use: Use Native Engine through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export declare class NativeEngine {
```

## sdk/typescript/packages/engine/index.d.ts::NativeSession

<a id="sdk-sdk-typescript-packages-engine-index-d-ts-nativesession"></a>

Source: [`sdk/typescript/packages/engine/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/index.d.ts).

When to use: Use Native Session through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export declare class NativeSession {
```

## sdk/typescript/packages/engine/index.d.ts::open

<a id="sdk-method-sdk-typescript-packages-engine-index-d-ts-89-open"></a>

Source: [`sdk/typescript/packages/engine/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/index.d.ts).

When to use: Use this open method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
static open(path: string, configJson: string): Promise<NativeEngine>
```

## sdk/typescript/packages/engine/index.d.ts::session

<a id="sdk-method-sdk-typescript-packages-engine-index-d-ts-160-session"></a>

Source: [`sdk/typescript/packages/engine/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/index.d.ts).

When to use: Use this session method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
session(conversation: string): NativeSession
```

## sdk/typescript/packages/engine/index.d.ts::remember

<a id="sdk-method-sdk-typescript-packages-engine-index-d-ts-247-remember"></a>

Source: [`sdk/typescript/packages/engine/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/index.d.ts).

When to use: Use this remember method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
remember(content: string, kind: string, optionsJson: string): Promise<string>
```

## sdk/typescript/packages/engine/index.d.ts::recall

<a id="sdk-method-sdk-typescript-packages-engine-index-d-ts-327-recall"></a>

Source: [`sdk/typescript/packages/engine/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/index.d.ts).

When to use: Use this recall method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
recall(query: string, limit: number, mode: string, filtersJson: string): Promise<string>
```

## sdk/typescript/packages/engine/index.d.ts::activate

<a id="sdk-method-sdk-typescript-packages-engine-index-d-ts-418-activate"></a>

Source: [`sdk/typescript/packages/engine/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/index.d.ts).

When to use: Use this activate method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
activate(query: string, budgetTokens: number): Promise<Buffer>
```

## sdk/typescript/packages/engine/index.d.ts::checkpoint

<a id="sdk-method-sdk-typescript-packages-engine-index-d-ts-483-checkpoint"></a>

Source: [`sdk/typescript/packages/engine/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/index.d.ts).

When to use: Use this checkpoint method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
checkpoint(turnId: string, blob: Buffer): Promise<string>
```

## sdk/typescript/packages/engine/index.d.ts::intend

<a id="sdk-method-sdk-typescript-packages-engine-index-d-ts-543-intend"></a>

Source: [`sdk/typescript/packages/engine/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/index.d.ts).

When to use: Use this intend method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
intend(inputJson: string): Promise<string>
```

## sdk/typescript/packages/engine/index.d.ts::predict

<a id="sdk-method-sdk-typescript-packages-engine-index-d-ts-588-predict"></a>

Source: [`sdk/typescript/packages/engine/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/index.d.ts).

When to use: Use this predict method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
predict(inputJson: string): Promise<string>
```

## sdk/typescript/packages/engine/index.d.ts::outcome

<a id="sdk-method-sdk-typescript-packages-engine-index-d-ts-634-outcome"></a>

Source: [`sdk/typescript/packages/engine/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/index.d.ts).

When to use: Use this outcome method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
outcome(inputJson: string): Promise<string>
```

## sdk/typescript/packages/engine/index.d.ts::inspect

<a id="sdk-method-sdk-typescript-packages-engine-index-d-ts-680-inspect"></a>

Source: [`sdk/typescript/packages/engine/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/index.d.ts).

When to use: Use this inspect method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
inspect(inputJson: string): Promise<string>
```

## sdk/typescript/packages/engine/index.d.ts::bind

<a id="sdk-method-sdk-typescript-packages-engine-index-d-ts-726-bind"></a>

Source: [`sdk/typescript/packages/engine/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/index.d.ts).

When to use: Use this bind method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
bind(inputJson: string): Promise<string>
```

## sdk/typescript/packages/engine/index.d.ts::attest

<a id="sdk-method-sdk-typescript-packages-engine-index-d-ts-769-attest"></a>

Source: [`sdk/typescript/packages/engine/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/index.d.ts).

When to use: Use this attest method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
attest(inputJson: string): Promise<string>
```

## sdk/typescript/packages/engine/index.d.ts::consolidate

<a id="sdk-method-sdk-typescript-packages-engine-index-d-ts-814-consolidate"></a>

Source: [`sdk/typescript/packages/engine/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/index.d.ts).

When to use: Use this consolidate method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
consolidate(inputJson: string): Promise<string>
```

## sdk/typescript/packages/engine/index.d.ts::believe

<a id="sdk-method-sdk-typescript-packages-engine-index-d-ts-864-believe"></a>

Source: [`sdk/typescript/packages/engine/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/index.d.ts).

When to use: Use this believe method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
believe(inputJson: string): Promise<string>
```

## sdk/typescript/packages/engine/index.d.ts::retract

<a id="sdk-method-sdk-typescript-packages-engine-index-d-ts-910-retract"></a>

Source: [`sdk/typescript/packages/engine/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/index.d.ts).

When to use: Use this retract method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
retract(beliefId: string, provenanceJson: string): Promise<string>
```

## sdk/typescript/packages/engine/index.d.ts::asOf

<a id="sdk-method-sdk-typescript-packages-engine-index-d-ts-979-asof"></a>

Source: [`sdk/typescript/packages/engine/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/index.d.ts).

When to use: Use this asOf method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
asOf(beliefType: string, canonicalIdentity: string, validAtNs?: string | undefined | null, knownAtLsn?: string | undefined | null): Promise<string>
```

## sdk/typescript/packages/engine/native/index.d.ts::NativeEngine

<a id="sdk-sdk-typescript-packages-engine-native-index-d-ts-nativeengine"></a>

Source: [`sdk/typescript/packages/engine/native/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/index.d.ts).

When to use: Use Native Engine through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export declare class NativeEngine {
```

## sdk/typescript/packages/engine/native/index.d.ts::NativeSession

<a id="sdk-sdk-typescript-packages-engine-native-index-d-ts-nativesession"></a>

Source: [`sdk/typescript/packages/engine/native/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/index.d.ts).

When to use: Use Native Session through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export declare class NativeSession {
```

## sdk/typescript/packages/engine/native/index.d.ts::open

<a id="sdk-method-sdk-typescript-packages-engine-native-index-d-ts-89-open"></a>

Source: [`sdk/typescript/packages/engine/native/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/index.d.ts).

When to use: Use this open method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
static open(path: string, configJson: string): Promise<NativeEngine>
```

## sdk/typescript/packages/engine/native/index.d.ts::session

<a id="sdk-method-sdk-typescript-packages-engine-native-index-d-ts-160-session"></a>

Source: [`sdk/typescript/packages/engine/native/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/index.d.ts).

When to use: Use this session method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
session(conversation: string): NativeSession
```

## sdk/typescript/packages/engine/native/index.d.ts::remember

<a id="sdk-method-sdk-typescript-packages-engine-native-index-d-ts-247-remember"></a>

Source: [`sdk/typescript/packages/engine/native/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/index.d.ts).

When to use: Use this remember method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
remember(content: string, kind: string): Promise<string>
```

## sdk/typescript/packages/engine/native/index.d.ts::recall

<a id="sdk-method-sdk-typescript-packages-engine-native-index-d-ts-306-recall"></a>

Source: [`sdk/typescript/packages/engine/native/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/index.d.ts).

When to use: Use this recall method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
recall(query: string, limit: number): Promise<string>
```

## sdk/typescript/packages/engine/native/index.d.ts::activate

<a id="sdk-method-sdk-typescript-packages-engine-native-index-d-ts-362-activate"></a>

Source: [`sdk/typescript/packages/engine/native/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/index.d.ts).

When to use: Use this activate method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
activate(query: string, budgetTokens: number): Promise<Buffer>
```

## sdk/typescript/packages/engine/native/index.d.ts::checkpoint

<a id="sdk-method-sdk-typescript-packages-engine-native-index-d-ts-427-checkpoint"></a>

Source: [`sdk/typescript/packages/engine/native/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/index.d.ts).

When to use: Use this checkpoint method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
checkpoint(turnId: string, blob: Buffer): Promise<string>
```

## sdk/typescript/packages/engine/native/index.d.ts::intend

<a id="sdk-method-sdk-typescript-packages-engine-native-index-d-ts-487-intend"></a>

Source: [`sdk/typescript/packages/engine/native/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/index.d.ts).

When to use: Use this intend method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
intend(inputJson: string): Promise<string>
```

## sdk/typescript/packages/engine/native/index.d.ts::bind

<a id="sdk-method-sdk-typescript-packages-engine-native-index-d-ts-532-bind"></a>

Source: [`sdk/typescript/packages/engine/native/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/index.d.ts).

When to use: Use this bind method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
bind(inputJson: string): Promise<string>
```

## sdk/typescript/packages/engine/native/index.d.ts::attest

<a id="sdk-method-sdk-typescript-packages-engine-native-index-d-ts-575-attest"></a>

Source: [`sdk/typescript/packages/engine/native/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/index.d.ts).

When to use: Use this attest method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
attest(inputJson: string): Promise<string>
```

## sdk/typescript/packages/engine/native/index.d.ts::consolidate

<a id="sdk-method-sdk-typescript-packages-engine-native-index-d-ts-620-consolidate"></a>

Source: [`sdk/typescript/packages/engine/native/index.d.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/index.d.ts).

When to use: Use this consolidate method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
consolidate(inputJson: string): Promise<string>
```

## sdk/typescript/packages/engine/native/src/lib.rs::NativeEngine

<a id="sdk-sdk-typescript-packages-engine-native-src-lib-rs-nativeengine"></a>

Source: [`sdk/typescript/packages/engine/native/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/src/lib.rs).

When to use: Use Native Engine through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```rust
pub struct NativeEngine {
```

## sdk/typescript/packages/engine/native/src/lib.rs::NativeSession

<a id="sdk-sdk-typescript-packages-engine-native-src-lib-rs-nativesession"></a>

Source: [`sdk/typescript/packages/engine/native/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/src/lib.rs).

When to use: Use Native Session through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```rust
pub struct NativeSession {
```

## sdk/typescript/packages/engine/native/src/lib.rs::open

<a id="sdk-method-sdk-typescript-packages-engine-native-src-lib-rs-2071-open"></a>

Source: [`sdk/typescript/packages/engine/native/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/src/lib.rs).

When to use: Use this open method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub async fn open(path: String, config_json: String) -> napi::Result<Self> {
```

## sdk/typescript/packages/engine/native/src/lib.rs::session

<a id="sdk-method-sdk-typescript-packages-engine-native-src-lib-rs-2981-session"></a>

Source: [`sdk/typescript/packages/engine/native/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/src/lib.rs).

When to use: Use this session method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub fn session(&self, conversation: String) -> napi::Result<NativeSession> {
```

## sdk/typescript/packages/engine/native/src/lib.rs::remember

<a id="sdk-method-sdk-typescript-packages-engine-native-src-lib-rs-3727-remember"></a>

Source: [`sdk/typescript/packages/engine/native/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/src/lib.rs).

When to use: Use this remember method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub async fn remember(
```

## sdk/typescript/packages/engine/native/src/lib.rs::recall

<a id="sdk-method-sdk-typescript-packages-engine-native-src-lib-rs-4823-recall"></a>

Source: [`sdk/typescript/packages/engine/native/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/src/lib.rs).

When to use: Use this recall method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub async fn recall(
```

## sdk/typescript/packages/engine/native/src/lib.rs::activate

<a id="sdk-method-sdk-typescript-packages-engine-native-src-lib-rs-5942-activate"></a>

Source: [`sdk/typescript/packages/engine/native/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/src/lib.rs).

When to use: Use this activate method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub async fn activate(&self, query: String, budget_tokens: u32) -> napi::Result<Buffer> {
```

## sdk/typescript/packages/engine/native/src/lib.rs::checkpoint

<a id="sdk-method-sdk-typescript-packages-engine-native-src-lib-rs-6527-checkpoint"></a>

Source: [`sdk/typescript/packages/engine/native/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/src/lib.rs).

When to use: Use this checkpoint method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub async fn checkpoint(&self, turn_id: String, blob: Buffer) -> napi::Result<String> {
```

## sdk/typescript/packages/engine/native/src/lib.rs::intend

<a id="sdk-method-sdk-typescript-packages-engine-native-src-lib-rs-7167-intend"></a>

Source: [`sdk/typescript/packages/engine/native/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/src/lib.rs).

When to use: Use this intend method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub async fn intend(&self, input_json: String) -> napi::Result<String> {
```

## sdk/typescript/packages/engine/native/src/lib.rs::predict

<a id="sdk-method-sdk-typescript-packages-engine-native-src-lib-rs-7475-predict"></a>

Source: [`sdk/typescript/packages/engine/native/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/src/lib.rs).

When to use: Use this predict method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub async fn predict(&self, input_json: String) -> napi::Result<String> {
```

## sdk/typescript/packages/engine/native/src/lib.rs::outcome

<a id="sdk-method-sdk-typescript-packages-engine-native-src-lib-rs-7786-outcome"></a>

Source: [`sdk/typescript/packages/engine/native/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/src/lib.rs).

When to use: Use this outcome method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub async fn outcome(&self, input_json: String) -> napi::Result<String> {
```

## sdk/typescript/packages/engine/native/src/lib.rs::inspect

<a id="sdk-method-sdk-typescript-packages-engine-native-src-lib-rs-8097-inspect"></a>

Source: [`sdk/typescript/packages/engine/native/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/src/lib.rs).

When to use: Use this inspect method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub async fn inspect(&self, input_json: String) -> napi::Result<String> {
```

## sdk/typescript/packages/engine/native/src/lib.rs::bind

<a id="sdk-method-sdk-typescript-packages-engine-native-src-lib-rs-8345-bind"></a>

Source: [`sdk/typescript/packages/engine/native/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/src/lib.rs).

When to use: Use this bind method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub async fn bind(&self, input_json: String) -> napi::Result<String> {
```

## sdk/typescript/packages/engine/native/src/lib.rs::attest

<a id="sdk-method-sdk-typescript-packages-engine-native-src-lib-rs-9029-attest"></a>

Source: [`sdk/typescript/packages/engine/native/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/src/lib.rs).

When to use: Use this attest method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub async fn attest(&self, input_json: String) -> napi::Result<String> {
```

## sdk/typescript/packages/engine/native/src/lib.rs::consolidate

<a id="sdk-method-sdk-typescript-packages-engine-native-src-lib-rs-9274-consolidate"></a>

Source: [`sdk/typescript/packages/engine/native/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/src/lib.rs).

When to use: Use this consolidate method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub async fn consolidate(&self, input_json: String) -> napi::Result<String> {
```

## sdk/typescript/packages/engine/native/src/lib.rs::believe

<a id="sdk-method-sdk-typescript-packages-engine-native-src-lib-rs-9534-believe"></a>

Source: [`sdk/typescript/packages/engine/native/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/src/lib.rs).

When to use: Use this believe method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub async fn believe(&self, input_json: String) -> napi::Result<String> {
```

## sdk/typescript/packages/engine/native/src/lib.rs::retract

<a id="sdk-method-sdk-typescript-packages-engine-native-src-lib-rs-10752-retract"></a>

Source: [`sdk/typescript/packages/engine/native/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/src/lib.rs).

When to use: Use this retract method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub async fn retract(
```

## sdk/typescript/packages/engine/native/src/lib.rs::as_of

<a id="sdk-method-sdk-typescript-packages-engine-native-src-lib-rs-11320-as-of"></a>

Source: [`sdk/typescript/packages/engine/native/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/native/src/lib.rs).

When to use: Use this as_of method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub async fn as_of(
```

## sdk/typescript/packages/engine/src/index.ts::EngineConfig

<a id="sdk-sdk-typescript-packages-engine-src-index-ts-engineconfig"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use Engine Config through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface EngineConfig {
```

## sdk/typescript/packages/engine/src/index.ts::HealthStatus

<a id="sdk-sdk-typescript-packages-engine-src-index-ts-healthstatus"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use Health Status through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type HealthStatus = "semantic_ready" | "semantic_lagging" | "lexical_only" | "unavailable";
```

## sdk/typescript/packages/engine/src/index.ts::RecallMode

<a id="sdk-sdk-typescript-packages-engine-src-index-ts-recallmode"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use Recall Mode through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type RecallMode = "semantic" | "lexical" | "entity" | "temporal" | "near" | "timeline" | "reconstruct";
```

## sdk/typescript/packages/engine/src/index.ts::Retention

<a id="sdk-sdk-typescript-packages-engine-src-index-ts-retention"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use Retention through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type Retention = "current_state" | "daily" | "durable" | "do_not_store";
```

## sdk/typescript/packages/engine/src/index.ts::Sensitivity

<a id="sdk-sdk-typescript-packages-engine-src-index-ts-sensitivity"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use Sensitivity through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type Sensitivity = "public" | "personal" | "secret";
```

## sdk/typescript/packages/engine/src/index.ts::AnchorFacet

<a id="sdk-sdk-typescript-packages-engine-src-index-ts-anchorfacet"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use Anchor Facet through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type AnchorFacet = "path" | "symbol" | "url" | "entity";
```

## sdk/typescript/packages/engine/src/index.ts::Health

<a id="sdk-sdk-typescript-packages-engine-src-index-ts-health"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use Health through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface Health {
```

## sdk/typescript/packages/engine/src/index.ts::Gap

<a id="sdk-sdk-typescript-packages-engine-src-index-ts-gap"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use Gap through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface Gap {
```

## sdk/typescript/packages/engine/src/index.ts::Envelope

<a id="sdk-sdk-typescript-packages-engine-src-index-ts-envelope"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use Envelope through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface Envelope {
```

## sdk/typescript/packages/engine/src/index.ts::RememberOptions

<a id="sdk-sdk-typescript-packages-engine-src-index-ts-rememberoptions"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use Remember Options through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface RememberOptions {
```

## sdk/typescript/packages/engine/src/index.ts::RecallOptions

<a id="sdk-sdk-typescript-packages-engine-src-index-ts-recalloptions"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use Recall Options through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface RecallOptions {
```

## sdk/typescript/packages/engine/src/index.ts::BindInput

<a id="sdk-sdk-typescript-packages-engine-src-index-ts-bindinput"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use Bind Input through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface BindInput {
```

## sdk/typescript/packages/engine/src/index.ts::AttestDisposition

<a id="sdk-sdk-typescript-packages-engine-src-index-ts-attestdisposition"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use Attest Disposition through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type AttestDisposition = "used" | "ignored" | "helpful" | "harmful";
```

## sdk/typescript/packages/engine/src/index.ts::AttestInput

<a id="sdk-sdk-typescript-packages-engine-src-index-ts-attestinput"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use Attest Input through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface AttestInput {
```

## sdk/typescript/packages/engine/src/index.ts::ConsolidateBudget

<a id="sdk-sdk-typescript-packages-engine-src-index-ts-consolidatebudget"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use Consolidate Budget through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface ConsolidateBudget {
```

## sdk/typescript/packages/engine/src/index.ts::ConsolidateInput

<a id="sdk-sdk-typescript-packages-engine-src-index-ts-consolidateinput"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use Consolidate Input through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type ConsolidateInput =
```

## sdk/typescript/packages/engine/src/index.ts::HyperMind

<a id="sdk-sdk-typescript-packages-engine-src-index-ts-hypermind"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use Hyper Mind through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export class HyperMind {
```

## sdk/typescript/packages/engine/src/index.ts::Session

<a id="sdk-sdk-typescript-packages-engine-src-index-ts-session"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use Session through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export class Session {
```

## sdk/typescript/packages/engine/src/index.ts::session

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-621-session"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this session method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
session(conversation: string): NativeSessionHandle;
```

## sdk/typescript/packages/engine/src/index.ts::remember

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-710-remember"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this remember method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
remember(content: string, kind: string, optionsJson: string): Promise<string>;
```

## sdk/typescript/packages/engine/src/index.ts::recall

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-791-recall"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this recall method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
recall(query: string, limit: number, mode: string, filtersJson: string): Promise<string>;
```

## sdk/typescript/packages/engine/src/index.ts::activate

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-883-activate"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this activate method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
activate(query: string, budgetTokens: number): Promise<Uint8Array>;
```

## sdk/typescript/packages/engine/src/index.ts::checkpoint

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-953-checkpoint"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this checkpoint method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
checkpoint(turnId: string, blob: Uint8Array): Promise<string>;
```

## sdk/typescript/packages/engine/src/index.ts::intend

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-1018-intend"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this intend method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
intend(input: string): Promise<string>;
```

## sdk/typescript/packages/engine/src/index.ts::predict

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-1060-predict"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this predict method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
predict(input: string): Promise<string>;
```

## sdk/typescript/packages/engine/src/index.ts::outcome

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-1103-outcome"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this outcome method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
outcome(input: string): Promise<string>;
```

## sdk/typescript/packages/engine/src/index.ts::inspect

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-1146-inspect"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this inspect method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
inspect(input: string): Promise<string>;
```

## sdk/typescript/packages/engine/src/index.ts::bind

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-1189-bind"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this bind method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
bind(input: string): Promise<string>;
```

## sdk/typescript/packages/engine/src/index.ts::attest

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-1229-attest"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this attest method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
attest(input: string): Promise<string>;
```

## sdk/typescript/packages/engine/src/index.ts::consolidate

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-1271-consolidate"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this consolidate method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
consolidate(input: string): Promise<string>;
```

## sdk/typescript/packages/engine/src/index.ts::believe

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-1318-believe"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this believe method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
believe(input: string): Promise<string>;
```

## sdk/typescript/packages/engine/src/index.ts::retract

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-1361-retract"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this retract method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
retract(beliefId: string, provenance: string): Promise<string>;
```

## sdk/typescript/packages/engine/src/index.ts::asOf

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-1427-asof"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this asOf method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
asOf(
```

## sdk/typescript/packages/engine/src/index.ts::open

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-4067-open"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this open method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
static async open(directory: string, config: EngineConfig): Promise<HyperMind> {
```

## sdk/typescript/packages/engine/src/index.ts::session

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-4424-session"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this session method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
session(conversation: string): Session {
```

## sdk/typescript/packages/engine/src/index.ts::constructor

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-4570-constructor"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this constructor method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
constructor(
```

## sdk/typescript/packages/engine/src/index.ts::remember

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-4678-remember"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this remember method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async remember(
```

## sdk/typescript/packages/engine/src/index.ts::recall

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-5213-recall"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this recall method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async recall(query: string, options: number | RecallOptions = {}): Promise<Envelope> {
```

## sdk/typescript/packages/engine/src/index.ts::activate

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-5605-activate"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this activate method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async activate(query: string, budgetTokens: number): Promise<Bundle> {
```

## sdk/typescript/packages/engine/src/index.ts::checkpoint

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-5756-checkpoint"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this checkpoint method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async checkpoint(turnId: string, blob: Uint8Array): Promise<bigint> {
```

## sdk/typescript/packages/engine/src/index.ts::intend

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-5896-intend"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this intend method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async intend(action: IntendAction): Promise<Envelope> {
```

## sdk/typescript/packages/engine/src/index.ts::predict

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-6091-predict"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this predict method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async predict(input: PredictInput): Promise<Envelope> {
```

## sdk/typescript/packages/engine/src/index.ts::outcome

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-6287-outcome"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this outcome method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async outcome(input: OutcomeInput): Promise<Envelope> {
```

## sdk/typescript/packages/engine/src/index.ts::inspect

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-6483-inspect"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this inspect method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async inspect(input: InspectInput = {}): Promise<Envelope> {
```

## sdk/typescript/packages/engine/src/index.ts::bind

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-6633-bind"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this bind method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async bind(input: BindInput): Promise<Envelope> {
```

## sdk/typescript/packages/engine/src/index.ts::attest

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-7155-attest"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this attest method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async attest(input: AttestInput): Promise<Envelope> {
```

## sdk/typescript/packages/engine/src/index.ts::consolidate

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-7459-consolidate"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this consolidate method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async consolidate(input: ConsolidateInput): Promise<Envelope> {
```

## sdk/typescript/packages/engine/src/index.ts::believe

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-8191-believe"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this believe method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async believe(input: BelieveInput & { runId?: string }): Promise<Envelope> {
```

## sdk/typescript/packages/engine/src/index.ts::retract

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-8802-retract"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this retract method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async retract(beliefId: string, provenance: BeliefProvenance[]): Promise<Envelope> {
```

## sdk/typescript/packages/engine/src/index.ts::asOf

<a id="sdk-method-sdk-typescript-packages-engine-src-index-ts-9017-asof"></a>

Source: [`sdk/typescript/packages/engine/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/engine/src/index.ts).

When to use: Use this asOf method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
async asOf(
```

## sdk/typescript/packages/migrate/src/cli.ts::main

<a id="sdk-sdk-typescript-packages-migrate-src-cli-ts-main"></a>

Source: [`sdk/typescript/packages/migrate/src/cli.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/migrate/src/cli.ts).

When to use: Use main through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function main(args: string[]): void {
```

## sdk/typescript/packages/migrate/src/index.ts::FORMAT

<a id="sdk-sdk-typescript-packages-migrate-src-index-ts-format"></a>

Source: [`sdk/typescript/packages/migrate/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/migrate/src/index.ts).

When to use: Use FORMAT through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export const FORMAT = 'hypermind.cortex-import.v1';
```

## sdk/typescript/packages/migrate/src/index.ts::KINDS

<a id="sdk-sdk-typescript-packages-migrate-src-index-ts-kinds"></a>

Source: [`sdk/typescript/packages/migrate/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/migrate/src/index.ts).

When to use: Use KINDS through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export const KINDS = ['observations', 'memories', 'edges', 'beliefs', 'ops', 'signals', 'generic'] as const;
```

## sdk/typescript/packages/migrate/src/index.ts::Kind

<a id="sdk-sdk-typescript-packages-migrate-src-index-ts-kind"></a>

Source: [`sdk/typescript/packages/migrate/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/migrate/src/index.ts).

When to use: Use Kind through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type Kind = typeof KINDS[number];
```

## sdk/typescript/packages/migrate/src/index.ts::SourceRecord

<a id="sdk-sdk-typescript-packages-migrate-src-index-ts-sourcerecord"></a>

Source: [`sdk/typescript/packages/migrate/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/migrate/src/index.ts).

When to use: Use Source Record through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface SourceRecord {
```

## sdk/typescript/packages/migrate/src/index.ts::Manifest

<a id="sdk-sdk-typescript-packages-migrate-src-index-ts-manifest"></a>

Source: [`sdk/typescript/packages/migrate/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/migrate/src/index.ts).

When to use: Use Manifest through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface Manifest {
```

## sdk/typescript/packages/migrate/src/index.ts::ExportOptions

<a id="sdk-sdk-typescript-packages-migrate-src-index-ts-exportoptions"></a>

Source: [`sdk/typescript/packages/migrate/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/migrate/src/index.ts).

When to use: Use Export Options through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface ExportOptions {source: string; format: 'json' | 'sqlite'; namespace?: string}
```

## sdk/typescript/packages/migrate/src/index.ts::exportCortex

<a id="sdk-sdk-typescript-packages-migrate-src-index-ts-exportcortex"></a>

Source: [`sdk/typescript/packages/migrate/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/migrate/src/index.ts).

When to use: Use export Cortex through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function exportCortex(options: ExportOptions): {manifest: Manifest; records: SourceRecord[]; jsonl: string} {
```

## sdk/typescript/packages/render/src/index.ts::TIERS

<a id="sdk-sdk-typescript-packages-render-src-index-ts-tiers"></a>

Source: [`sdk/typescript/packages/render/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/render/src/index.ts).

When to use: Use TIERS through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export const TIERS = [
```

## sdk/typescript/packages/render/src/index.ts::Tier

<a id="sdk-sdk-typescript-packages-render-src-index-ts-tier"></a>

Source: [`sdk/typescript/packages/render/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/render/src/index.ts).

When to use: Use Tier through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type Tier = (typeof TIERS)[number];
```

## sdk/typescript/packages/render/src/index.ts::Authority

<a id="sdk-sdk-typescript-packages-render-src-index-ts-authority"></a>

Source: [`sdk/typescript/packages/render/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/render/src/index.ts).

When to use: Use Authority through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export type Authority =
```

## sdk/typescript/packages/render/src/index.ts::BundleItem

<a id="sdk-sdk-typescript-packages-render-src-index-ts-bundleitem"></a>

Source: [`sdk/typescript/packages/render/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/render/src/index.ts).

When to use: Use Bundle Item through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface BundleItem {
```

## sdk/typescript/packages/render/src/index.ts::BundleSection

<a id="sdk-sdk-typescript-packages-render-src-index-ts-bundlesection"></a>

Source: [`sdk/typescript/packages/render/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/render/src/index.ts).

When to use: Use Bundle Section through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface BundleSection {
```

## sdk/typescript/packages/render/src/index.ts::Bundle

<a id="sdk-sdk-typescript-packages-render-src-index-ts-bundle"></a>

Source: [`sdk/typescript/packages/render/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/render/src/index.ts).

When to use: Use Bundle through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface Bundle {
```

## sdk/typescript/packages/render/src/index.ts::PromptItem

<a id="sdk-sdk-typescript-packages-render-src-index-ts-promptitem"></a>

Source: [`sdk/typescript/packages/render/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/render/src/index.ts).

When to use: Use Prompt Item through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface PromptItem {
```

## sdk/typescript/packages/render/src/index.ts::PromptSection

<a id="sdk-sdk-typescript-packages-render-src-index-ts-promptsection"></a>

Source: [`sdk/typescript/packages/render/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/render/src/index.ts).

When to use: Use Prompt Section through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface PromptSection {
```

## sdk/typescript/packages/render/src/index.ts::RenderedPrompt

<a id="sdk-sdk-typescript-packages-render-src-index-ts-renderedprompt"></a>

Source: [`sdk/typescript/packages/render/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/render/src/index.ts).

When to use: Use Rendered Prompt through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface RenderedPrompt {
```

## sdk/typescript/packages/render/src/index.ts::RenderOptions

<a id="sdk-sdk-typescript-packages-render-src-index-ts-renderoptions"></a>

Source: [`sdk/typescript/packages/render/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/render/src/index.ts).

When to use: Use Render Options through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export interface RenderOptions {
```

## sdk/typescript/packages/render/src/index.ts::ActivationSafetyError

<a id="sdk-sdk-typescript-packages-render-src-index-ts-activationsafetyerror"></a>

Source: [`sdk/typescript/packages/render/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/render/src/index.ts).

When to use: Use Activation Safety Error through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export class ActivationSafetyError extends Error {}
```

## sdk/typescript/packages/render/src/index.ts::isReconstruction

<a id="sdk-sdk-typescript-packages-render-src-index-ts-isreconstruction"></a>

Source: [`sdk/typescript/packages/render/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/render/src/index.ts).

When to use: Use is Reconstruction through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function isReconstruction(content: string): boolean {
```

## sdk/typescript/packages/render/src/index.ts::assertRememberable

<a id="sdk-sdk-typescript-packages-render-src-index-ts-assertrememberable"></a>

Source: [`sdk/typescript/packages/render/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/render/src/index.ts).

When to use: Use assert Rememberable through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function assertRememberable(content: string): void {
```

## sdk/typescript/packages/render/src/index.ts::render

<a id="sdk-sdk-typescript-packages-render-src-index-ts-render"></a>

Source: [`sdk/typescript/packages/render/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/render/src/index.ts).

When to use: Use render through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```typescript
export function render(bundle: Bundle, options: RenderOptions = {}): RenderedPrompt {
```

## sdk/typescript/packages/render/src/index.ts::return

<a id="sdk-method-sdk-typescript-packages-render-src-index-ts-1826-return"></a>

Source: [`sdk/typescript/packages/render/src/index.ts`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/sdk/typescript/packages/render/src/index.ts).

When to use: Use this return method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```typescript
return (
```

## crates/hm-serve/src/embedded.rs::EmbeddedConfig

<a id="sdk-crates-hm-serve-src-embedded-rs-embeddedconfig"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use Embedded Config through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```rust
pub struct EmbeddedConfig {
```

## crates/hm-serve/src/embedded.rs::HyperMind

<a id="sdk-crates-hm-serve-src-embedded-rs-hypermind"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use Hyper Mind through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```rust
pub struct HyperMind {
```

## crates/hm-serve/src/embedded.rs::Actor

<a id="sdk-crates-hm-serve-src-embedded-rs-actor"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use Actor through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```rust
pub struct Actor {
```

## crates/hm-serve/src/embedded.rs::Session

<a id="sdk-crates-hm-serve-src-embedded-rs-session"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use Session through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```rust
pub struct Session {
```

## crates/hm-serve/src/embedded.rs::MemoryKind

<a id="sdk-crates-hm-serve-src-embedded-rs-memorykind"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use Memory Kind through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```rust
pub enum MemoryKind {
```

## crates/hm-serve/src/embedded.rs::RenderModel

<a id="sdk-crates-hm-serve-src-embedded-rs-rendermodel"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use Render Model through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```rust
pub enum RenderModel {
```

## crates/hm-serve/src/embedded.rs::RenderAuthority

<a id="sdk-crates-hm-serve-src-embedded-rs-renderauthority"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use Render Authority through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```rust
pub enum RenderAuthority {
```

## crates/hm-serve/src/embedded.rs::RenderedItem

<a id="sdk-crates-hm-serve-src-embedded-rs-rendereditem"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use Rendered Item through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```rust
pub struct RenderedItem {
```

## crates/hm-serve/src/embedded.rs::RenderedSection

<a id="sdk-crates-hm-serve-src-embedded-rs-renderedsection"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use Rendered Section through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```rust
pub struct RenderedSection {
```

## crates/hm-serve/src/embedded.rs::RenderedBundle

<a id="sdk-crates-hm-serve-src-embedded-rs-renderedbundle"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use Rendered Bundle through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.

Do not use: Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.


```rust
pub struct RenderedBundle {
```

## crates/hm-serve/src/embedded.rs::open

<a id="sdk-method-crates-hm-serve-src-embedded-rs-1880-open"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use this open method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub async fn open(path: impl AsRef<Path>, config: EmbeddedConfig) -> Result<Self, Error> {
```

## crates/hm-serve/src/embedded.rs::actor

<a id="sdk-method-crates-hm-serve-src-embedded-rs-2382-actor"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use this actor method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub fn actor(&self) -> Actor {
```

## crates/hm-serve/src/embedded.rs::session

<a id="sdk-method-crates-hm-serve-src-embedded-rs-2467-session"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use this session method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub fn session(&self, conversation: impl AsRef<str>) -> Session {
```

## crates/hm-serve/src/embedded.rs::id

<a id="sdk-method-crates-hm-serve-src-embedded-rs-2616-id"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use this id method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub fn id(&self) -> ActorId {
```

## crates/hm-serve/src/embedded.rs::session

<a id="sdk-method-crates-hm-serve-src-embedded-rs-2701-session"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use this session method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub fn session(&self, conversation: impl AsRef<str>) -> Session {
```

## crates/hm-serve/src/embedded.rs::shutdown

<a id="sdk-method-crates-hm-serve-src-embedded-rs-2911-shutdown"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use this shutdown method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub async fn shutdown(self) -> Result<(), Error> {
```

## crates/hm-serve/src/embedded.rs::conversation

<a id="sdk-method-crates-hm-serve-src-embedded-rs-3044-conversation"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use this conversation method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub const fn conversation(&self) -> ConversationId {
```

## crates/hm-serve/src/embedded.rs::remember

<a id="sdk-method-crates-hm-serve-src-embedded-rs-3133-remember"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use this remember method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub async fn remember(&self, kind: MemoryKind, content: impl AsRef<str>) -> Result<LSN, Error> {
```

## crates/hm-serve/src/embedded.rs::recall

<a id="sdk-method-crates-hm-serve-src-embedded-rs-4829-recall"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use this recall method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub async fn recall(
```

## crates/hm-serve/src/embedded.rs::timeline

<a id="sdk-method-crates-hm-serve-src-embedded-rs-5152-timeline"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use this timeline method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub async fn timeline(&self, since_lsn: LSN, limit: usize) -> Result<Vec<RecallItem>, Error> {
```

## crates/hm-serve/src/embedded.rs::activate

<a id="sdk-method-crates-hm-serve-src-embedded-rs-5476-activate"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use this activate method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub async fn activate(
```

## crates/hm-serve/src/embedded.rs::render

<a id="sdk-method-crates-hm-serve-src-embedded-rs-5965-render"></a>

Source: [`crates/hm-serve/src/embedded.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/embedded.rs).

When to use: Use this render method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.

Do not use: Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.


```rust
pub fn render(bundle: &ActivationBundle, model: RenderModel) -> Result<RenderedBundle, Error> {
```
