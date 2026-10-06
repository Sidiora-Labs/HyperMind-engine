from __future__ import annotations

from hashlib import sha256
from pathlib import Path
import ast
import os
import sys

INTEGRATION_VERSION = 1
SOURCES = {
    'gideon/cognition/context_engine.py': '58c68290aa41a1b761e1faedc546234956973ad2f7db7c14cc733318b13ec425',
    'gideon/interfaces/dashboard/chat_runner.py': '671f6cafa5477fcb7a624d8b6d71a5f30bc1d08b489484e1ce8b1e769268b717',
}


def _replace(text: str, before: str, after: str) -> str:
    if text.count(before) != 1:
        raise ValueError('unsupported host integration boundary')
    return text.replace(before, after, 1)


def install(runtime: str | Path, *, default_runtime: bool = False) -> dict[str, str]:
    root = Path(runtime).resolve(strict=True)
    if any(name == 'gideon' or name.startswith('gideon.') for name in sys.modules):
        raise RuntimeError('install continuity before importing the host')
    sources = dict(SOURCES)
    if default_runtime:
        sources['gideon/engine/agents/native/runtime.py'] = '58bf75cf1e13529e806341cd07252372d2a62d314ba1e1da2f86c46ee72d2d40'
        sources['gideon/extensions/apps/native/ollama-models/provider.py'] = '2d9537932fc26b290f2c688dd45f2a1016b6113a57cfdb528b56fe723a76d24e'
    originals = {}
    for relative, expected in sources.items():
        path = root / relative
        if path.is_symlink() or path.resolve(strict=True) != path:
            raise ValueError('host integration requires canonical regular files')
        data = path.read_bytes()
        if sha256(data).hexdigest() != expected:
            raise ValueError('host source version differs from reviewed integration')
        originals[relative] = data.decode('utf-8')
    engine = originals['gideon/cognition/context_engine.py']
    engine = _replace(engine, 'def assemble_context(\n', '''HYPERMIND_CONTINUITY_INTEGRATION = 1


def _fails_closed(engine):
    return bool(getattr(engine, "fail_closed", False))


async def assemble_context_pending(builder, text, *, is_new_session, **kwargs):
    engine = get_engine()
    pending = getattr(engine, "assemble_pending", None)
    if callable(pending):
        return await pending(builder, text, is_new_session=is_new_session, **kwargs)
    return assemble_context(builder, text, is_new_session=is_new_session, **kwargs)


async def validate_context_dispatch(session_key, provider_input, model_id, client):
    engine = get_engine()
    validate = getattr(engine, "validate_dispatch", None)
    if callable(validate):
        await validate(session_key, provider_input, model_id, client)


async def validate_context_model_dispatch(runtime, messages, tools):
    engine = get_engine()
    owner = getattr(runtime, "_hypermind_primary_owner", None)
    if owner is not None and engine is not owner:
        raise ContextBoundaryRefusal("native model lost installed continuity authority")
    validate = getattr(engine, "validate_model_dispatch", None)
    if callable(validate):
        try:
            await validate(runtime, messages, tools)
        except Exception as error:
            if isinstance(error, ContextBoundaryRefusal):
                raise
            raise ContextBoundaryRefusal("native model boundary refused") from error


async def validate_context_provider_dispatch(provider, payload):
    engine = get_engine()
    owner = getattr(provider, "_hypermind_primary_owner", None)
    if owner is not None and engine is not owner:
        raise ContextBoundaryRefusal("native provider lost installed continuity authority")
    validate = getattr(engine, "validate_provider_dispatch", None)
    if callable(validate):
        try:
            await validate(provider, payload)
        except Exception as error:
            if isinstance(error, ContextBoundaryRefusal):
                raise
            raise ContextBoundaryRefusal("native provider boundary refused") from error


def normalize_context_provider_messages(provider, messages):
    engine = get_engine()
    owner = getattr(provider, "_hypermind_primary_owner", None)
    if owner is not None and engine is not owner:
        raise ContextBoundaryRefusal("native serializer lost installed continuity authority")
    normalize = getattr(engine, "normalize_provider_messages", None)
    return normalize(provider, messages) if callable(normalize) else messages


async def cancel_context_turn(session_key):
    engine = get_engine()
    if getattr(engine, "session_key", None) != session_key:
        return
    cancel = getattr(engine, "cancel_turn", None)
    if callable(cancel):
        await cancel()


def assemble_context(
''')
    engine = _replace(engine, '    except Exception:\n        logger.warning(\n            "Context engine %r failed in assemble', '    except Exception:\n        if _fails_closed(engine):\n            raise\n        logger.warning(\n            "Context engine %r failed in assemble')
    runner = originals['gideon/interfaces/dashboard/chat_runner.py']
    runner = _replace(runner, '    assemble_context,\n', '    assemble_context,\n    assemble_context_pending,\n    validate_context_dispatch,\n    cancel_context_turn,\n')
    runner = _replace(runner, '            _assembled = assemble_context(\n', '            _assembled = await assemble_context_pending(\n')
    runner = _replace(runner, '        event_stream = (\n', '        await validate_context_dispatch(session_key, full_message, _bound_model_id(session, client), client)\n\n        event_stream = (\n')
    runner = _replace(runner, '    finally:\n        session._batch_rejected = False\n', '    finally:\n        try:\n            await cancel_context_turn(session_key)\n        except Exception:\n            logger.exception("Continuity cancellation remains unresolved")\n        session._batch_rejected = False\n')
    candidates = dict(zip(SOURCES, (engine, runner)))
    if default_runtime:
        relative = 'gideon/engine/agents/native/runtime.py'
        native = originals[relative]
        native = _replace(native, '                async for event in runtime._model.complete(\n', '                from gideon.cognition.context_engine import validate_context_model_dispatch\n                await validate_context_model_dispatch(runtime, request_messages, tools)\n                async for event in runtime._model.complete(\n')
        native = _replace(native, '            except Exception as error:\n                from gideon.integrations.tool_providers.portable_schema import (', '            except Exception as error:\n                from gideon.cognition.context_engine import ContextBoundaryRefusal\n                if isinstance(error, ContextBoundaryRefusal):\n                    raise\n                from gideon.integrations.tool_providers.portable_schema import (')
        candidates[relative] = native
        relative = 'gideon/extensions/apps/native/ollama-models/provider.py'
        provider = _replace(originals[relative], '        payload: dict[str, Any] = {\n', '        from gideon.cognition.context_engine import normalize_context_provider_messages\n        messages = normalize_context_provider_messages(self, messages)\n        payload: dict[str, Any] = {\n')
        provider = _replace(provider, '        await self.start()\n        assert self._client is not None\n        answering = False\n', '        from gideon.cognition.context_engine import validate_context_provider_dispatch\n        await validate_context_provider_dispatch(self, payload)\n        await self.start()\n        assert self._client is not None\n        answering = False\n')
        candidates[relative] = provider
        candidates['gideon/cognition/context_engine.py'] += '\nHYPERMIND_DEFAULT_RUNTIME_SHA256 = ' + repr(sha256(native.encode()).hexdigest()) + '\nHYPERMIND_DEFAULT_PROVIDER_SHA256 = ' + repr(sha256(provider.encode()).hexdigest()) + '\n'
    for relative, text in candidates.items():
        ast.parse(text, filename=relative)
    staged = []
    try:
        for relative, text in candidates.items():
            path = root / relative
            temporary = path.with_name(path.name + '.continuity-new')
            descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL, path.stat().st_mode & 0o777)
            with os.fdopen(descriptor, 'wb') as file:
                file.write(text.encode())
                file.flush()
                os.fsync(file.fileno())
            staged.append((temporary, path))
        for temporary, path in staged:
            os.replace(temporary, path)
    finally:
        for temporary, _ in staged:
            temporary.unlink(missing_ok=True)
    return {relative: sha256(text.encode()).hexdigest() for relative, text in candidates.items()}
