import {ContextClient,ContextClientError,freezeSourceMessage} from './context';
import type {ActivationOptions,ContextDiagnostics,ContextToolClient,Cursor,Scope,SourceMessage,TokenBudget,SourceRelation,ContextJobAction,MemoryCommand,ImportBundle,CapabilityProfile} from './context';

export type MutationEffect='not_dispatched'|'unknown'|'rejected';
export interface ContinuationState {readonly cursor:Readonly<Cursor>;readonly generation:number;readonly modelId:string|null;readonly profile:Readonly<CapabilityProfile>|null}
export interface MutationOutcome {readonly operation:string;readonly effectState:MutationEffect;readonly identity:string;readonly detail:string}
export interface ContinuationCheckpoint {readonly scope:Readonly<Scope>;readonly actor:number;readonly sessionId:string;readonly conversation:string;readonly contextOwner:string;readonly accepted:ContinuationState|null;readonly uncertain:MutationOutcome|null;readonly unresolved:readonly MutationOutcome[]}
export interface ContinuityTransport extends ContextToolClient {close():Promise<void>}
export interface ContinuityBinding {context:ContextClient;transport:ContinuityTransport}
export class ContinuationError extends ContextClientError {}
const sameScope=(a:Readonly<Scope>,b:Readonly<Scope>)=>a.owner_id===b.owner_id&&a.project_id===b.project_id&&a.workspace_id===b.workspace_id;
const copyState=(state:ContinuationState|null):ContinuationState|null=>{if(state===null)return null;if(!Number.isSafeInteger(state.generation)||state.generation<0||!Number.isSafeInteger(state.cursor.epoch)||state.cursor.epoch<1||!Number.isSafeInteger(state.cursor.sequence)||state.cursor.sequence<0)throw new ContinuationError('invalid_checkpoint_state','not_dispatched');return Object.freeze({...state,cursor:Object.freeze({...state.cursor}),profile:state.profile===null?null:Object.freeze({...state.profile})});};
const profileKey=(profile:Readonly<CapabilityProfile>|null)=>profile===null?'null':JSON.stringify(Object.entries(profile).sort(([a],[b])=>a.localeCompare(b)));

