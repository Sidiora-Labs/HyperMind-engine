#![forbid(unsafe_code)]

use serde_json::json;
use std::{fs::{self,OpenOptions},io::Write,net::{TcpListener,TcpStream},os::unix::{fs::{OpenOptionsExt,FileTypeExt},fs::PermissionsExt},path::{Path,PathBuf},process::{Child,Command,Stdio},time::{Duration,Instant}};

type Result<T> = std::result::Result<T,Box<dyn std::error::Error>>;
struct Daemon(Child);
impl Drop for Daemon { fn drop(&mut self) { let _=self.0.kill();let _=self.0.wait(); } }
fn private(path:&Path,bytes:&[u8])->Result<()> {
    let mut file=OpenOptions::new().create_new(true).write(true).mode(0o600).open(path)?;
    file.write_all(bytes)?;file.sync_all()?;Ok(())
}
fn checked(command:&mut Command)->Result<()> {
    let output=command.output()?;
    if !output.status.success() { return Err(format!("actual command failed {}:\n{}\n{}",output.status,String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr)).into()); }
    print!("{}",String::from_utf8_lossy(&output.stdout));Ok(())
}
fn certificates(root:&Path)->Result<()> {
    checked(Command::new("openssl").current_dir(root).args(["req","-x509","-newkey","rsa:2048","-nodes","-keyout","ca.key","-out","ca.pem","-subj","/CN=hypermind-memory-journey","-days","1"]))?;
    for (name,purpose) in [("server","serverAuth"),("client","clientAuth")] {
        checked(Command::new("openssl").current_dir(root).args(["req","-newkey","rsa:2048","-nodes","-keyout",&format!("{name}.key"),"-out",&format!("{name}.csr"),"-subj",&format!("/CN={name}")]))?;
        private(&root.join(format!("{name}.ext")),format!("basicConstraints=CA:FALSE\nextendedKeyUsage={purpose}\nsubjectAltName=DNS:localhost,IP:127.0.0.1\n").as_bytes())?;
        checked(Command::new("openssl").current_dir(root).args(["x509","-req","-in",&format!("{name}.csr"),"-CA","ca.pem","-CAkey","ca.key","-CAcreateserial","-out",&format!("{name}.pem"),"-days","1","-extfile",&format!("{name}.ext")]))?;
    }
    Ok(())
}
fn binding(root:&Path,recipient:bool)->Result<()> {
    let scope=if recipient {json!({"owner_id":"memory-visitor","project_id":"visitor-project","workspace_id":null})}else{json!({"owner_id":"memory-host","project_id":"bench-project","workspace_id":null})};
    let path=root.join("scope.json");
    let bytes=serde_json::to_vec(&json!({"version":1,"actor":7,"scope":scope}))?;
    if path.exists() { fs::write(&path,bytes)?;fs::set_permissions(path,fs::Permissions::from_mode(0o600))?; }else{private(&path,&bytes)?;}
    Ok(())
}
fn configure(root:&Path)->Result<()> {
    fs::create_dir_all(root.join("data"))?;
    private(&root.join("hm.conf"),format!("socket={}\ndata={}\nuser={}\nkek={}\nadmin_token={}\nactor=7:{}\nprojection_map_bytes=67108864\n",root.join("hm.sock").display(),root.join("data").display(),"11".repeat(16),"22".repeat(32),"33".repeat(32),"44".repeat(32)).as_bytes())?;
    binding(root,false)
}
fn start(binary:&Path,root:&Path,certs:&Path,port:u16)->Result<Daemon> {
    let socket=root.join("hm.sock");
    if socket.exists() {
        if !fs::symlink_metadata(&socket)?.file_type().is_socket() { return Err("unexpected fixture socket replacement".into()); }
        fs::remove_file(socket)?;
    }
    let log=OpenOptions::new().write(true).create(true).append(true).mode(0o600).open(root.join("daemon.log"))?;
    let mut daemon=Daemon(Command::new(binary).args(["serve","--config"]).arg(root.join("hm.conf")).arg("--context-scope").arg(root.join("scope.json")).arg("--grpc-bind").arg(format!("127.0.0.1:{port}")).arg("--tls-cert").arg(certs.join("server.pem")).arg("--tls-key").arg(certs.join("server.key")).arg("--tls-client-ca").arg(certs.join("ca.pem")).env("HM_EMBEDDING_PROVIDER","local").env("HM_EMBEDDING_MODEL","bge-small-en-v1.5").stdin(Stdio::null()).stdout(log.try_clone()?).stderr(log).spawn()?);
    let began=Instant::now();
    loop {
        if root.join("hm.sock").exists() && TcpStream::connect(("127.0.0.1",port)).is_ok() {return Ok(daemon);}
        if daemon.0.try_wait()?.is_some() || began.elapsed()>Duration::from_secs(90) {return Err(format!("actual embedding daemon startup failed:\n{}",fs::read_to_string(root.join("daemon.log"))?).into());}
        std::thread::sleep(Duration::from_millis(25));
    }
}
fn consumer(repo:&Path,root:&Path,port:u16,phase:&str)->Result<()> {
    let python=std::env::var("HM_SDK_PYTHON").unwrap_or_else(|_|"python3".into());
    let mut command=Command::new(python);
    command.arg(root.join("host.py")).arg(repo).arg(root).arg(port.to_string()).arg(phase);
    if let Some(deps)=std::env::var_os("HM_SDK_PYTHON_DEPS") {command.env("PYTHONPATH",deps);}
    checked(&mut command)
}
#[test]
fn unified_memory_journey()->Result<()> {
    let repo=Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize()?;
    let binary=std::env::var_os("HM_DAEMON_BIN").map(PathBuf::from).unwrap_or(std::env::current_exe()?.parent().ok_or("missing test directory")?.parent().ok_or("missing target directory")?.join("hm"));
    if !binary.is_file(){return Err("actual hm daemon binary is required".into());}
    for key in ["GIDEON_RUNTIME_PATH","HM_EMBEDDING_MODEL_DIRECTORY"] {
        let path=std::env::var_os(key).ok_or_else(||format!("required actual dependency path missing: {key}"))?;
        if !Path::new(&path).is_dir(){return Err(format!("actual dependency directory unavailable: {key}").into());}
    }
    let directory=tempfile::tempdir()?;let root=directory.path();
    let original=root.join("original");let restored=root.join("restored");
    fs::create_dir(&original)?;fs::create_dir(&restored)?;
    configure(&original)?;configure(&restored)?;certificates(root)?;
    private(&root.join("host.py"),HOST.as_bytes())?;
    let port=TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();
    let daemon=start(&binary,&original,root,port)?;
    consumer(&repo,root,port,"init")?;
    drop(daemon);
    binding(&original,true)?;
    let daemon=start(&binary,&original,root,port)?;
    consumer(&repo,root,port,"grant-refusal")?;
    drop(daemon);
    binding(&original,false)?;
    let daemon=start(&binary,&original,root,port)?;
    consumer(&repo,root,port,"resume")?;
    drop(daemon);
    let daemon=start(&binary,&restored,root,port)?;
    consumer(&repo,root,port,"restore")?;
    drop(daemon);
    let daemon=start(&binary,&restored,root,port)?;
    consumer(&repo,root,port,"restore-resume")?;
    drop(daemon);
    println!("combined memory host journey passed: actual mTLS daemon/public Gideon host/Python SDK, exact source times and bytes, native revisions/grant refusal, pinned local BGE backfill and semantic retrieval, required memory and correction fences, durable jobs, canonical JSONL restore and fresh host/daemon restart");
    Ok(())
}

