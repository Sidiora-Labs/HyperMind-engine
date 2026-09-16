# Console

When to use: inspect one actor's memory through a browser or a Node process — what was ingested, which records supported an answer, what the daemon says about permissions, and what a removal would strand — without writing a bespoke client.

Do not use: as an administrative console, as an editor of stored memory, or as an independent authority. The console renders what the daemon returned and nothing else; an empty or degraded panel is a real observation, not a rendering defect to work around.

## What it is

`@hypermind/console` is a workspace package in the [TypeScript SDK](../reference/sdks.md) at [`sdk/typescript/packages/console`](../../sdk/typescript/packages/console/package.json). It is plain TypeScript compiled by the workspace compiler: no bundler, no framework, no runtime dependency beyond `@hypermind/client` and the digest primitive already pinned by that package.

Every panel is built from two pure functions — one that turns a daemon envelope into a view model and one that turns that view model into escaped HTML — so the same code produces the browser page and the assertions in `node --test`. The server data comes from the existing fourteen MCP verbs: read-only targets inside `inspect` (`hm://{actor}`, `hm://{actor}/sources`, `hm://{actor}/sources/{conversation}`, `hm://{actor}/lsn/{n}`, `hm://{actor}/evidence/{lsn}`, `hm://{actor}/access`, `hm://{actor}/removal/{lsn}`), the retrieval manifest carried by `activate`, the timeline mode of `recall`, `remember` for the stored domain profile, and the daemon's event subscription for the activity feed. The console adds no verb.

## Build

```sh
npm --prefix sdk/typescript install
npm --prefix sdk/typescript run build -w @hypermind/console
```

The package compiles twice from one flat `src/` directory. `tsconfig.json` emits CommonJS into `dist/` for the package's own tests; `tsconfig.browser.json` emits ES2022 modules into `dist/site/`, excluding the tests and the Node-only transport, and a small build step copies the page shell to `dist/site/index.html`. `dist/site` is the directory the daemon serves; it is flat, because the serving route resolves one file name against one directory and no deeper path.

Run the package's tests with `npm --prefix sdk/typescript test -w @hypermind/console`. They build `hm`, start a real daemon and drive the view builders through the Node transport against real envelopes.

## Serve

Point the daemon at the built directory. The option is documented with the rest of the parser in the [CLI reference](../reference/cli.md).

```sh
hm serve --config .hypermind/config.toml \
  --rest-bind 127.0.0.1:8443 \
  --tls-cert server.pem --tls-key server.key --tls-client-ca clients.pem \
  --console-directory sdk/typescript/packages/console/dist/site
```

`--console-directory` requires `--rest-bind`, and the assets are served only on the actor REST listener: the admin listener refuses them. `GET /console` returns the page shell and `GET /console/{file}` returns one asset from that directory. Only `.html`, `.js`, `.css`, `.json` and `.svg` names are served, the file name must be a simple lowercase stem with no path separators, an asset over four megabytes is refused, and every response carries `cache-control: no-store`. Serving assets never touches the daemon gateway; the console's data still arrives through the same authenticated `POST /v1/{verb}` routes as any other REST client.

The console also runs outside a browser. `clientTransport` wraps `@hypermind/client`, so a Node process can build the same views from the same envelopes — which is how the package is tested.

## The views

