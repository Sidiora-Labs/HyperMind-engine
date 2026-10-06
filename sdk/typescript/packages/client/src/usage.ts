import type { Scope } from "./context";
import type { ToolEnvelope } from "./anticipation";

export interface UsageAttribution { job_id:string;worker_id:string;session_id:string;turn_id:string;provider_id:string;model_id:string;source_ids:string[] }
export interface ExactUsagePrice { nanodollar_numerator:bigint;denominator:bigint;unit:string }
export interface UsageMoney { negative:boolean;magnitude:ExactUsagePrice }
export interface UsageTokens { input?:bigint|null;output?:bigint|null;cache_read?:bigint|null;cache_write?:bigint|null;reasoning?:bigint|null;input_semantics:"includes_cache"|"excludes_cache"|"unknown" }
export type UsageQuota = {kind:"count";value:bigint}|{kind:"usd";value:UsageMoney};
export interface UsageWindow {name:string;unit:string;limit:UsageQuota|null;remaining:UsageQuota|null;used:UsageQuota|null;refill_amount:UsageQuota|null;starts_at_ns:bigint|null;resets_at_ns:bigint|null;interval_ms:bigint|null}
export interface UsageProviderSnapshot {provider_id:string;format:"open_ai"|"anthropic"|"ollama"|"account_quota";observed_at_ns:bigint;expires_at_ns:bigint|null;tokens:UsageTokens;quota_windows:UsageWindow[];balance:UsageMoney|null;reported_charge:UsageMoney|null;funding:{credits:UsageMoney|null;spent:UsageMoney|null}|null;error_observed:boolean;evidence_digest:string}
export interface UsageCatalogCharge {basis:string;model_id:string;tier_index:number|null;nanodollars:ExactUsagePrice|null;unknown:string[]}
export interface UsageReservation {id:string;attribution:UsageAttribution;reserved_tokens:bigint;reserved_at_ms:bigint;attempt:bigint;expires_ms:bigint;dispatched:boolean;observation_id:string|null;settled_tokens:bigint|null}
export interface UsageObservation {id:string;reservation_id:string;attribution:UsageAttribution;accepted_response:boolean;evidence_digest:string;provider_usage:UsageProviderSnapshot;catalog_estimate:UsageCatalogCharge|null}
export interface UsageRollup {known_tokens:bigint;held_tokens:bigint;unknown_reservations:bigint;active_reservations:bigint;observed_calls:bigint;attributions:UsageAttribution[];observations:string[]}
export interface UsageState {limits:{total_tokens:bigint;hourly_tokens:bigint;daily_tokens:bigint;job_tokens:bigint;concurrency:bigint;lease_ms:bigint};spent:bigint;unknown_usage:Record<string,bigint>;reservations:UsageReservation[];observations:UsageObservation[]}
export interface UsageView {version:1;scope:Scope;rollup:UsageRollup;state:UsageState}
export class UsageDecodeError extends Error {}
export class UsageInspectionError extends Error {constructor(readonly code:string){super(code);}}