const HOST:&str=r#"
import asyncio, copy, hashlib, json, os, pathlib, sys
from dataclasses import asdict
repo,directory,port,phase=sys.argv[1:]
root=pathlib.Path(directory)
sys.path.insert(0,str(pathlib.Path(repo)/'sdk/python'))
runtime=pathlib.Path(os.environ['GIDEON_RUNTIME_PATH'])
if (runtime/'runtime'/'gideon').is_dir(): runtime=runtime/'runtime'
sys.path.insert(0,str(runtime))
os.environ['GIDEON_HOME']=str(root/'host-home')
from hypermind import Client
from hypermind.context import Scope, SourceMessage, TokenBudget, ContextClientError, make_memory_record, seal_memory_record, decode_memory_export, encode_context_identifier
from hypermind.gideon import GideonContextAdapter, ProviderProfile
from gideon.cognition.history import ConversationLog
from gideon.cognition.memory import MemoryJournal
from gideon.cognition.context import PromptAssembler
from gideon.cognition.context_engine import get_engine, prepare_context_turn, assemble_context, set_engine

SCOPE=Scope('memory-host','bench-project')
VISITOR=Scope('memory-visitor','visitor-project')
SESSION='combined-context'
CONVERSATION='combined-conversation'
HOST_SESSION='bench-host'
BUDGET=TokenBudget(16000,2000,1000)
STAMP='1791288000123456789'
CLOCK_RAW=b'{ "inspection": "calibration-A7", "recorded_ns": "1791288000123456789" }\r\n'
DOC='Thermoregulation uses a jacket that circulates chilled liquid around a laboratory vessel to control its temperature.'

