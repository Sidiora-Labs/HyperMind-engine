import assert from 'node:assert/strict';
import test from 'node:test';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {ContextContinuation,ContinuationError} from './continuity';
import {freezeSourceMessage} from './context';
import type {ContextClient,ContextToolClient,Scope} from './context';

test('real NAPI restart, source fork, model generation and cancelled uncertain mutation',async()=>{
 const repo=path.resolve(__dirname,'../../../../..'),native=process.env.HM_SDK_NATIVE_DIR??path.join(repo,'target/debug'),directory=fs.mkdtempSync(path.join(os.tmpdir(),'hypermind-continuity-'));
 const pkg=path.join(directory,'node_modules/@hypermind/engine-linux-x64-gnu');fs.mkdirSync(pkg,{recursive:true});fs.copyFileSync(path.join(native,'libhypermind_engine_napi.so'),path.join(pkg,'engine.node'));fs.writeFileSync(path.join(pkg,'index.js'),"module.exports=require('./engine.node');");
 const oldNodePath=process.env.NODE_PATH;process.env.NODE_PATH=path.join(directory,'node_modules');(require('node:module') as {_initPaths():void})._initPaths();
 const {HyperMind}=require(path.join(repo,'sdk/typescript/packages/engine/dist/index.js')) as {HyperMind:{open(path:string,config:unknown):Promise<ContextToolClient&{close():Promise<void>;context(scope:Scope,session:string,actor:number,owner:string,conversation?:string):ContextClient}>}};
 const scope:Scope={owner_id:'continuity-owner',project_id:'continuity-project',workspace_id:null};
 const open=()=>HyperMind.open(path.join(directory,'store'),{actor:7,userHex:'11'.repeat(16),kekHex:'22'.repeat(32),projectionMapBytes:67108864,contextScope:scope});
 let engine=await open();let continuation=new ContextContinuation(engine.context(scope,'parent/%?',7,'hypermind','parent'),engine);
 try{
 const source=freezeSourceMessage({id:'original',ordinal:0,role:'user',parts:[{kind:'text',text:'retained original'}],occurred_at_ns:1791287999123456789n,recorded_at_ns:1791288000123456789n,authority:'user_asserted',source_digest:''});
 await continuation.ingestSource(source,Buffer.from('retained original'));
 const budget={context_tokens:8192,reserved_output_tokens:512,required_tokens:0};await continuation.activate('retained',budget,{model_id:'gpt-4o'});const accepted=continuation.accepted!;
 await continuation.fork('child/%?','child');await continuation.close();engine=await open();await continuation.reconnect(async()=>({context:engine.context(scope,'parent/%?',7,'hypermind','parent'),transport:engine}));
 assert.deepEqual(continuation.accepted!.cursor,accepted.cursor);assert.equal(continuation.accepted!.generation,accepted.generation);assert.equal(continuation.generationCompatible,false);
 const child=engine.context(scope,'child/%?',7,'hypermind','child');assert.ok((await child.activate('retained',budget)).report.included.includes('original'));
 const history=await engine.callTool('inspect',{uri:child.inspectUri()+'/history'});const message=(history.items[0] as {messages:{source_digest:string;recorded_at_ns:string}[]}).messages[0]!;assert.equal(message.source_digest,source.source_digest);assert.equal(message.recorded_at_ns,'1791288000123456789');
 await continuation.activate('retained',budget,{model_id:'gpt-4o-mini'});assert.equal(continuation.accepted!.modelId,'gpt-4o-mini');assert.ok(continuation.accepted!.generation>accepted.generation);
 const cancelled=freezeSourceMessage({...source,id:'cancelled',ordinal:1,recorded_at_ns:1791288000123456790n,source_digest:''});const controller=new AbortController();const mutation=continuation.ingestSource(cancelled,Buffer.from('retained original'),controller.signal);queueMicrotask(()=>controller.abort());
 await assert.rejects(mutation,(error:unknown)=>error instanceof ContinuationError&&error.effectState==='unknown');assert.equal(continuation.uncertain!.effectState,'unknown');
 await assert.rejects(()=>continuation.ingestSource(cancelled,Buffer.from('retained original')),(error:unknown)=>error instanceof ContinuationError&&error.effectState==='not_dispatched');
 await continuation.inspect();assert.ok(continuation.uncertain);await continuation.abandonUncertain();const checkpoint=continuation.checkpoint();assert.ok(Object.isFrozen(checkpoint));assert.ok(Object.isFrozen(checkpoint.accepted!.cursor));
 continuation=new ContextContinuation(continuation.context,engine,checkpoint);assert.equal(continuation.unresolved.length,1);await assert.rejects(()=>continuation.ingestSource(cancelled,Buffer.from('retained original')),ContinuationError);
 }finally{await engine.close();if(oldNodePath===undefined)delete process.env.NODE_PATH;else process.env.NODE_PATH=oldNodePath;(require('node:module') as {_initPaths():void})._initPaths();fs.rmSync(directory,{recursive:true,force:true});}
});
