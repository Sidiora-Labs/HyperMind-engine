# HyperMind for Python

An async API for the native Rust memory engine and an mTLS client for the HyperMind daemon. Both expose the same `Session` tools; native mode owns the actor directory, while remote mode talks to the daemon that owns it.

## Build from this repository

Python 3.10+, the repository's Rust toolchain, and its native build prerequisites are required. From the repository root:

```sh
python3 -m venv .venv
. .venv/bin/activate
python -m pip install 'maturin>=1.9,<2' patchelf
maturin develop --manifest-path sdk/python/native/Cargo.toml
```

`patchelf` is needed for Linux shared-library packaging. This builds the local source package; published PyPI wheels are not yet a release claim.

## Embedded memory

Supply your own persistent actor identity and keys. Keep keys outside source control. No provider is enabled by default.

```python
import asyncio
import os
from hypermind import Engine, render

async def main():
    async with await Engine.open(
        "./memory-data",
        actor=7,
        user_hex=os.environ["HM_USER_HEX"],  # 16 bytes, hex-encoded
        kek_hex=os.environ["HM_KEK_HEX"],    # 32 bytes, hex-encoded
    ) as engine:
        session = engine.session("deployment")
        receipt = await session.remember("The deployment region is eu-central-1.")
        if not receipt["ok"]:
            raise RuntimeError(receipt)
        print(await session.recall("deployment region"))
        bundle = await session.activate("deployment region", budget_tokens=4096)
        print(render(bundle))

asyncio.run(main())
```

Use the same path, actor, user, and key to reopen the stored memory. Do not run another embedded engine, MCP server, or daemon against that actor directory concurrently.

## Remote memory

Start a daemon with a gRPC listener and provision certificates as described in the [deployment guide](../../docs/guides/deployment.md). Both a trusted client certificate and an actor capability are required:

```python
import os
from pathlib import Path
from hypermind import Client

async def recall_remote():
    async with Client(
        "localhost:7443",
        token=bytes.fromhex(os.environ["HM_ACTOR_TOKEN_HEX"]),
        ca=Path("client-ca.pem").read_bytes(),
        certificate=Path("client.pem").read_bytes(),
        private_key=Path("client.key").read_bytes(),
    ) as client:
        return await client.session("deployment").recall("deployment region")
```

The low-level client also exposes checkpoints, explicit recovery of pending sequenced writes, as-of queries, event subscription, and admin deletion. Unsequenced tool mutations are never automatically retried. Inspect `HyperMindError.effect_state` before deciding whether to retry a failed mutation.

## Safety and tests

`render` produces untrusted memory sections, never system or developer instructions. It drops items without provenance and supports same-turn exclusion. `Session.remember` rejects text labeled `RECONSTRUCTION`.

The tests use real native storage and a real TLS daemon. Build `hm` and install test dependencies first:

```sh
cargo build -p hm-cli --bin hm
python -m pip install 'pytest>=8.4,<9' 'pytest-asyncio>=1.2,<2'
python -m pytest sdk/python/tests -q
```

OpenSSL must be available for test certificates. Regenerate protocol bindings with `python sdk/python/tools/generate.py` after installing `grpcio-tools==1.75.1` and the FlatBuffers compiler.

See the [SDK reference](../../docs/reference/sdks.md) and [API catalog](../../docs/reference/generated/sdk-api.md) for the current shared contract. Licensed under [Apache-2.0](../../LICENSE).
