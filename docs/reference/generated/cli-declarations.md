# CLI command declarations

Generated from the current command parser. The human CLI guide explains execution mode, authority, and examples; these declarations expose the exact option names, required fields, defaults, and value enums accepted by the checked-in source. `--json` is global. Commands not declared here must not be advertised as available.

## crates/hm-cli/src/actors.rs command/options

<a id="cli-crates-hm-cli-src-actors-rs"></a>

Source: [`crates/hm-cli/src/actors.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cli/src/actors.rs).

When to use: Use these options when invoking the matching hm subcommand; select daemon or embedded access deliberately.

Do not use: Do not open an actor directory from an embedded command while a daemon owns it, or expose remote listeners without both mutual TLS and capabilities.


```rust
pub enum Command {
    Add {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        actor: u16,
    },
    List {
        #[arg(long)]
        config: PathBuf,
    },
    RotateKeys {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        new_kek_file: PathBuf,
    },
    Shred {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        actor: u16,
        #[arg(long)]
        confirm: u16,
    },
}
```

## crates/hm-cli/src/archive.rs command/options

<a id="cli-crates-hm-cli-src-archive-rs"></a>

Source: [`crates/hm-cli/src/archive.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cli/src/archive.rs).

When to use: Use these options when invoking the matching hm subcommand; select daemon or embedded access deliberately.

Do not use: Do not open an actor directory from an embedded command while a daemon owns it, or expose remote listeners without both mutual TLS and capabilities.


```rust
pub enum Command {
    Pack {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Unpack {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Verify {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        public_key: Option<String>,
    },
}
```

## crates/hm-cli/src/consolidate.rs command/options

<a id="cli-crates-hm-cli-src-consolidate-rs"></a>

Source: [`crates/hm-cli/src/consolidate.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cli/src/consolidate.rs).

When to use: Use these options when invoking the matching hm subcommand; select daemon or embedded access deliberately.

Do not use: Do not open an actor directory from an embedded command while a daemon owns it, or expose remote listeners without both mutual TLS and capabilities.


```rust
pub enum Command {
    Run {
        #[arg(long)]
        config: PathBuf,
        #[arg(long, value_enum)]
        mode: Mode,
        #[arg(long, default_value = "actor")]
        scope: String,
        #[arg(long)]
        cadence_key: String,
        #[arg(long)]
        max_llm_calls: u64,
        #[arg(long)]
        max_tokens: u64,
        #[arg(long)]
        max_microusd: u64,
        #[arg(long)]
        max_wall_ms: u64,
    },
    List {
        #[arg(long)]
        config: PathBuf,
    },
    Retract {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        reason: String,
    },
}
```

## crates/hm-cli/src/lib.rs command/options

<a id="cli-crates-hm-cli-src-lib-rs"></a>

Source: [`crates/hm-cli/src/lib.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cli/src/lib.rs).

When to use: Use these options when invoking the matching hm subcommand; select daemon or embedded access deliberately.

Do not use: Do not open an actor directory from an embedded command while a daemon owns it, or expose remote listeners without both mutual TLS and capabilities.


```rust
enum Command {
    Init {
        #[arg(long)]
        path: PathBuf,
        #[arg(long, default_value_t = 1)]
        actor: u16,
        #[arg(long)]
        if_missing: bool,
    },
    Serve {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        init_if_missing: bool,
        #[command(flatten)]
        remote: Box<serve::RemoteOptions>,
        #[arg(long, value_enum)]
        model: Option<models::Choice>,
        #[arg(long)]
        models_directory: Option<PathBuf>,
        #[arg(long)]
        telemetry_file: Option<PathBuf>,
        #[arg(long)]
        telemetry_service: Option<String>,
    },
    Doctor {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        models_directory: Option<PathBuf>,
        #[arg(long)]
        require_healthy: bool,
    },
    Actor {
        #[command(subcommand)]
        command: actors::Command,
    },
    Models {
        #[command(subcommand)]
        command: models::Command,
    },
    Bench {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        query: String,
        #[arg(long, default_value_t = 100)]
        iterations: usize,
    },
    Export {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Tui {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        conversation: String,
        #[arg(long, default_value = "")]
        query: String,
        #[arg(long, default_value_t = 4096)]
        budget_tokens: usize,
        #[arg(long, default_value_t = 1000)]
        interval_ms: u64,
        #[arg(long)]
        frames: Option<usize>,
    },
    Remember {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        conversation: String,
        #[arg(long)]
        content: String,
        #[arg(long, value_enum, default_value_t = MessageKind::User)]
        kind: MessageKind,
        #[arg(long)]
        embedded: bool,
    },
    Recall {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        query: String,
        #[arg(long, default_value_t = 32)]
        limit: usize,
        #[arg(long)]
        embedded: bool,
    },
    Activate {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        conversation: String,
        #[arg(long, default_value = "")]
        query: String,
        #[arg(long)]
        budget_tokens: usize,
        #[arg(long)]
        embedded: bool,
    },
    Consolidate {
        #[command(subcommand)]
        command: consolidate::Command,
    },
    Import {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        input: PathBuf,
    },
    Archive {
        #[command(subcommand)]
        command: archive::Command,
    },
    Verify {
        actor_directory: PathBuf,
        actor: u16,
        checkpoint: PathBuf,
        public_key: String,
    },
}
```

## crates/hm-cli/src/models.rs command/options

<a id="cli-crates-hm-cli-src-models-rs"></a>

Source: [`crates/hm-cli/src/models.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cli/src/models.rs).

When to use: Use these options when invoking the matching hm subcommand; select daemon or embedded access deliberately.

Do not use: Do not open an actor directory from an embedded command while a daemon owns it, or expose remote listeners without both mutual TLS and capabilities.


```rust
pub enum Command {
    Ensure {
        #[arg(long)]
        directory: PathBuf,
        #[arg(long, value_enum)]
        model: Choice,
    },
    List {
        #[arg(long)]
        directory: PathBuf,
    },
}
```

## crates/hm-cli/src/serve.rs command/options

<a id="cli-crates-hm-cli-src-serve-rs"></a>

Source: [`crates/hm-cli/src/serve.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-cli/src/serve.rs).

When to use: Use these options when invoking the matching hm subcommand; select daemon or embedded access deliberately.

Do not use: Do not open an actor directory from an embedded command while a daemon owns it, or expose remote listeners without both mutual TLS and capabilities.


```rust
pub(crate) struct RemoteOptions {
    #[arg(long, requires = "tls_source")]
    grpc_bind: Option<SocketAddr>,
    #[arg(long, requires = "tls_source")]
    grpc_admin_bind: Option<SocketAddr>,
    #[arg(long, requires = "tls_source")]
    rest_bind: Option<SocketAddr>,
    #[arg(long, requires = "tls_source")]
    rest_admin_bind: Option<SocketAddr>,
    #[arg(long, group = "tls_source", requires_all = ["tls_key", "tls_client_ca"])]
    tls_cert: Option<PathBuf>,
    #[arg(long, requires_all = ["tls_cert", "tls_client_ca"])]
    tls_key: Option<PathBuf>,
    #[arg(long, requires_all = ["tls_cert", "tls_key"])]
    tls_client_ca: Option<PathBuf>,
    #[arg(long, group = "tls_source", conflicts_with_all = ["tls_cert", "tls_key", "tls_client_ca"])]
    tls_from_env: bool,
    #[arg(long, requires = "rest_bind")]
    console_directory: Option<PathBuf>,
}
```
