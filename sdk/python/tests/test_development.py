import asyncio
from dataclasses import asdict
import unittest

from hypermind.client import HyperMindError
from hypermind.context import Scope, SourceMessage, TokenBudget
from hypermind.development import DevelopmentClient, DevelopmentBudget, SnapshotRequest, ScheduleMode, ServiceSchedule, ProposalDecision
from test_fabric import ActualDaemon


class DevelopmentConsumerTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_owner_actions_schedule_and_restart(self):
        daemon = ActualDaemon(development=True)
        try:
            client = await daemon.start()
            development = DevelopmentClient(client, daemon.scope, actor=7)
            view = await development.inspect()
            self.assertEqual(view['scope'], asdict(daemon.scope))
            self.assertIn('retrospective', [w['id'] for w in view['runtime_workers']])
            self.assertFalse((await development.inspect_profile('profile-before'))['enabled'])
            context = client.context(daemon.scope, 'owner-session/%?', actor=7, context_owner='hypermind', conversation='owner-conversation')
            content = b'Prefer concise calibration reports.'
            source = SourceMessage('original', 0, 'user', ({'kind': 'text', 'text': content.decode()},), 1791287999123456789, 1791288000123456789, 'user_asserted').freeze()
            await context.ingest_source(source, content)
            await context.activate('calibration reports', TokenBudget(8192, 512, 0))
            with self.assertRaises(HyperMindError): await development.collect_profile_session('without-consent', context.session_id, context.conversation)
            consent = await development.set_profile_enabled('consent', True)
            self.assertEqual(consent['scope'], asdict(daemon.scope))
            profile_sources = await development.collect_profile_session('collect-profile', context.session_id, context.conversation)
            self.assertTrue(profile_sources)
            self.assertEqual(await development.collect_profile_session('collect-profile-again', context.session_id, context.conversation), profile_sources)
            self.assertTrue(await development.collect_primer_session('collect-primer', context.session_id, context.conversation))
            backlog = await development.enqueue_primer('primer-queue', 'primer-job', 'primer-target')
            self.assertGreater(backlog['cursor'], consent['cursor'])
            self.assertTrue((await development.inspect_primers('primer-inspect'))['pending'])
            budget = DevelopmentBudget(8192, 65536, 65536, 2)
            registered = await development.register_worker('register', worker_id='retrospective', capability_id='retrospective-capability', session_id=context.session_id,
                conversation=context.conversation, budget=budget, lease_ms=600000, new_record_ids=('lesson', 'checkpoint'))
            self.assertEqual(registered['capability']['scope'], asdict(daemon.scope))
            self.assertIsInstance(registered['capability']['lease']['expires_at_ns'], str)
            schedule = ServiceSchedule('manual-retrospective', ScheduleMode('manual'), 'retrospective', SnapshotRequest('retrospective-capability'), 8192, 60000)
            await development.configure_schedule('configure', schedule)
            queued = await development.enqueue('enqueue', schedule.id)
            state = await development.inspect()
            jobs = state['scheduler']['jobs'] if 'jobs' in state['scheduler'] else state['scheduler']['state']['jobs']
            job_id = next(iter(jobs))
            dispatched = await development.dispatch('dispatch', job_id)
            self.assertEqual(dispatched['jobs'][job_id]['status']['state'], 'complete')
            self.assertTrue(dispatched['jobs'][job_id]['status']['receipt_id'].startswith('no-work-'))
            with self.assertRaises(HyperMindError):
                await development.review('review-absent', ProposalDecision('absent-proposal', 1, '0' * 64, 'accept'))
            cancel_schedule = ServiceSchedule('manual-cancel', ScheduleMode('manual'), 'retrospective', SnapshotRequest('retrospective-capability'), 8192, 60000)
            await development.configure_schedule('configure-cancel', cancel_schedule)
            await development.enqueue('enqueue-cancel', cancel_schedule.id)
            state = await development.inspect()
            jobs = state['scheduler']['jobs'] if 'jobs' in state['scheduler'] else state['scheduler']['state']['jobs']
            pending_id = next(key for key, value in jobs.items() if value['status']['state'] == 'pending')
            await development.cancel('cancel-job', pending_id)
            await development.revoke_worker('revoke', 'retrospective-capability', registered['capability']['revision'])
            with self.assertRaises(HyperMindError):
                await DevelopmentClient(client, Scope('foreign', 'sdk-service'), actor=7).set_profile_enabled('foreign', True)
            await daemon.stop()
            client = await daemon.start(); development = DevelopmentClient(client, daemon.scope, actor=7)
            self.assertTrue((await development.inspect_profile('profile-reopen'))['enabled'])
            self.assertTrue((await development.inspect_primers('primer-reopen'))['pending'])
            restored = await development.inspect()
            self.assertTrue(restored['capabilities']['retrospective-capability']['revoked'])
            disabled = await development.set_profile_enabled('disable', False)
            self.assertGreater(disabled['cursor'], consent['cursor'])
            self.assertFalse((await development.inspect_profile('profile-disabled'))['enabled'])
        finally: await daemon.close()


if __name__ == '__main__': unittest.main()
