import asyncio
from hashlib import sha256
import importlib.util
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
        providers = []
        def factory(key, **options):
            provider = provider_module.OllamaProvider({'endpoint': endpoint, 'model': options.get('model') or model,
                                                       'context_window': 16384, 'timeout_secs': 180,
                                                       'options': {'num_predict': 16, 'temperature': 0}})
            providers.append(provider)
            return provider
        config = AppConfig()
        config.session.pool_size = 0
        sessions = ConversationDirectory(config, provider_factory=factory)
        state = ConsoleState(sessions, time.time(), context_builder=builder, conversation_log=log, owner_id='owner')
        session = state.get_or_create_session('primary', model=model, memory_mode='temporary', workspace_dir=str(root))
        key = _history_key_for(session.key)
        prompt = 'The observed alloy is copper-29. Reply with its identifier only.'
        log.append(key, 'user', prompt)
        original = log.source_events(key)[0]
        session.append('user', prompt, 'msg msg-u')
        context = ContextClient(engine, scope, 'primary-context', actor=7, context_owner='hypermind', conversation=key)
        budget = TokenBudget(16384, 256, 1000)
        await context.activate('Register the explicitly selected native context.', budget, options={'model_id': model})
        adapter = GideonContinuityAdapter(context, log, key, budget=budget, provider=ProviderProfile(model), context_owner='hypermind')
        adapter.install()
        adapter.queue_mode('primary')
        started = time.monotonic()
        try:
            await asyncio.wait_for(run_chat(state, session, prompt), timeout=240)
            assert get_engine() is adapter
            assert not session._last_turn_errored, {'messages': session.messages, 'raw': adapter.final_input_count, 'framed': adapter.final_chat_count}
            answers = [message for message in session.messages if message.get('role') == 'assistant' and message.get('content')]
            assert answers, session.messages
            assert adapter.final_input_count is not None
            assert adapter.final_chat_count is not None
            assert adapter.final_input_count['tokens'] <= adapter.final_chat_count['tokens'] <= budget.available
            assert adapter._ticket is None and not adapter._turn_active
            recovered = context._check_envelope(await engine.tool('inspect', {'uri': context.inspect_uri() + '/source/' + encode_context_identifier(original.source_event_id)}))
            assert bytes(recovered['items'][0]['original_bytes']) == original.raw_bytes
            assert any(event.message.get('content') == prompt for event in log.source_events(key))
            assert sha256(original.raw_bytes).hexdigest() == original.source_digest
            native = await adapter.continuity.call({'action': 'inspect'}, generation=await adapter.continuity.generation())
            assert native['policy']['mode'] == 'primary' and native['active_hook'] is None
            print('actual_primary_host_provider_journey_passed', adapter.final_input_count['tokens'], adapter.final_chat_count['tokens'], round(time.monotonic() - started, 3))
        finally:
            adapter.uninstall()
            set_engine(None)
            await sessions.close_all()
            await engine.close()
            for task in list(state._background_tasks):
                task.cancel()
            if state._background_tasks:
                await asyncio.gather(*state._background_tasks, return_exceptions=True)


asyncio.run(journey())
