#![forbid(unsafe_code)]

use serde_json::json;
use std::{fs, net::{TcpListener, TcpStream}, os::unix::fs::PermissionsExt, path::{Path, PathBuf}, process::{Child, Command, Stdio}, time::{Duration, Instant}};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
struct RunningDaemon(Child);
impl Drop for RunningDaemon { fn drop(&mut self) { let _ = self.0.kill(); let _ = self.0.wait(); } }
fn run(command: &mut Command) -> Result<()> {
    let output = command.output()?;
    if !output.status.success() { return Err(format!("command failed: {}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr)).into()); }
    print!("{}", String::from_utf8_lossy(&output.stdout));
    Ok(())
}
fn private(path: &Path, value: &[u8]) -> Result<()> { fs::write(path, value)?; fs::set_permissions(path, fs::Permissions::from_mode(0o600))?; Ok(()) }
fn port() -> Result<u16> { Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.port()) }
fn certificates(dir: &Path) -> Result<()> {
    run(Command::new("openssl").current_dir(dir).args(["req","-x509","-newkey","rsa:2048","-nodes","-keyout","ca.key","-out","ca.pem","-subj","/CN=hypermind-context-sdk","-days","1"]))?;
    for (name, purpose) in [("server","serverAuth"),("client","clientAuth")] {
        run(Command::new("openssl").current_dir(dir).args(["req","-newkey","rsa:2048","-nodes","-keyout",&format!("{name}.key"),"-out",&format!("{name}.csr"),"-subj",&format!("/CN={name}")]))?;
        fs::write(dir.join(format!("{name}.ext")),format!("basicConstraints=CA:FALSE\nextendedKeyUsage={purpose}\nsubjectAltName=DNS:localhost,IP:127.0.0.1\n"))?;
        run(Command::new("openssl").current_dir(dir).args(["x509","-req","-in",&format!("{name}.csr"),"-CA","ca.pem","-CAkey","ca.key","-CAcreateserial","-out",&format!("{name}.pem"),"-days","1","-extfile",&format!("{name}.ext")]))?;
    }
    Ok(())
}
#[test]
fn python_typescript_real_embedded_and_remote_context() -> Result<()> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize()?;
    let target = std::env::var_os("HM_SDK_NATIVE_DIR").map(PathBuf::from).unwrap_or_else(||repo.join("target/debug"));
    let binary = std::env::var_os("HM_DAEMON_BIN").map(PathBuf::from).unwrap_or_else(||target.join("hm"));
    for file in [&binary, &target.join("lib_native.so"), &target.join("libhypermind_engine_napi.so")] { if !file.is_file() { return Err(format!("required real SDK artifact missing: {}",file.display()).into()); } }
    let temp = tempfile::tempdir()?; let dir = temp.path();
    certificates(dir)?;
    let grpc = port()?; let admin = port()?; let socket = dir.join("daemon.sock");
    let config = dir.join("hypermind.conf");
    private(&config,format!("socket={}\ndata={}\nuser={}\nkek={}\nadmin_token={}\nactor=7:{}\nprojection_map_bytes=67108864\n",socket.display(),dir.join("data").display(),"11".repeat(16),"22".repeat(32),"33".repeat(32),"44".repeat(32)).as_bytes())?;
    let binding = dir.join("context.json");
    private(&binding,serde_json::to_string(&json!({"version":1,"actor":7,"scope":{"owner_id":"sdk-owner","project_id":"sdk-project","workspace_id":null}}))?.as_bytes())?;
    let log = fs::File::create(dir.join("daemon.log"))?;
    let mut daemon = RunningDaemon(Command::new(binary).args(["serve","--config"]).arg(config).arg("--context-scope").arg(binding).arg("--grpc-bind").arg(format!("127.0.0.1:{grpc}")).arg("--grpc-admin-bind").arg(format!("127.0.0.1:{admin}")).arg("--tls-cert").arg(dir.join("server.pem")).arg("--tls-key").arg(dir.join("server.key")).arg("--tls-client-ca").arg(dir.join("ca.pem")).stdin(Stdio::null()).stdout(log.try_clone()?).stderr(log).spawn()?);
    let start = Instant::now();
    while !socket.exists() || TcpStream::connect(("127.0.0.1",grpc)).is_err() {
        if daemon.0.try_wait()?.is_some() || start.elapsed()>Duration::from_secs(20) { return Err(format!("daemon startup failed: {}",fs::read_to_string(dir.join("daemon.log"))?).into()); }
        std::thread::sleep(Duration::from_millis(25));
    }
    fs::write(dir.join("python_journey.py"), PYTHON)?; fs::write(dir.join("node_journey.js"), NODE)?;
    let python = std::env::var("HM_SDK_PYTHON").unwrap_or_else(|_|"python3".into());
    let node = std::env::var("HM_SDK_NODE").unwrap_or_else(|_|"node".into());
    run(Command::new(python).arg(dir.join("python_journey.py")).arg(&repo).arg(&target).arg(dir).arg(grpc.to_string()).env("PYTHONPATH",std::env::var("HM_SDK_PYTHON_DEPS").unwrap_or_else(|_|"/tmp/hypermind-python-deps".into())))?;
    run(Command::new(node).arg(dir.join("node_journey.js")).arg(&repo).arg(&target).arg(dir).arg(&socket))?;
    Ok(())
}

const PYTHON: &str = r#"
import asyncio, importlib.util, json, pathlib, sys
repo,target,directory,port=map(str,sys.argv[1:])
sys.path.insert(0,str(pathlib.Path(repo)/'sdk/python'))
from hypermind import Engine, Client
from hypermind.context import Scope, SourceMessage, TokenBudget, make_import_bundle, make_memory_record, seal_memory_record
spec=importlib.util.spec_from_file_location('hypermind._native',str(pathlib.Path(target)/'lib_native.so'))
module=importlib.util.module_from_spec(spec);sys.modules['hypermind._native']=module;spec.loader.exec_module(module)
scope=Scope('sdk-owner','sdk-project')
async def exercise(engine,label):
    context=engine.context(scope,label+'/opaque%?',actor=7,context_owner='hypermind',conversation=label)
    def message(id,ordinal,text):return SourceMessage(id,ordinal,'user',({'kind':'text','text':text},),1791287999123456789,1791288000123456789,'user_asserted').freeze()
    first=await context.ingest_source(message('m1',0,'original evidence'),b'original evidence')
    assert first['source_span']['source_id']=='m1'
    replay=await context.ingest_source(message('m1',0,'original evidence'),b'original evidence');assert replay['replayed']
    await context.ingest_source(message('m2',1,'replacement evidence'),b'replacement evidence')
    await context.relate({'kind':'edit','id':'edit1','original_id':'m1','replacement_id':'m2'})
    assembled=await context.activate('replacement',TokenBudget(8192,512,0));assert assembled['report']['scope']['owner_id']=='sdk-owner'
    inspected=await context.inspect();assert inspected['report']['digest']==assembled['report']['digest']
    forked=await context.fork(label+'-child',label+'-child');assert forked['session_id']==label+'-child'
    bundle=make_import_bundle(scope,label+'-import',[('import-scope','scope',{'scope':{'owner_id':scope.owner_id,'project_id':scope.project_id,'workspace_id':scope.workspace_id}})])
    imported=await context.import_bundle(bundle,1);assert imported['complete'] and imported['accepted']==1
    note={'id':label+'-note','kind':'Note','revision':1,'text':'durable SDK note','parents':[],'contradictions':[],'expires_at_ns':None,'predicate':'True','tombstoned':False}
    created=await context.job(label+'-create',{'action':'notes','command':{'Create':note}});assert created['ok']
    read=await context.job(label+'-read',{'action':'read_note','id':note['id'],'now_ns':'1791288000123456789','facts':{}});assert read['ok']
    record=make_memory_record(label+'-record','note','native knowledge evidence','1791288000123456789')
    made=await context.memory(label+'-memory-create',{'kind':'create','record':record});assert made['cursor']>0
    memory=await context.inspect_memory(record['id']);assert memory['record']['revision_digest']==record['revision_digest']
    revised=seal_memory_record({**record,'revision':2,'content':'revised native knowledge'})
    await context.memory(label+'-memory-revise',{'kind':'revise','record':revised,'expected_revision':1})
    memory=await context.inspect_memory(record['id']);assert memory['record']['content']=='revised native knowledge'
    await context.memory(label+'-memory-tombstone',{'kind':'tombstone','id':record['id'],'expected_revision':2})
    visible=await context.inspect_memory();assert not any(row['id']==record['id'] for row in visible['records'])
    print(label+' source/replay/relation/activate/inspect/fork/import/job/memory passed')
    return context.session_id,assembled['report']['digest']
async def main():
    data=pathlib.Path(directory)/'python-native'
    engine=await Engine.open(data,actor=7,user_hex='11'*16,kek_hex='22'*32,projection_map_bytes=67108864,context_scope=scope)
    session,digest=await exercise(engine,'python-native');await engine.close()
    engine=await Engine.open(data,actor=7,user_hex='11'*16,kek_hex='22'*32,projection_map_bytes=67108864,context_scope=scope)
    restored=await engine.context(scope,session,actor=7,context_owner='hypermind',conversation='python-native').inspect();assert restored['report']['digest']==digest;await engine.close()
    base=pathlib.Path(directory)
    client=Client('localhost:'+port,token=bytes.fromhex('44'*32),ca=(base/'ca.pem').read_bytes(),certificate=(base/'client.pem').read_bytes(),private_key=(base/'client.key').read_bytes())
    await exercise(client,'python-remote');await client.close()
    print('python native reopen and mTLS remote passed')
asyncio.run(main())
"#;

const NODE: &str = r#"
const assert=require('node:assert/strict'),path=require('node:path'),fs=require('node:fs');
const [repo,target,directory,socket]=process.argv.slice(2);
const clientPath=path.join(repo,'sdk/typescript/packages/client/dist/index.js');
const {Client,makeImportBundle,freezeSourceMessage,makeMemoryRecord,sealMemoryRecord}=require(clientPath);
const enginePath=path.join(repo,'sdk/typescript/packages/engine');
const nativePackage=path.join(directory,'node_modules/@hypermind/engine-linux-x64-gnu');fs.mkdirSync(nativePackage,{recursive:true});
fs.copyFileSync(path.join(target,'libhypermind_engine_napi.so'),path.join(nativePackage,'engine.node'));
fs.writeFileSync(path.join(nativePackage,'index.js'),"module.exports=require('./engine.node');");
process.env.NODE_PATH=path.join(directory,'node_modules');require('node:module').Module._initPaths();
const {HyperMind}=require(path.join(enginePath,'dist/index.js'));
const scope={owner_id:'sdk-owner',project_id:'sdk-project',workspace_id:null};
async function exercise(engine,label){
 const context=engine.context(scope,label+'/opaque%?',7,'hypermind',label);
 const message=(id,ordinal,text)=>freezeSourceMessage({id,ordinal,role:'user',parts:[{kind:'text',text}],occurred_at_ns:1791287999123456789n,recorded_at_ns:1791288000123456789n,authority:'user_asserted',source_digest:''});
 const first=await context.ingestSource(message('m1',0,'original evidence'),Buffer.from('original evidence'));assert.equal(first.source_span.source_id,'m1');
 const replay=await context.ingestSource(message('m1',0,'original evidence'),Buffer.from('original evidence'));assert.equal(replay.replayed,true);
 await context.ingestSource(message('m2',1,'replacement evidence'),Buffer.from('replacement evidence'));
 await context.relate({kind:'edit',id:'edit1',original_id:'m1',replacement_id:'m2'});
 const assembled=await context.activate('replacement',{context_tokens:8192,reserved_output_tokens:512,required_tokens:0});
 const inspected=await context.inspect();assert.equal(inspected.report.digest,assembled.report.digest);
 const forked=await context.fork(label+'-child');assert.equal(forked.session_id,label+'-child');
 const imported=await context.importBundle(makeImportBundle(scope,label+'-import',[{source_id:'import-scope',kind:'scope',payload:{scope}}]),1);assert.equal(imported.complete,true);
 const note={id:label+'-note',kind:'Note',revision:1,text:'durable SDK note',parents:[],contradictions:[],expires_at_ns:null,predicate:'True',tombstoned:false};
 assert.equal((await context.job(label+'-create',{action:'notes',command:{Create:note}})).ok,true);
 assert.equal((await context.job(label+'-read',{action:'read_note',id:note.id,now_ns:1791288000123456789n,facts:{}})).ok,true);
 const record=makeMemoryRecord(label+'-record','note','native knowledge evidence',1791288000123456789n);
 assert.ok((await context.memory(label+'-memory-create',{kind:'create',record})).cursor>0);
 assert.equal((await context.inspectMemory(record.id)).record.revision_digest,record.revision_digest);
 const revised=sealMemoryRecord({...record,revision:2,content:'revised native knowledge'});
 await context.memory(label+'-memory-revise',{kind:'revise',record:revised,expected_revision:1});
 assert.equal((await context.inspectMemory(record.id)).record.content,'revised native knowledge');
 await context.memory(label+'-memory-tombstone',{kind:'tombstone',id:record.id,expected_revision:2});
 assert.equal((await context.inspectMemory()).records.some(row=>row.id===record.id),false);
 console.log(label+' source/replay/relation/activate/inspect/fork/import/job/memory passed');return [context.sessionId,assembled.report.digest];
}
(async()=>{
 const config={actor:7,userHex:'11'.repeat(16),kekHex:'22'.repeat(32),projectionMapBytes:67108864,contextScope:scope};
 let engine=await HyperMind.open(path.join(directory,'node-native'),config);
 const [session,digest]=await exercise(engine,'node-native');await engine.close();
 engine=await HyperMind.open(path.join(directory,'node-native'),config);
 assert.equal((await engine.context(scope,session,7,'hypermind','node-native').inspect()).report.digest,digest);await engine.close();
 const client=await Client.connect({socketPath:socket,capabilityToken:Buffer.from('44'.repeat(32),'hex')});
 await exercise(client,'node-remote');await client.close();
 console.log('typescript native reopen and UDS remote passed');
})().catch(error=>{console.error(error);process.exitCode=1;});
"#;
