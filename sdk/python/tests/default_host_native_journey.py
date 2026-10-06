import asyncio
from hashlib import sha256
import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import time

runtime = Path(os.environ['GIDEON_RUNTIME_PATH'])
sys.path.insert(0, str(runtime))
from hypermind import Engine
spec = importlib.util.spec_from_file_location('hypermind._native', os.environ['HYPERMIND_NATIVE_LIBRARY'])
module = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = module
spec.loader.exec_module(module)
from hypermind.context import ContextClient, Scope, TokenBudget, encode_context_identifier
from hypermind.continuity_host import GideonContinuityAdapter
from hypermind.gideon import ProviderProfile
from gideon.cognition.context import PromptAssembler
from gideon.cognition.context_engine import get_engine, set_engine
from gideon.cognition.history import ConversationLog
from gideon.cognition.memory import MemoryJournal
from gideon.core.config.loader import AppConfig
from gideon.extensions.providers.provider_bridge import create_provider_factory
from gideon.integrations.llm.registry import ProviderEntry, get_default_registry
from gideon.integrations.llm.capabilities import Capability
from gideon.extensions.providers.use_cases import save_active_models
from gideon.security.approval_answer import YOU
from gideon.operations.stats import Stats
from gideon.engine.hooks import ScriptHookStore, set_global_hook_store
from gideon.engine.session import ConversationDirectory
from gideon.interfaces.dashboard.state import ConsoleState
from gideon.interfaces.dashboard.chat_runner import run_chat
from gideon.interfaces.dashboard.chat_utils import _history_key_for

provider_spec = importlib.util.spec_from_file_location('continuity_ollama_provider', runtime / 'gideon/extensions/apps/native/ollama-models/provider.py')
provider_module = importlib.util.module_from_spec(provider_spec)
sys.modules[provider_spec.name] = provider_module
provider_spec.loader.exec_module(provider_module)


