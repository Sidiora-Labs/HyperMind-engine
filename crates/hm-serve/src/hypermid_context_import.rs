use crate::{actor::{ActorEngine, IncomingEvent}, hypermid_import::{ImportBundle, ImportEntry, ImportError, ImportReceipt}};
use hm_context::{history::{SourceHistory, SourceRelation}, Authority, ContextError, Cursor, MessagePart, MessageRole, Scope, SourceMessage, digest_bytes, validate_id};
use hm_core::{ConversationId, LSN};
use hm_ledger::frame::EventKind;
use hm_schema::{event::{self, CURRENT_SCHEMA_VERSION}, events::{EventEnvelope, EventPayload, ProviderFrame, Retention, Sensitivity}};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const PROVIDER: &str = "hypermind/hypermid-context-import/v1";
const MAX_FRAME: usize = 512 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextSourceMaterial {
    pub version: u32,
    pub scope: Scope,
    pub session_id: String,
    pub item: Value,
    pub original_bytes: Vec<u8>,
}
#[derive(Clone, Debug)]
pub struct PreparedContextImport { bundle: ImportBundle, session_id: String, conversation: String, history: SourceHistory }
#[derive(Clone, Debug)]
pub struct ImportedHistory { pub scope: Scope, pub session_id: String, pub conversation: String, pub history: SourceHistory, pub source_uris: Vec<String>, pub originals: Vec<ContextSourceMaterial> }
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Item {
    item_id: String, source_event_id: String, source_digest: String, scope: Scope, session_id: String,
    cursor: Cursor, role: String, parts: Vec<Part>, #[serde(default)] relations: Vec<Relation>, created_at: String,
    recoverable: bool, #[serde(default)] tombstone: bool,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Part {
    part_id: String, kind: String, content_digest: String, text: Option<String>, call_id: Option<String>, tool_name: Option<String>,
    arguments_json: Option<String>, result_json: Option<String>, media_type: Option<String>, source_uri: Option<String>, width: Option<u64>, height: Option<u64>, metadata: Option<Value>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Relation { kind: String, item_id: String, source_digest: Option<String> }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag="kind", rename_all="snake_case", deny_unknown_fields)]
enum Record {
    Stage { version: u32, scope: Scope, import_id: String, bundle_digest: String, index: usize, total: usize, entry: ImportEntry, sources: Vec<ContextSourceMaterial> },
    Commit { version: u32, scope: Scope, import_id: String, bundle_digest: String, session_id: String, conversation: String, total: usize },
}
fn invalid(message: &str) -> ContextError { ContextError::Invalid(message.into()) }
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, ContextError> { value.get(key).and_then(Value::as_str).ok_or_else(||invalid("missing context source identity")) }
fn digest(value: &str) -> Result<(), ContextError> { if value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()) { Ok(()) } else { Err(invalid("invalid context source digest")) } }
fn structured(fields: &[&[u8]]) -> String { let mut bytes = Vec::new(); for field in fields { bytes.extend_from_slice(&(field.len() as u64).to_be_bytes()); bytes.extend_from_slice(field); } digest_bytes(&bytes) }

