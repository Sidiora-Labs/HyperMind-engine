import assert from "node:assert/strict";
import test from "node:test";
import { availableContextTokens, freezeSourceMessage, ContextClient, ContextClientError } from "./context";

test("versioned scope and budgets serialize using the Rust contract fields",()=>{
  const client=new ContextClient(undefined as never,{owner_id:"owner",project_id:"project",workspace_id:null},"session",65535,"host");
  const budget={context_tokens:100,reserved_output_tokens:20,required_tokens:5};
  assert.equal(availableContextTokens(budget),75);
  assert.deepEqual(JSON.parse(JSON.stringify(client.requestContext(budget))),{version:1,scope:{owner_id:"owner",project_id:"project",workspace_id:null},session_id:"session",budget,generation:0});
  assert.equal(client.version,undefined);
  assert.throws(()=>availableContextTokens({context_tokens:10,reserved_output_tokens:11,required_tokens:0}));
  assert.throws(()=>new ContextClient(undefined as never,{owner_id:"owner space",project_id:"project",workspace_id:null},"session",0,"host"));
  assert.throws(()=>client.accept({ok:false,items:[],provenance:[],gaps:[],health:{},warnings:[],effect_state:"unknown"}),ContextClientError);
});

test("source identity is canonical and immutable",()=>{
  const part={kind:"text" as const,text:"hello λ"};
  const source=freezeSourceMessage({id:"m1",ordinal:0,role:"user",parts:[part],occurred_at_ns:null,recorded_at_ns:0,authority:"user_asserted",source_digest:""});
  assert.equal(source.source_digest,"92d70f349962401c258abcb874228c42b5caf13d0b59dcfad98912bbb4a70b6a");
  part.text="changed";
  assert.equal((source.parts[0] as {text:string}).text,"hello λ");
  assert.throws(()=>{(source.parts[0] as {text:string}).text="changed";});
  assert.throws(()=>freezeSourceMessage({...source,id:"different"}));
});


test("current-scale timestamps preserve identical wire identity",()=>{
  const source=freezeSourceMessage({id:"m1",ordinal:0,role:"user",parts:[{kind:"text",text:"hello λ"}],occurred_at_ns:1791287999123456789n,recorded_at_ns:1791288000123456789n,authority:"user_asserted",source_digest:""});
  assert.equal(source.source_digest,"78af0610090d418ca2335976c873cfcc7647f9ce431f98696040d8fa1660500c");
  const decoded=JSON.parse(JSON.stringify(source));
  assert.equal(decoded.recorded_at_ns,"1791288000123456789");
  assert.deepEqual(freezeSourceMessage(decoded),source);
  for(const bad of [1791288000123456789,"01","-0","9223372036854775808"]){assert.throws(()=>freezeSourceMessage({...source,recorded_at_ns:bad}));}
});


test("nested context operations preserve scope, source identity and opaque session URI",()=>{
  const client=new ContextClient(undefined as never,{owner_id:"owner",project_id:"project",workspace_id:null},"s/a%?!'()*",7,"hypermind","conversation");
  assert.equal(client.inspectUri(),"hm://7/context/s%2Fa%25%3F%21%27%28%29%2A");
  assert.equal(decodeURIComponent(client.inspectUri().split("/").at(-1)!),client.sessionId);
  const message=freezeSourceMessage({id:"m1",ordinal:0,role:"user",parts:[{kind:"text",text:"hello λ"}],occurred_at_ns:1791287999123456789n,recorded_at_ns:1791288000123456789n,authority:"user_asserted",source_digest:""});
  const source=JSON.parse(JSON.stringify(client.sourceRequest(message,new TextEncoder().encode("hello"))));
  assert.equal(source.conversation,"conversation");assert.equal(source.context.operation,"source");
  assert.equal(source.context.request.message.source_digest,"78af0610090d418ca2335976c873cfcc7647f9ce431f98696040d8fa1660500c");
  assert.deepEqual(source.context.request.original_bytes,[104,101,108,108,111]);
  const relation={kind:"tombstone" as const,id:"r1",source_id:"m1"};
  assert.deepEqual(client.relationRequest(relation).context.request.relation,relation);
  const fork=client.forkRequest("child/session","child-conversation");
  assert.equal(fork.conversation,"child-conversation");assert.equal(fork.context.request.parent_conversation,"conversation");
  const job=client.jobRequest("j1",{action:"maintenance_enqueue",session_id:client.sessionId,request:{kind:"verification",sources:[message],cursor:{epoch:1,sequence:1},source_revision:1,policy_revision:1,reservation:5}});
  assert.equal(job.context.operation,"job");assert.doesNotThrow(()=>JSON.stringify(job));
});
