import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import process from "node:process";

const RAW_FORMAT = "hypermind.cross-sdk-contract-raw.v1";
const SCENARIO_FORMAT = "hypermind.cross-sdk-contract-scenario.v1";
const SDK = "typescript";
const TRANSPORT = "unix";
const REQUIRED = ["client", "socket", "token", "scenario", "phase", "out"];
const CONNECTION_ID = Uint8Array.from({ length: 16 }, () => 0x55);

function options(argv) {
  const parsed = new Map();
  for (let index = 0; index < argv.length; index += 2) {
    const flag = argv[index];
    if (!flag.startsWith("--") || argv[index + 1] === undefined) {
      throw new Error(`the leg does not accept ${flag}`);
    }
    parsed.set(flag.slice(2), argv[index + 1]);
  }
  for (const name of REQUIRED) {
    if (!parsed.has(name)) throw new Error(`the leg requires --${name}`);
  }
  return parsed;
}

function transportEnvelope(error) {
  return {
    ok: false,
    items: [],
    provenance: [],
    gaps: [],
    health: {},
    warnings: [],
    effect_state: typeof error?.effectState === "string" ? error.effectState : "unknown",
  };
}

const parsed = options(process.argv.slice(2));
const scenario = JSON.parse(readFileSync(parsed.get("scenario"), "utf8"));
if (scenario.format !== SCENARIO_FORMAT) {
  throw new Error(`scenario format is ${scenario.format}, expected ${SCENARIO_FORMAT}`);
}
const steps = scenario.calls.filter((call) => call.phase === parsed.get("phase"));
if (steps.length === 0) throw new Error(`no scenario step runs in phase ${parsed.get("phase")}`);

const { Client } = createRequire(import.meta.url)(parsed.get("client"));
const client = await Client.connect({
  socketPath: parsed.get("socket"),
  capabilityToken: Buffer.from(parsed.get("token"), "hex"),
  connectionId: CONNECTION_ID,
});
const calls = [];
try {
  for (const call of steps) {
    let envelope;
    try {
      envelope = await client.callTool(call.verb, call.arguments);
    } catch (error) {
      envelope = transportEnvelope(error);
    }
    calls.push({ step: call.step, verb: call.verb, envelope });
  }
} finally {
  client.close();
}
writeFileSync(
  parsed.get("out"),
  JSON.stringify({ format: RAW_FORMAT, sdk: SDK, transport: TRANSPORT, calls }),
);