type Schema = string | readonly [string,Schema|readonly string[]] | {[key:string]:Schema} | ((value:unknown)=>unknown);
function exact(value:unknown,signed=false):bigint {
  let result:bigint;
  if(typeof value==="number" && Number.isSafeInteger(value)) result=BigInt(value);
  else if(typeof value==="string" && value.length<=20 && /^(0|[1-9][0-9]*|-[1-9][0-9]*)$/.test(value)) result=BigInt(value);
  else throw new UsageDecodeError("noncanonical or unsafe usage quantity");
  if(typeof value === "string" && result.toString() !== value) throw new UsageDecodeError("noncanonical usage quantity");
  if(result<(signed?-(1n<<63n):0n)||result>(signed?(1n<<63n)-1n:(1n<<64n)-1n)) throw new UsageDecodeError("usage quantity outside native range");
  return result;
}
function object(value:unknown):Record<string,unknown> {if(!value||typeof value!=="object"||Array.isArray(value))throw new UsageDecodeError("usage object");return value as Record<string,unknown>;}
function decode(value:unknown,schema:Schema):unknown {
  if(schema==="u64"||schema==="i64")return exact(value,schema==="i64");
  if(schema==="str"){if(typeof value!=="string")throw new UsageDecodeError("usage text");return value;}
  if(schema==="bool"){if(typeof value!=="boolean")throw new UsageDecodeError("usage boolean");return value;}
  if(schema==="index"){if(typeof value!=="number"||!Number.isSafeInteger(value)||value<0)throw new UsageDecodeError("usage index");return value;}
  if(typeof schema==="function")return schema(value);
  if(Array.isArray(schema)) {
    const [kind,inner]=schema;
    if(kind==="null")return value===null?null:decode(value,inner as Schema);
    if(kind==="list"){if(!Array.isArray(value))throw new UsageDecodeError("usage list");return value.map(v=>decode(v,inner as Schema));}
    if(kind==="map")return Object.fromEntries(Object.entries(object(value)).map(([k,v])=>[k,decode(v,inner as Schema)]));
    if(kind==="enum"){if(typeof value!=="string"||!(inner as readonly string[]).includes(value))throw new UsageDecodeError("usage enum");return value;}
  }
  if(typeof schema==="object") {
    const fields=schema as Record<string,Schema>, input=object(value), output:Record<string,unknown>={};
    if(Object.keys(input).some(k=>!Object.hasOwn(fields,k)))throw new UsageDecodeError("unknown usage field");
    for(const [key,field] of Object.entries(fields)) {
      const optional=Array.isArray(field)&&field[0]==="optional";
      if(!Object.hasOwn(input,key)){if(optional)continue;throw new UsageDecodeError("missing usage field "+key);}
      output[key]=decode(input[key],optional?(field as readonly [string,Schema])[1]:field);
    }
    return output;
  }
  throw new UsageDecodeError("usage schema");
}
const strings=(keys:string[])=>Object.fromEntries(keys.map(k=>[k,"str"])) as Record<string,Schema>;
const counts=(keys:string[])=>Object.fromEntries(keys.map(k=>[k,"u64"])) as Record<string,Schema>;
const attribution:Schema={...strings(["job_id","worker_id","session_id","turn_id","provider_id","model_id"]),source_ids:["list","str"]};
const price:Schema={nanodollar_numerator:"u64",denominator:"u64",unit:"str"};
const money:Schema={negative:"bool",magnitude:price};
const quota:Schema=(value:unknown)=>{const v=object(value);if(v.kind!=="count"&&v.kind!=="usd")throw new UsageDecodeError("quota amount");return decode(value,{kind:["enum",["count","usd"]],value:v.kind==="count"?"u64":money});};
const tokens:Schema={...Object.fromEntries(["input","output","cache_read","cache_write","reasoning"].map(k=>[k,["optional",["null","u64"]]])),input_semantics:["enum",["includes_cache","excludes_cache","unknown"]]};
const window:Schema={name:"str",unit:"str",...Object.fromEntries(["limit","remaining","used","refill_amount"].map(k=>[k,["null",quota]])),starts_at_ns:["null","i64"],resets_at_ns:["null","i64"],interval_ms:["null","u64"]};
const snapshot:Schema={provider_id:"str",format:["enum",["open_ai","anthropic","ollama","account_quota"]],observed_at_ns:"i64",expires_at_ns:["null","i64"],tokens,quota_windows:["list",window],balance:["null",money],reported_charge:["null",money],funding:["null",{credits:["null",money],spent:["null",money]}],error_observed:"bool",evidence_digest:"str"};
const catalog:Schema={basis:"str",model_id:"str",tier_index:["null","index"],nanodollars:["null",price],unknown:["list","str"]};
const reservation:Schema={id:"str",attribution,...counts(["reserved_tokens","reserved_at_ms","attempt","expires_ms"]),dispatched:"bool",observation_id:["null","str"],settled_tokens:["null","u64"]};
const observation:Schema={id:"str",reservation_id:"str",attribution,accepted_response:"bool",evidence_digest:"str",provider_usage:snapshot,catalog_estimate:["null",catalog]};
const view:Schema={version:"index",scope:{owner_id:"str",project_id:"str",workspace_id:["null","str"]},rollup:{...counts(["known_tokens","held_tokens","unknown_reservations","active_reservations","observed_calls"]),attributions:["list",attribution],observations:["list","str"]},state:{limits:counts(["total_tokens","hourly_tokens","daily_tokens","job_tokens","concurrency","lease_ms"]),spent:"u64",unknown_usage:["map","u64"],reservations:["list",reservation],observations:["list",observation]}};
function identifier(value:string):void {if(!value||new TextEncoder().encode(value).length>256||!/^[!-~]+$/.test(value))throw new UsageDecodeError("usage identity");}
export function decodeUsageView(value:unknown,scope?:Scope):UsageView {
  const result=decode(value,view) as UsageView;
  if(result.version!==1)throw new UsageDecodeError("unsupported usage version");
  identifier(result.scope.owner_id);identifier(result.scope.project_id);if(result.scope.workspace_id!==null)identifier(result.scope.workspace_id);
  if(scope && (result.scope.owner_id!==scope.owner_id||result.scope.project_id!==scope.project_id||result.scope.workspace_id!==scope.workspace_id))throw new UsageDecodeError("usage scope mismatch");
  for(const a of [...result.rollup.attributions,...result.state.reservations.map(r=>r.attribution),...result.state.observations.map(r=>r.attribution)])for(const [key,value] of Object.entries(a))for(const item of key==="source_ids"?value as string[]:[value as string])identifier(item);
  return result;
}
export function decodeProviderUsage(value:unknown):UsageProviderSnapshot {return decode(value,snapshot) as UsageProviderSnapshot;}
export interface UsageToolClient {callTool(verb:"inspect",input:{uri:string}):Promise<ToolEnvelope>}
export class UsageClient {
  readonly scope:Readonly<Scope>;
  constructor(private readonly client:UsageToolClient,scope:Scope,readonly actor:number){if(!Number.isInteger(actor)||actor<1||actor>65535)throw new UsageDecodeError("usage actor");this.scope=Object.freeze({...scope});}
  async inspect():Promise<UsageView>{
    const envelope=await this.client.callTool("inspect",{uri:`hm://${this.actor}/context-usage`});
    if(envelope.ok!==true){const item=envelope.items[0] as {error?:string}|undefined;throw new UsageInspectionError(item?.error??"kOperationUnavailable");}
    return decodeUsageView(envelope.items[0],this.scope);
  }
}
