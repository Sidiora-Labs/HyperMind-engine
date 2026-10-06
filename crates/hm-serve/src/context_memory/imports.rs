use super::*;
fn invalid(message: &str) -> MemoryError {
    ContextError::Invalid(message.into()).into()
}
fn string(v: &Value, k: &str) -> Result<String, MemoryError> {
    v.get(k)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| invalid(k))
}
fn integer(v: &Value, k: &str) -> Result<u64, MemoryError> {
    v.get(k).and_then(Value::as_u64).ok_or_else(|| invalid(k))
}
fn ms(v: &Value, k: &str) -> Result<Option<i64>, MemoryError> {
    if v.get(k).is_none_or(Value::is_null) {
        return Ok(None);
    }
    let value = v[k].as_i64().ok_or_else(|| invalid(k))?;
    Ok(Some(
        value.checked_mul(1_000_000).ok_or(ContextError::Capacity)?,
    ))
}
fn confidence(v: &Value, k: &str) -> Result<u32, MemoryError> {
    let number = v[k].as_f64().ok_or_else(|| invalid(k))?;
    if !number.is_finite() || !(0.0..=1.0).contains(&number) {
        return Err(invalid(k));
    }
    Ok((number * 1_000_000.0).round() as u32)
}
fn kind(v: &str) -> Result<RecordKind, MemoryError> {
    Ok(match v {
        "fact" => RecordKind::Fact,
        "episode" => RecordKind::Episode,
        "note" => RecordKind::Note,
        "smart_note" => RecordKind::ConditionalNote,
        "anchor" => RecordKind::Anchor,
        "summary" => RecordKind::Summary,
        _ => return Err(invalid("record kind")),
    })
}
fn status(v: &str) -> Result<RecordStatus, MemoryError> {
    Ok(match v {
        "active" => RecordStatus::Active,
        "archived" => RecordStatus::Archived,
        "stale" => RecordStatus::Stale,
        "tombstoned" => RecordStatus::Tombstoned,
        _ => return Err(invalid("record status")),
    })
}
impl MemoryProjection {
    pub(super) fn apply_import(
        &mut self,
        entries: &[crate::hypermid_import::ImportEntry],
        lsn: u64,
    ) -> Result<(), MemoryError> {
        let mut imported = MemoryProjection::new(self.scope.clone());
        let mut record_rows = BTreeMap::new();
        for e in entries {
            if e.digest != digest_bytes(&serde_json::to_vec(&e.payload)?) {
                return Err(invalid("entry digest"));
            }
            let v = &e.payload;
            crate::hypermid_import::reject_excluded(v)?;
            if v.get("owner_scope_digest")
                .is_some_and(|d| d.as_str() != Some(&super::principal_digest(&self.scope)))
            {
                return Err(ContextError::ScopeMismatch.into());
            }
            match e.kind.as_str() {
                "scope" => {
                    let scope: Scope = serde_json::from_value(v["scope"].clone())?;
                    if scope != self.scope {
                        return Err(ContextError::ScopeMismatch.into());
                    }
                }
                "record" => {
                    let id = string(v, "record_id")?;
                    if self.records.contains_key(&id)
                        || record_rows.insert(id.clone(), v.clone()).is_some()
                    {
                        return Err(ContextError::Conflict.into());
                    }
                    let mut r = MemoryRecord::new(
                        id.clone(),
                        kind(&string(v, "kind")?)?,
                        "",
                        ms(v, "created_at_ms")?.ok_or_else(|| invalid("record created time"))?,
                    );
                    r.category = string(v, "category")?;
                    r.status = status(&string(v, "status")?)?;
                    r.revision = integer(v, "current_revision")?;
                    r.revision_digest = string(v, "current_revision_digest")?;
                    r.importance = confidence(v, "importance")?;
                    r.confidence = confidence(v, "confidence")?;
                    r.occurred_at_ns = ms(v, "observed_from_ms")?;
                    r.expires_at_ns = ms(v, "expires_at_ms")?;
                    r.retention_until_ns = ms(v, "retention_until_ms")?;
                    r.authority = Authority::ExternalObserved;
                    r.metadata = v.clone();
                    r.last_lsn = lsn;
                    imported.records.insert(id, r);
                }
                "source" => {
                    let source = MemorySource {
                        id: string(v, "source_id")?,
                        digest: string(v, "source_digest")?,
                        content: v["captured_content"]
                            .as_str()
                            .unwrap_or_default()
                            .as_bytes()
                            .to_vec(),
                        locator: v["locator"].as_str().unwrap_or_default().into(),
                        occurred_at_ns: ms(v, "observed_at_ms")?,
                        recorded_at_ns: ms(v, "created_at_ms")?
                            .ok_or_else(|| invalid("source time"))?,
                        tombstoned: false,
                    };
                    if v["captured_content"].is_string()
                        && digest_bytes(&source.content) != source.digest
                    {
                        return Err(invalid("source digest"));
                    }
                    if imported.sources.insert(source.id.clone(), source).is_some() {
                        return Err(ContextError::Conflict.into());
                    }
                }
                "revision" | "episode_detail" | "smart_note_detail" | "summary_detail"
                | "provenance" | "lineage" | "verification" | "mutation" | "grant" => {}
                "source_reference" | "source_snapshot" | "summary" | "projection"
                | "policy_revision" | "cache_generation" | "reduction" => {
                    return Err(ContextError::Unavailable(
                        "context portability row requires source-history migration".into(),
                    )
                    .into());
                }
                _ => return Err(invalid("unsupported memory export row")),
            }
            if self.origins.contains_key(&e.source_id)
                || imported
                    .origins
                    .insert(e.source_id.clone(), e.clone())
                    .is_some()
            {
                return Err(ContextError::Conflict.into());
            }
        }
        for e in entries.iter().filter(|e| e.kind == "revision") {
            let v = &e.payload;
            let id = string(v, "record_id")?;
            let base = imported
                .records
                .get(&id)
                .ok_or_else(|| invalid("revision record"))?;
            let mut r = base.clone();
            r.revision = integer(v, "revision")?;
            r.revision_digest = string(v, "revision_digest")?;
            r.content = string(v, "content")?;
            if r.content.len() > 262144
                || digest_bytes(r.content.as_bytes()) != string(v, "content_digest")?
            {
                return Err(invalid("revision content digest"));
            }
            r.recorded_at_ns =
                ms(v, "authored_at_ms")?.ok_or_else(|| invalid("revision authored time"))?;
            r.metadata = serde_json::json!({"record":base.metadata,"revision":v});
            if v["immutable_anchor"].as_bool() == Some(true) {
                r.kind = RecordKind::Anchor;
            }
            if imported.revisions.insert((id, r.revision), r).is_some() {
                return Err(ContextError::Conflict.into());
            }
        }
        for ((id, revision), record) in &imported.revisions {
            let original = entries
                .iter()
                .find(|e| {
                    e.kind == "revision"
                        && e.payload["record_id"] == *id
                        && e.payload["revision"].as_u64() == Some(*revision)
                })
                .ok_or_else(|| invalid("revision origin"))?;
            if *revision > 1 {
                let prior = imported
                    .revisions
                    .get(&(id.clone(), revision - 1))
                    .ok_or_else(|| invalid("revision chain gap"))?;
                if original.payload["parent_revision_digest"].as_str()
                    != Some(&prior.revision_digest)
                {
                    return Err(invalid("revision parent digest"));
                }
                if record.kind == RecordKind::Anchor && record.content != prior.content {
                    return Err(invalid("anchor rewritten"));
                }
            }
        }
        for (id, row) in &record_rows {
            let revision = integer(row, "current_revision")?;
            let mut current = imported
                .revisions
                .get(&(id.clone(), revision))
                .ok_or_else(|| invalid("current revision missing"))?
                .clone();
            if current.revision_digest != string(row, "current_revision_digest")? {
                return Err(invalid("current revision mismatch"));
            }
            current.status = status(&string(row, "status")?)?;
            imported.records.insert(id.clone(), current);
        }
        for e in entries.iter().filter(|e| e.kind == "provenance") {
            let v = &e.payload;
            let id = string(v, "record_id")?;
            let revision = integer(v, "revision")?;
            let source_id = string(v, "source_id")?;
            let source = imported
                .sources
                .get(&source_id)
                .or_else(|| self.sources.get(&source_id))
                .ok_or_else(|| invalid("provenance source"))?;
            let span_start = v["span_start"].as_u64().unwrap_or(0);
            let span_end = v["span_end"]
                .as_u64()
                .unwrap_or(source.content.len() as u64);
            if span_start > span_end {
                return Err(invalid("provenance span"));
            }
            let quoted_digest = v["quoted_digest"]
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| digest_bytes(&source.content));
            let p = Provenance {
                source_id,
                source_digest: source.digest.clone(),
                span_start,
                span_end,
                quoted_digest,
            };
            if !source.content.is_empty() {
                let bytes = source
                    .content
                    .get(span_start as usize..span_end as usize)
                    .ok_or_else(|| invalid("provenance bounds"))?;
                if digest_bytes(bytes) != p.quoted_digest {
                    return Err(invalid("quoted digest"));
                }
            }
            let r = imported
                .revisions
                .get_mut(&(id.clone(), revision))
                .ok_or_else(|| invalid("provenance revision"))?;
            r.provenance.push(p.clone());
            if let Some(current) = imported.records.get_mut(&id) {
                if current.revision == revision {
                    current.provenance.push(p);
                }
            }
        }
        for e in entries.iter().filter(|e| {
            matches!(
                e.kind.as_str(),
                "episode_detail" | "smart_note_detail" | "summary_detail"
            )
        }) {
            let v = &e.payload;
            let id = string(v, "record_id")?;
            let r = imported
                .records
                .get_mut(&id)
                .ok_or_else(|| invalid("detail record"))?;
            if !r.metadata.is_object() {
                r.metadata = serde_json::json!({})
            }
            r.metadata
                .as_object_mut()
                .unwrap()
                .insert(e.kind.clone(), v.clone());
            if e.kind == "smart_note_detail" {
                r.smart_condition = Some(serde_json::from_value(v["predicate"].clone())?);
                r.smart_condition.as_ref().unwrap().validate()?;
            }
            if e.kind == "episode_detail" {
                r.occurred_at_ns = ms(v, "observed_from_ms")?;
            }
        }
        for e in entries.iter().filter(|e| e.kind == "lineage") {
            let v = &e.payload;
            let l = Lineage {
                child_record_id: string(v, "child_record_id")?,
                child_revision: integer(v, "child_revision")?,
                parent_record_id: string(v, "parent_record_id")?,
                parent_revision_digest: string(v, "parent_revision_digest")?,
                relation: string(v, "relation")?,
                created_at_ns: ms(v, "created_at_ms")?.ok_or_else(|| invalid("lineage time"))?,
            };
            if !imported
                .revisions
                .values()
                .chain(self.revisions.values())
                .any(|r| {
                    r.id == l.parent_record_id && r.revision_digest == l.parent_revision_digest
                })
            {
                return Err(invalid("lineage parent"));
            }
            let child = imported
                .revisions
                .get_mut(&(l.child_record_id.clone(), l.child_revision))
                .ok_or_else(|| invalid("lineage child"))?;
            child.lineage.push(l.clone());
            if l.relation == "contradicts" {
                child.contradictions.push(l.parent_record_id.clone());
            }
            if imported
                .records
                .get(&l.child_record_id)
                .is_some_and(|r| r.revision == l.child_revision)
            {
                imported
                    .records
                    .get_mut(&l.child_record_id)
                    .unwrap()
                    .lineage
                    .push(l.clone());
                if l.relation == "contradicts" {
                    imported
                        .records
                        .get_mut(&l.child_record_id)
                        .unwrap()
                        .contradictions
                        .push(l.parent_record_id.clone());
                }
            }
        }
        for e in entries.iter().filter(|e| e.kind == "verification") {
            let v = &e.payload;
            let state = match string(v, "state")?.as_str() {
                "supported" => VerificationState::Supported,
                "disputed" | "refuted" => VerificationState::Contradicted,
                "unverified" | "unknown" => VerificationState::Unresolved,
                _ => return Err(invalid("verification state")),
            };
            let verification = Verification {
                id: string(v, "event_id")?,
                record_id: string(v, "record_id")?,
                revision_digest: string(v, "revision_digest")?,
                state,
                evidence_source_id: v["evidence_source_id"].as_str().map(str::to_owned),
                confidence: confidence(v, "confidence")?,
                created_at_ns: ms(v, "created_at_ms")?
                    .ok_or_else(|| invalid("verification time"))?,
            };
            if !imported.revisions.values().any(|r| {
                r.id == verification.record_id && r.revision_digest == verification.revision_digest
            }) {
                return Err(invalid("verification revision"));
            }
            imported
                .verifications
                .insert(verification.id.clone(), verification);
        }
        for e in entries.iter().filter(|e| e.kind == "mutation") {
            let v = &e.payload;
            let audit = MutationAudit {
                id: string(v, "event_id")?,
                epoch: integer(v, "epoch")?,
                sequence: integer(v, "sequence")?,
                operation: string(v, "operation")?,
                record_id: v["record_id"].as_str().map(str::to_owned),
                previous_revision_digest: v["previous_revision_digest"].as_str().map(str::to_owned),
                result_revision_digest: v["result_revision_digest"].as_str().map(str::to_owned),
                actor_scope_digest: string(v, "actor_scope_digest")?,
                created_at_ns: ms(v, "created_at_ms")?.ok_or_else(|| invalid("mutation time"))?,
            };
            imported.audits.insert(audit.id.clone(), audit);
        }
        for e in entries.iter().filter(|e| e.kind == "grant") {
            let v = &e.payload;
            let principal_digest = string(v, "grantee_scope_digest")?;
            let categories = v["categories"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect::<BTreeSet<_>>()
                })
                .unwrap_or_default();
            let id = string(v, "grant_id")?;
            let principal = self.scope.clone();
            let mut g = MemoryGrant {
                principal_digest: Some(principal_digest),
                id: id.clone(),
                principal,
                record_ids: BTreeSet::new(),
                categories,
                read: v["operations"]
                    .as_array()
                    .is_some_and(|ops| ops.iter().any(|op| op == "read" || op == "search")),
                expires_at_ns: ms(v, "expires_at_ms")?,
                revoked: v["revoked_at_ms"].is_number(),
                revision: integer(v, "revision")?,
                record_revisions: BTreeMap::new(),
            };
            for r in imported
                .records
                .values()
                .filter(|r| g.categories.is_empty() || g.categories.contains(&r.category))
            {
                g.record_ids.insert(r.id.clone());
                g.record_revisions
                    .insert(r.id.clone(), r.revision_digest.clone());
            }
            imported.grants.insert(id, g);
        }
        let mut merged = self.clone();
        merged.records.extend(imported.records);
        merged.revisions.extend(imported.revisions);
        for (id, source) in imported.sources {
            if merged.sources.get(&id).is_some_and(|old| old != &source) {
                return Err(ContextError::Conflict.into());
            }
            merged.sources.insert(id, source);
        }
        merged.verifications.extend(imported.verifications);
        merged.audits.extend(imported.audits);
        merged.grants.extend(imported.grants);
        merged.origins.extend(imported.origins);
        for record in merged.records.values() {
            validate_id(&record.id)?;
            let mut stack = record
                .lineage
                .iter()
                .filter(|l| super::acyclic_relation(&l.relation))
                .map(|l| l.parent_record_id.clone())
                .collect::<Vec<_>>();
            let mut visited = BTreeSet::new();
            while let Some(id) = stack.pop() {
                if id == record.id {
                    return Err(ContextError::Conflict.into());
                }
                if visited.insert(id.clone()) {
                    if let Some(r) = merged.records.get(&id) {
                        stack.extend(
                            r.lineage
                                .iter()
                                .filter(|l| super::acyclic_relation(&l.relation))
                                .map(|l| l.parent_record_id.clone()),
                        );
                    }
                }
            }
        }
        *self = merged;
        Ok(())
    }
}
