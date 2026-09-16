"""Shared high-level API for embedded and remote engines."""
from .bundle import assert_rememberable


class Session:
    def __init__(self, backend, conversation: str):
        if not conversation: raise ValueError("conversation must be nonempty")
        self.backend, self.conversation = backend, conversation

    async def remember(self, content: str, *, kind="user", **options):
        assert_rememberable(content)
        return await self.backend.tool("remember", dict(conversation=self.conversation, content=content, kind=kind, **options))

    async def recall(self, query: str = "", *, mode="lexical", limit=32, **options):
        return await self.backend.tool("recall", dict(conversation=self.conversation, query=query, mode=mode, limit=limit, **options))

    async def activate(self, query: str = "", *, budget_tokens=4096):
        return await self.backend.activate(self.conversation, query, budget_tokens)

    async def attest(self, **arguments): return await self.backend.tool("attest", arguments)
    async def believe(self, **arguments): return await self.backend.tool("believe", arguments)
    async def intend(self, **arguments): return await self.backend.tool("intend", arguments)
    async def bind(self, **arguments): return await self.backend.tool("bind", arguments)
    async def predict(self, **arguments): return await self.backend.tool("predict", arguments)
    async def outcome(self, **arguments): return await self.backend.tool("outcome", arguments)
    async def consolidate(self, **arguments): return await self.backend.tool("consolidate", arguments)
    async def inspect(self, **arguments): return await self.backend.tool("inspect", arguments)
    async def retract(self, **arguments): return await self.backend.tool("retract", arguments)
    async def dispute(self, **arguments): return await self.backend.tool("dispute", arguments)
    async def forget(self, **arguments): return await self.backend.tool("forget", arguments)
