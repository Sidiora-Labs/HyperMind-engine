#![forbid(unsafe_code)]
use std::{fs,path::{Path,PathBuf},process::Command};
#[test]
fn python_typescript_lossless_memory_portability() -> Result<(),Box<dyn std::error::Error>> {
 let repo=Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize()?;
 let target=std::env::var_os("HM_SDK_NATIVE_DIR").map(PathBuf::from).unwrap_or_else(||repo.join("target/debug"));
 let temp=tempfile::tempdir()?;
 for (name,program,script) in [("python.py",std::env::var("HM_SDK_PYTHON").unwrap_or("python3".into()),PYTHON),("node.js",std::env::var("HM_SDK_NODE").unwrap_or("node".into()),NODE)] {
  let path=temp.path().join(name);fs::write(&path,script)?;
  let output=Command::new(program).arg(path).arg(&repo).arg(&target).arg(temp.path()).env("PYTHONPATH",std::env::var("HM_SDK_PYTHON_DEPS").unwrap_or_default()).output()?;
  if !output.status.success(){return Err(format!("{}: {}\n{}",name,String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr)).into());}print!("{}",String::from_utf8_lossy(&output.stdout));
 } Ok(())
}
const PYTHON:&str=r#"
import asyncio,importlib.util,pathlib,sys,hashlib
repo,target,directory=map(pathlib.Path,sys.argv[1:]);sys.path.insert(0,str(repo/'sdk/python'))
from hypermind import Engine
from hypermind.context import Scope,make_memory_record,seal_memory_record,ContextClientError
spec=importlib.util.spec_from_file_location('hypermind._native',str(target/'lib_native.so'));module=importlib.util.module_from_spec(spec);sys.modules['hypermind._native']=module;spec.loader.exec_module(module)
scope=Scope('portability-owner','portability-project')
async def open(name,scope=scope):return await Engine.open(directory/name,actor=7,user_hex='11'*16,kek_hex='22'*32,projection_map_bytes=67108864,context_scope=scope)
async def main():
 source=await open('py-source');context=source.context(scope,'session',actor=7,context_owner='hypermind')
 raw=b'original source';digest=hashlib.sha256(raw).hexdigest();await context.memory('source',{'kind':'source','source':{'id':'source','digest':digest,'content':list(raw),'locator':'document://source','occurred_at_ns':None,'recorded_at_ns':'1791288000123456789','tombstoned':False}})
 record=make_memory_record('record','note','portable knowledge','1791288000123456789');record['provenance']=[{'source_id':'source','source_digest':digest,'span_start':0,'span_end':len(raw),'quoted_digest':digest}];record=seal_memory_record(record);await context.memory('create',{'kind':'create','record':record})
 artifact=await context.export_memory();assert artifact.cursor==2 and artifact.events[1]['command']['record']['recorded_at_ns']=='1791288000123456789'
 destination=await open('py-destination');restored=destination.context(scope,'session',actor=7,context_owner='hypermind');await restored.restore_memory('restore',artifact)
 assert (await restored.inspect_memory('record'))['record']['content']=='portable knowledge'
 assert bytes((await restored.inspect_memory('record',source_id='source'))['source']['content'])==raw
 for request,raw,digest in [('corrupt',list(artifact.data),'0'*64),('foreign',list(artifact.data),artifact.artifact_digest)]:
  engine=destination if request=='corrupt' else await open('py-foreign',Scope('foreign-owner','foreign-project'))
  consumer=restored if request=='corrupt' else engine.context(Scope('foreign-owner','foreign-project'),'session',actor=7,context_owner='hypermind')
  before=(await consumer.inspect_memory())['cursor']
  try:await consumer.memory(request,{'kind':'restore_jsonl','jsonl':raw,'artifact_digest':digest})
  except ContextClientError:pass
  else:raise AssertionError('invalid restore accepted')
  assert (await consumer.inspect_memory())['cursor']==before
  if engine is not destination:await engine.close()
 await destination.close();destination=await open('py-destination');assert (await destination.context(scope,'session',actor=7,context_owner='hypermind').inspect_memory('record'))['record']['recorded_at_ns']=='1791288000123456789';await destination.close();await source.close()
 print('Python actual export/restore/reopen/tamper/foreign-scope passed')
