import argparse
import asyncio
import json
from pathlib import Path

RAW_FORMAT = "hypermind.cross-sdk-contract-raw.v1"
SCENARIO_FORMAT = "hypermind.cross-sdk-contract-scenario.v1"
SDK = "python"
TRANSPORT = "grpc"
CONNECTION_ID = bytes([0x55]) * 16


def options():
    parser = argparse.ArgumentParser(add_help=False)
    for name in ("endpoint", "tls-root", "token", "scenario", "phase", "out"):
        parser.add_argument(f"--{name}", required=True)
    return parser.parse_args()


def transport_envelope(error):
    effect = getattr(error, "effect_state", None)
    return {
        "ok": False,
        "items": [],
        "provenance": [],
        "gaps": [],
        "health": {},
        "warnings": [],
        "effect_state": effect if isinstance(effect, str) else "unknown",
    }


async def main():
    from hypermind import Client, HyperMindError

    parsed = options()
    scenario = json.loads(Path(parsed.scenario).read_text(encoding="utf-8"))
    if scenario["format"] != SCENARIO_FORMAT:
        raise SystemExit(f"scenario format is {scenario['format']}, expected {SCENARIO_FORMAT}")
    steps = [call for call in scenario["calls"] if call["phase"] == parsed.phase]
    if not steps:
        raise SystemExit(f"no scenario step runs in phase {parsed.phase}")

    tls = Path(parsed.tls_root)
    client = Client(
        parsed.endpoint,
        token=bytes.fromhex(parsed.token),
        ca=(tls / "ca.pem").read_bytes(),
        certificate=(tls / "client.pem").read_bytes(),
        private_key=(tls / "client.key").read_bytes(),
        connection_id=CONNECTION_ID,
    )
    calls = []
    try:
        await client.connect()
        for call in steps:
            try:
                envelope = await client.tool(call["verb"], call["arguments"])
            except HyperMindError as error:
                envelope = transport_envelope(error)
            calls.append({"step": call["step"], "verb": call["verb"], "envelope": envelope})
    finally:
        await client.close()
    Path(parsed.out).write_text(
        json.dumps({"format": RAW_FORMAT, "sdk": SDK, "transport": TRANSPORT, "calls": calls}),
        encoding="utf-8",
    )


asyncio.run(main())
