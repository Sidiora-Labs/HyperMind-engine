#![forbid(unsafe_code)]

use hm_context::{Authority, MessagePart, MessageRole, Scope, SourceMessage};
use hm_core::ActorId;
use hm_serve::{
    context_config::TrustedContextConfig,
    embedded::{EmbeddedConfig, HyperMind},
    session_context::SessionContextRequest,
};
use serde_json::{json, Value};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    net::{TcpListener, TcpStream},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
struct Daemon(Child);
impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn scope() -> Scope {
    Scope {
        owner_id: "sdk-owner".into(),
        project_id: "sdk-project".into(),
        workspace_id: None,
    }
}
fn private(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
fn run(command: &mut Command) -> Result<()> {
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "consumer command failed {}: {}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    print!("{}", String::from_utf8_lossy(&output.stdout));
    Ok(())
}
fn port() -> Result<u16> {
    Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
}
fn certificates(root: &Path) -> Result<()> {
    run(Command::new("openssl").current_dir(root).args([
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        "ca.key",
        "-out",
        "ca.pem",
        "-subj",
        "/CN=hypermind-sdk-journey",
        "-days",
        "1",
    ]))?;
    for (name, purpose) in [("server", "serverAuth"), ("client", "clientAuth")] {
        run(Command::new("openssl").current_dir(root).args([
            "req",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-keyout",
            &format!("{name}.key"),
            "-out",
            &format!("{name}.csr"),
            "-subj",
            &format!("/CN={name}"),
        ]))?;
        private(&root.join(format!("{name}.ext")),format!("basicConstraints=CA:FALSE\nextendedKeyUsage={purpose}\nsubjectAltName=DNS:localhost,IP:127.0.0.1\n").as_bytes())?;
        run(Command::new("openssl").current_dir(root).args([
            "x509",
            "-req",
            "-in",
            &format!("{name}.csr"),
            "-CA",
            "ca.pem",
            "-CAkey",
            "ca.key",
            "-CAcreateserial",
            "-out",
            &format!("{name}.pem"),
            "-days",
            "1",
            "-extfile",
            &format!("{name}.ext"),
        ]))?;
    }
    Ok(())
}
fn start(binary: &Path, root: &Path, grpc: u16) -> Result<Daemon> {
    let log = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .mode(0o600)
        .open(root.join("daemon.log"))?;
    let mut daemon = Daemon(
        Command::new(binary)
            .args(["serve", "--config"])
            .arg(root.join("hm.conf"))
            .arg("--context-scope")
            .arg(root.join("scope.json"))
            .arg("--grpc-bind")
            .arg(format!("127.0.0.1:{grpc}"))
            .arg("--tls-cert")
            .arg(root.join("server.pem"))
            .arg("--tls-key")
            .arg(root.join("server.key"))
            .arg("--tls-client-ca")
            .arg(root.join("ca.pem"))
            .stdin(Stdio::null())
            .stdout(log.try_clone()?)
            .stderr(log)
            .spawn()?,
    );
    let started = Instant::now();
    while !root.join("hm.sock").exists() || TcpStream::connect(("127.0.0.1", grpc)).is_err() {
        if daemon.0.try_wait()?.is_some() || started.elapsed() > Duration::from_secs(20) {
            return Err(format!(
                "actual daemon startup failed: {}",
                fs::read_to_string(root.join("daemon.log"))?
            )
            .into());
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    Ok(daemon)
}
fn fixture() -> Result<Value> {
    let mut source = SourceMessage {
        id: "m1".into(),
        ordinal: 0,
        role: MessageRole::User,
        parts: vec![MessagePart::Text {
            text: "hello λ".into(),
        }],
        occurred_at_ns: Some(1_791_287_999_123_456_789),
        recorded_at_ns: 1_791_288_000_123_456_789,
        authority: Authority::UserAsserted,
        source_digest: String::new(),
    };
    source.source_digest = source.computed_digest()?;
    Ok(
        json!({"version":1,"scope":scope(),"source":source,"original_bytes":[255,0,13,10,32,206,187]}),
    )
}
fn rust_consumer(root: &Path, fixture: &Value, phase: &str) -> Result<()> {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let config=EmbeddedConfig {actor:ActorId::new(7),user:[0x11;16],kek:[0x22;32],projection_map_bytes:64*1024*1024};
        let engine=HyperMind::open_with_context(root.join("rust-embedded"),config,TrustedContextConfig {version:1,actor:7,scope:scope()}).await?;
        let actor=engine.actor();let context=actor.context_session("rust-embedded/session?%", "rust-embedded")?;
        let source:SourceMessage=serde_json::from_value(fixture["source"].clone())?;let original:Vec<u8>=serde_json::from_value(fixture["original_bytes"].clone())?;
        let state=if phase=="init" {
            let receipt=context.source(source.clone(),original.clone()).await?;assert!(!receipt.replayed);
            let request:SessionContextRequest=serde_json::from_value(json!({"version":1,"scope":scope(),"session_id":context.session_id(),"generation":0,"model_id":"gpt-4o","query":"","budget":{"context_tokens":8192,"reserved_output_tokens":512,"required_tokens":0}}))?;
            context.activate(request).await?
        } else {context.inspect().await?};
        assert_eq!(state["version"],1);assert_eq!(state["report"]["scope"],fixture["scope"]);
        let history=context.history().await?;let span=history.history.source_span(&source.id)?;
        assert_eq!(context.recover(&span).await?,original);assert_eq!(history.history.message(&source.id)?,&source);
        let checkpoint=json!({"version":state["version"],"scope":state["report"]["scope"],"source":history.history.message(&source.id)?,"original_bytes":context.recover(&span).await?,"digest":state["report"]["digest"]});
        let path=root.join("rust-embedded.checkpoint.json");if phase=="init" {private(&path,&serde_json::to_vec(&checkpoint)?)?;}else {assert_eq!(checkpoint,serde_json::from_slice::<Value>(&fs::read(path)?)?);assert!(context.source(source,original).await?.replayed);}
        drop(context);actor.shutdown().await?;drop(engine);Ok::<(),Box<dyn std::error::Error>>(())
    })
}
fn verify_checkpoints(root: &Path, fixture: &Value) -> Result<()> {
    for label in [
        "rust-embedded",
        "python-embedded",
        "python-mtls",
        "typescript-embedded",
        "typescript-uds",
        "go-uds",
        "swift-cabi",
    ] {
        let checkpoint: Value =
            serde_json::from_slice(&fs::read(root.join(format!("{label}.checkpoint.json")))?)?;
        assert_eq!(checkpoint["version"], fixture["version"], "{label} version");
        assert_eq!(checkpoint["scope"], fixture["scope"], "{label} scope");
        assert_eq!(
            checkpoint["source"], fixture["source"],
            "{label} canonical source/ns/digest"
        );
        assert_eq!(
            checkpoint["original_bytes"], fixture["original_bytes"],
            "{label} exact recovery"
        );
        assert_eq!(
            checkpoint["digest"]
                .as_str()
                .ok_or("missing real report digest")?
                .len(),
            64
        );
    }
    Ok(())
}
fn swift_build(repo: &Path, native: &Path, root: &Path) -> Result<()> {
    let compiler = std::env::var("HM_SDK_SWIFTC").unwrap_or_else(|_| "swiftc".into());
    let mut sources = fs::read_dir(repo.join("sdk/swift/Sources/HyperMind"))?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    sources.retain(|p| p.extension().is_some_and(|e| e == "swift"));
    sources.sort();
    run(Command::new(&compiler)
        .args([
            "-emit-library",
            "-emit-module",
            "-module-name",
            "HyperMind",
            "-I",
        ])
        .arg(repo.join("sdk/swift/Sources/CHyperMind"))
        .arg("-L")
        .arg(native)
        .arg("-lhypermind")
        .args(sources)
        .arg("-o")
        .arg(root.join("libHyperMind.so"))
        .arg("-emit-module-path")
        .arg(root.join("HyperMind.swiftmodule")))?;
    run(Command::new(compiler)
        .args(["-parse-as-library", "-I"])
        .arg(root)
        .arg("-I")
        .arg(repo.join("sdk/swift/Sources/CHyperMind"))
        .arg("-L")
        .arg(root)
        .arg("-lHyperMind")
        .arg("-L")
        .arg(native)
        .arg("-lhypermind")
        .arg(root.join("consumer.swift"))
        .arg("-o")
        .arg(root.join("swift-consumer")))?;
    Ok(())
}
fn consumers(repo: &Path, native: &Path, root: &Path, grpc: u16, phase: &str) -> Result<()> {
    let python = std::env::var("HM_SDK_PYTHON").unwrap_or_else(|_| "python3".into());
    let node = std::env::var("HM_SDK_NODE").unwrap_or_else(|_| "node".into());
    run(Command::new(python)
        .arg(root.join("consumer.py"))
        .arg(repo)
        .arg(native)
        .arg(root)
        .arg(grpc.to_string())
        .arg(phase)
        .env(
            "PYTHONPATH",
            std::env::var("HM_SDK_PYTHON_DEPS")
                .or_else(|_| std::env::var("PYTHONPATH"))
                .unwrap_or_default(),
        ))?;
    run(Command::new(node)
        .arg(root.join("consumer.js"))
        .arg(repo)
        .arg(native)
        .arg(root)
        .arg(phase))?;
    run(Command::new(root.join("go-consumer")).arg(root).arg(phase))?;
    let library_path = std::env::join_paths([root.to_path_buf(), native.to_path_buf()])?;
    let mut swift = if let Some(wrapper) = std::env::var_os("HM_SDK_SWIFT_EXECUTOR") {
        let mut command = Command::new(wrapper);
        command.arg(root.join("swift-consumer"));
        command
    } else {
        Command::new(root.join("swift-consumer"))
    };
    run(swift
        .arg(root)
        .arg(phase)
        .env("LD_LIBRARY_PATH", library_path))?;
    Ok(())
}
#[test]
fn unified_sdk_journey() -> Result<()> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let default_target = std::env::current_exe()?
        .parent()
        .ok_or("missing test binary directory")?
        .parent()
        .ok_or("missing target directory")?
        .to_path_buf();
    let native = std::env::var_os("HM_SDK_NATIVE_DIR")
        .map(PathBuf::from)
        .unwrap_or(default_target);
    let binary = std::env::var_os("HM_DAEMON_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| native.join("hm"));
    for path in [
        binary.clone(),
        native.join("lib_native.so"),
        native.join("libhypermind_engine_napi.so"),
        native.join("libhypermind.so"),
    ] {
        if !path.is_file() {
            return Err(format!(
                "required actual SDK artifact unavailable: {}",
                path.display()
            )
            .into());
        }
    }
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    let common = fixture()?;
    private(&root.join("fixture.json"), &serde_json::to_vec(&common)?)?;
    private(&root.join("hm.conf"),format!("socket={}\ndata={}\nuser={}\nkek={}\nadmin_token={}\nactor=7:{}\nprojection_map_bytes=67108864\n",root.join("hm.sock").display(),root.join("daemon-data").display(),"11".repeat(16),"22".repeat(32),"33".repeat(32),"44".repeat(32)).as_bytes())?;
    private(
        &root.join("scope.json"),
        &serde_json::to_vec(&json!({"version":1,"actor":7,"scope":scope()}))?,
    )?;
    private(&root.join("consumer.py"), PYTHON.as_bytes())?;
    private(&root.join("consumer.js"), NODE.as_bytes())?;
    private(&root.join("consumer.go"), GO.as_bytes())?;
    private(&root.join("consumer.swift"), SWIFT.as_bytes())?;
    private(&root.join("go.mod"),format!("module hypermind-sdk-journey\n\ngo 1.25.0\n\nrequire centra/core/cortexclient v0.0.0\nreplace centra/core/cortexclient => {:?}\n",repo.join("sdk/go").to_string_lossy()).as_bytes())?;
    let go = std::env::var("HM_SDK_GO").unwrap_or_else(|_| "go".into());
    run(Command::new(go)
        .current_dir(root)
        .args(["build", "-mod=mod", "-o"])
        .arg(root.join("go-consumer"))
        .arg(root.join("consumer.go")))?;
    swift_build(&repo, &native, root)?;
    certificates(root)?;
    let grpc = port()?;
    let daemon = start(&binary, root, grpc)?;
    rust_consumer(root, &common, "init")?;
    consumers(&repo, &native, root, grpc, "init")?;
    verify_checkpoints(root, &common)?;
    drop(daemon);
    let daemon = start(&binary, root, grpc)?;
    rust_consumer(root, &common, "resume")?;
    consumers(&repo, &native, root, grpc, "resume")?;
    verify_checkpoints(root, &common)?;
    drop(daemon);
    println!("unified SDK journey passed: Rust embedded, Python embedded/mTLS, TypeScript embedded/UDS, Go UDS, Swift C ABI; common source/ns/digest/exact bytes and actual engine/daemon restart. Apple/mobile runtime qualification unavailable on this host.");
    Ok(())
}

const PYTHON: &str = r#"
import asyncio, importlib.util, json, pathlib, sys
repo,native,directory,port,phase=sys.argv[1:]
base=pathlib.Path(directory); fixture=json.loads((base/'fixture.json').read_text())
sys.path.insert(0,str(pathlib.Path(repo)/'sdk/python'))
from hypermind import Engine,Client
from hypermind.context import Scope,SourceMessage,TokenBudget,encode_context_identifier
spec=importlib.util.spec_from_file_location('hypermind._native',str(pathlib.Path(native)/'lib_native.so'))
module=importlib.util.module_from_spec(spec);sys.modules['hypermind._native']=module;spec.loader.exec_module(module)
scope=Scope(**fixture['scope']);source=SourceMessage(**{**fixture['source'],'parts':tuple(fixture['source']['parts'])}).freeze()
assert source.source_digest==fixture['source']['source_digest']
async def consume(engine,label):
    context=engine.context(scope,label+'/session?%',actor=7,context_owner='hypermind',conversation=label)
    if phase=='init':
        assert not (await context.ingest_source(source,bytes(fixture['original_bytes'])))['replayed']
        state=await context.activate('',TokenBudget(8192,512,0))
    else: state=await context.inspect()
    assert state['version']==1 and context.version==1 and state['report']['scope']==fixture['scope']
    recovered=await engine.tool('inspect',{'uri':context.inspect_uri()+'/source/'+encode_context_identifier(source.id)})
    assert recovered['ok'];actual=recovered['items'][0]
    assert actual['source']==fixture['source'] and actual['original_bytes']==fixture['original_bytes']
    checkpoint={'version':state['version'],'scope':state['report']['scope'],'source':actual['source'],'original_bytes':actual['original_bytes'],'digest':state['report']['digest']}
    path=base/(label+'.checkpoint.json')
    if phase=='init':path.write_text(json.dumps(checkpoint,ensure_ascii=False))
    else:
        assert checkpoint==json.loads(path.read_text())
        assert (await context.ingest_source(source,bytes(fixture['original_bytes'])))['replayed']
    print(label+' '+phase+' actual source/time/recovery/version/scope passed')
async def main():
    engine=await Engine.open(base/'python-embedded',actor=7,user_hex='11'*16,kek_hex='22'*32,projection_map_bytes=67108864,context_scope=scope)
    try:await consume(engine,'python-embedded')
    finally:await engine.close()
    client=Client('localhost:'+port,token=bytes.fromhex('44'*32),ca=(base/'ca.pem').read_bytes(),certificate=(base/'client.pem').read_bytes(),private_key=(base/'client.key').read_bytes())
    try:await consume(client,'python-mtls')
    finally:await client.close()
asyncio.run(main())
"#;
const NODE: &str = r#"
const assert=require('node:assert/strict'),path=require('node:path'),fs=require('node:fs');
const [repo,native,directory,phase]=process.argv.slice(2),fixture=JSON.parse(fs.readFileSync(path.join(directory,'fixture.json'),'utf8'));
const {Client,freezeSourceMessage,encodeContextIdentifier}=require(path.join(repo,'sdk/typescript/packages/client/dist/index.js'));
const nativePackage=path.join(directory,'node_modules/@hypermind/engine-linux-x64-gnu');fs.mkdirSync(nativePackage,{recursive:true});
fs.copyFileSync(path.join(native,'libhypermind_engine_napi.so'),path.join(nativePackage,'engine.node'));fs.writeFileSync(path.join(nativePackage,'index.js'),"module.exports=require('./engine.node');");
process.env.NODE_PATH=path.join(directory,'node_modules');require('node:module').Module._initPaths();
const {HyperMind}=require(path.join(repo,'sdk/typescript/packages/engine/dist/index.js'));
const source=freezeSourceMessage(fixture.source);assert.equal(source.source_digest,fixture.source.source_digest);
async function consume(engine,label){
 const context=engine.context(fixture.scope,label+'/session?%',7,'hypermind',label);
 let state;
 if(phase==='init'){assert.equal((await context.ingestSource(source,Buffer.from(fixture.original_bytes))).replayed,false);state=await context.activate('',{context_tokens:8192,reserved_output_tokens:512,required_tokens:0});}
 else state=await context.inspect();
 assert.equal(state.version,1);assert.equal(context.version,1);assert.deepEqual(state.report.scope,fixture.scope);
 const recovered=await engine.callTool('inspect',{uri:'hm://7/context/'+encodeContextIdentifier(context.sessionId)+'/source/'+encodeContextIdentifier(source.id)});
 assert.equal(recovered.ok,true);const actual=recovered.items[0];assert.deepEqual(actual.source,fixture.source);assert.deepEqual(actual.original_bytes,fixture.original_bytes);
 const checkpoint={version:state.version,scope:state.report.scope,source:actual.source,original_bytes:actual.original_bytes,digest:state.report.digest};
 const file=path.join(directory,label+'.checkpoint.json');
 if(phase==='init')fs.writeFileSync(file,JSON.stringify(checkpoint));else{assert.deepEqual(checkpoint,JSON.parse(fs.readFileSync(file,'utf8')));assert.equal((await context.ingestSource(source,Buffer.from(fixture.original_bytes))).replayed,true);}
 console.log(label+' '+phase+' actual source/time/recovery/version/scope passed');
}
(async()=>{
 const engine=await HyperMind.open(path.join(directory,'typescript-embedded'),{actor:7,userHex:'11'.repeat(16),kekHex:'22'.repeat(32),projectionMapBytes:67108864,contextScope:fixture.scope});
 try{await consume(engine,'typescript-embedded');}finally{await engine.close();}
 const client=await Client.connect({socketPath:path.join(directory,'hm.sock'),capabilityToken:Buffer.from('44'.repeat(32),'hex')});
 try{await consume(client,'typescript-uds');}finally{await client.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
"#;
const GO: &str = r#"
package main
import("bytes";"context";"encoding/json";"fmt";"os";"path/filepath";"time";sdk "centra/core/cortexclient")
func check(err error){if err!=nil{panic(err)}}
func main(){
 root,phase:=os.Args[1],os.Args[2]
 var f struct{Version uint32 `json:"version"`;Scope sdk.ContextScope `json:"scope"`;Source sdk.ContextSource `json:"source"`;Original sdk.ContextBytes `json:"original_bytes"`}
 data,err:=os.ReadFile(filepath.Join(root,"fixture.json"));check(err);check(json.Unmarshal(data,&f));frozen,err:=f.Source.Freeze();check(err);if frozen.SourceDigest!=f.Source.SourceDigest{panic("canonical digest differs")}
 var token [32]byte;for i:=range token{token[i]=0x44};var connection [16]byte;for i:=range connection{connection[i]=0x77}
 client,err:=sdk.Dial(sdk.Config{SocketPath:filepath.Join(root,"hm.sock"),CapabilityToken:token,ConnectionID:connection,DialTimeout:5*time.Second,RequestTimeout:15*time.Second});check(err);defer client.Close()
 consumer,err:=sdk.NewContextClient(client,sdk.ContextConfig{Scope:f.Scope,SessionID:"go-uds/session?%",Conversation:"go-uds",Actor:7,ContextOwner:"hypermind"});check(err)
 ctx,cancel:=context.WithTimeout(context.Background(),30*time.Second);defer cancel()
 var state sdk.ContextDiagnostics
 if phase=="init"{receipt,err:=consumer.IngestSource(ctx,f.Source,f.Original);check(err);if receipt.Replayed{panic("initial ingest replayed")};state,err=consumer.Activate(ctx,sdk.ContextBudget{ContextTokens:8192,ReservedOutputTokens:512},sdk.ContextActivation{});check(err)}else{state,err=consumer.Inspect(ctx);check(err)}
 if state.Version!=1||consumer.Version()!=1||state.Report.Scope.OwnerID!=f.Scope.OwnerID||state.Report.Scope.ProjectID!=f.Scope.ProjectID{panic("version/scope mismatch")}
 recovered,err:=consumer.Recover(ctx,f.Source.ID);check(err);if !bytes.Equal(recovered.OriginalBytes,f.Original)||recovered.Source.SourceDigest!=f.Source.SourceDigest||recovered.Source.RecordedAtNS!=f.Source.RecordedAtNS||*recovered.Source.OccurredAtNS!=*f.Source.OccurredAtNS{panic("source/time/exact recovery mismatch")}
 checkpoint,err:=json.Marshal(map[string]any{"version":state.Version,"scope":state.Report.Scope,"source":recovered.Source,"original_bytes":recovered.OriginalBytes,"digest":state.Report.Digest});check(err)
 file:=filepath.Join(root,"go-uds.checkpoint.json")
 if phase=="init"{check(os.WriteFile(file,checkpoint,0600))}else{old,err:=os.ReadFile(file);check(err);var a,b any;check(json.Unmarshal(old,&a));check(json.Unmarshal(checkpoint,&b));aa,_:=json.Marshal(a);bb,_:=json.Marshal(b);if !bytes.Equal(aa,bb){panic("restart changed checkpoint")};receipt,err:=consumer.IngestSource(ctx,f.Source,f.Original);check(err);if !receipt.Replayed{panic("restart replay duplicated source")}}
 fmt.Println("go-uds",phase,"actual source/time/recovery/version/scope passed")
}
"#;
const SWIFT: &str = r#"
import Foundation
import HyperMind
struct Fixture: Decodable { let version:UInt32;let scope:ContextScope;let source:ContextSourceMessage;let original_bytes:[UInt8] }
@main struct Consumer {
 static func main() async throws {
  let root=URL(fileURLWithPath:CommandLine.arguments[1]),phase=CommandLine.arguments[2]
  let fixtureData=try Data(contentsOf:root.appendingPathComponent("fixture.json"));let f=try JSONDecoder().decode(Fixture.self,from:fixtureData)
  let engine=try HyperMindEngine(credentials:HyperMindCredentials(actor:7,userHex:String(repeating:"11",count:16),kekHex:String(repeating:"22",count:32),projectionMapBytes:67108864),stateDirectory:root.appendingPathComponent("swift-cabi").path,contextScope:f.scope)
  defer{try? engine.close()}
  let client=try HyperMindContextClient(engine:engine,scope:f.scope,sessionID:"swift-cabi/session?%",actorID:7,ownership:.hypermind,conversation:"swift-cabi")
  let state:ContextDiagnostics
  if phase=="init"{let receipt=try await client.ingest(f.source,originalBytes:Data(f.original_bytes));guard !receipt.replayed else{fatalError("initial source replayed")};state=try await client.activate(query:"",budget:ContextTokenBudget(contextTokens:8192,reservedOutputTokens:512))}else{state=try await client.inspect()}
  guard state.version==1,state.report.scope==f.scope else{fatalError("scope/version mismatch")}
  let original=try await client.recover(sourceID:f.source.id);guard original==Data(f.original_bytes) else{fatalError("source byte mismatch")}
  let history=try await client.history();guard let actual=history.messages.first,actual.sourceDigest==f.source.sourceDigest,actual.recordedAtNS==f.source.recordedAtNS,actual.occurredAtNS==f.source.occurredAtNS else{fatalError("history source/ns mismatch")}
  var fixture=try JSONSerialization.jsonObject(with:fixtureData) as! [String:Any];fixture.removeValue(forKey:"original_bytes");fixture.removeValue(forKey:"source")
  fixture["source"]=try JSONSerialization.jsonObject(with:JSONEncoder().encode(actual));fixture["original_bytes"]=Array(original);fixture["digest"]=state.report.digest
  let file=root.appendingPathComponent("swift-cabi.checkpoint.json");let checkpoint=try JSONSerialization.data(withJSONObject:fixture,options:[.sortedKeys])
  if phase=="init"{try checkpoint.write(to:file)}else{let old=try Data(contentsOf:file);guard old==checkpoint else{fatalError("CABI reopen changed checkpoint")};let receipt=try await client.ingest(f.source,originalBytes:original);guard receipt.replayed else{fatalError("CABI replay duplicated source")}}
  print("swift-cabi \(phase) actual source/time/recovery/version/scope passed")
 }
}
"#;