| View | What it shows |
| --- | --- |
| Overview | The bound actor, the applied LSN, log event and byte counts, the verification result with its root and last checkpoint LSN, and one row per projection with the LSN it has applied. |
| Activity | Committed events streamed from the daemon subscription, sorted into ingestion, recall, consolidation and other lanes, deduplicated by LSN and bounded in length. The cursor is held per actor key, so a cursor is never replayed against a different actor. |
| Sources and entities | Every conversation the ledger holds, with its record count, LSN range, last wall timestamp and per-event-kind tally; opening one source lists the entities that source contributed, the entities it shares with other conversations, and a citation link for each record. |
| Evidence path | The reference walk from one record, ordered from the root, each step carrying the record's LSN, kind, authority, integrity receipt and the relation that led to it. Alongside it, the reverse lookup reports the answers that cited that record and its attestation counters. |
| Access | The actor this connection is bound to, the isolation statement, whether an admin capability token is configured, the admin-only operations, one row per verb saying whether it mutates and which of its actions additionally require the admin capability, and the protected belief types. |
| Removal preview | For one record: its direct dependents, the records that would be left unreachable, the resulting island count, the attestation counters and the number of frames scanned. |
| Domain profile | The entity types, fields and relationships an operator has described, with the profile's schema string, integer version and canonical digest, plus any validation errors. |
| Upload sessions | A multi-document ingestion that survived a reload: the conversation, the totals, the batches the daemon already accepted with their LSN ranges, the documents still to send, and the stage. |

Each new `inspect` surface scans a bounded number of ledger frames and refuses beyond it rather than reading the whole log, and each list result is capped and marks itself truncated when it hits its cap. A truncated list is reported as truncated; the console does not hide the cap.

## Boundaries this console keeps

**Every view renders authoritative server data.** There is no local cache of memory content, no reconstruction and no inference. When a field the view needs is missing or has the wrong shape, the builder throws `ConsoleDataError` naming the surface and the field instead of filling in a plausible value.

**Permissions come from the access surface and are never computed locally.** The access view reads `hm://{actor}/access`, which reports the daemon's own mutation table, and asserts that the item declares `server` as its source. If that declaration is absent the view throws rather than falling back to a guessed permission table, so the console can never show a permission the daemon does not enforce.

**The evidence path is a record reference walk, not a model's reasoning.** Each edge exists because one stored record's payload names another record's LSN. It is a fact about the ledger. It is not a chain of thought, not an explanation of why a model answered as it did, and it does not claim the model attended to any of those records.

**Retrieved, included and attested are three different facts.** `retrieved` means the record was a candidate in the retrieval manifest. `included` means it survived the budget and reached the assembled context. `attested` means the ledger holds non-zero attestation counters for it. A record can be retrieved and never included, and included and never attested. The evidence class table keeps the three columns separate and never collapses them into a single "used" flag.

**Removal previews are diagnostics that write nothing.** The surface answers what would be stranded if a record disappeared. It appends no event, retracts nothing and leaves the ledger exactly as it was; the server item declares itself a removal preview that performs no retraction, the view refuses to build if those declarations are missing, and every rendered string says so.

**The domain profile configures console presentation and never an internal event schema.** A profile is an ordinary versioned document — schema `hypermind.domain-profile.v1` with an integer version that moves on every content change — stored through `remember` in its own `console-domain-profile` conversation. Editing it changes how the console groups and labels what it shows. It does not change `schemas/events.fbs`, any event kind, any projection or anything the kernel validates.

**Upload sessions report degraded storage rather than claiming completion.** A session records two separate honesty flags. `degraded` means the session record was written but the payloads still waiting to be sent were not, so those documents must be chosen again before the upload can finish. `persisted` false means nothing reached storage at all and the session will not survive a reload. Neither state is rendered as success, and a session with documents still pending is never reported as complete.

## Stated limitations

The REST listener requires mutual TLS. This is not relaxed for asset serving, so a browser opening `/console` must present a client certificate the daemon's configured client CA accepts, in addition to supplying a capability token and connection identity in the connection form. A browser without a client certificate cannot reach the page at all. Serving the console over plain HTTP, or behind a proxy that terminates TLS and drops client identity, is not supported by this daemon.

No test in this repository renders the console in a browser, drives a DOM or asserts anything about browser behaviour. The tests assert view models built from real daemon envelopes, the exact HTML strings the renderers emit, and the bytes the REST route serves. The browser page is assembled from those same modules, but its behaviour in a browser is unverified here; treat a browser problem as unverified territory rather than as covered by a green gate.
