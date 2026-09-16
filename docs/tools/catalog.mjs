import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const mode = process.argv[2];
if (!['--write', '--check'].includes(mode)) throw new Error('usage: node docs/tools/catalog.mjs --write|--check');
const read = file => fs.readFileSync(path.join(root, file), 'utf8');
const exists = file => fs.existsSync(path.join(root, file));
const walk = directory => !exists(directory) ? [] : fs.readdirSync(path.join(root, directory), { withFileTypes: true })
  .sort((a, b) => a.name.localeCompare(b.name, 'en'))
  .flatMap(entry => entry.isDirectory() && !['target', 'node_modules', 'dist', 'build', 'book', '__pycache__', '.venv', 'venv', 'vendor', '__pypackages__', '.tox', '.mypy_cache', '.pytest_cache', '.ruff_cache', '.git'].includes(entry.name)
    ? walk(`${directory}/${entry.name}`) : entry.isFile() ? [`${directory}/${entry.name}`] : []);
const usage = JSON.parse(read('docs/tools/usage.json'));
const repositorySourcePrefix = 'https://github.com/Sidiora-Labs/HyperMind-engine/blob/main/';
const sourceLink = file => `${repositorySourcePrefix}${file}`;
const fence = (language, text) => {
  const marker = '`'.repeat(Math.max(3, ...[...text.matchAll(/`+/g)].map(match => match[0].length + 1)));
  return `\n\n${marker}${language}\n${text.trim()}\n${marker}\n`;
};
const title = name => name.replace(/([a-z0-9])([A-Z])/g, '$1 $2');
const slug = value => value.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
const inventory = [];

function withoutFences(text) {
  let marker = null;
  return text.split('\n').map(line => {
    const match = line.match(/^ {0,3}(`{3,}|~{3,})/);
    if (marker) {
      if (match && match[1][0] === marker[0] && match[1].length >= marker.length) marker = null;
      return '';
    }
    if (match) { marker = match[1]; return ''; }
    return line;
  }).join('\n');
}

function block(text, start) {
  let depth = 0, opened = false, quoted = false, escaped = false;
  for (let index = start; index < text.length; index++) {
    const character = text[index];
    if (quoted) {
      if (escaped) escaped = false;
      else if (character === '\\') escaped = true;
      else if (character === '"') quoted = false;
      continue;
    }
    if (character === '"') { quoted = true; continue; }
    if (character === '{') { depth++; opened = true; }
    if (character === '}' && opened && --depth === 0) return text.slice(start, index + 1).trim();
    if (!opened && character === ';') return text.slice(start, index + 1).trim();
    if (!opened && character === '\n' && !text.slice(start, index).includes('{') && /^(export|class|def|async def)/.test(text.slice(start)))
      return text.slice(start, index).trim();
  }
  throw new Error(`unterminated public declaration: ${text.slice(start, start + 100)}`);
}

function guidance(file, name) {
  const crate = file.split('/')[1];
  const purpose = usage.crates[crate];
  if (!purpose) throw new Error(`missing public-type guidance for crate ${crate}`);
  let when = `Use \`${name}\` for ${purpose.when}`;
  let avoid = purpose.avoid;
  if (/Input$|Request$/.test(name)) {
    when += ' Supply the fields below to the owning operation; request data remains subject to its admission and capability checks.';
    avoid += ' Do not treat constructing or serializing a request as evidence that it was accepted or executed.';
  } else if (/Result$|Report$|Receipt$|Response$|Ack$/.test(name)) {
    when += ' Inspect its status, coverage, identifiers, and evidence before reporting success.';
    avoid += ' Do not discard gaps, partial coverage, or unknown effect state.';
  } else if (/Config$|Options$|Budget$/.test(name)) {
    when += ' Set explicit deployment limits before opening the associated resource.';
    avoid += ' Do not log secret fields or substitute defaults for an explicitly authorized budget.';
  }
  return { when, avoid };
}

function section(id, name, file, when, avoid, declaration, language) {
  inventory.push({ id, name, file });
  return `\n## ${name}\n\n<a id="${slug(id)}"></a>\n\nSource: [\`${file}\`](${sourceLink(file)}).\n\nWhen to use: ${when}\n\nDo not use: ${avoid}\n${declaration ? fence(language, declaration) : ''}`;
}

