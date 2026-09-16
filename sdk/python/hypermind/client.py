"""Async-first, mTLS-only transport for the frozen NCPR v3 envelope."""
import asyncio
import hashlib
import json
import secrets
import struct
from dataclasses import dataclass
import flatbuffers
import grpc
from . import hypermind_pb2 as pb
from .hypermind_pb2_grpc import HyperMindStub
from ._errors import CODES
from .bundle import parse_bundle
from .protocol.WireEnvelope import WireEnvelope, WireEnvelopeT
from .protocol.WirePayload import WirePayload
from .protocol.Request import RequestT
from .protocol.RequestPayload import RequestPayload
from .protocol.ResponsePayload import ResponsePayload
from .protocol.Hello import HelloT
from .protocol.Activate import ActivateT
from .protocol.Checkpoint import CheckpointT
from .protocol.LatestCheckpoint import LatestCheckpointT
from .protocol.ToolRequest import ToolRequestT
from .protocol.Subscribe import SubscribeT
from .protocol.CryptoDelete import CryptoDeleteT
from .protocol.AsOf import AsOfT


class HyperMindError(RuntimeError):
    def __init__(self, code: str, message: str = "", effect_state: str | None = None):
        super().__init__(f"{code}: {message}" if message else code)
        self.code, self.effect_state = code, effect_state


def conversation_id(name: str) -> bytes:
    if not name: raise ValueError("conversation must be nonempty")
    return hashlib.sha256(b"neocortex-conversation-v1\0" + name.encode()).digest()[:16]


def _pack(payload, kind: int) -> bytes:
    envelope = WireEnvelopeT()
    envelope.protoVersion, envelope.payloadType, envelope.payload = 3, kind, payload
    builder = flatbuffers.Builder(512)
    offset = envelope.Pack(builder)
    builder.Finish(offset, file_identifier=b"NCPR")
    return bytes(builder.Output())


def _decode(data: bytes):
    if len(data) < 8 or len(data) > 17 * 1024 * 1024 or data[4:8] != b"NCPR":
        raise HyperMindError("kProtocolInvalid", "invalid envelope size or identifier")
    try:
        value = WireEnvelopeT.InitFromObj(WireEnvelope.GetRootAs(data))
        if value.protoVersion not in (2, 3) or value.payload is None:
            raise ValueError("unsupported protocol or missing payload")
        return value
    except (ValueError, IndexError, TypeError, OverflowError, struct.error) as error:
        raise HyperMindError("kProtocolInvalid", str(error)) from error


def _response(data: bytes, request_id: int):
    envelope = _decode(data)
    response = envelope.payload
    if envelope.payloadType != WirePayload.Response or response.requestId != request_id:
        raise HyperMindError("kProtocolInvalid", "response identity mismatch")
    if response.status != 0:
        if response.payloadType != ResponsePayload.ErrorDetail or response.payload is None:
            raise HyperMindError("kProtocolInvalid", "missing error detail")
        error = response.payload
        effect = (None, "not_dispatched", "unknown", "rejected")
        raise HyperMindError(CODES.get(error.code, "kProtocolInvalid"), effect_state=effect[error.effectState] if error.effectState < len(effect) else "unknown")
    if response.payload is None:
        raise HyperMindError("kProtocolInvalid", "missing successful payload")
    return response.payload


@dataclass(frozen=True)
class Event:
    lsn: int
    kind: int
    actor: int
    conversation: bytes
    payload: bytes