async def refused(operation, expected):
    try: await operation
    except ContextClientError as error:
        actual=error.detail['items'][0]['error']
        assert actual in expected,(actual,error.detail)
    else: raise AssertionError('forbidden native operation was accepted')

async def history(client,context):
    value=await client.tool('inspect',{'uri':context.inspect_uri()+'/history'})
    assert value['ok'],value
    return value['items'][0]

async def exact_source(client,context,source_id,original):
    value=await client.tool('inspect',{'uri':context.inspect_uri()+'/source/'+encode_context_identifier(source_id)})
    assert value['ok'],value
    assert bytes(value['items'][0]['original_bytes'])==original
    return value['items'][0]

async def checked_backfill(context):
    embedded=0
    for _ in range(64):
        report=await context.backfill(2,65536)
        assert report['unavailable'] is None,report
        fingerprint=report['registration']['fingerprint']
        assert fingerprint['dimensions']==384 and fingerprint['model'].endswith('bge-small-en-v1.5'),report
        assert fingerprint['revision']==hashlib.sha256(b'c5ac6c397e27c80e0229ec647987f2e553fc0ba9:Cosine:L2').hexdigest(),report
        assert report['usage'].startswith('unknown:'),report
        embedded+=report['embedded']
        if report['remaining']==0: return embedded
        assert report['embedded']>0,report
    raise AssertionError('bounded embedding backfill did not converge')

