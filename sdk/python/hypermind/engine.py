import asyncio
import json
from dataclasses import asdict
from .context import ContextClient, Scope, ContextOwner
from .bundle import parse_bundle
from .client import HyperMindError, conversation_id
from .session import Session


class Engine:
    """Async Python interface to the same in-process Rust actor and MCP tools."""
    def __init__(self, native):
        self._native = native

    @classmethod
    async def open(cls, path, *, actor: int, user_hex: str, kek_hex: str, projection_map_bytes=268_435_456, enable_providers=False, context_scope: Scope | None = None):
        from ._native import NativeEngine
        native = await asyncio.to_thread(NativeEngine, str(path), actor, user_hex, kek_hex, projection_map_bytes, enable_providers, json.dumps({"version": 1, "actor": actor, "scope": asdict(context_scope)}) if context_scope is not None else None)
        return cls(native)

    def session(self, conversation: str): return Session(self, conversation)

    def context(self, scope: Scope, session_id: str, *, actor: int, context_owner: ContextOwner, conversation: str | None = None) -> ContextClient:
        return ContextClient(self, scope, session_id, actor=actor, context_owner=context_owner, conversation=conversation)


    async def tokenizer_identity(self, model_id: str):
        return json.loads(await asyncio.to_thread(self._native.tokenizer_identity, model_id))

    async def count_provider_input(self, model_id: str, expected_generation: int, final_input: bytes):
        return json.loads(await asyncio.to_thread(self._native.count_provider_input, model_id, expected_generation, bytes(final_input)))

    async def count_ollama_user_input(self, model_id: str, expected_generation: int, content: str):
        return json.loads(await asyncio.to_thread(self._native.count_ollama_user_input, model_id, expected_generation, content))

    async def count_ollama_chat_input(self, model_id: str, expected_generation: int, payload: bytes, budget: dict):
        return json.loads(await asyncio.to_thread(self._native.count_ollama_chat_input, model_id, expected_generation, bytes(payload), json.dumps(budget, separators=(",", ":"))))

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
