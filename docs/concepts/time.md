# Time and beliefs

HyperMind records three distinct times for a belief. Event time says when the source observation occurred, observation LSN says when the ledger learned it, and the validity interval says when the claim was true in the world.

An as-of read must choose one axis:

- `valid_at` selects the version whose validity interval contains the requested time, using all versions known at the current projection checkpoint.
- `known_at` selects the newest version whose observation LSN is no later than the requested LSN.

These queries can intentionally disagree. If Europe was valid through time 100 and America became valid at 101, a later `known_at` query returns America while `valid_at: 50` returns Europe.

Replacing a belief appends a new version with `supersedes_version`; it does not overwrite history. Retraction appends a tombstone, so reads before the retraction remain reproducible while reads after it return no live belief.

Activation uses live heads for current claims. A superseded value stated as current is a stale fact and fails the temporal evaluation gate. Historical values remain available only through explicit as-of or timeline recall.