asyncio.run(main())
"#;
const NODE:&str=r#"
const assert=require('node:assert/strict'),path=require('node:path'),fs=require('node:fs'),crypto=require('node:crypto');
const [repo,target,directory]=process.argv.slice(2),pkg=path.join(directory,'node_modules/@hypermind/engine-linux-x64-gnu');fs.mkdirSync(pkg,{recursive:true});fs.copyFileSync(path.join(target,'libhypermind_engine_napi.so'),path.join(pkg,'engine.node'));fs.writeFileSync(path.join(pkg,'index.js'),"module.exports=require('./engine.node');");process.env.NODE_PATH=path.join(directory,'node_modules');require('node:module').Module._initPaths();
const {HyperMind}=require(path.join(repo,'sdk/typescript/packages/engine/dist/index.js'));const {makeMemoryRecord,sealMemoryRecord,ContextClientError}=require(path.join(repo,'sdk/typescript/packages/client/dist/index.js'));
const scope={owner_id:'portability-owner',project_id:'portability-project',workspace_id:null};
const open=(name,contextScope=scope)=>HyperMind.open(path.join(directory,name),{actor:7,userHex:'11'.repeat(16),kekHex:'22'.repeat(32),projectionMapBytes:67108864,contextScope});
async function main(){const source=await open('ts-source'),context=source.context(scope,'session',7,'hypermind');const raw=Buffer.from('original source'),digest=crypto.createHash('sha256').update(raw).digest('hex');await context.memory('source',{kind:'source',source:{id:'source',digest,content:Array.from(raw),locator:'document://source',occurred_at_ns:null,recorded_at_ns:1791288000123456789n,tombstoned:false}});const record=sealMemoryRecord({...makeMemoryRecord('record','note','portable knowledge',1791288000123456789n),provenance:[{source_id:'source',source_digest:digest,span_start:0,span_end:raw.length,quoted_digest:digest}]});await context.memory('create',{kind:'create',record});const artifact=await context.exportMemory();assert.equal(artifact.cursor,2);assert.equal(artifact.events[1].command.record.recorded_at_ns,'1791288000123456789');let destination=await open('ts-destination'),restored=destination.context(scope,'session',7,'hypermind');await restored.restoreMemory('restore',artifact);assert.equal((await restored.inspectMemory('record')).record.content,'portable knowledge');assert.deepEqual(Buffer.from((await restored.inspectMemory('record',undefined,'source')).source.content),raw);
for(const foreign of [false,true]){const foreignScope={owner_id:'foreign-owner',project_id:'foreign-project',workspace_id:null},engine=foreign?await open('ts-foreign',foreignScope):destination,consumer=foreign?engine.context(foreignScope,'session',7,'hypermind'):restored;const before=(await consumer.inspectMemory()).cursor;await assert.rejects(()=>consumer.memory(foreign?'foreign':'corrupt',{kind:'restore_jsonl',jsonl:Array.from(artifact.data),artifact_digest:foreign?artifact.artifact_digest:'0'.repeat(64)}),ContextClientError);assert.equal((await consumer.inspectMemory()).cursor,before);if(foreign)await engine.close();}
await destination.close();destination=await open('ts-destination');assert.equal((await destination.context(scope,'session',7,'hypermind').inspectMemory('record')).record.recorded_at_ns,'1791288000123456789');await destination.close();await source.close();console.log('TypeScript actual export/restore/reopen/tamper/foreign-scope passed');}
main().catch(error=>{console.error(error);process.exitCode=1});
"#;