class Client:
    def __init__(self, target: str, *, token: bytes, ca: bytes, certificate: bytes, private_key: bytes,
                 connection_id: bytes | None = None, timeout: float = 30):
        if len(token) != 32 or not all((ca, certificate, private_key)):
            raise ValueError("32-byte capability and all mTLS credentials are required")
        self.connection_id = secrets.token_bytes(16) if connection_id is None else bytes(connection_id)
        if len(self.connection_id) != 16: raise ValueError("connection_id must be 16 bytes")
        credentials = grpc.ssl_channel_credentials(ca, private_key, certificate)
        self._channel = grpc.aio.secure_channel(target, credentials, options=[("grpc.max_receive_message_length", 17 * 1024 * 1024)])
        self._stub, self._timeout = HyperMindStub(self._channel), timeout
        hello = HelloT()
        hello.protoVersion, hello.connectionId, hello.capabilityToken = 3, self.connection_id, bytes(token)
        self._hello = _pack(hello, WirePayload.Hello)
        self._lock, self._welcome, self._next_id, self._seq, self._pending = asyncio.Lock(), None, 1, 1, None

    @staticmethod
    def _transport_error(error, mutation=False):
        metadata = {key: value for key, value in (error.trailing_metadata() or ())}
        effect = metadata.get("effect-state")
        if mutation and effect not in ("not_dispatched", "rejected"):
            effect = "unknown"
        return HyperMindError("kOperationUnavailable", error.code().name, effect)

    async def _connect(self):
        try:
            result = await self._stub.Connect(pb.Envelope(ncpr=self._hello), timeout=self._timeout)
        except grpc.aio.AioRpcError as error:
            raise self._transport_error(error) from error
        envelope = _decode(result.ncpr)
        if envelope.payloadType != WirePayload.Welcome:
            raise HyperMindError("kProtocolInvalid", "expected Welcome")
        self._welcome = envelope.payload
        if self._pending is None:
            self._seq = self._welcome.nextClientSeq
        elif self._welcome.nextClientSeq not in (self._seq, self._seq + 1):
            raise HyperMindError("kSequenceViolation", "pending mutation cannot be reconciled", "unknown")
        return self._welcome

    async def connect(self):
        async with self._lock: return await self._connect()

    async def close(self):
        await self._channel.close()

    async def __aenter__(self):
        await self.connect()
        return self

    async def __aexit__(self, *unused): await self.close()

    def session(self, conversation: str):
        from .session import Session
        return Session(self, conversation)

    def _request_bytes(self, payload):
        request = RequestT()
        request.requestId = self._next_id
        self._next_id += 1
        request.payloadType = getattr(RequestPayload, type(payload).__name__.removesuffix("T"))
        request.payload = payload
        return request.requestId, _pack(request, WirePayload.Request)

    async def _exchange(self, request_id, data, mutation):
        try:
            result = await self._stub.Exchange(pb.ExchangeRequest(hello=self._hello, request=data), timeout=self._timeout)
        except grpc.aio.AioRpcError as error:
            self._welcome = None
            raise self._transport_error(error, mutation) from error
        return _response(result.ncpr, request_id)

    async def request(self, payload, *, mutation=False, sequenced=False):
        async with self._lock:
            if self._pending is not None:
                raise HyperMindError("kSequenceViolation", "recover the pending mutation before another request", "unknown")
            if self._welcome is None: await self._connect()
            if sequenced: payload.clientSeq = self._seq
            request_id, data = self._request_bytes(payload)
            if sequenced: self._pending = (request_id, data)
            try:
                response = await self._exchange(request_id, data, mutation or sequenced)
            except HyperMindError as error:
                if error.effect_state in ("rejected", "not_dispatched"): self._pending = None
                raise
            if sequenced:
                self._seq += 1
                self._pending = None
            return response

    async def recover(self):
        async with self._lock:
            if self._pending is None: raise ValueError("no pending mutation")
            await self._connect()
            request_id, data = self._pending
            response = await self._exchange(request_id, data, True)
            self._seq += 1
            self._pending = None
            return response

    async def tool(self, verb: str, arguments: dict):
        payload = ToolRequestT()
        payload.verb, payload.argumentsJson = verb, json.dumps(arguments, separators=(",", ":")).encode()
        response = await self.request(payload, mutation=verb not in ("recall", "activate", "inspect"))
        if not hasattr(response, "bytes"): raise HyperMindError("kProtocolInvalid", "expected tool envelope")
        return json.loads(bytes(response.bytes))

    async def activate(self, conversation: str, query: str, budget_tokens: int):
        payload = ActivateT()
        payload.conversation, payload.query, payload.turnText = conversation_id(conversation), query.encode(), b""
        payload.budgetTokens, payload.tokenWeights, payload.tokenItemOverhead = budget_tokens, [256] * 256, 0
        result = await self.request(payload)
        return parse_bundle(bytes(result.bytes))

    async def checkpoint(self, turn_id: bytes, blob: bytes):
        payload = CheckpointT()
        payload.turnId, payload.blob = turn_id, blob
        return await self.request(payload, mutation=True, sequenced=True)

    async def latest_checkpoint(self, turn_id: bytes):
        payload = LatestCheckpointT(); payload.turnId = turn_id
        return await self.request(payload)

    async def as_of(self, belief_type: int, identity: str, *, valid_at_ns=0, known_at_lsn=0):
        if bool(valid_at_ns) == bool(known_at_lsn): raise ValueError("select exactly one temporal axis")
        payload = AsOfT()
        payload.beliefType, payload.canonicalIdentity = belief_type, identity
        payload.validTimeNs, payload.knownLsn = valid_at_ns, known_at_lsn
        return await self.request(payload)

    async def crypto_delete(self, actor: int):
        payload = CryptoDeleteT(); payload.actor = actor
        return await self.request(payload, mutation=True)

    async def subscribe(self, *, conversation: str | None = None, since_lsn: int = 0):
        async with self._lock:
            if self._welcome is None: await self._connect()
            payload = SubscribeT()
            payload.conversation = conversation_id(conversation) if conversation is not None else None
            payload.sinceLsn = since_lsn
            request_id, data = self._request_bytes(payload)
        call = self._stub.Subscribe(pb.ExchangeRequest(hello=self._hello, request=data))
        try:
            acknowledgment = await call.read()
            if acknowledgment is grpc.aio.EOF: raise HyperMindError("kProtocolInvalid", "missing subscription ack")
            subscription = _response(acknowledgment.ncpr, request_id)
            if not hasattr(subscription, "subscriptionId"): raise HyperMindError("kProtocolInvalid", "invalid subscription ack")
            last_lsn = since_lsn
            while True:
                message = await call.read()
                if message is grpc.aio.EOF: return
                envelope = _decode(message.ncpr)
                event = envelope.payload
                if envelope.payloadType != WirePayload.Event or event.subscriptionId != subscription.subscriptionId or event.lsn <= last_lsn:
                    raise HyperMindError("kProtocolInvalid", "invalid subscription event")
                last_lsn = event.lsn
                yield Event(event.lsn, event.kind, event.actor, bytes(event.conversation), bytes(event.payload))
        except grpc.aio.AioRpcError as error:
            raise self._transport_error(error) from error
        finally:
            call.cancel()