export class ContextContinuation {
 private state:ContinuationState|null=null;
 private pending:MutationOutcome|null=null;
 private abandoned:readonly MutationOutcome[]=Object.freeze([]);
 private compatible=false;
 private tail:Promise<void>=Promise.resolve();
 constructor(public context:ContextClient,private transport:ContinuityTransport,checkpoint?:ContinuationCheckpoint){
  if(checkpoint){if(!sameScope(checkpoint.scope,context.scope)||checkpoint.actor!==context.actor||checkpoint.sessionId!==context.sessionId||checkpoint.conversation!==context.conversation||checkpoint.contextOwner!==context.contextOwner)throw new ContinuationError('checkpoint_identity_mismatch','not_dispatched');this.state=copyState(checkpoint.accepted);this.pending=checkpoint.uncertain===null?null:Object.freeze({...checkpoint.uncertain});this.abandoned=Object.freeze(checkpoint.unresolved.map(item=>Object.freeze({...item})));}
 }
 get accepted():ContinuationState|null{return this.state;}
 get uncertain():MutationOutcome|null{return this.pending;}
 get unresolved():readonly MutationOutcome[]{return this.abandoned;}
 get generationCompatible():boolean{return this.compatible;}
 checkpoint():ContinuationCheckpoint{return Object.freeze({scope:Object.freeze({...this.context.scope}),actor:this.context.actor,sessionId:this.context.sessionId,conversation:this.context.conversation,contextOwner:this.context.contextOwner,accepted:copyState(this.state),uncertain:this.pending,unresolved:this.abandoned});}
 private async locked<T>(invoke:()=>Promise<T>):Promise<T>{const previous=this.tail;let release!:()=>void;this.tail=new Promise(resolve=>{release=resolve;});await previous;try{return await invoke();}finally{release();}}
 private accept(diagnostics:ContextDiagnostics,modelId:string|null,profile:Readonly<CapabilityProfile>|null):ContextDiagnostics {
  const {cursor,generation}=diagnostics.report;if(!Number.isSafeInteger(generation)||generation<0)throw new ContinuationError('invalid_generation','unknown');
  if(this.state&&(cursor.epoch<this.state.cursor.epoch||(cursor.epoch===this.state.cursor.epoch&&cursor.sequence<this.state.cursor.sequence)))throw new ContinuationError('stale_cursor','rejected');
  this.state=Object.freeze({cursor:Object.freeze({...cursor}),generation,modelId,profile:profile===null?null:Object.freeze({...profile})});this.compatible=true;return diagnostics;
 }
 private async mutation<T>(operation:string,identity:string,invoke:()=>Promise<T>,signal?:AbortSignal):Promise<T>{return this.locked(async()=>{
  if(this.pending)throw new ContinuationError('uncertain_mutation_requires_reconciliation','not_dispatched',this.pending);
  if(this.abandoned.some(item=>item.operation===operation&&item.identity===identity))throw new ContinuationError('unresolved_mutation_identity','not_dispatched');
  if(signal?.aborted)throw new ContinuationError('cancelled_before_dispatch','not_dispatched');
  let abort:()=>void=()=>{};
  try{const actual=invoke();if(!signal)return await actual;const interrupted=new Promise<never>((_,reject)=>{abort=()=>reject(new ContinuationError('cancelled_after_dispatch','unknown'));signal.addEventListener('abort',abort,{once:true});if(signal.aborted)abort();});return await Promise.race([actual,interrupted]);}
  catch(error){const effect=(error as {effectState?:string})?.effectState;const state:MutationEffect=effect==='not_dispatched'||effect==='rejected'?effect:'unknown';if(state==='unknown')this.pending=Object.freeze({operation,effectState:state,identity,detail:error instanceof Error?error.name:'unknown'});throw new ContinuationError(operation+'_failed',state,error);}
  finally{signal?.removeEventListener('abort',abort);}
 });}
 async activate(query:string,budget:TokenBudget,options:ActivationOptions={},signal?:AbortSignal):Promise<ContextDiagnostics>{
  const selected={...options,profile:options.profile?{...options.profile}:undefined},model=selected.model_id??'gpt-4o',profile=selected.profile??null;
  if(this.state&&(model!==this.state.modelId||profileKey(profile)!==profileKey(this.state.profile)))this.compatible=false;
  const generation=this.state&&this.compatible?this.state.generation:0;
  const diagnostics=await this.mutation('activate','context-generation',()=>this.context.activate(query,budget,{generation,context:selected}),signal);return this.accept(diagnostics,model,profile);
 }
 async inspect():Promise<ContextDiagnostics>{return this.locked(async()=>{const compatible=this.compatible;const diagnostics=await this.context.inspect();const result=this.accept(diagnostics,this.state?.modelId??null,this.state?.profile??null);this.compatible=compatible;return result;});}
 async reconnect(factory:()=>Promise<ContinuityBinding>):Promise<ContextDiagnostics>{return this.locked(async()=>{const fresh=await factory(),previous=this.context;
  if(!sameScope(fresh.context.scope,previous.scope)||fresh.context.actor!==previous.actor||fresh.context.sessionId!==previous.sessionId||fresh.context.conversation!==previous.conversation||fresh.context.contextOwner!==previous.contextOwner)throw new ContinuationError('reconnect_identity_mismatch','not_dispatched');
  const diagnostics=await fresh.context.inspect();this.accept(diagnostics,this.state?.modelId??null,this.state?.profile??null);this.context=fresh.context;this.transport=fresh.transport;this.compatible=false;return diagnostics;
 });}
 async close():Promise<void>{return this.locked(async()=>{await this.transport.close();this.compatible=false;});}
 async abandonUncertain():Promise<MutationOutcome>{return this.locked(async()=>{if(!this.pending)throw new Error('no uncertain mutation');const diagnostics=await this.context.inspect();this.accept(diagnostics,this.state?.modelId??null,this.state?.profile??null);const outcome=this.pending;this.abandoned=Object.freeze([...this.abandoned,outcome]);this.pending=null;this.compatible=false;return outcome;});}
 async reconcileSource(message:Readonly<SourceMessage>):Promise<boolean>{return this.locked(async()=>{if(!this.pending||this.pending.operation!=='source'||this.pending.identity!==message.id)throw new Error('uncertain source identity required');
  const envelope=await this.transport.callTool('inspect',{uri:this.context.inspectUri()+'/history'});if(!envelope.ok)throw new ContinuationError('source_inspection_failed',envelope.effect_state??'unknown',envelope);
  const history=envelope.items[0] as {messages:SourceMessage[]};const digest=freezeSourceMessage(message).source_digest;if(!history.messages.some(item=>item.id===message.id&&item.source_digest===digest))return false;
  this.accept(await this.context.inspect(),this.state?.modelId??null,this.state?.profile??null);this.pending=null;this.compatible=false;return true;
 });}
 ingestSource(message:Readonly<SourceMessage>,originalBytes:Uint8Array,signal?:AbortSignal){return this.mutation('source',message.id,()=>this.context.ingestSource(message,originalBytes),signal);}
 relate(relation:SourceRelation,signal?:AbortSignal){return this.mutation('relation',relation.id,()=>this.context.relate(relation),signal);}
 fork(childSessionId:string,childConversation=childSessionId,signal?:AbortSignal){return this.mutation('fork',childSessionId,()=>this.context.fork(childSessionId,childConversation),signal);}
 memory(requestId:string,command:MemoryCommand,signal?:AbortSignal){return this.mutation('memory',requestId,()=>this.context.memory(requestId,command),signal);}
 job(requestId:string,action:ContextJobAction,signal?:AbortSignal){return this.mutation('job',requestId,()=>this.context.job(requestId,action),signal);}
 importBundle(bundle:ImportBundle,maxEntries=128,signal?:AbortSignal){return this.mutation('import',bundle.import_id,()=>this.context.importBundle(bundle,maxEntries),signal);}
}
