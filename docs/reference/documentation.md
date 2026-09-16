# Documentation coverage contract

When to use: change a public type, schema, tool, SDK declaration, prompt, or CLI option and keep its documentation synchronized with the implementation.

Do not use: generated catalogs as a substitute for behavior tests, source availability as a release claim, or a documentation pass as proof of benchmark quality.

Regenerate checked-in catalogs from the repository root:

```sh
node docs/tools/catalog.mjs --write
```

Then run the task’s gate:

```sh
mdbook build docs
cargo run -p hm-eval -- docs-gate
```

The generator inventories canonical FlatBuffers/protobuf declarations, authored public Rust types throughout `crates/*/src`, macro-generated core IDs, MCP dispatch verbs, authored SDK exports/methods, CLI declarations, configuration keys, and versioned prompt files. Every catalog entry includes when-to-use and do-not-use guidance and its actual source declaration. MCP operation guidance/examples are reviewed in `docs/tools/usage.json` and unknown verbs fail generation.

Generated FlatBuffers builders/accessors/object wrappers and generated SDK wire code are represented by the canonical schema catalog, not repeated as independent APIs. Internal test functions are not SDK APIs. Authored public declarations in internal crate modules are conservatively included; listing one does not make its module externally exported or promise API stability.

The read-only gate reconstructs the catalogs and requires exact equality, validates required chapters/ADRs and all ten README files, requires chapter reachability from the mdBook summary, and checks local Markdown links and heading anchors in the root READMEs, `docs/`, `deploy/`, and authored SDK documentation. Dependency trees, build outputs, caches, and generated SDK wire trees are excluded. Source changes require regeneration; missing coverage and broken links fail rather than being skipped. Links under the exact canonical `https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/` prefix are checked against this checkout without HTTP requests, allowing standalone mdBook pages to link to repository files outside the book. Other external HTTP links are not network-checked. Translations are manually authored, not language-detected by the gate.

`coverage.json` beside the generated catalogs records each symbol’s source identity. Do not edit generated pages or that manifest directly; update source/guidance and regenerate. Review changes before publication, especially when they reflect newly added interfaces whose behavioral qualification is pending.
