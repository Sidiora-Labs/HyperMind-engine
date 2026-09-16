#!/usr/bin/env node
import { appendFileSync } from 'node:fs';

function requireValue(name, pattern) {
  const value = process.env[name] ?? '';
  if (!value || (pattern && !pattern.test(value))) throw new Error(`${name} is missing or invalid`);
  return value;
}

async function main() {
  const runId = requireValue('EVIDENCE_RUN_ID', /^[1-9][0-9]{0,15}$/);
  if (!Number.isSafeInteger(Number(runId))) throw new Error('EVIDENCE_RUN_ID is outside the safe integer range');
  const expectedDigest = requireValue('EVIDENCE_SHA256', /^[a-fA-F0-9]{64}$/).toLowerCase();
  const repository = requireValue('GITHUB_REPOSITORY', /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/);
  const defaultBranch = requireValue('DEFAULT_BRANCH', /^[^\x00-\x20\x7f]+$/);
  const token = requireValue('GH_TOKEN');
  const base = `https://api.github.com/repos/${repository}`;
  async function get(suffix) {
    const response = await fetch(`${base}${suffix}`, {
      headers: { Accept: 'application/vnd.github+json', Authorization: `Bearer ${token}`,
        'X-GitHub-Api-Version': '2022-11-28', 'User-Agent': 'hypermind-evidence-verifier' },
      redirect: 'error', signal: AbortSignal.timeout(20_000),
    });
    if (!response.ok) throw new Error(`GitHub evidence metadata request failed with HTTP ${response.status}`);
    const body = await response.text();
    if (body.length > 8_000_000) throw new Error('GitHub evidence metadata exceeded the size limit');
    return JSON.parse(body);
  }
  const project = await get('');
  if (project.full_name?.toLowerCase() !== repository.toLowerCase() || project.default_branch !== defaultBranch) {
    throw new Error('Configured repository/default branch does not match GitHub metadata');
  }
  const run = await get(`/actions/runs/${runId}`);
  if (String(run.id) !== runId || run.status !== 'completed' || run.conclusion !== 'success') {
    throw new Error('Evidence producer is not the requested successful completed run');
  }
  if (!['push', 'workflow_dispatch'].includes(run.event) || run.head_branch !== defaultBranch) {
    throw new Error('Evidence producer must be a default-branch push or manual run');
  }
  if (run.repository?.id !== project.id || run.head_repository?.id !== project.id
      || run.repository?.full_name?.toLowerCase() !== repository.toLowerCase()
      || run.head_repository?.full_name?.toLowerCase() !== repository.toLowerCase()
      || !/^[a-f0-9]{40}$/.test(run.head_sha ?? '')) {
    throw new Error('Evidence producer is not a same-repository commit');
  }
  const artifacts = [];
  let total;
  for (let page = 1; page <= 10; page += 1) {
    const result = await get(`/actions/runs/${runId}/artifacts?per_page=100&page=${page}`);
    if (!Number.isSafeInteger(result.total_count) || result.total_count < 0 || result.total_count > 1000
        || !Array.isArray(result.artifacts) || result.artifacts.length > 100) {
      throw new Error('Invalid or oversized artifact inventory');
    }
    if (total !== undefined && result.total_count !== total) throw new Error('Artifact inventory changed during verification');
    total = result.total_count;
    artifacts.push(...result.artifacts);
    if (artifacts.length >= total) break;
    if (result.artifacts.length === 0) throw new Error('Incomplete artifact inventory');
  }
  if (artifacts.length !== total || new Set(artifacts.map((item) => item.id)).size !== total) {
    throw new Error('Incomplete or duplicate artifact inventory');
  }
  const matching = artifacts.filter((item) => item.name === 'slice7-public-evidence' && item.expired === false);
  if (matching.length !== 1) throw new Error('Expected exactly one unexpired slice7-public-evidence artifact');
  const artifact = matching[0];
  if (!Number.isSafeInteger(artifact.id) || artifact.id <= 0
      || !Number.isFinite(Date.parse(artifact.expires_at)) || Date.parse(artifact.expires_at) <= Date.now()
      || artifact.digest?.toLowerCase() !== `sha256:${expectedDigest}`) {
    throw new Error('Evidence artifact is expired or does not match the reviewed SHA-256 pin');
  }
  const producer = artifact.workflow_run;
  if (String(producer?.id) !== runId || producer.repository_id !== project.id
      || producer.head_repository_id !== project.id || producer.head_branch !== defaultBranch
      || producer.head_sha !== run.head_sha) {
    throw new Error('Evidence artifact provenance does not match the verified producer');
  }
  if (process.env.GITHUB_OUTPUT) {
    appendFileSync(process.env.GITHUB_OUTPUT,
      `artifact-id=${artifact.id}\nartifact-digest=sha256:${expectedDigest}\nproducer-sha=${run.head_sha}\n`);
  }
  console.log(JSON.stringify({ verified: true, run_id: runId, artifact_id: artifact.id,
    digest: `sha256:${expectedDigest}`, producer_sha: run.head_sha }));
}

main().catch((error) => {
  const token = process.env.GH_TOKEN;
  const message = error instanceof Error ? error.message : 'Evidence verification failed';
  console.error(token ? message.split(token).join('[REDACTED]') : message);
  process.exitCode = 1;
});
