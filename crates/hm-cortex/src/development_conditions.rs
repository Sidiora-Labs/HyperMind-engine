use hm_context::{ContextError, Scope, digest_bytes, notes::Predicate};
use rustix::fs::{Mode, OFlags, open, openat};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File, Metadata},
    io::Read,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ConditionLimits {
    pub max_bytes: usize,
    pub max_nodes: usize,
    pub timeout_ms: u64,
}
impl Default for ConditionLimits {
    fn default() -> Self {
        Self {
            max_bytes: 65536,
            max_nodes: 128,
            timeout_ms: 2000,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOutcome {
    True,
    False,
    Unknown,
    Refused,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FactObservation {
    pub key: String,
    pub value: Option<String>,
    pub identity: String,
    pub reason: Option<String>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConditionEvidence {
    pub scope: Scope,
    pub condition_digest: String,
    pub outcome: ConditionOutcome,
    pub facts: Vec<FactObservation>,
    pub fingerprint: String,
}
#[derive(Clone, Debug)]
struct Root {
    requested: PathBuf,
    canonical: PathBuf,
    identity: String,
}
#[derive(Clone, Debug)]
pub struct AuthorizedFactReader {
    scope: Scope,
    roots: BTreeMap<String, Root>,
    git: PathBuf,
}
fn metadata_id(m: &Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        format!(
            "{}:{}:{}:{}:{}",
            m.dev(),
            m.ino(),
            m.len(),
            m.mtime(),
            m.mtime_nsec()
        )
    }
    #[cfg(not(unix))]
    {
        format!("{}:{:?}", m.len(), m.modified().ok())
    }
}
fn directory_id(m: &Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        format!("{}:{}", m.dev(), m.ino())
    }
    #[cfg(not(unix))]
    {
        format!("{:?}", m.created().ok())
    }
}
impl AuthorizedFactReader {
    pub fn open(
        scope: Scope,
        roots: BTreeMap<String, PathBuf>,
        git_binary: PathBuf,
    ) -> Result<Self, ContextError> {
        scope.validate()?;
        if roots.is_empty() || roots.len() > 16 {
            return Err(ContextError::Capacity);
        }
        let mut authorized = BTreeMap::new();
        for (id, path) in roots {
            hm_context::validate_id(&id)?;
            if id.contains(':') {
                return Err(ContextError::Invalid("condition root identifier".into()));
            }
            let canonical = fs::canonicalize(&path)?;
            let metadata = fs::metadata(&canonical)?;
            if !metadata.is_dir() {
                return Err(ContextError::Invalid(
                    "condition root is not a directory".into(),
                ));
            };
            authorized.insert(
                id,
                Root {
                    requested: path,
                    canonical,
                    identity: directory_id(&metadata),
                },
            );
        }
        let git = fs::canonicalize(git_binary)?;
        if !fs::metadata(&git)?.is_file() {
            return Err(ContextError::Invalid("Git executable".into()));
        }
        Ok(Self {
            scope,
            roots: authorized,
            git,
        })
    }
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    fn root(&self, id: &str) -> Result<&Root, String> {
        let root = self.roots.get(id).ok_or("unauthorized root")?;
        let canonical = fs::canonicalize(&root.requested).map_err(|_| "root unavailable")?;
        let metadata = fs::metadata(&canonical).map_err(|_| "root unavailable")?;
        if canonical != root.canonical || directory_id(&metadata) != root.identity {
            return Err("root identity changed".into());
        };
        Ok(root)
    }
    fn file(&self, id: &str, path: &str) -> Result<Option<File>, String> {
        let root = self.root(id)?;
        let path = Path::new(path);
        let components: Vec<_> = path.components().collect();
        if components.is_empty()
            || components.len() > 64
            || components
                .iter()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err("invalid relative condition path".into());
        }
        let mut directory: File = open(
            &root.canonical,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| "root open refused")?
        .into();
        if directory_id(&directory.metadata().map_err(|_| "root metadata")?) != root.identity {
            return Err("root identity changed".into());
        }
        for (index, component) in components.iter().enumerate() {
            let Component::Normal(name) = component else {
                unreachable!()
            };
            let flags = OFlags::RDONLY
                | OFlags::NOFOLLOW
                | OFlags::CLOEXEC
                | OFlags::NONBLOCK
                | if index + 1 < components.len() {
                    OFlags::DIRECTORY
                } else {
                    OFlags::empty()
                };
            match openat(&directory, *name, flags, Mode::empty()) {
                Ok(fd) => directory = fd.into(),
                Err(rustix::io::Errno::NOENT) => return Ok(None),
                Err(_) => return Err("condition path refused".into()),
            }
        }
        self.root(id)?;
        Ok(Some(directory))
    }
    pub fn evaluate(
        &self,
        predicate: &Predicate,
        scope: &Scope,
        limits: ConditionLimits,
    ) -> Result<ConditionEvidence, ContextError> {
        if limits.max_bytes == 0
            || limits.max_bytes > 4 * 1024 * 1024
            || limits.max_nodes == 0
            || limits.max_nodes > 128
            || limits.timeout_ms == 0
            || limits.timeout_ms > 10000
        {
            return Err(ContextError::Invalid("condition limits".into()));
        }
        let condition_digest = digest_bytes(&serde_json::to_vec(predicate)?);
        let started = Instant::now();
        let mut facts = BTreeMap::new();
        let mut nodes = limits.max_nodes;
        let outcome = if scope != &self.scope {
            ConditionOutcome::Refused
        } else {
            self.visit(predicate, 0, &mut nodes, &mut facts, limits, started)
        };
        let mut evidence = ConditionEvidence {
            scope: scope.clone(),
            condition_digest,
            outcome,
            facts: facts.into_values().map(|(_, fact)| fact).collect(),
            fingerprint: String::new(),
        };
        evidence.fingerprint = digest_bytes(&serde_json::to_vec(&evidence)?);
        Ok(evidence)
    }
    fn visit(
        &self,
        p: &Predicate,
        depth: usize,
        nodes: &mut usize,
        facts: &mut BTreeMap<String, (ConditionOutcome, FactObservation)>,
        limits: ConditionLimits,
        started: Instant,
    ) -> ConditionOutcome {
        if depth > 16 || *nodes == 0 {
            return ConditionOutcome::Refused;
        };
        *nodes -= 1;
        if started.elapsed() >= Duration::from_millis(limits.timeout_ms) {
            return ConditionOutcome::Unknown;
        }
        match p {
            Predicate::True => ConditionOutcome::True,
            Predicate::Exists(key) | Predicate::Equals { key, .. } => {
                let mut available = limits;
                available.max_bytes = limits.max_bytes.saturating_sub(
                    facts
                        .values()
                        .filter_map(|(_, f)| f.value.as_ref())
                        .map(String::len)
                        .sum::<usize>(),
                );
                let (state, fact) = facts
                    .entry(key.clone())
                    .or_insert_with(|| self.observe(key, available, started));
                if matches!(state, ConditionOutcome::Unknown | ConditionOutcome::Refused) {
                    return state.clone();
                };
                let result = match p {
                    Predicate::Exists(key) => fact.value.as_ref().is_some_and(|v| {
                        if key.starts_with("file.exists:") || key.starts_with("git.") {
                            v == "true"
                        } else {
                            true
                        }
                    }),
                    Predicate::Equals { value, .. } => fact.value.as_ref() == Some(value),
                    _ => false,
                };
                if result {
                    ConditionOutcome::True
                } else {
                    ConditionOutcome::False
                }
            }
            Predicate::Not(p) => match self.visit(p, depth + 1, nodes, facts, limits, started) {
                ConditionOutcome::True => ConditionOutcome::False,
                ConditionOutcome::False => ConditionOutcome::True,
                other => other,
            },
            Predicate::All(ps) | Predicate::Any(ps) => {
                if ps.is_empty() || ps.len() > 32 {
                    return ConditionOutcome::Refused;
                };
                let values: Vec<_> = ps
                    .iter()
                    .map(|p| self.visit(p, depth + 1, nodes, facts, limits, started))
                    .collect();
                if values.contains(&ConditionOutcome::Refused) {
                    return ConditionOutcome::Refused;
                };
                let all = matches!(p, Predicate::All(_));
                if (all && values.contains(&ConditionOutcome::False))
                    || (!all && values.contains(&ConditionOutcome::True))
                {
                    return if all {
                        ConditionOutcome::False
                    } else {
                        ConditionOutcome::True
                    };
                };
                if values.contains(&ConditionOutcome::Unknown) {
                    ConditionOutcome::Unknown
                } else if all {
                    ConditionOutcome::True
                } else {
                    ConditionOutcome::False
                }
            }
        }
    }
    fn observe(
        &self,
        key: &str,
        limits: ConditionLimits,
        started: Instant,
    ) -> (ConditionOutcome, FactObservation) {
        let mut fact = FactObservation {
            key: key.into(),
            value: None,
            identity: String::new(),
            reason: None,
        };
        if key.len() > 4096 {
            return (ConditionOutcome::Refused, fact);
        }
        let result = if let Some((kind, tail)) = key.split_once(':') {
            if matches!(kind, "file.exists" | "file.content" | "file.mtime") {
                self.file_fact(kind, tail, limits)
            } else if matches!(kind, "git.ancestor" | "git.tag") {
                self.git_fact(kind, tail, limits, started)
            } else {
                Err((
                    ConditionOutcome::Refused,
                    "unsupported condition fact".into(),
                ))
            }
        } else {
            Err((
                ConditionOutcome::Refused,
                "unsupported condition fact".into(),
            ))
        };
        if started.elapsed() >= Duration::from_millis(limits.timeout_ms) {
            fact.reason = Some("condition deadline".into());
            return (ConditionOutcome::Unknown, fact);
        }
        match result {
            Ok((value, identity)) => {
                fact.value = value;
                fact.identity = identity;
                (ConditionOutcome::True, fact)
            }
            Err((outcome, reason)) => {
                fact.reason = Some(reason);
                (outcome, fact)
            }
        }
    }
    fn file_fact(
        &self,
        kind: &str,
        tail: &str,
        limits: ConditionLimits,
    ) -> Result<(Option<String>, String), (ConditionOutcome, String)> {
        let (root, path) = tail
            .split_once(':')
            .ok_or((ConditionOutcome::Refused, "file root required".into()))?;
        let mut file = match self
            .file(root, path)
            .map_err(|r| (ConditionOutcome::Refused, r))?
        {
            Some(file) => file,
            None => {
                return Ok((
                    if kind == "file.exists" {
                        Some("false".into())
                    } else {
                        None
                    },
                    format!(
                        "{}:missing:{path}",
                        self.root(root)
                            .map_err(|r| (ConditionOutcome::Refused, r))?
                            .identity
                    ),
                ));
            }
        };
        let before = file.metadata().map_err(|_| {
            (
                ConditionOutcome::Unknown,
                "file metadata unavailable".into(),
            )
        })?;
        if !before.is_file() {
            return Err((
                ConditionOutcome::Refused,
                "condition requires regular file".into(),
            ));
        }
        let mut identity = format!("{root}:{path}:{}", metadata_id(&before));
        let value = match kind {
            "file.exists" => "true".into(),
            "file.mtime" => {
                let time = before.modified().map_err(|_| {
                    (
                        ConditionOutcome::Unknown,
                        "unknown modification time".into(),
                    )
                })?;
                match time.duration_since(UNIX_EPOCH) {
                    Ok(t) => t.as_nanos().to_string(),
                    Err(t) => format!("-{}", t.duration().as_nanos()),
                }
            }
            "file.content" => {
                if before.len() > limits.max_bytes as u64 {
                    return Err((ConditionOutcome::Refused, "file byte limit".into()));
                };
                let mut bytes = Vec::new();
                file.by_ref()
                    .take(limits.max_bytes as u64 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|_| (ConditionOutcome::Unknown, "file read unavailable".into()))?;
                if bytes.len() > limits.max_bytes {
                    return Err((ConditionOutcome::Refused, "file byte limit".into()));
                };
                identity.push_str(&format!(":{}", digest_bytes(&bytes)));
                String::from_utf8(bytes)
                    .map_err(|_| (ConditionOutcome::Unknown, "non-text file".into()))?
            }
            _ => unreachable!(),
        };
        let after = file.metadata().map_err(|_| {
            (
                ConditionOutcome::Unknown,
                "file metadata unavailable".into(),
            )
        })?;
        let current = self
            .file(root, path)
            .map_err(|r| (ConditionOutcome::Refused, r))?
            .ok_or((
                ConditionOutcome::Unknown,
                "file changed during observation".into(),
            ))?;
        if metadata_id(&before) != metadata_id(&after)
            || metadata_id(&before)
                != metadata_id(&current.metadata().map_err(|_| {
                    (
                        ConditionOutcome::Unknown,
                        "file metadata unavailable".into(),
                    )
                })?)
        {
            return Err((
                ConditionOutcome::Unknown,
                "file changed during observation".into(),
            ));
        }
        Ok((Some(value), identity))
    }
    fn git_output(
        &self,
        root: &Root,
        args: &[&str],
        limits: ConditionLimits,
        started: Instant,
    ) -> Result<(i32, Vec<u8>), (ConditionOutcome, String)> {
        if started.elapsed() >= Duration::from_millis(limits.timeout_ms) {
            return Err((ConditionOutcome::Unknown, "Git deadline".into()));
        }
        let mut child = Command::new(&self.git)
            .env_clear()
            .env("PATH", self.git.parent().unwrap_or(Path::new("/usr/bin")))
            .env("LC_ALL", "C")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_NO_REPLACE_OBJECTS", "1")
            .args([
                "--no-optional-locks",
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "protocol.allow=never",
                "-C",
            ])
            .arg(&root.canonical)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| (ConditionOutcome::Unknown, "Git unavailable".into()))?;
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let cap = limits.max_bytes.min(65536);
        let out = thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stdout.take(cap as u64 + 1).read_to_end(&mut bytes);
            bytes
        });
        let err = thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stderr.take(4097).read_to_end(&mut bytes);
            bytes
        });
        let mut timeout = false;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) => {
                    if started.elapsed() >= Duration::from_millis(limits.timeout_ms) {
                        timeout = true;
                        let _ = child.kill();
                        break child.wait().ok();
                    };
                    thread::sleep(Duration::from_millis(5))
                }
                Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
            }
        };
        let bytes = out.join().unwrap_or_default();
        let errors = err.join().unwrap_or_default();
        if timeout {
            return Err((ConditionOutcome::Unknown, "Git deadline".into()));
        };
        if bytes.len() > cap || errors.len() > 4096 {
            return Err((ConditionOutcome::Refused, "Git output limit".into()));
        };
        Ok((status.and_then(|s| s.code()).unwrap_or(-1), bytes))
    }
    fn repository_identity(
        &self,
        root: &Root,
        limits: ConditionLimits,
        started: Instant,
    ) -> Result<String, (ConditionOutcome, String)> {
        let mut pending = vec![root.canonical.join(".git")];
        let mut identities = Vec::new();
        while let Some(path) = pending.pop() {
            if identities.len() >= 4096
                || started.elapsed() >= Duration::from_millis(limits.timeout_ms)
            {
                return Err((
                    ConditionOutcome::Unknown,
                    "repository observation limit".into(),
                ));
            };
            let metadata = fs::symlink_metadata(&path)
                .map_err(|_| (ConditionOutcome::Unknown, "repository changed".into()))?;
            if metadata.file_type().is_symlink() {
                return Err((
                    ConditionOutcome::Refused,
                    "repository symlink refused".into(),
                ));
            };
            identities.push((
                path.strip_prefix(&root.canonical)
                    .unwrap()
                    .to_string_lossy()
                    .to_string(),
                metadata_id(&metadata),
            ));
            if metadata.is_dir() {
                for entry in fs::read_dir(path)
                    .map_err(|_| (ConditionOutcome::Unknown, "repository unavailable".into()))?
                {
                    pending.push(
                        entry
                            .map_err(|_| (ConditionOutcome::Unknown, "repository changed".into()))?
                            .path(),
                    )
                }
            }
        }
        identities.sort();
        Ok(digest_bytes(&serde_json::to_vec(&identities).map_err(
            |_| (ConditionOutcome::Unknown, "repository identity".into()),
        )?))
    }
    fn git_fact(
        &self,
        kind: &str,
        tail: &str,
        limits: ConditionLimits,
        started: Instant,
    ) -> Result<(Option<String>, String), (ConditionOutcome, String)> {
        let (id, refs) = tail
            .split_once(':')
            .ok_or((ConditionOutcome::Refused, "Git root required".into()))?;
        let root = self.root(id).map_err(|r| (ConditionOutcome::Refused, r))?;
        let repository_identity = self.repository_identity(root, limits, started)?;
        for path in [".git", ".git/objects", ".git/refs"] {
            let metadata = fs::symlink_metadata(root.canonical.join(path))
                .map_err(|_| (ConditionOutcome::Unknown, "repository unavailable".into()))?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err((ConditionOutcome::Refused, "repository path refused".into()));
            }
        }
        if root.canonical.join(".git/objects/info/alternates").exists() {
            return Err((
                ConditionOutcome::Refused,
                "external Git objects refused".into(),
            ));
        }
        let config = self
            .file(id, ".git/config")
            .map_err(|r| (ConditionOutcome::Refused, r))?
            .ok_or((
                ConditionOutcome::Unknown,
                "repository config unavailable".into(),
            ))?;
        let mut bytes = Vec::new();
        config.take(65537).read_to_end(&mut bytes).map_err(|_| {
            (
                ConditionOutcome::Unknown,
                "repository config unavailable".into(),
            )
        })?;
        if bytes.len() > 65536
            || String::from_utf8_lossy(&bytes)
                .to_ascii_lowercase()
                .contains("[include")
        {
            return Err((
                ConditionOutcome::Refused,
                "repository configuration refused".into(),
            ));
        }
        let valid = |value: &str| {
            !value.is_empty()
                && value.len() <= 256
                && !value.starts_with('-')
                && !value.contains("..")
                && value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_./-".contains(&b))
        };
        let (value, identity) = if kind == "git.tag" {
            if !valid(refs) {
                return Err((ConditionOutcome::Refused, "invalid Git tag".into()));
            };
            let reference = format!("refs/tags/{refs}");
            let (code, out) = self.git_output(
                root,
                &["rev-parse", "--verify", "--quiet", &reference],
                limits,
                started,
            )?;
            if code == 0 {
                (true, digest_bytes(&out))
            } else if code == 1 {
                (false, "missing".into())
            } else {
                return Err((ConditionOutcome::Unknown, "Git tag unavailable".into()));
            }
        } else {
            let (base, head) = refs
                .split_once(':')
                .ok_or((ConditionOutcome::Refused, "Git refs required".into()))?;
            if !valid(base) || !valid(head) {
                return Err((ConditionOutcome::Refused, "invalid Git ref".into()));
            };
            let mut resolved = Vec::new();
            for reference in [base, head] {
                let commit = format!("{reference}^{{commit}}");
                let (code, out) = self.git_output(
                    root,
                    &["rev-parse", "--verify", "--quiet", &commit],
                    limits,
                    started,
                )?;
                if code != 0 {
                    return Err((ConditionOutcome::Unknown, "Git ref unavailable".into()));
                };
                let sha = String::from_utf8(out)
                    .map_err(|_| (ConditionOutcome::Unknown, "Git ref encoding".into()))?
                    .trim()
                    .to_owned();
                if !matches!(sha.len(), 40 | 64) || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err((ConditionOutcome::Refused, "Git ref identity".into()));
                };
                resolved.push(sha)
            }
            let (code, _) = self.git_output(
                root,
                &["merge-base", "--is-ancestor", &resolved[0], &resolved[1]],
                limits,
                started,
            )?;
            if code > 1 || code < 0 {
                return Err((ConditionOutcome::Unknown, "Git ancestry unavailable".into()));
            };
            (code == 0, resolved.join(":"))
        };
        self.root(id).map_err(|r| (ConditionOutcome::Refused, r))?;
        if repository_identity != self.repository_identity(root, limits, started)? {
            return Err((
                ConditionOutcome::Unknown,
                "repository changed during observation".into(),
            ));
        }
        Ok((
            Some(value.to_string()),
            format!("{id}:{}:{repository_identity}:{identity}", root.identity),
        ))
    }
}
