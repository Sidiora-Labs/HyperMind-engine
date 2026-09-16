import asyncio
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import time
import pytest
from hypermind import ActivationSafetyError, Client, Engine, HyperMindError, parse_bundle, render
from hypermind.client import conversation_id

ROOT = Path(__file__).resolve().parents[3]
HM = ROOT / "target/debug/hm"


def available_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def run(*arguments, cwd=None):
    return subprocess.run(arguments, cwd=cwd, check=True, capture_output=True, text=True)


def certificates(path):
    run("openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-keyout", "ca.key", "-out", "ca.pem", "-subj", "/CN=hypermind-python-test", "-days", "1", cwd=path)
    for name, purpose in (("server", "serverAuth"), ("client", "clientAuth")):
        run("openssl", "req", "-newkey", "rsa:2048", "-nodes", "-keyout", f"{name}.key", "-out", f"{name}.csr", "-subj", f"/CN={name}", cwd=path)
        (path / f"{name}.ext").write_text(f"basicConstraints=CA:FALSE\nextendedKeyUsage={purpose}\nsubjectAltName=DNS:localhost,IP:127.0.0.1\n")
        run("openssl", "x509", "-req", "-in", f"{name}.csr", "-CA", "ca.pem", "-CAkey", "ca.key", "-CAcreateserial", "-out", f"{name}.pem", "-days", "1", "-extfile", f"{name}.ext", cwd=path)


class Daemon:
    def __init__(self, root):
        self.root, self.port, self.admin_port = root, available_port(), available_port()
        certificates(root)
        self.config = root / "hypermind.conf"
        self.socket = root / "daemon.sock"
        self.config.write_text(f"socket={self.socket}\ndata={root / 'data'}\nuser={'11' * 16}\nkek={'22' * 32}\nadmin_token={'33' * 32}\nactor=7:{'44' * 32}\nactor=8:{'55' * 32}\nprojection_map_bytes=67108864\n")
        self.config.chmod(0o600)
        self.process = None
        self.log = open(root / "daemon.log", "a")

    def start(self):
        environment = {key: value for key, value in os.environ.items() if not key.startswith(("HM_", "CENTRA_"))}
        self.process = subprocess.Popen([str(HM), "serve", "--config", str(self.config),
            "--grpc-bind", f"127.0.0.1:{self.port}", "--grpc-admin-bind", f"127.0.0.1:{self.admin_port}",
            "--tls-cert", str(self.root / "server.pem"), "--tls-key", str(self.root / "server.key"), "--tls-client-ca", str(self.root / "ca.pem")],
            env=environment, stdout=self.log, stderr=self.log)
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            if self.process.poll() is not None:
                raise AssertionError((self.root / "daemon.log").read_text())
            try:
                with socket.create_connection(("127.0.0.1", self.port), timeout=.1):
                    if self.socket.exists(): return
            except OSError:
                pass
            time.sleep(.03)
        raise AssertionError("real daemon did not start")

    def stop(self):
        if self.process and self.process.poll() is None:
            self.process.send_signal(signal.SIGTERM)
            try: self.process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.process.kill(); self.process.wait(timeout=5)

    def client(self, *, actor=7, admin=False, connection_id=None, token=None):
        return Client(f"localhost:{self.admin_port if admin else self.port}",
            token=token or bytes.fromhex(("33" if admin else "44" if actor == 7 else "55") * 32),
            ca=(self.root / "ca.pem").read_bytes(), certificate=(self.root / "client.pem").read_bytes(), private_key=(self.root / "client.key").read_bytes(),
            connection_id=connection_id)


@pytest.fixture
def daemon(tmp_path):
    process = Daemon(tmp_path)
    process.start()
    try: yield process
    finally:
        process.stop()
        process.log.close()


