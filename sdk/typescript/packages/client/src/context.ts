import { createHash } from "node:crypto";
import type { ToolEnvelope, JsonValue } from "./anticipation";

export const CONTEXT_VERSION = 1;
export interface Scope { owner_id: string; project_id: string; workspace_id: string | null }
export interface Cursor { epoch: number; sequence: number }
export interface TokenBudget { context_tokens: number; reserved_output_tokens: number; required_tokens: number }
export type ContextAuthority = "user_asserted" | "external_observed" | "tool_observed" | "runtime_fact" | "assistant_generated" | "derived_inference";
export type MessagePart = {kind:"text";text:string} | {kind:"tool_call";call_id:string;name:string;arguments:string} | {kind:"tool_result";call_id:string;content:string;failed:boolean} | {kind:"opaque";media_type:string;reference:string;digest:string};
export interface SourceMessage { id:string; ordinal:number; role:"user"|"assistant"|"tool"; parts:readonly MessagePart[]; occurred_at_ns:bigint|string|number|null; recorded_at_ns:bigint|string|number; authority:ContextAuthority; source_digest:string }
export interface SourceSpan { source_id:string; source_digest:string; byte_start:number; byte_end:number }
export interface ContextBlock { id:string; text:string; authority:ContextAuthority; provenance:SourceSpan[]; tokens:number; required:boolean }
export interface ContextReport { version:number; scope:Scope; session_id:string; cursor:Cursor; generation:number; blocks:ContextBlock[]; included:string[]; omitted:{id:string;reason:string}[]; gaps:string[]; token_count:number; digest:string }
export interface ContextDiagnostics { version:number; session_id:string; report:ContextReport; messages:RenderedMessage[]; history:{message_count:number;cursor:Cursor}; coverage:unknown; cache:unknown; jobs:unknown; provenance:unknown; migrations:unknown; gaps:unknown }
export type ContextOwner = "host" | "hypermind";
export interface ContextHostAdapter { readonly contextOwner:ContextOwner; sourceMessages(scope:Scope,sessionId:string,cursor:Cursor):Promise<readonly SourceMessage[]>; applyContext(diagnostics:ContextDiagnostics):Promise<void> }
export interface ContextToolClient { callTool(verb:"activate"|"inspect"|"remember",input:unknown):Promise<ToolEnvelope> }
export class ContextClientError extends Error {
  constructor(readonly code:string,readonly effectState:"not_dispatched"|"unknown"|"rejected",readonly detail?:unknown) { super(code); }
}
function identifier(value:string):void { if(typeof value!=="string" || !/^[!-~]{1,256}$/.test(value)) throw new Error("invalid identifier"); }
function integer(value:number,minimum=0):void { if(!Number.isSafeInteger(value)||value<minimum) throw new Error("integer outside interoperable range"); }
export function availableContextTokens(budget:TokenBudget):number {
  integer(budget.context_tokens,1); integer(budget.reserved_output_tokens,1); integer(budget.required_tokens);
  const available=budget.context_tokens-budget.reserved_output_tokens-budget.required_tokens;
  if(available<0) throw new Error("capacity exceeded"); return available;
}
export function timestampDecimal(value:bigint|string|number):string {
  if(!["bigint","string","number"].includes(typeof value))throw new Error("invalid timestamp");
  if(typeof value==="number"&&!Number.isSafeInteger(value))throw new Error("unsafe numeric timestamp");
  if(typeof value==="string"&&!/^(0|-?[1-9][0-9]*)$/.test(value))throw new Error("noncanonical timestamp");
  const number=BigInt(value);
  if(number<-(1n<<63n)||number>=(1n<<63n))throw new Error("timestamp outside i64 range");
  return number.toString();
}
export function freezeSourceMessage(source:SourceMessage):Readonly<SourceMessage> {
  identifier(source.id); integer(source.ordinal);
  if(!["user","assistant","tool"].includes(source.role) || !["user_asserted","external_observed","tool_observed","runtime_fact","assistant_generated","derived_inference"].includes(source.authority)) throw new Error("invalid source authority or role");
  if(source.parts.length===0||source.parts.length>4096) throw new Error("invalid source parts");
  const shapes:Record<string,string[]>={text:["kind","text"],tool_call:["kind","call_id","name","arguments"],tool_result:["kind","call_id","content","failed"],opaque:["kind","media_type","reference","digest"]};
  const parts=source.parts.map(part=>{
    const keys=shapes[part.kind]; const value=part as unknown as Record<string,unknown>;
    if(!keys||Object.keys(part).length!==keys.length||keys.some(key=>typeof value[key] !== (key==="failed"?"boolean":"string"))) throw new Error("invalid source part");
    return Object.freeze(Object.fromEntries(keys.map(key=>[key,value[key]]))) as MessagePart;
  });
  const value:SourceMessage={id:source.id,ordinal:source.ordinal,role:source.role,parts:Object.freeze(parts),occurred_at_ns:source.occurred_at_ns===null?null:timestampDecimal(source.occurred_at_ns),recorded_at_ns:timestampDecimal(source.recorded_at_ns),authority:source.authority,source_digest:""};
  const digest=createHash("sha256").update(JSON.stringify(value)).digest("hex");
  if(source.source_digest && source.source_digest!==digest) throw new Error("source identity changed");
  return Object.freeze({...value,source_digest:digest});
}
export class ContextClient {
  readonly scope:Readonly<Scope>;
  private resume:Cursor={epoch:1,sequence:0};
  private negotiated:number|undefined;
  constructor(private readonly client:ContextToolClient,scope:Scope,readonly sessionId:string,readonly actor:number,readonly contextOwner:ContextOwner,readonly conversation=sessionId) {
    identifier(scope.owner_id);identifier(scope.project_id);if(scope.workspace_id!==null)identifier(scope.workspace_id);identifier(sessionId);identifier(conversation);integer(actor);if(actor>65535)throw new Error("invalid actor");
    if(contextOwner!=="host"&&contextOwner!=="hypermind")throw new Error("explicit context owner required");this.scope=Object.freeze({...scope});
  }
  get cursor():Readonly<Cursor>{return Object.freeze({...this.resume});}
  get version():number|undefined{return this.negotiated;}
  requestContext(budget:TokenBudget,generation=0,options:ActivationOptions={}):unknown {availableContextTokens(budget);integer(generation);return {version:CONTEXT_VERSION,scope:this.scope,session_id:this.sessionId,budget:{...budget},generation,...options};}
  accept(envelope:ToolEnvelope):ContextDiagnostics {
    if(envelope.ok!==true)throw new ContextClientError("context_failed",envelope.effect_state??"unknown",envelope);
    const diagnostics=envelope.items[0] as ContextDiagnostics|undefined;
    const report=diagnostics?.report;
    if(!diagnostics||!report)throw new ContextClientError("invalid_response","unknown");
    if(diagnostics.version!==CONTEXT_VERSION||report.version!==CONTEXT_VERSION)throw new ContextClientError("unsupported_version","rejected");
    if(report.session_id!==this.sessionId||report.scope?.owner_id!==this.scope.owner_id||report.scope?.project_id!==this.scope.project_id||report.scope?.workspace_id!==this.scope.workspace_id)throw new ContextClientError("scope_mismatch","rejected");
    try{integer(report.cursor.epoch,1);integer(report.cursor.sequence);}catch{throw new ContextClientError("invalid_response","unknown");}
    if(report.cursor.epoch<this.resume.epoch||(report.cursor.epoch===this.resume.epoch&&report.cursor.sequence<this.resume.sequence))throw new ContextClientError("stale_cursor","rejected");
    this.resume={...report.cursor};this.negotiated=CONTEXT_VERSION;return diagnostics;
  }
  async activate(query:string,budget:TokenBudget,options:{turnText?:string;generation?:number;context?:ActivationOptions}={}):Promise<ContextDiagnostics>{
    const context=this.requestContext(budget,options.generation,options.context);
    return this.accept(await this.client.callTool("activate",{conversation:this.conversation,query,turn_text:options.turnText??"",budget_tokens:availableContextTokens(budget),context}));
  }
  inspectUri():string{return `hm://${this.actor}/context/${encodeContextIdentifier(this.sessionId)}`;}
  async inspect():Promise<ContextDiagnostics>{return this.accept(await this.client.callTool("inspect",{uri:this.inspectUri()}));}
  retrievalRequest(operation:RetrievalOperation):RetrievalRememberInput{return {conversation:this.conversation,content:"",kind:"user",context:canonicalNanoseconds(operation) as RetrievalOperation};}
  async retrieval(operation:RetrievalOperation):Promise<ToolEnvelope>{return this.checkEnvelope(await this.client.callTool("remember",this.retrievalRequest(operation)));}
  async registerRetrievalSource(source:RetrievalSourceRecord,expectedRevision:number):Promise<ToolEnvelope>{return this.retrieval({operation:"retrieval_source",source,expected_revision:expectedRevision});}
  async tombstoneRetrievalSource(kind:EvidenceKind,id:string,expectedRevision:number):Promise<ToolEnvelope>{return this.retrieval({operation:"retrieval_tombstone",kind,id,expected_revision:expectedRevision});}
  async grantEvidence(grant:EvidenceGrant):Promise<ToolEnvelope>{return this.retrieval({operation:"retrieval_grant",grant});}
  async revokeEvidence(recipientScope:Scope,kind:EvidenceKind,id:string):Promise<ToolEnvelope>{return this.retrieval({operation:"retrieval_revoke",recipient_scope:recipientScope,kind,id});}
  async configureEmbedding(enabled:boolean,expectedRevision:number):Promise<ToolEnvelope>{return this.retrieval({operation:"embedding",enabled,expected_revision:expectedRevision});}
  async backfill(maximumItems:number,maximumBytes:number):Promise<ToolEnvelope>{return this.retrieval({operation:"backfill",maximum_items:maximumItems,maximum_bytes:maximumBytes});}
  async inspectRetrieval():Promise<ToolEnvelope>{return this.checkEnvelope(await this.client.callTool("inspect",{uri:`hm://${this.actor}/context-retrieval`}));}
  async inspectMemory(recordId?:string,ownerScope?:Scope,sourceId?:string):Promise<MemoryView>{
    let uri=`hm://${this.actor}/context-memory`;if(recordId!==undefined)uri+=`/${encodeContextIdentifier(recordId)}`;
    if(sourceId!==undefined){if(recordId===undefined)throw new Error("source inspection requires a record");uri+=`/source/${encodeContextIdentifier(sourceId)}`;}
    const owner=ownerScope??this.scope;if(ownerScope!==undefined)uri+=`?scope=${encodeURIComponent(JSON.stringify(owner))}`;
    const envelope=this.checkEnvelope(await this.client.callTool("inspect",{uri}));const view=envelope.items[0] as MemoryView;
    if(!view||view.version!==1||view.scope?.owner_id!==owner.owner_id||view.scope?.project_id!==owner.project_id||view.scope?.workspace_id!==owner.workspace_id)throw new ContextClientError("invalid_memory_view","unknown");return view;
  }
  memoryRequest(requestId:string,command:MemoryCommand):MemoryRememberInput {identifier(requestId);return {conversation:this.conversation,content:"",kind:"user",context:{operation:"memory",request:{version:1,scope:this.scope,request_id:requestId,command:canonicalNanoseconds(command) as MemoryCommand}}};}
  async memory(requestId:string,command:MemoryCommand):Promise<MemoryReceipt>{
    const envelope=this.checkEnvelope(await this.client.callTool("remember",this.memoryRequest(requestId,command)));const receipt=envelope.items[0] as MemoryReceipt;
    if(!receipt||receipt.version!==1||receipt.scope?.owner_id!==this.scope.owner_id||receipt.scope?.project_id!==this.scope.project_id||receipt.scope?.workspace_id!==this.scope.workspace_id)throw new ContextClientError("invalid_memory_receipt","unknown");return receipt;
  }
  importRequest(bundle:ImportBundle,maxEntries=128):ImportRememberInput {
    integer(maxEntries,1);if(maxEntries>256||bundle.scope.owner_id!==this.scope.owner_id||bundle.scope.project_id!==this.scope.project_id||bundle.scope.workspace_id!==this.scope.workspace_id)throw new Error("invalid import scope or batch size");
    return {conversation:this.conversation,content:"",kind:"user",context:{operation:"import",request:bundle,max_entries:maxEntries}};
  }
  async importBundle(bundle:ImportBundle,maxEntries=128):Promise<ImportReceipt>{
    const envelope=this.checkEnvelope(await this.client.callTool("remember",this.importRequest(bundle,maxEntries)));
    const receipt=envelope.items[0] as ImportReceipt;
    if(!receipt||![1,2].includes(receipt.version)||receipt.scope?.owner_id!==this.scope.owner_id||receipt.scope?.project_id!==this.scope.project_id||receipt.scope?.workspace_id!==this.scope.workspace_id||receipt.import_id!==bundle.import_id||receipt.bundle_digest!==bundle.digest)throw new ContextClientError("invalid_import_receipt","unknown");return receipt;
  }
  sourceRequest(message:SourceMessage,originalBytes:Uint8Array):SourceRememberInput {
    return {conversation:this.conversation,content:"",kind:"user",context:{operation:"source",request:{version:1,scope:this.scope,session_id:this.sessionId,conversation:this.conversation,message:freezeSourceMessage(message),original_bytes:Array.from(originalBytes)}}};
  }
  async ingestSource(message:SourceMessage,originalBytes:Uint8Array):Promise<HistoryReceipt>{return this.historyReceipt(await this.client.callTool("remember",this.sourceRequest(message,originalBytes)));}
  relationRequest(relation:SourceRelation):RelationRememberInput {return {conversation:this.conversation,content:"",kind:"user",context:{operation:"relation",request:{version:1,scope:this.scope,session_id:this.sessionId,conversation:this.conversation,relation}}};}
  async relate(relation:SourceRelation):Promise<HistoryReceipt>{return this.historyReceipt(await this.client.callTool("remember",this.relationRequest(relation)));}
  forkRequest(childSessionId:string,childConversation=childSessionId):ForkRememberInput {identifier(childSessionId);identifier(childConversation);return {conversation:childConversation,content:"",kind:"user",context:{operation:"fork",request:{version:1,scope:this.scope,parent_session_id:this.sessionId,parent_conversation:this.conversation,child_session_id:childSessionId,child_conversation:childConversation}}};}
  async fork(childSessionId:string,childConversation=childSessionId):Promise<HistoryReceipt>{return this.historyReceipt(await this.client.callTool("remember",this.forkRequest(childSessionId,childConversation)),childSessionId);}
  jobRequest(requestId:string,action:ContextJobAction):JobRememberInput {identifier(requestId);return {conversation:this.conversation,content:"",kind:"user",context:{operation:"job",request:{version:1,scope:this.scope,request_id:requestId,action:canonicalNanoseconds(canonicalJobSources(action)) as ContextJobAction}}};}
  async job(requestId:string,action:ContextJobAction):Promise<ToolEnvelope>{return this.checkEnvelope(await this.client.callTool("remember",this.jobRequest(requestId,action)));}
  private checkEnvelope(result:ToolEnvelope):ToolEnvelope {if(result.ok!==true)throw new ContextClientError("context_operation_failed",result.effect_state??"unknown",result);return result;}
  private historyReceipt(result:ToolEnvelope,sessionId=this.sessionId):HistoryReceipt {
    this.checkEnvelope(result);const receipt=result.items[0] as HistoryReceipt;
    if(!receipt||receipt.version!==1||receipt.session_id!==sessionId||receipt.scope?.owner_id!==this.scope.owner_id||receipt.scope?.project_id!==this.scope.project_id||receipt.scope?.workspace_id!==this.scope.workspace_id)throw new ContextClientError("invalid_receipt","unknown");
    try{integer(receipt.cursor.epoch,1);integer(receipt.cursor.sequence);}catch{throw new ContextClientError("invalid_receipt","unknown");}
    if(sessionId===this.sessionId)this.resume={...receipt.cursor};return receipt;
  }

