import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

from hypermind.gideon_continuity_install import install, SOURCES


class DefaultIntegrationInstallation(unittest.TestCase):
    def test_version_fenced_clean_default_host_native_journey(self):
        original = Path(os.environ['GIDEON_RUNTIME_PATH']).resolve()
        with tempfile.TemporaryDirectory() as directory:
            runtime = Path(directory) / 'runtime'
            shutil.copytree(original, runtime, symlinks=True)
            outputs = install(runtime, default_runtime=True)
            self.assertEqual(set(outputs), set(SOURCES) | {'gideon/engine/agents/native/runtime.py', 'gideon/extensions/apps/native/ollama-models/provider.py'})
            print('default_host_installed_hashes', outputs, flush=True)
            runner = (runtime / 'gideon/interfaces/dashboard/chat_runner.py').read_text()
            self.assertIn('await validate_context_dispatch(session_key, full_message, _bound_model_id(session, client), client)', runner)
            self.assertLess(runner.index('await validate_context_dispatch(session_key, full_message'), runner.index('        event_stream = ('))
            self.assertIn('await cancel_context_turn(session_key)', runner)
            with self.assertRaises(ValueError):
                install(runtime)
            environment = {**os.environ, 'GIDEON_RUNTIME_PATH': str(runtime), 'GIDEON_HOME': str(Path(directory) / 'home')}
            result = subprocess.run([sys.executable, str(Path(__file__).with_name('default_host_native_journey.py'))], env=environment, stdout=subprocess.PIPE,
                                    stderr=subprocess.STDOUT, timeout=540, check=False)
            self.assertEqual(result.returncode, 0, result.stdout.decode(errors='replace'))
            self.assertIn(b'actual_default_host_provider_tool_journey_passed', result.stdout)
            print(result.stdout.decode(errors='replace'))