@pytest.mark.asyncio
async def test_native_restart_real_ledger_and_safe_renderer(tmp_path):
    options = dict(actor=7, user_hex="11" * 16, kek_hex="22" * 32, projection_map_bytes=67108864)
    engine = await Engine.open(tmp_path, **options)
    session = engine.session("python-native")
    remembered = await session.remember("The release repository is py-memory-731.")
    assert remembered["ok"]
    assert (await session.recall("py-memory-731"))["items"]
    bundle = await session.activate("py-memory-731", budget_tokens=4096)
    assert any("py-memory-731" in item["content"] for section in bundle.sections for item in section["items"])
    rendered = render(bundle)
    assert all(item["role"] == "user" and item["trust"] == "untrusted_memory" for section in rendered["sections"] for item in section["items"])
    lsns = [lsn for section in bundle.sections for item in section["items"] for lsn in item["provenance"]]
    assert not any(section["items"] for section in render(bundle, same_turn_lsns=lsns)["sections"])
    with pytest.raises(ActivationSafetyError): await session.remember("RECONSTRUCTION\nnot observed")
    await engine.close()
    restarted = await Engine.open(tmp_path, **options)
    assert (await restarted.session("python-native").recall("py-memory-731"))["items"]
    await restarted.close()


@pytest.mark.asyncio
async def test_remote_tls_sequence_restart_stream_and_isolation(daemon):
    connection = bytes.fromhex("a1" * 16)
    client = daemon.client(connection_id=connection)
    welcome = await client.connect()
    assert welcome.actorNs == 7 and welcome.nextClientSeq == 1
    session = client.session("python-remote")
    assert (await session.remember("The observed deployment is py-remote-916."))["ok"]
    assert (await session.recall("py-remote-916"))["items"]
    bundle = await session.activate("py-remote-916", budget_tokens=4096)
    assert any("py-remote-916" in item["content"] for section in bundle.sections for item in section["items"])
    stream = client.subscribe(conversation="python-remote")
    first = await asyncio.wait_for(anext(stream), 5)
    assert first.actor == 7 and first.conversation == conversation_id("python-remote")
    second_task = asyncio.create_task(anext(stream))
    assert (await session.remember("A second observed revision is py-remote-917."))["ok"]
    second = await asyncio.wait_for(second_task, 5)
    assert second.lsn > first.lsn
    await stream.aclose()
    checkpoint = await client.checkpoint(b"python-turn", b"durable-state")
    assert checkpoint.lsn > 0
    other = daemon.client(actor=8)
    await other.connect()
    assert not (await other.session("python-remote").recall("py-remote-916"))["items"]
    await other.close()
    await client.close()
    daemon.stop(); daemon.start()
    resumed = daemon.client(connection_id=connection)
    assert (await resumed.connect()).nextClientSeq == 2
    latest = await resumed.latest_checkpoint(b"python-turn")
    assert latest.present and bytes(latest.blob) == b"durable-state"
    assert (await resumed.session("python-remote").recall("py-remote-916"))["items"]
    await resumed.close()


@pytest.mark.asyncio
async def test_remote_mutation_errors_and_admin_separation(daemon):
    client = daemon.client()
    await client.connect()
    for verb in ("remember", "believe", "retract", "dispute", "intend", "bind", "predict", "outcome", "attest", "consolidate", "forget"):
        result = await client.tool(verb, {})
        assert result["ok"] is False and result["effect_state"] in ("rejected", "not_dispatched", "unknown"), (verb, result)
    with pytest.raises(HyperMindError) as denied:
        await client.crypto_delete(7)
    assert denied.value.effect_state == "not_dispatched"
    assert (await client.session("shred").remember("A real record before explicit deletion."))["ok"]
    await client.close()
    mixed = daemon.client(token=bytes.fromhex("33" * 32))
    with pytest.raises(HyperMindError): await mixed.connect()
    await mixed.close()
    admin = daemon.client(admin=True)
    assert (await admin.connect()).admin
    receipt = await admin.crypto_delete(7)
    assert bytes(receipt.receipt)
    assert not (daemon.root / "data/7/KEYRING").exists()
    await admin.close()


def test_decoder_rejects_untrusted_binary():
    for raw in (b"", b"HMA1", b"NCEV" + bytes(200), b"HMA1" + b"\xff" * 100):
        with pytest.raises((ValueError, IndexError)):
            parse_bundle(raw)