async def init(client,context):
    source=SourceMessage('precise-clock-reading',0,'user',({'kind':'text','text':'The inspection identifier is calibration-A7.'},),STAMP,STAMP,'user_asserted').freeze()
    receipt=await context.ingest_source(source,CLOCK_RAW)
    assert not receipt['replayed']
    await context.configure_embedding(True,0)
    log=ConversationLog(root/'sessions')
    log.append(HOST_SESSION,'user','The laboratory vessel coolant limit is 18 C.')
    log.append(HOST_SESSION,'user','Which component controls the heat of the laboratory vessel?')
    events=log.source_events(HOST_SESSION)
    original=events[0]
    adapter=GideonContextAdapter(context,log,HOST_SESSION,budget=BUDGET,provider=ProviderProfile('gpt-4o'),context_owner='hypermind',ordinal_offset=1)
    assert not adapter.owns_compaction
    adapter.install()
    try:
        first_bytes=b'The bench coolant limit is 18 C.'
        first_digest=hashlib.sha256(first_bytes).hexdigest()
        await context.memory('instrument-source-1',{'kind':'source','source':{'id':'coolant-source-1','digest':first_digest,'content':list(first_bytes),'locator':'instrument://bench/Cobalt-74','occurred_at_ns':STAMP,'recorded_at_ns':STAMP,'tombstoned':False}})
        record=make_memory_record('coolant-primer','primer',first_bytes.decode(),STAMP)
        record['pinned']=True
        record['provenance']=[{'source_id':'coolant-source-1','source_digest':first_digest,'span_start':0,'span_end':len(first_bytes),'quoted_digest':first_digest}]
        record=seal_memory_record(record)
        await context.memory('primer-create',{'kind':'create','record':record})
        anchor=make_memory_record('bench-anchor','anchor','The bench serial is Cobalt-74.',STAMP)
        await context.memory('anchor-create',{'kind':'create','record':anchor})
        document={'scope':asdict(SCOPE),'kind':'document','id':'jacket-guide','revision':1,'text':DOC,'content_digest':hashlib.sha256(DOC.encode()).hexdigest(),'authority':'external_observed','provenance':[{'source_id':'guide://thermal-jacket','source_digest':hashlib.sha256(DOC.encode()).hexdigest(),'byte_start':0,'byte_end':len(DOC.encode())}],'occurred_at_ns':None,'recorded_at_ns':STAMP,'expires_at_ns':None,'tombstoned':False}
        await context.register_retrieval_source(document,0)
        await prepare_context_turn(HOST_SESSION)
        assert get_engine() is adapter and adapter.owns_compaction
        assert any(block['required'] and block['text']==record['content'] for block in adapter.diagnostics['report']['blocks'])
        lexical=await context.activate('thermoregulation',BUDGET)
        assert any(item['candidate']['id']=='jacket-guide' for item in lexical['retrieval']['results']),lexical['retrieval']
        assert await checked_backfill(context)>0
        semantic=await context.activate('How is heat removed from the reaction container?',BUDGET)
        assert semantic['retrieval']['semantic']['state']=='available' and semantic['retrieval']['semantic']['scored']>0,semantic['retrieval']
        matching=[item for item in semantic['retrieval']['results'] if item['candidate']['id']=='jacket-guide']
        assert matching and matching[0]['semantic_score'] is not None,semantic['retrieval']
        vector=matching[0]['candidate']['vector']
        assert len(vector['values'])==384 and vector['content_digest']==document['content_digest']
        assert vector['fingerprint']['revision']==hashlib.sha256(b'c5ac6c397e27c80e0229ec647987f2e553fc0ba9:Cosine:L2').hexdigest()
        await prepare_context_turn(HOST_SESSION)
        journal=MemoryJournal(workspace=root/'journal');journal.init()
        builder=PromptAssembler(memory=journal,conversation_log=log)
        assembled=assemble_context(builder,events[-1].message['content'],is_new_session=False,session_key=HOST_SESSION,blocks_reads=True)
        assert any(component.source=='hypermind' for component in assembled.components)
        assert assembled.metadata['hypermind']['context_owner']=='hypermind'
        assert record['content'] in assembled.message
        first_fence=adapter.diagnostics['cache']['fence']
        second_bytes=b'The corrected bench coolant limit is 17 C.'
        second_digest=hashlib.sha256(second_bytes).hexdigest()
        await context.memory('instrument-source-2',{'kind':'source','source':{'id':'coolant-source-2','digest':second_digest,'content':list(second_bytes),'locator':'instrument://bench/Cobalt-74/correction','occurred_at_ns':STAMP,'recorded_at_ns':STAMP,'tombstoned':False}})
        revised=copy.deepcopy(record);revised.update(revision=2,content=second_bytes.decode(),provenance=[{'source_id':'coolant-source-2','source_digest':second_digest,'span_start':0,'span_end':len(second_bytes),'quoted_digest':second_digest}]);revised=seal_memory_record(revised)
        await context.memory('primer-revise',{'kind':'revise','record':revised,'expected_revision':1})
        await refused(context.memory('primer-stale-revision',{'kind':'revise','record':revised,'expected_revision':1}),{'kSequenceViolation','kIdempotencyConflict'})
        await prepare_context_turn(HOST_SESSION)
        assert record['content'] not in [block['text'] for block in adapter.diagnostics['report']['blocks']]
        assert any(block['required'] and block['text']==revised['content'] for block in adapter.diagnostics['report']['blocks'])
        assert adapter.diagnostics['cache']['fence']!=first_fence
        native_source=await context.inspect_memory('coolant-primer',source_id='coolant-source-2')
        assert bytes(native_source['source']['content'])==second_bytes and native_source['source']['recorded_at_ns']==STAMP
        denied_grant={'principal_digest':None,'id':'visitor-refused','principal':asdict(VISITOR),'record_ids':['bench-anchor'],'categories':[],'read':False,'expires_at_ns':None,'revoked':False,'revision':1,'record_revisions':{'bench-anchor':anchor['revision_digest']}}
        await context.memory('visitor-denied-grant',{'kind':'set_grant','grant':denied_grant})
        log.append(HOST_SESSION,'user','Correction: the laboratory vessel coolant limit is 17 C.')
        replacement=log.source_events(HOST_SESSION)[-1]
        await adapter.edit('coolant-host-correction',original.source_event_id,replacement.source_event_id)
        await context.memory('withdraw-instrument-source',{'kind':'tombstone_source','id':'coolant-source-2'})
        await prepare_context_turn(HOST_SESSION)
        state=adapter.diagnostics
        assert original.source_event_id not in state['report']['included']
        assert original.message['content'] not in json.dumps(state['messages'])
        assert revised['content'] not in [block['text'] for block in state['report']['blocks']]
        assert replacement.source_event_id in state['report']['included']
        assert state['cache']['fence']!=first_fence
        precise=await exact_source(client,context,source.id,CLOCK_RAW)
        assert precise['source']==source.wire() and precise['source']['recorded_at_ns']==STAMP
        await exact_source(client,context,original.source_event_id,original.raw_bytes)
        current=await history(client,context)
        source_wire=next(item for item in current['messages'] if item['id']==source.id)
        job=(await context.job('durable-verification-enqueue',{'action':'maintenance_enqueue','session_id':SESSION,'request':{'kind':'verification','sources':[source_wire],'cursor':current['cursor'],'source_revision':current['cursor']['sequence'],'policy_revision':1,'reservation':64}}))['items'][0]['result']
        artifact=await context.export_memory()
        assert artifact.data.endswith(b'\n') and hashlib.sha256(artifact.data).hexdigest()==artifact.artifact_digest
        assert len(artifact.data)<=artifact.restore_max_bytes
        (root/'memory.jsonl').write_bytes(artifact.data)
        metadata={'version':1,'scope':asdict(SCOPE),'cursor':artifact.cursor,'media_type':'application/x-ndjson','bytes':list(artifact.data),'byte_count':len(artifact.data),'artifact_digest':artifact.artifact_digest,'export_digest':artifact.export_digest,'restore_max_bytes':artifact.restore_max_bytes}
        (root/'memory-export.json').write_text(json.dumps(metadata))
        checkpoint={'adapter':adapter.checkpoint(),'context_digest':state['report']['digest'],'source':source.wire(),'original_event_id':original.source_event_id,'original_bytes':list(original.raw_bytes),'job_id':job,'anchor':anchor,'record_revision':revised['revision_digest'],'cursor':asdict(context.cursor)}
        (root/'checkpoint.json').write_text(json.dumps(checkpoint))
        print('actual public host: precise source/native revisions/required memory/lexical+pinned BGE semantic retrieval/correction/export/pending job passed')
    finally:
        adapter.uninstall();set_engine(None)

