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


def install(runtime: str | Path) -> dict[str, str]:
    root = Path(runtime).resolve(strict=True)
    if any(name == 'gideon' or name.startswith('gideon.') for name in sys.modules):
        raise RuntimeError('install continuity before importing the host')
    originals = {}
    for relative, expected in SOURCES.items():
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
