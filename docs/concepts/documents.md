# Documents

When to use: bring a file — prose, a spreadsheet export, a PDF, a saved mail message — into the ledger so its text can be recalled, revised and retracted like any other memory.

Do not use: as a place to put text you already have as a string, as a transcription service for images or audio, or as a way to make extracted text count as evidence the user asserted.

A document is a ledger source kind, not a store. Nothing about it lives outside the log. Ingesting one appends three records in two steps, and everything a reader later sees about that document is derived from those records through a generation-scoped projection.

## The original bytes are retained once

`DocumentIngested` carries the artefact verbatim: its name, its media type, the bytes as supplied, and the blake3 digest of those bytes. The record is externally observed, because a file entering the ledger is an external artefact and not a claim anyone made. It is written once per distinct byte string and never rewritten. Re-sending the same bytes under the same name retains nothing new; sending different bytes under that name retains the new artefact alongside the old one, and the catalogue points at the newer of the two.

A document's identity is derived from its name, so a revised file is the same document rather than a new one. The artefact's own identity is its content digest, which is what distinguishes one revision from the next.

## Extraction is versioned, and partial results are data

`DocumentExtracted` holds the text a loader produced, the loader's id, the extraction version that produced it, the page spans that partition the text, and — when something could not be read — the reason and the units that failed. Extraction is derived inference. It is never observed evidence, because no one asserted the text: a parser did.

A page that will not parse does not cost the rest of the document. The pages that did parse are extracted, the pages that did not are named in `failed_units`, and the call still succeeds. The envelope reports this as a `partial_extraction` gap and a warning saying the document was stored with part of its content unextracted. Only a document where nothing at all extracts is an error.

The extraction version is recorded on the record, so a later, better loader can be told apart from the one that ran at the time, and text produced by an older version is never silently confused with text produced by a newer one.

## Chunk identity is content-derived

`DocumentChunked` names every chunk the document has, in order. Each chunk's byte range is an offset into the extracted text, never into the original bytes, and the ranges tile the text exactly: the first starts at zero, each starts where the previous ended, and the last ends at the text length. Nothing is trimmed, so concatenating the ranges reproduces the extracted text byte for byte. The schema enforces both the ordinal sequence and the tiling chain at admission.

A chunk's id is derived from the document's identity, the blake3 hash of the chunk's own text, and an occurrence number. The occurrence disambiguates chunks whose text is identical inside one document, and it is counted over the document's final order, so two identical paragraphs keep two stable, distinct identities. Because the id follows the content, a chunk that survives an edit keeps everything already derived from it.

## The change plan and its reconstruction gate

A re-ingest is planned, not recut from scratch. The planner reads the published chunk set through the documents projection, finds the regions where the old and new texts differ, widens each region to the boundaries of the stored chunks it overlaps, and re-chunks only those regions. Every chunk outside a changed region keeps its stored id and its text.

Each chunk in the plan carries a change kind. `retained` is a stored chunk that kept both its id and its position. `moved` kept its id and shifted position. `replaced` is a freshly cut chunk whose text is byte-identical to one of the chunks it replaces, so it inherits that id. `added` is genuinely new content.

The plan is then put through a reconstruction gate that trusts none of the planner's arithmetic. The gate re-derives the document from the plan alone and refuses unless the ordinals are dense and unique, the ranges tile the new text, every id follows from its content and occurrence, every retained or replaced id is one the ledger actually holds, and the concatenated chunk texts equal the new text byte for byte. A plan that fails the gate is refused with `kInvariantViolation` and nothing is appended.

`remember` exposes the plan directly. With `plan_only` set the call returns the whole preview — the per-chunk change kinds, ids, ranges and token estimates, and the retained, moved, replaced and added counts — and appends nothing at all.

## Publication is a retractable generation

Publishing reuses the consolidation machinery unchanged. One document revision is one run: a `ConsolidationOpened` declaring a single `extract` phase and no prompts, the started phase event, the extraction and chunk records staged under the run id, the completed phase event, and a `ConsolidationClosed` with two derived records. The close is the compare-and-swap that makes the new chunk set visible, and retracting the run restores the previous one, chunk ids and all.

The run's cadence key is derived from the document's identity and the digest of the extracted text, so a genuinely new revision always opens a new run and re-sending bytes that are already published reports the run that already exists rather than opening a second one.

This is two appends, not one, and it is not atomic. The `DocumentIngested` record must be committed before the derived records can name its LSN as their source, because an LSN only exists once it has been assigned. A crash between the two appends leaves the original bytes retained with no generation published, which is a readable state: the catalogue holds the document, the extraction and chunk lists are empty, and a retry re-plans from the same document identity and opens a fresh run. It is never a partial generation.

## Reading a document back

Stored chunks are read through the generation-scoped documents projection, never by rescanning frames. That is what makes retraction and lineage govern what a plan is computed against: a retracted run's chunks are not visible, so they are not planned against, and a staged run that was never closed is not visible either.