async def resume(client,context):
    saved=json.loads((root/'checkpoint.json').read_text())
    log=ConversationLog(root/'sessions')
    adapter=GideonContextAdapter.restore(context,log,saved['adapter'],budget=BUDGET,provider=ProviderProfile('gpt-4o'))
    adapter.install()
    try:
        resumed=await adapter.resume()
        assert resumed['report']['digest']==saved['context_digest'],(resumed['report'],saved)
        assert asdict(context.cursor)==saved['cursor']
        await prepare_context_turn(HOST_SESSION)
        assert adapter.owns_compaction and adapter.diagnostics['report']['digest']==saved['context_digest']
        precise=await exact_source(client,context,'precise-clock-reading',CLOCK_RAW)
        assert precise['source']==saved['source']
        await exact_source(client,context,saved['original_event_id'],bytes(saved['original_bytes']))
        state=(await context.job('durable-job-inspect',{'action':'inspect'}))['items'][0]
        assert state['maintenance']['jobs'][saved['job_id']]['status']=='Pending'
        lease=(await context.job('durable-job-claim',{'action':'maintenance_claim'}))['items'][0]['result']
        assert lease['job_id']==saved['job_id']
        await context.job('durable-job-cancel',{'action':'maintenance_cancel','id':lease['job_id']})
        state=(await context.job('durable-job-cancel-inspect',{'action':'inspect'}))['items'][0]
        assert state['maintenance']['unknown_usage'][lease['job_id']+':'+str(lease['attempt'])]==64
        memory=await context.inspect_memory()
        assert [record['id'] for record in memory['records']]==['bench-anchor']
        assert (await context.inspect_memory('bench-anchor'))['record']['revision_digest']==saved['anchor']['revision_digest']
        print('fresh actual public host/adapter after killed daemon: context/source/job/unknown usage/native memory recovered')
    finally:
        adapter.uninstall();set_engine(None)