async def journey():
    model = os.environ.get('HM_PRIMARY_MODEL', 'qwen2.5:3b')
    endpoint = os.environ['HM_OLLAMA_BASE_URL']
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        os.environ['GIDEON_HOME'] = str(root / 'home')
        scope = Scope('owner', 'primary-host')
        engine = await Engine.open(root / 'engine', actor=7, user_hex='11' * 16, kek_hex='22' * 32,
                                   projection_map_bytes=67108864, context_scope=scope)
        log = ConversationLog(root / 'sessions')
        memory = MemoryJournal(workspace=root / 'memory')
        memory.init()
        builder = PromptAssembler(memory=memory, conversation_log=log)
        config = AppConfig()
        config.session.pool_size = 0
        config.agent.approval_mode = 'interactive'
        config.save()
        get_default_registry().register_entry(ProviderEntry('local', 'ollama', model,
            options={'endpoint': endpoint, 'context_window': 16384, 'timeout_secs': 180,
                     'options': {'num_predict': 256, 'temperature': 0}},
            declared_capabilities=frozenset({Capability.CHAT, Capability.CODE_TOOLS})))
        save_active_models({'chat': ['local:' + model]})
        sessions = ConversationDirectory(config, provider_factory=create_provider_factory())
        state = ConsoleState(sessions, time.time(), context_builder=builder, conversation_log=log, owner_id='owner')
        state._hook_store = ScriptHookStore()
        set_global_hook_store(state._hook_store)
        session = state.get_or_create_session('primary', model=model, memory_mode='temporary', workspace_dir=str(root))
        key = _history_key_for(session.key)
        target = root / 'observation.txt'
        target.write_text('The observed identifier is copper-29.\n')
        output = root / 'confirmed-identifier.txt'
        prompt = 'First call read_file only, to read ' + str(target) + '. After the read_file result arrives, call write_file only, writing the exact observed identifier with no newline into ' + str(output) + '. Do not call grep or bash. Submit write_file to the real approval UI rather than asking in prose. After the write succeeds, reply with only the identifier. Do not guess the identifier.'
        log.append(key, 'user', prompt)
        original = log.source_events(key)[0]
        session.append('user', prompt, 'msg msg-u')
        context = ContextClient(engine, scope, 'primary-context', actor=7, context_owner='hypermind', conversation=key)
        budget = TokenBudget(16384, 256, 1000)
        await context.activate('Register the explicitly selected native context.', budget, options={'model_id': model})
        adapter = GideonContinuityAdapter(context, log, key, budget=budget, provider=ProviderProfile(model), context_owner='hypermind')
        adapter.install()
        adapter.queue_mode('primary')
        approvals = []
        async def approve_owner_requests():
            while True:
                for request_id in list(session._approval_futures):
                    if request_id not in approvals:
                        permission = next((message for message in reversed(session.messages) if message.get('role') == 'permission' and json.loads(message.get('cls', '{}')).get('request_id') == request_id), None)
                        if permission is None:
                            continue
                        metadata = json.loads(permission['cls'])
                        arguments = json.loads(metadata.get('tool_input') or '{}')
                        allowed = permission.get('content') == 'write_file' and arguments.get('path') == str(output) and arguments.get('content') == 'copper-29'
                        resolved = state.resolve_session_approval(session, request_id, 'approved' if allowed else 'rejected', by=YOU)
                        if resolved and allowed:
                            approvals.append(request_id)
                await asyncio.sleep(0.05)
        approver = asyncio.create_task(approve_owner_requests())
        tokens_before = Stats().snapshot()['input_tokens']
        started = time.monotonic()
        try:
            await asyncio.wait_for(run_chat(state, session, prompt), timeout=480)
            assert get_engine() is adapter
            assert type(sessions._sessions[key].provider).__name__ == 'NativeAgentRuntime'
            assert type(sessions._sessions[key].provider._model).__name__ == 'OllamaProvider'
            assert not session._last_turn_errored, {'messages': session.messages, 'raw': adapter.final_input_count, 'framed': adapter.final_chat_count}
            answers = [message for message in session.messages if message.get('role') == 'assistant' and message.get('content')]
            assert answers, session.messages
            assert adapter.final_chat_count is not None
            assert len(adapter.provider_dispatches) >= 2, adapter.provider_dispatches
            assert all(measurement['tokens'] <= budget.available for measurement in adapter.provider_dispatches)
            assert Stats().snapshot()['input_tokens'] - tokens_before == sum(measurement['tokens'] for measurement in adapter.provider_dispatches)
            assert adapter.provider_dispatches[-1]['tool_occurrences'], adapter.provider_dispatches
            assert approvals, session.messages
            assert output.read_text() == 'copper-29'
            assert any(message.get('role') == 'permission' and json.loads(message.get('cls', '{}')).get('resolved') == 'approved' for message in session.messages)
            assert any('copper-29' in message.get('content', '') for message in answers)
            assert any(message.get('role') == 'tool' for message in session.messages), session.messages
            assert adapter._ticket is None and not adapter._turn_active
            recovered = context._check_envelope(await engine.tool('inspect', {'uri': context.inspect_uri() + '/source/' + encode_context_identifier(original.source_event_id)}))
            assert bytes(recovered['items'][0]['original_bytes']) == original.raw_bytes
            assert any(event.message.get('content') == prompt for event in log.source_events(key))
            assert sha256(original.raw_bytes).hexdigest() == original.source_digest
            native = await adapter.continuity.call({'action': 'inspect'}, generation=await adapter.continuity.generation())
            assert native['policy']['mode'] == 'primary' and native['active_hook'] is None
            print('actual_default_host_provider_tool_journey_passed', [item['tokens'] for item in adapter.provider_dispatches], len(approvals), round(time.monotonic() - started, 3))
        finally:
            approver.cancel()
            await asyncio.gather(approver, return_exceptions=True)
            adapter.uninstall()
            set_engine(None)
            await sessions.close_all()
            await engine.close()
            for task in list(state._background_tasks):
                task.cancel()
            if state._background_tasks:
                await asyncio.gather(*state._background_tasks, return_exceptions=True)


asyncio.run(journey())
