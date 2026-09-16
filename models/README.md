![HyperMind](../spec/readme_img.png)

# Embedding models

HyperMind supports the pinned int8 ONNX exports in `manifest.toml`. `ModelStore::ensure` downloads a missing model and tokenizer on first use, verifies each SHA-256 digest before publishing it, and re-verifies cached files on every open. Revisions are immutable commit identifiers; changing a revision creates a separate embedding-space generation.

Prepare pinned artifacts explicitly with `hm models ensure --directory ./models-cache --model bge-small --json` (or `--model nomic`). `hm models list --directory ./models-cache --json` checks their digests without downloading. `hm serve --config ./state/hypermind.conf --model bge-small --models-directory ./models-cache` performs the same first-use preparation before starting the daemon; ordinary `serve` does not download models. Artifact preparation alone does not configure a semantic embedding provider or establish that local inference is healthy.

Local inference uses Hugging Face `tokenizers` and ONNX Runtime through `ort`. Set `ORT_DYLIB_PATH` to the deployment's ONNX Runtime shared library. Nomic query and document prefixes and the BGE retrieval-query prefix are applied before tokenization. Output token vectors are attention-mask mean pooled and L2-normalised.

Remote OpenAI, Voyage, Vertex and Ollama adapters share the same `Embedder` contract. Provider credentials remain request headers and are never included in a space identity or cache key. Tests use recorded request/response fixtures. The optional Centra live test reads `CENTRA_GATEWAY_URL` and `CENTRA_GATEWAY_API_KEY` and can be run explicitly:

```sh
set -a
. ./.env
set +a
cargo test -p hm-embed --test live_provider -- --ignored
```

`HashFeatureEmbedder` is a deterministic lexical baseline, not a semantic encoder. Its encoder and report label are always `lexical_only`; results from it must not be blended with a semantic encoder generation.