async def restore(context,reopen):
    metadata=json.loads((root/'memory-export.json').read_text())
    artifact=decode_memory_export(metadata,SCOPE)
    assert artifact.data==(root/'memory.jsonl').read_bytes()
    if not reopen:
        assert (await context.inspect_memory())['records']==[]
        receipt=await context.restore_memory('canonical-jsonl-restore',artifact)
        assert receipt['last_lsn']>0
    records=(await context.inspect_memory())['records']
    saved=json.loads((root/'checkpoint.json').read_text())
    assert [record['id'] for record in records]==['bench-anchor']
    assert records[0]['revision_digest']==saved['anchor']['revision_digest'] and records[0]['recorded_at_ns']==STAMP
    exported=await context.export_memory()
    assert exported.data.endswith(b'\n') and hashlib.sha256(exported.data).hexdigest()==exported.artifact_digest
    restores=[event['command'] for event in exported.events if event['command']['kind']=='restore_jsonl']
    assert len(restores)==1
    restore_command=restores[0]
    assert bytes(restore_command['jsonl'])==artifact.data
    assert restore_command['artifact_digest']==artifact.artifact_digest
    restored_metadata=dict(metadata,bytes=restore_command['jsonl'],artifact_digest=restore_command['artifact_digest'])
    preserved=decode_memory_export(restored_metadata,SCOPE)
    assert preserved.data==artifact.data and preserved.events==artifact.events
    commands=[event['command'] for event in preserved.events]
    sources={command['source']['id']:command['source'] for command in commands if command['kind']=='source'}
    for source_id,raw in [('coolant-source-1',b'The bench coolant limit is 18 C.'),('coolant-source-2',b'The corrected bench coolant limit is 17 C.')]:
        source=sources[source_id]
        assert bytes(source['content'])==raw and source['digest']==hashlib.sha256(raw).hexdigest()
        assert source['recorded_at_ns']==STAMP and source['occurred_at_ns']==STAMP
    revisions=[command['record'] for command in commands if command['kind'] in ('create','revise') and command['record']['id']=='coolant-primer']
    assert [record['revision'] for record in revisions]==[1,2]
    assert revisions[1]['revision_digest']==saved['record_revision']
    for revision,source_id in zip(revisions,['coolant-source-1','coolant-source-2']):
        source=sources[source_id]
        assert revision['content']==bytes(source['content']).decode()
        assert revision['provenance']==[{'source_id':source_id,'source_digest':source['digest'],'span_start':0,'span_end':len(source['content']),'quoted_digest':source['digest']}]
    grants=[command['grant'] for command in commands if command['kind']=='set_grant']
    assert len(grants)==1 and grants[0]['id']=='visitor-refused' and grants[0]['read'] is False
    assert grants[0]['principal']==asdict(VISITOR) and grants[0]['record_revisions']=={'bench-anchor':saved['anchor']['revision_digest']}
    assert [command['id'] for command in commands if command['kind']=='tombstone_source']==['coolant-source-2']
    await refused(context.inspect_memory('coolant-primer'),{'kOperationUnavailable'})
    restored_context=context.client.context(SCOPE,'restored-host-context',actor=7,context_owner='hypermind',conversation='restored-host-conversation')
    log=ConversationLog(root/'restored-sessions')
    if not reopen:
        await context.configure_embedding(True,0)
        log.append('restored-host','user','What is the bench serial?')
        adapter=GideonContextAdapter(restored_context,log,'restored-host',budget=BUDGET,provider=ProviderProfile('gpt-4o'),context_owner='hypermind')
    else:
        host_state=json.loads((root/'restored-host.json').read_text())
        adapter=GideonContextAdapter.restore(restored_context,log,host_state['adapter'],budget=BUDGET,provider=ProviderProfile('gpt-4o'))
    adapter.install()
    try:
        if reopen:
            resumed=await adapter.resume()
            assert resumed['report']['digest']==host_state['digest']
        await prepare_context_turn('restored-host')
        assert adapter.owns_compaction
        assert any(block['required'] and block['text']==saved['anchor']['content'] for block in adapter.diagnostics['report']['blocks'])
        journal=MemoryJournal(workspace=root/'restored-journal');journal.init()
        builder=PromptAssembler(memory=journal,conversation_log=log)
        assembled=assemble_context(builder,'What is the bench serial?',is_new_session=False,session_key='restored-host',blocks_reads=True)
        assert saved['anchor']['content'] in assembled.message
        if not reopen:
            (root/'restored-host.json').write_text(json.dumps({'adapter':adapter.checkpoint(),'digest':adapter.diagnostics['report']['digest']}))
        else:
            assert adapter.diagnostics['report']['digest']==host_state['digest']
    finally:
        adapter.uninstall();set_engine(None)
    print('fresh root canonical raw JSONL '+('restore/reopen' if reopen else 'restore')+' preserves precise native revisions/provenance/grants/tombstones and applies required memory through the actual public host')

async def main():
    client=Client('localhost:'+port,token=bytes.fromhex('44'*32),ca=(root/'ca.pem').read_bytes(),certificate=(root/'client.pem').read_bytes(),private_key=(root/'client.key').read_bytes(),timeout=90)
    try:
        scope=VISITOR if phase=='grant-refusal' else SCOPE
        context=client.context(scope,SESSION,actor=7,context_owner='hypermind',conversation=CONVERSATION)
        if phase=='init': await init(client,context)
        elif phase=='grant-refusal':
            await refused(context.inspect_memory('bench-anchor',owner_scope=SCOPE),{'kCapabilityDenied'})
            print('actual authenticated recipient scope cannot read an explicitly denied native grant')
        elif phase=='resume': await resume(client,context)
        elif phase=='restore': await restore(context,False)
        elif phase=='restore-resume': await restore(context,True)
        else: raise ValueError('unknown journey phase')
    finally: await client.close()
asyncio.run(main())
"#;