  async remember(content:string,kind:"user"|"assistant"|"document"="user"):Promise<ToolEnvelope>{
    if(kind!=="user"&&kind!=="assistant"&&kind!=="document")throw new Error("unsupported message kind");
    const result=await this.client.callTool("remember",{conversation:this.conversation,content,kind});
    if(result.ok!==true)throw new ContextClientError("remember_failed",result.effect_state??"unknown",result);return result;
  }

}

export type SourceRelation = {kind:"edit"|"regenerate";id:string;original_id:string;replacement_id:string}|{kind:"tombstone";id:string;source_id:string};
export interface HistoryReceipt {version:number;scope:Scope;session_id:string;cursor:Cursor;last_lsn:number;replayed:boolean;source_span:SourceSpan|null}
export interface SourceChunk {sources:readonly SourceMessage[];spans:SourceSpan[];digest:string}
export interface SummaryTier {text:string;coverage:SourceSpan[]}
export interface HistorianResult {source_digest:string;tiers:[SummaryTier,SummaryTier,SummaryTier,SummaryTier]}
export type HistorianState={status:"pending";ready_at_ms:number}|{status:"claimed";worker:string;expires_at_ms:number}|{status:"complete"|"cancelled"};
export interface HistorianJob {id:string;session_id:string;cursor:Cursor;policy_revision:number;chunk:SourceChunk;attempt:number;state:HistorianState;result:HistorianResult|null}
export interface HistorianClaim {job:HistorianJob;worker:string;attempt:number}
export type JobKind="historian"|"verification"|"curation"|"extraction"|"indexing"|"consolidation";
export interface MaintenanceRequest {kind:JobKind;sources:readonly SourceMessage[];cursor:Cursor;source_revision:number;policy_revision:number;reservation:number}
export interface PublicationFence {input_digest:string;source_revision:number;policy_revision:number}
export interface JobLease {job_id:string;attempt:number;expires_ms:number;fence:PublicationFence}
export type Usage="Unknown"|{Known:number};
export type NotePredicate="True"|{Exists:string}|{Equals:{key:string;value:string}}|{Not:NotePredicate}|{All:NotePredicate[]}|{Any:NotePredicate[]};
export interface Note {id:string;kind:"Anchor"|"Note"|"Primer";revision:number;text:string;parents:string[];contradictions:string[];expires_at_ns:bigint|string|number|null;predicate:NotePredicate;tombstoned:boolean}
export interface NoteGrant {principal:string;note_id:string;read:boolean;write:boolean}
export interface AttributeProposal {id:string;key:string;value:string;base_revision:number;proposer:string}
export type NotesCommand={Create:Note}|{Revise:{note:Note;expected_revision:number}}|{Tombstone:{id:string;expected_revision:number}}|{SetGrant:NoteGrant}|{SetAttributeGrant:{principal:string;key:string;write:boolean}}|{RevokeGrant:{principal:string;note_id:string}}|{ProposeAttribute:AttributeProposal}|{AcceptAttribute:{proposal_id:string}}|{RejectAttribute:{proposal_id:string}};
export type ContextJobAction=
 |{action:"notes";command:NotesCommand}
 |{action:"read_note";id:string;now_ns:bigint|string|number;facts:Record<string,string>}
 |{action:"read_attribute";key:string}
 |{action:"set_policy";session_id:string;expected_revision:number;revision:number}
 |{action:"historian_enqueue";session_id:string;cursor:Cursor;policy_revision:number;chunk:SourceChunk;reservation?:number;now_ms?:number}
 |{action:"historian_claim";worker:string;now_ms:number;lease_ms:number}
 |{action:"historian_heartbeat";claim:HistorianClaim;now_ms:number;lease_ms:number}
 |{action:"historian_complete";claim:HistorianClaim;result:HistorianResult;usage?:Usage;now_ms?:number}
 |{action:"historian_fail";claim:HistorianClaim;usage?:Usage;now_ms:number;cooldown_ms:number}
 |{action:"historian_cancel";id:string}
 |{action:"historian_expire";now_ms:number;cooldown_ms:number}
 |{action:"maintenance_enqueue";session_id:string;request:MaintenanceRequest}
 |{action:"maintenance_claim";now_ms:number}
 |{action:"maintenance_complete";lease:JobLease;usage:Usage;output_digest:string;now_ms:number}
 |{action:"maintenance_fail";lease:JobLease;usage:Usage;now_ms:number}
 |{action:"maintenance_cancel";id:string;now_ms:number}
 |{action:"maintenance_settle";id:string;attempt:number;actual:number}
 |{action:"inspect"};
