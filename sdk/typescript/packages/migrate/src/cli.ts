#!/usr/bin/env node
import { writeFileSync } from 'node:fs';
import { exportCortex, ExportOptions } from './index';

export function main(args: string[]): void {
  const options: Record<string, string> = {};
  while (args.length) {
    const flag = args.shift()!;
    if (!['--source', '--format', '--namespace', '--output'].includes(flag) || !args.length || flag in options) throw new Error(`Invalid argument ${flag}`);
    options[flag] = args.shift()!;
  }
  if (!options['--source'] || !['json', 'sqlite'].includes(options['--format'])) throw new Error('Usage: migrate-from-cortex --source PATH --format json|sqlite [--namespace NAME] [--output PATH]');
  const result = exportCortex({source: options['--source'], format: options['--format'] as ExportOptions['format'], namespace: options['--namespace']});
  if (options['--output']) writeFileSync(options['--output'], result.jsonl, {flag: 'wx', mode: 0o600});
  else process.stdout.write(result.jsonl);
}

if (require.main === module) {
  try { main(process.argv.slice(2)); }
  catch (error) { process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`); process.exitCode = 1; }
}