const outputs = new Map();
let rust = '# Rust public type catalog\n\nGenerated from authored `pub struct`, `enum`, `trait`, and `type` declarations in every crate’s `src` directory, including macro-defined core identifiers. FlatBuffers-generated builders, offsets, and object wrappers are represented by their canonical definitions in the schema catalog rather than duplicated here. Public declarations in internal modules are included conservatively; this catalog does not promise every path is a stable external API.\n';
for (const file of walk('crates').filter(file => /^crates\/[^/]+\/src\/.*\.rs$/.test(file) && !/_generated\.rs$|\/generated\//.test(file))) {
  const text = read(file);
  for (const match of text.matchAll(/^\s*pub\s+(?:unsafe\s+)?(struct|enum|trait|type)\s+([A-Za-z_][A-Za-z0-9_]*)/gm)) {
    const name = match[2], start = match.index + match[0].indexOf('pub');
    const guide = guidance(file, name);
    rust += section(`rust:${file}:${name}`, `${file.split('/')[1]}::${name}`, file, guide.when, guide.avoid, block(text, start), 'rust');
  }
  if (file === 'crates/hm-core/src/ids.rs') {
    for (const match of text.matchAll(/^(scalar_id|byte_id)!\((\w+)(?:,\s*(\w+))?\);/gm)) {
      const guide = guidance(file, match[2]);
      rust += section(`rust:${file}:${match[2]}`, `hm-core::${match[2]}`, file, guide.when, guide.avoid,
        `pub struct ${match[2]}(${match[3] ?? '[u8; 16]'});`, 'rust');
    }
  }
}
outputs.set('docs/reference/generated/rust-types.md', rust);

let schemas = '# Schema type catalog\n\nGenerated from the canonical FlatBuffers and protobuf schemas. Field names, numeric values, defaults, and union members below are copied from source. Generated language bindings are representations of these types, not separate authority or storage formats.\n';
for (const file of ['schemas/events.fbs', 'schemas/protocol.fbs', 'schemas/hypermind.proto']) {
  const text = read(file);
  for (const match of text.matchAll(/^(table|struct|enum|union|message|service)\s+(\w+)/gm)) {
    const name = match[2];
    let when, avoid;
    if (file.endsWith('events.fbs')) {
      when = `Use ${title(name)} when encoding or interpreting the corresponding versioned ledger record or discriminator. The fields below are the canonical on-disk contract; append through the actor so ordering, authority, and provenance are validated.`;
      avoid = 'Do not write directly to a projection, bypass admission, or infer observed authority from serialized content. Protected identity, preference, and constraint writes require user or admin authority; derived content is not proof of an external effect.';
    } else {
      when = `Use ${title(name)} when implementing the matching protocol request, response, or transport wrapper. Negotiate the protocol version and capability first, and preserve connection identity, request identity, and error/effect state.`;
      avoid = 'Do not send actor operations on an admin connection, reuse an idempotency sequence with different content, or assume transport failure means an operation did not execute. Remote access requires mutual TLS and a separate capability token.';
    }
    schemas += section(`schema:${file}:${name}`, `${file.split('/').at(-1)}::${name}`, file, when, avoid, block(text, match.index), file.endsWith('.proto') ? 'protobuf' : 'text');
  }
}
outputs.set('docs/reference/generated/schema-types.md', schemas);

const dispatcher = read('crates/hm-mcp/src/dispatcher.rs');
const verbs = [...dispatcher.matchAll(/^\s*"([a-z_]+)" => match serde_json::from_slice/gm)].map(match => match[1]).sort();
if (verbs.length === 0 || new Set(verbs).size !== verbs.length) throw new Error('could not identify unique MCP verb dispatch table');
const exposedVerbs = [...read('crates/hm-mcp/src/lib.rs').matchAll(/#\[tool\([^]*?\)\]\s*async fn (\w+)/g)].map(match => match[1]).sort();
if (JSON.stringify(exposedVerbs) !== JSON.stringify(verbs)) throw new Error('MCP public handlers and transport dispatcher disagree; document the actual exposed contract');
let tools = '# MCP tool catalog\n\nGenerated from the actual MCP dispatch table and reviewed operation guidance. The implementation exposes **14 verbs**; older prose describing thirteen is not the wire inventory. All tools return the shared `ok`, `items`, `provenance`, `budget`, `gaps`, `health`, and `warnings` envelope. Mutation errors also carry `effect_state`. Examples are arguments, not fabricated successful responses.\n';
for (const verb of verbs) {
  const guide = usage.tools[verb];
  if (!guide?.when || !guide?.avoid || !guide?.example) throw new Error(`missing specific guidance/example for MCP tool ${verb}`);
  tools += section(`tool:${verb}`, verb, 'crates/hm-mcp/src/dispatcher.rs', guide.when, guide.avoid, JSON.stringify(guide.example, null, 2), 'json');
}
const obsoleteTools = Object.keys(usage.tools).filter(name => !verbs.includes(name));
if (obsoleteTools.length) throw new Error(`documented MCP tools are absent: ${obsoleteTools}`);
outputs.set('docs/reference/generated/tools.md', tools);

let prompts = '# Prompt registry\n\nGenerated from the versioned prompt files. Prompt text is part of the evidence contract: edits require a version change and measured qualification. Retired versions remain available to reproduce historical runs; their presence is not a claim that the current runtime selects them.\n';
for (const file of walk('prompts').filter(file => file.endsWith('.md'))) {
  const name = path.basename(file, '.md');
  prompts += section(`prompt:${name}`, name, file,
    `Use ${name} to reproduce its named background cognition operation with the exact recorded model, inputs, citations, and budget.`,
    'Do not put this model call on the activate/recall hot path, accept uncited derived claims, or change historical prompt content without changing the version.', read(file), 'text');
}
outputs.set('docs/reference/generated/prompts.md', prompts);

let sdk = '# Authored SDK API catalog\n\nGenerated from authored Rust embedded, TypeScript, Python, and Go SDK sources currently present in the tree. Generated wire bindings are covered by the schema catalog. A declaration’s presence does not certify package publication or cross-language qualification. See the SDK guide for availability and supported execution paths.\n';
const sdkFiles = [...walk('sdk'), 'crates/hm-serve/src/embedded.rs'].filter(file =>
  !/\/wire\/|\/generated\/|\/internal\/flatbuffers\/|\.test\.|\/tests?\/|\/test_[^/]+\.py$|_test\.go$|_pb2(?:_grpc)?\.py$|\.pb\.go$|_generated\.rs$/.test(file)
  && /\.(ts|py|go|rs)$/.test(file));
for (const file of sdkFiles) {
  const text = read(file);
  if (/automatically generated by the FlatBuffers compiler|Code generated .*DO NOT EDIT/.test(text.slice(0, 500))) continue;
  const pattern = file.endsWith('.ts') ? /^export\s+(?:declare\s+)?(?:abstract\s+)?(?:async\s+)?(class|interface|type|enum|function|const)\s+(\w+)/gm
    : file.endsWith('.py') ? /^\s*(class|def|async def)\s+([A-Za-z]\w*)/gm
    : file.endsWith('.go') ? /^(type|func)\s+(?:\([^\n)]*\)\s+)?([A-Z]\w*)/gm
    : /^\s*pub\s+(struct|enum|trait|type)\s+(\w+)/gm;
  for (const match of text.matchAll(pattern)) {
    const name = match[2];
    if (name.startsWith('_')) continue;
    const line = text.slice(match.index).split('\n')[0].trim();
    sdk += section(`sdk:${file}:${name}`, `${file}::${name}`, file,
      `Use ${title(name)} through this SDK’s owning module for the declaration shown below. Preserve the common envelope, source authority, provenance, and mutation effect state.`,
      'Do not bypass safe rendering, feed reconstructed text back as observed memory, or assume a package is released merely because its source exists.', line,
      file.endsWith('.ts') ? 'typescript' : file.endsWith('.py') ? 'python' : file.endsWith('.go') ? 'go' : 'rust');
  }
  const methodPattern = file.endsWith('.ts') ? /^\s{2}(?:(?:public|static|async)\s+)*(\w+)\s*\([^\n]*/gm
    : file.endsWith('.rs') ? /^\s*pub\s+(?:async\s+|const\s+)*(?:fn)\s+(\w+)[^\n]*/gm : null;
  if (methodPattern) for (const match of text.matchAll(methodPattern)) {
    if (['if', 'for', 'while', 'switch', 'catch', 'super'].includes(match[1])) continue;
    sdk += section(`sdk-method:${file}:${match.index}:${match[1]}`, `${file}::${match[1]}`, file,
      `Use this ${match[1]} method through its owning SDK type. Its declared parameters below are source-derived; preserve returned status, evidence, and mutation uncertainty.`,
      'Do not substitute unchecked request objects, discard provenance, or assume this method independently establishes external success.', match[0].trim(), file.endsWith('.ts') ? 'typescript' : 'rust');
  }
  if (file.endsWith('.rs')) {
    for (const match of text.matchAll(/#\[pyclass[^\]]*\]\s*(?:#\[[^\]]*\]\s*)*(?:pub\s+)?struct\s+(\w+)/g)) {
      sdk += section(`python-native:${file}:${match[1]}`, `${file}::${match[1]} (Python native class)`, file,
        'Use this PyO3-exported class through the Python extension for its documented embedded ownership contract.',
        'Do not open an actor already owned by another process, copy sample keys, or bypass the Python safe-rendering layer.', match[0], 'rust');
    }
    for (const implementation of text.matchAll(/#\[pymethods\]\s*impl\s+(\w+)/g)) {
      const declaration = block(text, implementation.index);
      for (const method of declaration.matchAll(/^\s*(?:pub\s+)?fn\s+(\w+)[^\n]*/gm)) {
        sdk += section(`python-native-method:${file}:${implementation[1]}:${method[1]}`, `${implementation[1]}::${method[1]} (Python native)`, file,
          'Use this exported PyO3 method through its owning Python class; its signature is taken from the native extension.',
          'Do not assume native execution bypasses actor ownership, evidence admission, or mutation-error handling.', method[0].trim(), 'rust');
      }
    }
  }
}
outputs.set('docs/reference/generated/sdk-api.md', sdk);

let config = '# Configuration key declarations\n\nGenerated from the real configuration loader. The human configuration page explains provisioning, defaults, secret handling, and provider settings. Unknown config-file keys are rejected.\n';
const configSource = 'crates/hm-serve/src/config.rs';
const configText = read(configSource);
for (const match of configText.matchAll(/^\s*"([a-z_]+)" =>[^\n]*/gm)) {
  const name = match[1];
  config += section(`config:${name}`, name, configSource,
    `Use the ${name} key to configure the corresponding ServerConfig field before starting the actor owner. The loader expression below is authoritative.`,
    'Do not add unrecognized keys, expose key/capability values, or use relative paths without controlling the process working directory.',
    match[0].trim(), 'rust');
}
outputs.set('docs/reference/generated/config-keys.md', config);

let cli = '# CLI command declarations\n\nGenerated from the current command parser. The human CLI guide explains execution mode, authority, and examples; these declarations expose the exact option names, required fields, defaults, and value enums accepted by the checked-in source. `--json` is global. Commands not declared here must not be advertised as available.\n';
for (const file of walk('crates/hm-cli/src').filter(file => file.endsWith('.rs'))) {
  const text = read(file);
  for (const match of text.matchAll(/(?:pub(?:\(crate\))?\s+)?(?:enum\s+Command|struct\s+RemoteOptions)\b/g)) {
    const name = `${file} command/options`;
    cli += section(`cli:${file}`, name, file, 'Use these options when invoking the matching hm subcommand; select daemon or embedded access deliberately.',
      'Do not open an actor directory from an embedded command while a daemon owns it, or expose remote listeners without both mutual TLS and capabilities.', block(text, match.index), 'rust');
  }
}
outputs.set('docs/reference/generated/cli-declarations.md', cli);

outputs.set('docs/reference/generated/coverage.json', `${JSON.stringify({ version: 1, inventory }, null, 2)}\n`);
if (mode === '--write') {
  for (const [file, content] of outputs) {
    fs.mkdirSync(path.dirname(path.join(root, file)), { recursive: true });
    fs.writeFileSync(path.join(root, file), content);
  }
  console.log(JSON.stringify({ generated_files: outputs.size, documented_items: inventory.length }));
  process.exit(0);
}

const errors = [];
for (const [file, expected] of outputs) {
  if (!exists(file)) errors.push(`missing generated catalog: ${file}`);
  else if (read(file) !== expected) errors.push(`stale generated catalog: ${file}; run node docs/tools/catalog.mjs --write`);
}
const community = ['CONTRIBUTING.md', 'SECURITY.md', 'CONTRIBUTORS.md', 'CODE_OF_CONDUCT.md', 'SUPPORT.md'];
const required = [...community, 'NOTICE', 'LICENSE', 'docs/start/quickstart.md', 'docs/concepts/architecture.md', 'docs/concepts/authority.md',
  'docs/concepts/retrieval.md', 'docs/concepts/time.md', 'docs/concepts/consolidation.md', 'docs/concepts/anticipation.md',
  'docs/guides/migration.md', 'docs/guides/deployment.md', 'docs/guides/operations.md', 'docs/guides/ci.md', 'docs/reference/schematics.md',
  'docs/reference/protocol.md', 'docs/reference/mcp.md', 'docs/reference/cli.md', 'docs/reference/config.md', 'docs/reference/sdks.md',
  'docs/internals/recovery.md', 'docs/internals/generations.md', 'docs/internals/sealing.md', 'docs/internals/mmr.md',
  'docs/evaluation/methodology.md', 'docs/evaluation/results.md', 'docs/security/threat-model.md'];
for (let index = 1; index <= 10; index++) required.push(`docs/adr/ADR-${String(index).padStart(3, '0')}.md`);
const languages = ['en', 'zh-CN', 'hi', 'es', 'fr', 'ar', 'pt', 'ru', 'ja', 'de'];
for (const language of languages) required.push(language === 'en' ? 'README.md' : `README.${language}.md`);
for (const file of required) if (!exists(file)) errors.push(`required page absent: ${file}`);
const summary = exists('docs/SUMMARY.md') ? read('docs/SUMMARY.md') : '';
const summaryPages = new Set([...summary.matchAll(/\]\(([^)#]+)(?:#[^)]*)?\)/g)].map(match => `docs/${match[1]}`));
for (const file of [...required.filter(file => file.startsWith('docs/')), ...outputs.keys()].filter(file => file.endsWith('.md')))
  if (!summaryPages.has(file)) errors.push(`page is unreachable from mdBook summary: ${file}`);

const markdown = [...walk('docs'), ...walk('deploy'), ...walk('sdk').filter(file => !/\/(?:generated|wire)\/|\/internal\/flatbuffers\//.test(file))]
  .filter(file => file.endsWith('.md')).concat(required.filter(file => file.startsWith('README')), community);
const headings = new Map();
function anchors(file) {
  if (headings.has(file)) return headings.get(file);
  const result = new Set();
  if (exists(file) && file.endsWith('.md')) {
    const seen = new Map();
    for (const match of withoutFences(read(file)).matchAll(/^#{1,6}\s+(.+)$/gm)) {
      const heading = match[1].replace(/\[([^\]]+)\]\([^)]*\)/g, '$1').replace(/[`*_]/g, '')
        .toLowerCase().replace(/[^\p{L}\p{N}\s_-]/gu, '').replaceAll(' ', '-');
      const count = seen.get(heading) ?? 0;
      result.add(count ? `${heading}-${count}` : heading);
      seen.set(heading, count + 1);
    }
    for (const match of read(file).matchAll(/\bid=["']([^"']+)["']/g)) result.add(match[1]);
  }
  headings.set(file, result);
  return result;
}
let checkedLinks = 0;
for (const file of markdown.filter(exists)) {
  const text = withoutFences(read(file));
  const links = [...text.matchAll(/!?\[[^\]]*\]\((<?[^\s)]+>?)(?:\s+["'][^)]*)?\)/g)].map(match => match[1]);
  links.push(...[...text.matchAll(/^\[[^\]]+\]:\s*(\S+)/gm)].map(match => match[1]));
  for (let link of links) {
    link = link.replace(/^<|>$/g, '');
    const repositorySource = link.startsWith(repositorySourcePrefix);
    if (repositorySource) link = link.slice(repositorySourcePrefix.length);
    else if (/^[a-z][a-z0-9+.-]*:/i.test(link) || link.startsWith('//')) continue;
    checkedLinks++;
    const [rawPath, anchor] = link.split('#');
    const target = path.relative(root, path.resolve(root, repositorySource ? '.' : path.dirname(file), decodeURIComponent(rawPath.split('?')[0] || path.basename(file))));
    if (target.startsWith('..') || !exists(target)) errors.push(`broken local link in ${file}: ${link}`);
    else if (anchor && target.endsWith('.md') && !anchors(target).has(decodeURIComponent(anchor)))
      errors.push(`broken local anchor in ${file}: ${link}`);
  }
}
const report = { gate: 'docs', passed: errors.length === 0, public_items: inventory.length, mcp_verbs: verbs.length,
  readme_languages: languages.length, checked_local_links: checkedLinks, generated_files: outputs.size, errors };
console.log(JSON.stringify(report, null, 2));
if (errors.length) process.exit(1);
