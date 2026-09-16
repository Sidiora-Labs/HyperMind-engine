CREATE TABLE IF NOT EXISTS memories (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, definition TEXT NOT NULL,
    category TEXT NOT NULL, salience REAL NOT NULL DEFAULT 0.5,
    confidence REAL NOT NULL DEFAULT 0.5, access_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL, last_accessed TEXT NOT NULL,
    source_files TEXT NOT NULL DEFAULT '[]', embedding BLOB NOT NULL DEFAULT (x''),
    tags TEXT NOT NULL DEFAULT '[]',
    fsrs_stability REAL NOT NULL DEFAULT 3.1262, fsrs_difficulty REAL NOT NULL DEFAULT 7.2102,
    fsrs_reps INTEGER NOT NULL DEFAULT 0, fsrs_lapses INTEGER NOT NULL DEFAULT 0,
    fsrs_state TEXT NOT NULL DEFAULT 'new', fsrs_last_review TEXT,
    faded INTEGER DEFAULT 0, salience_original REAL,
    prov_model_id TEXT, prov_model_family TEXT, prov_client TEXT, prov_agent TEXT,
    last_retrieval_score REAL, last_hop_count INTEGER, memory_origin TEXT
  );

CREATE TABLE IF NOT EXISTS observations (
    id TEXT PRIMARY KEY, content TEXT NOT NULL,
    source_file TEXT NOT NULL DEFAULT '', source_section TEXT NOT NULL DEFAULT '',
    salience REAL NOT NULL DEFAULT 0.5, processed INTEGER NOT NULL DEFAULT 0,
    prediction_error REAL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
    embedding BLOB, keywords TEXT NOT NULL DEFAULT '[]',
    content_type TEXT DEFAULT 'declarative',
    prov_model_id TEXT, prov_model_family TEXT, prov_client TEXT, prov_agent TEXT
  );

CREATE TABLE IF NOT EXISTS edges (
    id TEXT PRIMARY KEY, source_id TEXT NOT NULL, target_id TEXT NOT NULL,
    relation TEXT NOT NULL, weight REAL NOT NULL DEFAULT 1.0,
    evidence TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL
  );

CREATE TABLE IF NOT EXISTS ops (
    id TEXT PRIMARY KEY, content TEXT NOT NULL,
    type TEXT NOT NULL DEFAULT 'log', status TEXT NOT NULL DEFAULT 'active',
    project TEXT, session_ref TEXT NOT NULL DEFAULT '',
    keywords TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL, expires_at TEXT NOT NULL,
    prov_model_id TEXT, prov_model_family TEXT, prov_client TEXT, prov_agent TEXT
  );

CREATE TABLE IF NOT EXISTS signals (
    id TEXT PRIMARY KEY, type TEXT NOT NULL, description TEXT NOT NULL,
    concept_ids TEXT NOT NULL DEFAULT '[]', priority REAL NOT NULL DEFAULT 0.5,
    resolved INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL,
    resolution_note TEXT, resolved_at TEXT, observation_id TEXT
  );

CREATE TABLE IF NOT EXISTS beliefs (
    id TEXT PRIMARY KEY, concept_id TEXT NOT NULL,
    old_definition TEXT NOT NULL, new_definition TEXT NOT NULL,
    reason TEXT NOT NULL, changed_at TEXT NOT NULL,
    valid_from TEXT, valid_to TEXT
  );

CREATE TABLE IF NOT EXISTS generic_docs (
    collection TEXT NOT NULL, id TEXT NOT NULL, data TEXT NOT NULL,
    PRIMARY KEY (collection, id)
  );