export function encodeContextIdentifier(value:string):string {identifier(value);return encodeURIComponent(value).replace(/[!'()*]/g,c=>`%${c.charCodeAt(0).toString(16).toUpperCase()}`);}

function canonicalJobSources(action:ContextJobAction):ContextJobAction {
  if(action.action==="maintenance_enqueue")return {...action,request:{...action.request,sources:action.request.sources.map(freezeSourceMessage)}};
  if(action.action==="historian_enqueue")return {...action,chunk:{...action.chunk,sources:action.chunk.sources.map(freezeSourceMessage)}};
  if(action.action==="historian_heartbeat"||action.action==="historian_complete"||action.action==="historian_fail")return {...action,claim:{...action.claim,job:{...action.claim.job,chunk:{...action.claim.job.chunk,sources:action.claim.job.chunk.sources.map(freezeSourceMessage)}}}};
  return action;
}

interface RememberContextInput<C> {conversation:string;content:"";kind:"user";context:C}
export type SourceRememberInput=RememberContextInput<{operation:"source";request:{version:1;scope:Scope;session_id:string;conversation:string;message:Readonly<SourceMessage>;original_bytes:number[]}}>;
export type RelationRememberInput=RememberContextInput<{operation:"relation";request:{version:1;scope:Scope;session_id:string;conversation:string;relation:SourceRelation}}>;
export type ForkRememberInput=RememberContextInput<{operation:"fork";request:{version:1;scope:Scope;parent_session_id:string;parent_conversation:string;child_session_id:string;child_conversation:string}}>;
export type JobRememberInput=RememberContextInput<{operation:"job";request:{version:1;scope:Scope;request_id:string;action:ContextJobAction}}>;

export interface ImportEntry {source_id:string;kind:string;digest:string;payload:JsonValue}
export interface ImportBundle {version:number;import_id:string;scope:Scope;entries:ImportEntry[];digest:string}
export interface ImportReceipt {version:number;scope:Scope;import_id:string;bundle_digest:string;accepted:number;total:number;complete:boolean;last_lsn:number}
export type ImportRememberInput=RememberContextInput<{operation:"import";request:ImportBundle;max_entries:number}>;
function canonicalJson(value:JsonValue):JsonValue {
  if(Array.isArray(value))return value.map(canonicalJson);
  if(value!==null&&typeof value==="object")return Object.fromEntries(Object.keys(value).sort().map(key=>[key,canonicalJson(value[key]!)]));
  if(typeof value==="number"&&!Number.isSafeInteger(value))throw new Error("import numeric payload requires safe integers");return value;
}
export function makeImportBundle(scope:Scope,importId:string,entries:readonly {source_id:string;kind:string;payload:JsonValue}[]):ImportBundle {
  identifier(importId);const wireEntries=entries.map(entry=>{identifier(entry.source_id);const payload=canonicalJson(entry.payload);return {source_id:entry.source_id,kind:entry.kind,digest:createHash("sha256").update(canonicalJSONString(payload)).digest("hex"),payload};});
  const bundle:ImportBundle={version:1,import_id:importId,scope:{owner_id:scope.owner_id,project_id:scope.project_id,workspace_id:scope.workspace_id},entries:wireEntries,digest:""};
  const encodedEntries=wireEntries.map(e=>`{"source_id":${JSON.stringify(e.source_id)},"kind":${JSON.stringify(e.kind)},"digest":${JSON.stringify(e.digest)},"payload":${canonicalJSONString(e.payload)}}`).join(",");
  const encoded=`{"version":1,"import_id":${JSON.stringify(importId)},"scope":${JSON.stringify(bundle.scope)},"entries":[${encodedEntries}],"digest":""}`;
  bundle.digest=createHash("sha256").update(encoded).digest("hex");return bundle;
}

function canonicalNanoseconds(value:unknown):unknown {
  if(Array.isArray(value))return value.map(canonicalNanoseconds);
  if(value!==null&&typeof value==="object")return Object.fromEntries(Object.entries(value).map(([key,item])=>[key,key.endsWith("_ns")?(item===null?null:timestampDecimal(item as bigint|string|number)):canonicalNanoseconds(item)]));
  return value;
}

export type RecordKind="fact"|"episode"|"note"|"conditional_note"|"anchor"|"summary"|"primer";
export type RecordStatus="active"|"archived"|"stale"|"tombstoned";
export type Nanoseconds=bigint|string|number;
export interface MemoryProvenance {source_id:string;source_digest:string;span_start:number;span_end:number;quoted_digest:string}
export interface MemoryLineage {child_record_id:string;child_revision:number;parent_record_id:string;parent_revision_digest:string;relation:string;created_at_ns:Nanoseconds}
export interface SmartCondition {operator:string;clauses:{field:string;comparison:string;value:JsonValue}[]}
export interface MemoryRecord {id:string;kind:RecordKind;category:string;status:RecordStatus;revision:number;revision_digest:string;content:string;authority:ContextAuthority;confidence:number;importance:number;occurred_at_ns:Nanoseconds|null;recorded_at_ns:Nanoseconds;expires_at_ns:Nanoseconds|null;pinned:boolean;provenance:MemoryProvenance[];lineage:MemoryLineage[];contradictions:string[];last_lsn:number;predicate:NotePredicate|null;smart_condition:SmartCondition|null;retention_until_ns:Nanoseconds|null;metadata:JsonValue}
export interface MemorySource {id:string;digest:string;content:number[];locator:string;occurred_at_ns:Nanoseconds|null;recorded_at_ns:Nanoseconds;tombstoned:boolean}
export interface MemoryVerification {id:string;record_id:string;revision_digest:string;state:"supported"|"contradicted"|"unresolved";evidence_source_id:string|null;confidence:number;created_at_ns:Nanoseconds}
export interface MemoryGrant {principal_digest:string|null;id:string;principal:Scope;record_ids:string[];categories:string[];read:boolean;expires_at_ns:Nanoseconds|null;revoked:boolean;revision:number;record_revisions:Record<string,string>}
export type MemoryCommand={kind:"create";record:MemoryRecord}|{kind:"revise";record:MemoryRecord;expected_revision:number}|{kind:"tombstone";id:string;expected_revision:number}|{kind:"set_status";id:string;status:RecordStatus;expected_revision:number}|{kind:"source";source:MemorySource}|{kind:"tombstone_source";id:string}|{kind:"verify";verification:MemoryVerification}|{kind:"set_grant";grant:MemoryGrant}|{kind:"revoke_grant";id:string}|{kind:"lineage";lineage:MemoryLineage};
export interface MemoryReceipt {version:number;scope:Scope;cursor:number;last_lsn:number;replayed:boolean}
export type MemoryRememberInput=RememberContextInput<{operation:"memory";request:{version:1;scope:Scope;request_id:string;command:MemoryCommand}}>;
export function sealMemoryRecord(record:MemoryRecord):MemoryRecord {
  const value:MemoryRecord={id:record.id,kind:record.kind,category:record.category,status:record.status,revision:record.revision,revision_digest:"",content:record.content,authority:record.authority,confidence:record.confidence,importance:record.importance,occurred_at_ns:record.occurred_at_ns===null?null:timestampDecimal(record.occurred_at_ns),recorded_at_ns:timestampDecimal(record.recorded_at_ns),expires_at_ns:record.expires_at_ns===null?null:timestampDecimal(record.expires_at_ns),pinned:record.pinned,provenance:record.provenance.map(p=>({source_id:p.source_id,source_digest:p.source_digest,span_start:p.span_start,span_end:p.span_end,quoted_digest:p.quoted_digest})),lineage:record.lineage.map(l=>({child_record_id:l.child_record_id,child_revision:l.child_revision,parent_record_id:l.parent_record_id,parent_revision_digest:l.parent_revision_digest,relation:l.relation,created_at_ns:timestampDecimal(l.created_at_ns)})),contradictions:record.contradictions,last_lsn:0,predicate:record.predicate,smart_condition:record.smart_condition===null?null:{operator:record.smart_condition.operator,clauses:record.smart_condition.clauses.map(c=>({field:c.field,comparison:c.comparison,value:canonicalJson(c.value)}))},retention_until_ns:record.retention_until_ns===null?null:timestampDecimal(record.retention_until_ns),metadata:canonicalJson(record.metadata)};
  value.revision_digest=createHash("sha256").update(canonicalJSONString(value as unknown as JsonValue)).digest("hex");value.last_lsn=record.last_lsn;return value;
}
export function makeMemoryRecord(id:string,kind:RecordKind,content:string,recordedAtNs:Nanoseconds):MemoryRecord {
  identifier(id);return sealMemoryRecord({id,kind,category:"general",status:"active",revision:1,revision_digest:"",content,authority:"user_asserted",confidence:1000000,importance:500000,occurred_at_ns:null,recorded_at_ns:recordedAtNs,expires_at_ns:null,pinned:false,provenance:[],lineage:[],contradictions:[],last_lsn:0,predicate:null,smart_condition:null,retention_until_ns:null,metadata:null});
}

function canonicalJSONString(value:JsonValue):string {
  if(Array.isArray(value))return `[${value.map(canonicalJSONString).join(",")}]`;
  if(value!==null&&typeof value==="object")return `{${Object.keys(value).sort((a,b)=>Buffer.compare(Buffer.from(a),Buffer.from(b))).map(key=>`${JSON.stringify(key)}:${canonicalJSONString(value[key]!)}`).join(",")}}`;
  if(typeof value==="number"&&!Number.isSafeInteger(value))throw new Error("canonical JSON numeric payload requires safe integers");return JSON.stringify(value);
}

export type MemoryView={version:number;scope:Scope;principal:Scope;cursor:number;records:MemoryRecord[]}|{version:number;scope:Scope;record:MemoryRecord;cursor:number}|{version:number;scope:Scope;record_id:string;source:MemorySource};

export interface CapabilityProfile {user:boolean;assistant:boolean;tool:boolean;text:boolean;tool_calls:boolean;tool_results:boolean;opaque:boolean}
export interface RenderedMessage {id:string;role:"user"|"assistant"|"tool";parts:MessagePart[];authority:ContextAuthority;provenance:SourceSpan[]}
export interface ActivationOptions {model_id?:string;profile?:CapabilityProfile;tier?:"detailed"|"condensed"|"brief"|"outline";defer_reductions?:boolean;required_message_ids?:string[];utc_offset_seconds?:number|null;memory_scopes?:Scope[];evidence_grants?:EvidenceGrant[]}

export type EvidenceKind="memory"|"conversation"|"file"|"commit"|"document"|"entity"|"relationship";
export interface RetrievalSourceRecord {scope:Scope;kind:EvidenceKind;id:string;revision:number;text:string;content_digest:string;authority:ContextAuthority;provenance:SourceSpan[];occurred_at_ns:Nanoseconds|null;recorded_at_ns:Nanoseconds;expires_at_ns:Nanoseconds|null;tombstoned:boolean}
export interface EvidenceGrant {source_scope:Scope;recipient_scope:Scope;kind:EvidenceKind;source_id:string;source_digest:string;source_revision:number;expires_at_ns:Nanoseconds}
export type RetrievalOperation={operation:"retrieval_source";expected_revision:number;source:RetrievalSourceRecord}|{operation:"retrieval_tombstone";kind:EvidenceKind;id:string;expected_revision:number}|{operation:"retrieval_grant";grant:EvidenceGrant}|{operation:"retrieval_revoke";recipient_scope:Scope;kind:EvidenceKind;id:string}|{operation:"embedding";expected_revision:number;enabled:boolean}|{operation:"backfill";maximum_items:number;maximum_bytes:number};
export type RetrievalRememberInput=RememberContextInput<RetrievalOperation>;
