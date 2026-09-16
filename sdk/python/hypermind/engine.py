import asyncio
import json
from .bundle import parse_bundle
from .client import HyperMindError, conversation_id
from .session import Session


class Engine:
    """Async Python interface to the same in-process Rust actor and MCP tools."""
    def __init__(self, native):
        self._native = native

    @classmethod
    async def open(cls, path, *, actor: int, user_hex: str, kek_hex: str, projection_map_bytes=268_435_456, enable_providers=False):
        from ._native import NativeEngine
        native = await asyncio.to_thread(NativeEngine, str(path), actor, user_hex, kek_hex, projection_map_bytes, enable_providers)
        return cls(native)

    def session(self, conversation: str): return Session(self, conversation)

    async def tool(self, verb: str, arguments: dict):
        try:
            return json.loads(await asyncio.to_thread(self._native.call, verb, json.dumps(arguments, separators=(",", ":"))))
        except RuntimeError as error:
            raise HyperMindError("kOperationUnavailable", str(error), "unknown" if verb not in ("recall", "activate", "inspect") else None) from error

    async def activate(self, conversation: str, query: str, budget_tokens: int):
        raw = await asyncio.to_thread(self._native.activate, conversation_id(conversation).hex(), query, budget_tokens)
        return parse_bundle(bytes(raw))

    async def close(self): await asyncio.to_thread(self._native.close)
    async def __aenter__(self): return self
    async def __aexit__(self, *unused): await self.close()
