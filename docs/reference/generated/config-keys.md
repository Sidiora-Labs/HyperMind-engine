# Configuration key declarations

Generated from the real configuration loader. The human configuration page explains provisioning, defaults, secret handling, and provider settings. Unknown config-file keys are rejected.

## socket

<a id="config-socket"></a>

Source: [`crates/hm-serve/src/config.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/config.rs).

When to use: Use the socket key to configure the corresponding ServerConfig field before starting the actor owner. The loader expression below is authoritative.

Do not use: Do not add unrecognized keys, expose key/capability values, or use relative paths without controlling the process working directory.


```rust
"socket" => socket_path = Some(PathBuf::from(value)),
```

## data

<a id="config-data"></a>

Source: [`crates/hm-serve/src/config.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/config.rs).

When to use: Use the data key to configure the corresponding ServerConfig field before starting the actor owner. The loader expression below is authoritative.

Do not use: Do not add unrecognized keys, expose key/capability values, or use relative paths without controlling the process working directory.


```rust
"data" => data_directory = Some(PathBuf::from(value)),
```

## user

<a id="config-user"></a>

Source: [`crates/hm-serve/src/config.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/config.rs).

When to use: Use the user key to configure the corresponding ServerConfig field before starting the actor owner. The loader expression below is authoritative.

Do not use: Do not add unrecognized keys, expose key/capability values, or use relative paths without controlling the process working directory.


```rust
"user" => user = Some(decode_hex(value)?),
```

## kek

<a id="config-kek"></a>

Source: [`crates/hm-serve/src/config.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/config.rs).

When to use: Use the kek key to configure the corresponding ServerConfig field before starting the actor owner. The loader expression below is authoritative.

Do not use: Do not add unrecognized keys, expose key/capability values, or use relative paths without controlling the process working directory.


```rust
"kek" => kek = Some(decode_hex(value)?),
```

## admin_token

<a id="config-admin-token"></a>

Source: [`crates/hm-serve/src/config.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/config.rs).

When to use: Use the admin_token key to configure the corresponding ServerConfig field before starting the actor owner. The loader expression below is authoritative.

Do not use: Do not add unrecognized keys, expose key/capability values, or use relative paths without controlling the process working directory.


```rust
"admin_token" => admin_token = Some(decode_hex(value)?),
```

## actor

<a id="config-actor"></a>

Source: [`crates/hm-serve/src/config.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/config.rs).

When to use: Use the actor key to configure the corresponding ServerConfig field before starting the actor owner. The loader expression below is authoritative.

Do not use: Do not add unrecognized keys, expose key/capability values, or use relative paths without controlling the process working directory.


```rust
"actor" => {
```

## maximum_connections

<a id="config-maximum-connections"></a>

Source: [`crates/hm-serve/src/config.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/config.rs).

When to use: Use the maximum_connections key to configure the corresponding ServerConfig field before starting the actor owner. The loader expression below is authoritative.

Do not use: Do not add unrecognized keys, expose key/capability values, or use relative paths without controlling the process working directory.


```rust
"maximum_connections" => maximum_connections = parse_size(value, 1, 4096)?,
```

## maximum_output_frames

<a id="config-maximum-output-frames"></a>

Source: [`crates/hm-serve/src/config.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/config.rs).

When to use: Use the maximum_output_frames key to configure the corresponding ServerConfig field before starting the actor owner. The loader expression below is authoritative.

Do not use: Do not add unrecognized keys, expose key/capability values, or use relative paths without controlling the process working directory.


```rust
"maximum_output_frames" => maximum_output_frames = parse_size(value, 1, 4096)?,
```

## maximum_output_bytes

<a id="config-maximum-output-bytes"></a>

Source: [`crates/hm-serve/src/config.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/config.rs).

When to use: Use the maximum_output_bytes key to configure the corresponding ServerConfig field before starting the actor owner. The loader expression below is authoritative.

Do not use: Do not add unrecognized keys, expose key/capability values, or use relative paths without controlling the process working directory.


```rust
"maximum_output_bytes" => {
```

## projection_map_bytes

<a id="config-projection-map-bytes"></a>

Source: [`crates/hm-serve/src/config.rs`](https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/crates/hm-serve/src/config.rs).

When to use: Use the projection_map_bytes key to configure the corresponding ServerConfig field before starting the actor owner. The loader expression below is authoritative.

Do not use: Do not add unrecognized keys, expose key/capability values, or use relative paths without controlling the process working directory.


```rust
"projection_map_bytes" => {
```