pub fn prepare(bundle: &ImportBundle) -> Result<PreparedContextImport, ImportError> {
    bundle.validate()?;
    if bundle.entries.is_empty() || bundle.context_sources.is_empty() { return Err(ContextError::Unavailable("verified source resolver material is required".into()).into()); }
    let session_id = string(&bundle.entries[0].payload,"session_id")?.to_owned(); validate_id(&session_id)?;
    let conversation = session_id.clone();
    let mut history = SourceHistory::new(bundle.scope.clone(),&session_id)?;
    let mut resolved = BTreeMap::new();
    for material in &bundle.context_sources {
        if material.version != 1 || material.scope != bundle.scope || material.session_id != session_id { return Err(ContextError::ScopeMismatch.into()); }
        let item: Item = serde_json::from_value(material.item.clone())?;
        if item.scope != bundle.scope || item.session_id != session_id || !item.recoverable || material.original_bytes.is_empty() || digest_bytes(&material.original_bytes) != item.source_digest { return Err(invalid("unverified context resolver material").into()); }
        if resolved.insert(item.item_id.clone(),(item,material)).is_some() { return Err(ContextError::Conflict.into()); }
    }
    let mut previous = None;
    let mut seen = BTreeSet::new();
    let mut prior: BTreeMap<String,(String,String)> = BTreeMap::new();
    for entry in &bundle.entries {
        if entry.kind != "source_reference" { return Err(ContextError::Unavailable("context derived portability records require a declared adapter".into()).into()); }
        let payload = &entry.payload;
        if serde_json::from_value::<Scope>(payload["scope"].clone())? != bundle.scope || string(payload,"session_id")? != session_id { return Err(ContextError::ScopeMismatch.into()); }
        let id = string(payload,"item_id")?; validate_id(id)?;
        let source_id = string(payload,"source_event_id")?; validate_id(source_id)?;
        let raw_digest = string(payload,"source_digest")?; digest(raw_digest)?;
        let cursor: Cursor = serde_json::from_value(payload["cursor"].clone())?; cursor.validate()?;
        if cursor.sequence == 0 || previous.is_some_and(|old:Cursor| cursor.epoch != old.epoch || cursor <= old) || !seen.insert(id.to_owned()) { return Err(invalid("unordered context source references").into()); }
        previous = Some(cursor);
        let (item,material) = resolved.get(id).ok_or_else(||ContextError::Unavailable("context source resolver has no matching item".into()))?;
        if item.item_id != id || item.source_event_id != source_id || item.source_digest != raw_digest || item.cursor != cursor || item.parts.is_empty() || item.parts.len()>4096 || item.relations.len()>64 { return Err(invalid("context reference does not match resolver item").into()); }
        validate_id(&item.item_id)?;
        let (role,authority) = match item.role.as_str() {
            "user" => (MessageRole::User,Authority::ExternalObserved),
            "assistant" => (MessageRole::Assistant,Authority::AssistantGenerated),
            "tool" => (MessageRole::Tool,Authority::ToolObserved),
            _ => return Err(ContextError::Unavailable("privileged context roles cannot be imported".into()).into()),
        };
        let mut parts = Vec::new();
        let mut part_ids = BTreeSet::new();
        for part in &item.parts {
            validate_id(&part.part_id)?; digest(&part.content_digest)?;
            if !part_ids.insert(&part.part_id) || part.metadata.as_ref().is_some_and(|v| v.get("authority").is_some()) { return Err(invalid("invalid context part identity or authority").into()); }
            let mapped = match part.kind.as_str() {
                "text" => { let text=part.text.as_ref().ok_or_else(||invalid("context text missing"))?; if text.len()>1_048_576 || digest_bytes(text.as_bytes())!=part.content_digest { return Err(invalid("context text digest mismatch").into()); } MessagePart::Text{text:text.clone()} },
                "tool_call" => { let call=part.call_id.as_deref().ok_or_else(||invalid("call identity missing"))?; let name=part.tool_name.as_deref().ok_or_else(||invalid("tool name missing"))?; let arguments=part.arguments_json.as_deref().ok_or_else(||invalid("tool arguments missing"))?; validate_id(call)?;validate_id(name)?; if role!=MessageRole::Assistant || !serde_json::from_str::<Value>(arguments)?.is_object() || structured(&[call.as_bytes(),name.as_bytes(),arguments.as_bytes()])!=part.content_digest {return Err(invalid("context tool call invalid").into());} MessagePart::ToolCall{call_id:call.into(),name:name.into(),arguments:arguments.into()} },
                "tool_result" => { let call=part.call_id.as_deref().ok_or_else(||invalid("call identity missing"))?;let result=part.result_json.as_deref().ok_or_else(||invalid("tool result missing"))?;validate_id(call)?;serde_json::from_str::<Value>(result)?;if role!=MessageRole::Tool || structured(&[call.as_bytes(),result.as_bytes()])!=part.content_digest{return Err(invalid("context tool result invalid").into());}MessagePart::ToolResult{call_id:call.into(),content:result.into(),failed:false} },
                "image"|"file" => {let media=part.media_type.as_deref().ok_or_else(||invalid("media type missing"))?;let uri=part.source_uri.as_deref().ok_or_else(||invalid("media reference missing"))?;let width=part.width.unwrap_or(0).to_be_bytes();let height=part.height.unwrap_or(0).to_be_bytes();if media.is_empty()||media.len()>128||uri.is_empty()||uri.len()>4096||part.width.is_some_and(|v|v==0||v>100000)||part.height.is_some_and(|v|v==0||v>100000)||structured(&[media.as_bytes(),uri.as_bytes(),&width,&height])!=part.content_digest{return Err(invalid("context media digest mismatch").into());}MessagePart::Opaque{media_type:media.into(),reference:uri.into(),digest:part.content_digest.clone()}},
                _ => return Err(ContextError::Unavailable("context part requires a declared rendering adapter".into()).into()),
            }; parts.push(mapped);
        }
        let recorded = chrono::DateTime::parse_from_rfc3339(&item.created_at).map_err(|_|invalid("invalid context creation timestamp"))?.timestamp_nanos_opt().ok_or_else(||invalid("context timestamp outside nanosecond range"))?;
        let mut source=SourceMessage{id:source_id.into(),ordinal:cursor.sequence,role,parts,occurred_at_ns:None,recorded_at_ns:recorded,authority,source_digest:String::new()};source.source_digest=source.computed_digest()?;
        history.ingest(source,material.original_bytes.clone())?;
        if item.tombstone && item.relations.is_empty(){return Err(invalid("unrelated context tombstone").into());}
        for (index,relation) in item.relations.iter().enumerate() {
            let (target,target_digest)=prior.get(&relation.item_id).ok_or_else(||invalid("context relation target missing"))?;
            if relation.source_digest.as_ref().is_some_and(|digest|digest!=target_digest){return Err(ContextError::Stale.into());}
            let relation_id=format!("hypermid:{}:{index}",digest_bytes(item.item_id.as_bytes()));
            let mapped=if item.tombstone {Some(SourceRelation::Tombstone{id:relation_id,source_id:target.clone()})} else {match relation.kind.as_str(){"supersedes"=>Some(SourceRelation::Edit{id:relation_id,original_id:target.clone(),replacement_id:source_id.into()}),"regenerates"=>Some(SourceRelation::Regenerate{id:relation_id,original_id:target.clone(),replacement_id:source_id.into()}),"continues"|"forks_from"|"imports"|"derived_from"|"contributed_by"=>None,_=>return Err(invalid("unsupported context relation").into())}};
            if let Some(relation)=mapped{history.relate(relation)?;}
        }
        if item.tombstone {history.relate(SourceRelation::Tombstone{id:format!("hypermid-marker:{}",digest_bytes(id.as_bytes())),source_id:source_id.into()})?;}
        prior.insert(id.into(),(source_id.into(),raw_digest.into()));
    }
    if seen.len()!=resolved.len(){return Err(invalid("unreferenced context resolver material").into());}
    let prepared=PreparedContextImport{bundle:bundle.clone(),session_id,conversation,history};
    for index in 0..bundle.entries.len(){if serde_json::to_vec(&stage(&prepared,index))?.len()>MAX_FRAME{return Err(ContextError::Capacity.into());}}
    Ok(prepared)
}
fn stage(prepared:&PreparedContextImport,index:usize)->Record {
    let entry=prepared.bundle.entries[index].clone();let id=entry.payload["item_id"].as_str();
    let sources=prepared.bundle.context_sources.iter().filter(|source|source.item["item_id"].as_str()==id).cloned().collect();
    Record::Stage{version:1,scope:prepared.bundle.scope.clone(),import_id:prepared.bundle.import_id.clone(),bundle_digest:prepared.bundle.digest.clone(),index,total:prepared.bundle.entries.len(),entry,sources}
}
fn incoming(record:&Record)->Result<IncomingEvent,ImportError>{
    let (scope,id)=match record{Record::Stage{scope,import_id,..}|Record::Commit{scope,import_id,..}=>(scope,import_id)};
    Ok(IncomingEvent{kind:EventKind::ProviderFrame,conversation:ConversationId::derive(&format!("hypermid-context:{}:{id}",scope.digest()?)),payload:event::encode_event_envelope(&EventEnvelope{schema_version:CURRENT_SCHEMA_VERSION,payload:EventPayload::ProviderFrame(Box::new(ProviderFrame{provider:PROVIDER.into(),api_content:serde_json::to_vec(record)?})),connection_id:None,client_seq:0,client_event_index:0,client_event_count:1,origin_actor:0,run_id:None,model_provenance:None,authority:hm_schema::events::Authority::RuntimeFact,retention:Retention::Durable,sensitivity:Sensitivity::Personal,event_time_ns:0})})
}
async fn records(actor:&ActorEngine)->Result<Vec<(u64,Record)>,ImportError>{
    let mut records=Vec::new();for frame in actor.frames_since(LSN::new(0),None,usize::MAX).await?{if frame.header.kind!=EventKind::ProviderFrame{continue;}let verified=actor.verified_event(frame.header.lsn).await?;if let EventPayload::ProviderFrame(p)=verified.envelope.payload{if p.provider==PROVIDER{records.push((frame.header.lsn.get(),serde_json::from_slice(&p.api_content)?));}}}Ok(records)
}
pub async fn import_batch_locked(actor:&ActorEngine,prepared:&PreparedContextImport,max_entries:usize)->Result<ImportReceipt,ImportError>{
    if max_entries==0{return Err(invalid("zero context batch").into());}
    let mut tail=actor.stats().await?.applied.last_lsn;
    let mut accepted=0;let mut last_lsn=0;let mut committed=false;
    for (lsn,record) in records(actor).await?{let original=record.clone();match record{
        Record::Stage{scope,import_id,bundle_digest,index,..} if scope==prepared.bundle.scope&&import_id==prepared.bundle.import_id=>{if bundle_digest!=prepared.bundle.digest||index!=accepted||index>=prepared.bundle.entries.len()||original!=stage(prepared,index){return Err(ContextError::Conflict.into());}accepted+=1;last_lsn=lsn;},
        Record::Commit{scope,import_id,bundle_digest,session_id,conversation,total,..} if scope==prepared.bundle.scope&&import_id==prepared.bundle.import_id=>{if bundle_digest!=prepared.bundle.digest||session_id!=prepared.session_id||conversation!=prepared.conversation||total!=prepared.bundle.entries.len()||accepted!=total{return Err(ContextError::Conflict.into());}committed=true;last_lsn=lsn;},_=>{}}
    }
    if !committed {
        let native=crate::context_history::replay(actor,&prepared.bundle.scope,&prepared.session_id,&prepared.conversation).await.map_err(|error|match error{crate::context_history::HistoryError::Context(e)=>ImportError::Context(e),crate::context_history::HistoryError::Ledger(e)=>ImportError::Ledger(e)})?;
        if !native.history.messages().is_empty(){return Err(ContextError::Conflict.into());}
        let end=accepted.saturating_add(max_entries).min(prepared.bundle.entries.len());
        while accepted<end{let outcome=actor.append_if_tail(tail,vec![incoming(&stage(prepared,accepted))?]).await?;tail=outcome.last_lsn;last_lsn=tail.get();accepted+=1;}
        if accepted==prepared.bundle.entries.len(){let commit=Record::Commit{version:1,scope:prepared.bundle.scope.clone(),import_id:prepared.bundle.import_id.clone(),bundle_digest:prepared.bundle.digest.clone(),session_id:prepared.session_id.clone(),conversation:prepared.conversation.clone(),total:accepted};let outcome=actor.append_if_tail(tail,vec![incoming(&commit)?]).await?;last_lsn=outcome.last_lsn.get();committed=true;}
    }
    Ok(ImportReceipt{version:2,scope:prepared.bundle.scope.clone(),import_id:prepared.bundle.import_id.clone(),bundle_digest:prepared.bundle.digest.clone(),accepted,total:prepared.bundle.entries.len(),complete:committed,last_lsn})
}
pub async fn replay_committed(actor:&ActorEngine,content:&[u8])->Result<Option<ImportedHistory>,ImportError>{
    let commit:Record=serde_json::from_slice(content)?;
    let Record::Commit{version,scope,import_id,bundle_digest,session_id,conversation,total}=commit else{return Ok(None)};
    if version!=1{return Err(invalid("unsupported context publication").into());}
    let mut entries=Vec::new();let mut context_sources=Vec::new();let mut source_uris=Vec::new();
    for (lsn,record) in records(actor).await?{if let Record::Stage{version,scope:s,import_id:id,bundle_digest:d,index,total:t,entry,sources}=record{if s==scope&&id==import_id{if version!=1||d!=bundle_digest||index!=entries.len()||t!=total{return Err(ContextError::Conflict.into());}entries.push(entry);context_sources.extend(sources);source_uris.push(format!("hm://{}/lsn/{lsn}",actor.actor()));}}}
    if entries.len()!=total{return Err(invalid("incomplete context publication").into());}
    let bundle=ImportBundle{version:1,scope:scope.clone(),import_id,entries,digest:bundle_digest,context_sources};let prepared=prepare(&bundle)?;
    if session_id!=prepared.session_id||conversation!=prepared.conversation{return Err(ContextError::ScopeMismatch.into());}
    Ok(Some(ImportedHistory{scope,session_id,conversation,history:prepared.history,source_uris,originals:bundle.context_sources}))
}
