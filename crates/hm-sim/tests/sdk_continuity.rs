#![forbid(unsafe_code)]
use serde_json::json;
use std::{
    fs,
    net::{TcpListener, TcpStream},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command},
    time::{Duration, Instant},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn run(command: &mut Command) -> Result<()> {
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "SDK continuity command failed: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    print!("{}", String::from_utf8_lossy(&output.stdout));
    Ok(())
}
struct Daemon(Child);
impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn port() -> Result<u16> {
    Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
}
fn start(binary: &Path, dir: &Path, grpc: u16, admin: u16) -> Result<Daemon> {
    let log = fs::File::create(dir.join("daemon.log"))?;
    let mut daemon = Daemon(
        Command::new(binary)
            .arg("serve")
            .arg("--config")
            .arg(dir.join("hm.conf"))
            .arg("--context-scope")
            .arg(dir.join("scope.json"))
            .arg("--grpc-bind")
            .arg(format!("127.0.0.1:{grpc}"))
            .arg("--grpc-admin-bind")
            .arg(format!("127.0.0.1:{admin}"))
            .arg("--tls-cert")
            .arg(dir.join("server.pem"))
            .arg("--tls-key")
            .arg(dir.join("server.key"))
            .arg("--tls-client-ca")
            .arg(dir.join("ca.pem"))
            .stdout(log.try_clone()?)
            .stderr(log)
            .spawn()?,
    );
    let begin = Instant::now();
    while TcpStream::connect(("127.0.0.1", grpc)).is_err() {
        if daemon.0.try_wait()?.is_some() || begin.elapsed() > Duration::from_secs(20) {
            return Err(fs::read_to_string(dir.join("daemon.log"))?.into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(daemon)
}
fn certificates(dir: &Path) -> Result<()> {
    run(Command::new("openssl").current_dir(dir).args([
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
        "/CN=hypermind-continuity",
        "-days",
        "1",
    ]))?;
    for (name, purpose) in [("server", "serverAuth"), ("client", "clientAuth")] {
        run(Command::new("openssl").current_dir(dir).args([
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
        fs::write(
            dir.join(format!("{name}.ext")),
            format!(
                "basicConstraints=CA:FALSE\nextendedKeyUsage={purpose}\nsubjectAltName=DNS:localhost,IP:127.0.0.1\n"
            ),
        )?;
        run(Command::new("openssl").current_dir(dir).args([
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
fn copy_directory(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            copy_directory(&entry.path(), &destination.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), destination.join(entry.file_name()))?;
        }
    }
    Ok(())
}
#[test]
fn sdk_continuity() -> Result<()> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let native = std::env::var_os("HM_SDK_NATIVE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("target/debug"));
    let binary = std::env::var_os("HM_DAEMON_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| native.join("hm"));
    for p in [
        &binary,
        &native.join("lib_native.so"),
        &native.join("libhypermind_engine_napi.so"),
        &native.join("libhypermind.so"),
    ] {
        if !p.is_file() {
            return Err(format!("current runtime artifact required: {}", p.display()).into());
        }
    }
    let python = std::env::var("HM_SDK_PYTHON").unwrap_or("python3".into());
    let node = std::env::var("HM_SDK_NODE").unwrap_or("node".into());
    let go = std::env::var("HM_SDK_GO").unwrap_or("go".into());
    let deps = std::env::var("HM_SDK_PYTHON_DEPS").unwrap_or_default();
    let py_path = format!("{}:{}", repo.join("sdk/python").display(), deps);
    let temp = tempfile::tempdir()?;
    let dir = temp.path();
    run(Command::new(&node)
        .current_dir(repo.join("sdk/typescript"))
        .arg("node_modules/typescript/bin/tsc")
        .args(["-p", "packages/client/tsconfig.json"]))?;
    run(Command::new(&python)
        .current_dir(&repo)
        .args([
            "-m",
            "unittest",
            "discover",
            "-s",
            "sdk/python/tests",
            "-p",
            "test_continuity.py",
            "-v",
        ])
        .env("PYTHONPATH", &py_path)
        .env("HM_SDK_NATIVE_DIR", &native))?;
    run(Command::new(&node)
        .current_dir(&repo)
        .arg("--test")
        .arg(repo.join("sdk/typescript/packages/client/dist/continuity.test.js"))
        .env("HM_SDK_NATIVE_DIR", &native))?;
    run(Command::new(go)
        .current_dir(repo.join("sdk/go"))
        .args([
            "test",
            "-run",
            "^TestContextContinuationRealDaemon$",
            "-count=1",
            "-v",
        ])
        .env("HM_DAEMON_BIN", &binary))?;
    let swift_package = dir.join("swift-package");
    copy_directory(
        &repo.join("sdk/swift/Sources"),
        &swift_package.join("Sources"),
    )?;
    fs::copy(
        repo.join("sdk/swift/Package.swift"),
        swift_package.join("Package.swift"),
    )?;
    fs::create_dir_all(swift_package.join("Tests/HyperMindTests"))?;
    fs::copy(
        repo.join("sdk/swift/Tests/HyperMindTests/ContextContinuityTests.swift"),
        swift_package.join("Tests/HyperMindTests/ContextContinuityTests.swift"),
    )?;
    fs::write(
        swift_package.join("Sources/CHyperMind/module.modulemap"),
        format!(
            "module CHyperMind {{ header {:?} link \"hypermind\" export * }}",
            repo.join("crates/hm-capi/include/hypermind.h")
        ),
    )?;
    fs::write(
        swift_package.join("Tests/HyperMindTests/ContinuityVersionTests.swift"),
        SWIFT_VERSION,
    )?;
    let mut swift = Command::new(std::env::var("HM_SDK_SWIFT_EXECUTOR").unwrap_or("swift".into()));
    if std::env::var_os("HM_SDK_SWIFT_EXECUTOR").is_some() {
        swift.arg("swift");
    }
    let libraries = format!(
        "{}:{}",
        native.display(),
        std::env::var("LD_LIBRARY_PATH").unwrap_or_default()
    );
    run(swift
        .current_dir(&swift_package)
        .arg("test")
        .arg("--scratch-path")
        .arg(dir.join("swift-build"))
        .args(["-Xlinker", "-L", "-Xlinker"])
        .arg(&native)
        .args(["--filter", "ContextContinuityTests|ContinuityVersionTests"])
        .env("LD_LIBRARY_PATH", libraries))?;
    certificates(dir)?;
    let grpc = port()?;
    let admin = port()?;
    fs::write(
        dir.join("hm.conf"),
        format!(
            "socket={}\ndata={}\nuser={}\nkek={}\nadmin_token={}\nactor=7:{}\nprojection_map_bytes=67108864\n",
            dir.join("hm.sock").display(),
            dir.join("data").display(),
            "11".repeat(16),
            "22".repeat(32),
            "33".repeat(32),
            "44".repeat(32)
        ),
    )?;
    fs::write(
        dir.join("scope.json"),
        serde_json::to_vec(
            &json!({"version":1,"actor":7,"scope":{"owner_id":"continuity-owner","project_id":"continuity-project","workspace_id":null}}),
        )?,
    )?;
    for name in ["hm.conf", "scope.json"] {
        fs::set_permissions(dir.join(name), fs::Permissions::from_mode(0o600))?;
    }
    fs::write(dir.join("remote.py"), PYTHON)?;
    fs::write(dir.join("remote.js"), NODE)?;
    let go_probe = dir.join("go-probe");
    fs::create_dir_all(&go_probe)?;
    fs::write(go_probe.join("main.go"), GO_VERSION)?;
    fs::write(
        go_probe.join("go.mod"),
        format!(
            "module hypermind-continuity-version\n\ngo 1.25.0\nrequire centra/core/cortexclient v0.0.0\nreplace centra/core/cortexclient => {:?}\n",
            repo.join("sdk/go")
        ),
    )?;
    for phase in ["init", "resume", "uncertain_restart"] {
        let daemon = start(&binary, dir, grpc, admin)?;
        run(Command::new(&python)
            .arg(dir.join("remote.py"))
            .arg(dir)
            .arg(grpc.to_string())
            .arg(phase)
            .arg(daemon.0.id().to_string())
            .env("PYTHONPATH", &py_path))?;
        run(Command::new(&node)
            .arg(dir.join("remote.js"))
            .arg(&repo)
            .arg(dir)
            .arg(phase)
            .arg(daemon.0.id().to_string()))?;
        run(
            Command::new(std::env::var("HM_SDK_GO").unwrap_or("go".into()))
                .current_dir(&go_probe)
                .args(["run", "-mod=mod", "."])
                .arg(dir)
                .arg(phase)
                .arg(daemon.0.id().to_string()),
        )?;
        drop(daemon);
    }
    println!(
        "SDK continuity aggregate: Python embedded+mTLS, TypeScript NAPI+UDS, Go authenticated UDS, Swift Linux CABI passed; Swift in-flight/stream cancellation and Apple platforms unqualified"
    );
    Ok(())
}
const PYTHON: &str = r#"
import asyncio,json,pathlib,pickle,sys,os,signal
from hypermind import Client
from hypermind.context import Scope,SourceMessage,TokenBudget
from hypermind.continuity import ContextContinuation,ContinuationError
root=pathlib.Path(sys.argv[1]);port,phase,pid=sys.argv[2:];pid=int(pid);scope=Scope('continuity-owner','continuity-project');budget=TokenBudget(8192,512,0)
source=SourceMessage('m1',0,'user',({'kind':'text','text':'hello λ'},),1791287999123456789,1791288000123456789,'user_asserted').freeze()
def client(token=b'\x44'*32):return Client('localhost:'+port,token=token,ca=(root/'ca.pem').read_bytes(),certificate=(root/'client.pem').read_bytes(),private_key=(root/'client.key').read_bytes())
def context(engine):return engine.context(scope,'python/%?',actor=7,context_owner='hypermind',conversation='python')
async def main():
 engine=client();await engine.connect();continuation=ContextContinuation(context(engine),pickle.loads((root/('python.unknown' if phase=='uncertain_restart' else 'python.checkpoint')).read_bytes()) if phase!='init' else None)
 try:
  if phase=='init':
   await continuation.ingest_source(source,'hello λ'.encode());await continuation.activate('hello',budget);await continuation.fork('python-child/%?','python-child');(root/'python.checkpoint').write_bytes(pickle.dumps(continuation.checkpoint()))
  elif phase=='uncertain_restart':
   async def authenticated():return context(engine)
   await continuation.reconnect(authenticated);assert continuation.uncertain.effect_state=='unknown'
   next_source=SourceMessage('cancelled',1,'user',({'kind':'text','text':'possibly accepted'},),None,1791288000123456790,'user_asserted').freeze()
   try:await continuation.ingest_source(next_source,b'possibly accepted')
   except ContinuationError as error:assert error.effect_state=='not_dispatched'
   else:raise AssertionError('restart replayed uncertain source')
   await continuation.inspect();assert continuation.uncertain;await continuation.abandon_uncertain();assert continuation.checkpoint().unresolved
  else:
   bad=client(b'\xff'*32)
   async def rejected():await bad.connect();return context(bad)
   try:await continuation.reconnect(rejected)
   except Exception:pass
   else:raise AssertionError('invalid credential accepted')
   finally:await bad.close()
   checkpoint=continuation.checkpoint()
   async def authenticated():return context(engine)
   await continuation.reconnect(authenticated);assert continuation.accepted.cursor==checkpoint.accepted.cursor and continuation.accepted.generation==checkpoint.accepted.generation
   child=engine.context(scope,'python-child/%?',actor=7,context_owner='hypermind',conversation='python-child');await child.activate('hello',budget);history=(await engine.tool('inspect',{'uri':child.inspect_uri()+'/history'}))['items'][0];assert history['messages'][0]['source_digest']==source.source_digest and history['messages'][0]['recorded_at_ns']=='1791288000123456789'
   wire=context(engine).request_context(budget);wire['version']=2;refused=await engine.tool('activate',{'conversation':'python','query':'hello','budget_tokens':budget.available,'context':wire});assert not refused['ok']
   await continuation.activate('hello',budget,options={'model_id':'gpt-4o-mini'});assert continuation.accepted.generation>checkpoint.accepted.generation
   next_source=SourceMessage('cancelled',1,'user',({'kind':'text','text':'possibly accepted'},),None,1791288000123456790,'user_asserted').freeze();os.kill(pid,signal.SIGSTOP)
   try:
    try:await asyncio.wait_for(continuation.ingest_source(next_source,b'possibly accepted'),.15)
    except asyncio.TimeoutError:pass
    else:raise AssertionError('stopped daemon completed mutation')
    assert continuation.uncertain.effect_state=='unknown'
    try:await continuation.ingest_source(next_source,b'possibly accepted')
    except ContinuationError as error:assert error.effect_state=='not_dispatched'
    else:raise AssertionError('possibly accepted write replayed')
   finally:os.kill(pid,signal.SIGCONT)
   (root/'python.unknown').write_bytes(pickle.dumps(continuation.checkpoint()))
   await continuation.inspect();assert continuation.uncertain;await continuation.abandon_uncertain();assert continuation.checkpoint().unresolved
 finally:await engine.close()
 print('Python mTLS '+phase+' authenticated continuity passed')
asyncio.run(main())
"#;
const NODE: &str = r#"
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');const[repo,directory,phase,pid]=process.argv.slice(2);
const{Client,ContextContinuation,ContinuationError,freezeSourceMessage}=require(path.join(repo,'sdk/typescript/packages/client/dist/index.js'));
const scope={owner_id:'continuity-owner',project_id:'continuity-project',workspace_id:null},budget={context_tokens:8192,reserved_output_tokens:512,required_tokens:0};
const source=freezeSourceMessage({id:'m1',ordinal:0,role:'user',parts:[{kind:'text',text:'hello λ'}],occurred_at_ns:1791287999123456789n,recorded_at_ns:1791288000123456789n,authority:'user_asserted',source_digest:''});
const connect=token=>Client.connect({socketPath:path.join(directory,'hm.sock'),capabilityToken:token??Buffer.alloc(32,0x44)});const context=client=>client.context(scope,'node/%?',7,'hypermind','node');
async function main(){const engine=await connect(),continuation=new ContextContinuation(context(engine),engine,phase!=='init'?JSON.parse(fs.readFileSync(path.join(directory,phase==='uncertain_restart'?'node.unknown':'node.checkpoint'),'utf8')):undefined);
try{if(phase==='init'){await continuation.ingestSource(source,Buffer.from('hello λ'));await continuation.activate('hello',budget);await continuation.fork('node-child/%?','node-child');fs.writeFileSync(path.join(directory,'node.checkpoint'),JSON.stringify(continuation.checkpoint()));}
else if(phase==='uncertain_restart'){await continuation.reconnect(async()=>({context:context(engine),transport:engine}));assert.equal(continuation.uncertain.effectState,'unknown');const next=freezeSourceMessage({...source,id:'cancelled',ordinal:1,recorded_at_ns:1791288000123456790n,source_digest:''});await assert.rejects(()=>continuation.ingestSource(next,Buffer.from('hello λ')),error=>error instanceof ContinuationError&&error.effectState==='not_dispatched');await continuation.inspect();assert.ok(continuation.uncertain);await continuation.abandonUncertain();assert.ok(continuation.checkpoint().unresolved.length);}
else{const checkpoint=continuation.checkpoint();await assert.rejects(()=>continuation.reconnect(async()=>{const bad=await connect(Buffer.alloc(32,0xff));try{return {context:context(bad),transport:bad};}catch(error){await bad.close();throw error;}}));await continuation.reconnect(async()=>({context:context(engine),transport:engine}));assert.deepEqual(continuation.accepted.cursor,checkpoint.accepted.cursor);assert.equal(continuation.accepted.generation,checkpoint.accepted.generation);
const child=engine.context(scope,'node-child/%?',7,'hypermind','node-child');await child.activate('hello',budget);const history=(await engine.callTool('inspect',{uri:child.inspectUri()+'/history'})).items[0];assert.equal(history.messages[0].source_digest,source.source_digest);assert.equal(history.messages[0].recorded_at_ns,'1791288000123456789');
const wire=context(engine).requestContext(budget);wire.version=2;assert.equal((await engine.callTool('activate',{conversation:'node',query:'hello',budget_tokens:7680,context:wire})).ok,false);await continuation.activate('hello',budget,{model_id:'gpt-4o-mini'});assert.ok(continuation.accepted.generation>checkpoint.accepted.generation);
const next=freezeSourceMessage({...source,id:'cancelled',ordinal:1,recorded_at_ns:1791288000123456790n,source_digest:''}),abort=new AbortController();process.kill(Number(pid),'SIGSTOP');try{const timer=setTimeout(()=>abort.abort(),150);try{await assert.rejects(()=>continuation.ingestSource(next,Buffer.from('hello λ'),abort.signal),error=>error instanceof ContinuationError&&error.effectState==='unknown');}finally{clearTimeout(timer);}assert.equal(continuation.uncertain.effectState,'unknown');await assert.rejects(()=>continuation.ingestSource(next,Buffer.from('hello λ')),error=>error instanceof ContinuationError&&error.effectState==='not_dispatched');}finally{process.kill(Number(pid),'SIGCONT');}
fs.writeFileSync(path.join(directory,'node.unknown'),JSON.stringify(continuation.checkpoint()));await continuation.inspect();assert.ok(continuation.uncertain);await continuation.abandonUncertain();assert.ok(continuation.checkpoint().unresolved.length);}}
finally{await engine.close();}console.log('TypeScript UDS '+phase+' authenticated continuity passed');}
main().catch(error=>{console.error(error);process.exitCode=1});
"#;

const SWIFT_VERSION: &str = r#"
import Foundation
import XCTest
@testable import HyperMind
final class ContinuityVersionTests:XCTestCase {
 func testActualNativeVersionRefusal() async throws {
  let root=FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString);defer{try? FileManager.default.removeItem(at:root)}
  let scope=try ContextScope(ownerID:"continuity-owner",projectID:"continuity-project")
  let engine=try HyperMindEngine(credentials:HyperMindCredentials(actor:7,userHex:String(repeating:"11",count:16),kekHex:String(repeating:"22",count:32),projectionMapBytes:67108864),stateDirectory:root.path,contextScope:scope);defer{try? engine.close()}
  let arguments:[String:Any]=["conversation":"version","query":"version","budget_tokens":7680,"context":["version":2,"scope":["owner_id":"continuity-owner","project_id":"continuity-project","workspace_id":NSNull()],"session_id":"version","budget":["context_tokens":8192,"reserved_output_tokens":512,"required_tokens":0],"generation":0]]
  let data=try JSONSerialization.data(withJSONObject:arguments);let response=try await engine.call(verb:"activate",argumentsJSON:String(decoding:data,as:UTF8.self));let envelope=try JSONSerialization.jsonObject(with:Data(response.utf8)) as! [String:Any];XCTAssertEqual(envelope["ok"] as? Bool,false);let items=envelope["items"] as! [[String:Any]];XCTAssertEqual(items.first?["error"] as? String,"kProtocolVersion")
  let context=try HyperMindContextClient(engine:engine,scope:scope,sessionID:"cancel",actorID:7,ownership:.hypermind)
  let continuation=try ContextContinuation(context:context)
  let message=try ContextSourceMessage(id:"never-dispatched",ordinal:0,role:.user,parts:[.text("cancelled before dispatch")],occurredAtNS:nil,recordedAtNS:1791288000123456789,authority:.userAsserted)
  let cancelled=Task {withUnsafeCurrentTask{$0?.cancel()};return try await continuation.ingest(message,originalBytes:Data("cancelled before dispatch".utf8))}
  do{_=try await cancelled.value;XCTFail("cancelled mutation dispatched")}catch let error as ContextContinuationError{XCTAssertEqual(error.effectState,"not_dispatched")}
  _=try await continuation.activate(query:"cancel",budget:ContextTokenBudget(contextTokens:8192,reservedOutputTokens:512))
  let history=try await context.history();XCTAssertTrue(history.messages.isEmpty)

 }
}
"#;
const GO_VERSION: &str = r#"
package main
import("context";"encoding/json";"errors";"fmt";"os";"path/filepath";"strconv";"syscall";"time";sdk "centra/core/cortexclient")
func check(err error){if err!=nil{panic(err)}}
func main(){root,phase:=os.Args[1],os.Args[2];pid,err:=strconv.Atoi(os.Args[3]);check(err);var token [32]byte;for i:=range token{token[i]=0x44};scope:=sdk.ContextScope{OwnerID:"continuity-owner",ProjectID:"continuity-project"};cfg:=sdk.ContextConfig{Scope:scope,SessionID:"go-probe",Conversation:"go-probe",Actor:7,ContextOwner:"hypermind"};dial:=func()(*sdk.ContextClient,error){client,e:=sdk.Dial(sdk.Config{SocketPath:filepath.Join(root,"hm.sock"),CapabilityToken:token,RequestTimeout:5*time.Second});if e!=nil{return nil,e};consumer,e:=sdk.NewContextClient(client,cfg);if e!=nil{client.Close()};return consumer,e};consumer,err:=dial();check(err);var checkpoint *sdk.ContinuationCheckpoint;if phase!="init"{name:="go.checkpoint";if phase=="uncertain_restart"{name="go.unknown"};data,e:=os.ReadFile(filepath.Join(root,name));check(e);checkpoint=&sdk.ContinuationCheckpoint{};check(json.Unmarshal(data,checkpoint))};continuation,err:=sdk.NewContextContinuation(consumer,checkpoint);check(err);defer continuation.Close();ctx:=context.Background();budget:=sdk.ContextBudget{ContextTokens:8192,ReservedOutputTokens:512};source:=sdk.ContextSource{ID:"original",Ordinal:0,Role:"user",Parts:[]sdk.ContextPart{{Kind:"text",Text:"hello λ"}},RecordedAtNS:1791288000123456789,Authority:"user_asserted"};next:=source;next.ID="cancelled";next.Ordinal=1;next.RecordedAtNS=1791288000123456790;save:=func(name string){data,e:=json.Marshal(continuation.Checkpoint());check(e);check(os.WriteFile(filepath.Join(root,name),data,0600))};refuse:=func(){_,e:=continuation.IngestSource(ctx,next,[]byte("hello λ"));var typed *sdk.ContextClientError;if !errors.As(e,&typed)||typed.EffectState!="not_dispatched"{panic("unknown identity replay permitted")}}
if phase=="init"{_,err=continuation.IngestSource(ctx,source,[]byte("hello λ"));check(err);_,err=continuation.Activate(ctx,budget,sdk.ContinuationActivation{ContextActivation:sdk.ContextActivation{Query:"hello"}});check(err);save("go.checkpoint")}else{_,err=continuation.Reconnect(ctx,func(context.Context)(*sdk.ContextClient,error){return dial()});check(err);if phase=="uncertain_restart"{if continuation.Checkpoint().Uncertain==nil{panic("restart lost unknown")};refuse();_,err=continuation.AbandonUncertain(ctx);check(err);refuse()}else{_,err=continuation.Activate(ctx,budget,sdk.ContinuationActivation{ContextActivation:sdk.ContextActivation{Query:"hello",ModelID:"gpt-4o-mini"}});check(err);check(syscall.Kill(pid,syscall.SIGSTOP));func(){defer syscall.Kill(pid,syscall.SIGCONT);timeout,cancel:=context.WithTimeout(ctx,150*time.Millisecond);defer cancel();_,e:=continuation.IngestSource(timeout,next,[]byte("hello λ"));var typed *sdk.ContextClientError;if !errors.As(e,&typed)||typed.EffectState!="unknown"{panic(fmt.Sprintf("blocked transport cancellation: %v",e))};refuse()}();probe,e:=sdk.Dial(sdk.Config{SocketPath:filepath.Join(root,"hm.sock"),CapabilityToken:token});check(e);envelope,e:=probe.CallTool(ctx,"activate",map[string]any{"conversation":"go-version","query":"version","budget_tokens":7680,"context":map[string]any{"version":2,"scope":scope,"session_id":"go-version","budget":budget,"generation":0}});check(e);if envelope["ok"]!=false{panic("unsupported context version accepted")};probe.Close();save("go.unknown");_,err=continuation.Inspect(ctx);check(err);if continuation.Checkpoint().Uncertain==nil{panic("inspection lost unknown")}}};fmt.Println("Go authenticated",phase,"actual continuity passed")}
"#;
