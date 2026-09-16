#!/usr/bin/env python3
"""Fail closed on repository CI policy; actionlint handles workflow syntax."""

from pathlib import Path
import re
import sys

import yaml


class WorkflowLoader(yaml.SafeLoader):
    pass


# YAML 1.1 treats GitHub's `on` key as boolean. Retain only true/false.
WorkflowLoader.yaml_implicit_resolvers = {
    key: [(tag, expression) for tag, expression in values
          if tag != "tag:yaml.org,2002:bool"]
    for key, values in yaml.SafeLoader.yaml_implicit_resolvers.items()
}
WorkflowLoader.add_implicit_resolver(
    "tag:yaml.org,2002:bool", re.compile(r"^(?:true|false)$", re.IGNORECASE), list("tTfF")
)


def unique_mapping(loader, node, deep=False):
    result = {}
    for key_node, value_node in node.value:
        key = loader.construct_object(key_node, deep=deep)
        if key in result:
            raise ValueError(f"duplicate YAML key: {key}")
        result[key] = loader.construct_object(value_node, deep=deep)
    return result


WorkflowLoader.add_constructor("tag:yaml.org,2002:map", unique_mapping)
ROOT = Path(__file__).resolve().parents[2]
PIN = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_./-]+)?@[a-f0-9]{40}$")
CONTAINER_PIN = re.compile(r"^docker://[^\s@]+@sha256:[a-f0-9]{64}$")
PAID = re.compile(
    r"(?<![\w-])--live(?:[=\s]|$)|"
    r"\b(?:CENTRA_GATEWAY_API_KEY|OPEN_ROUTER|OPENROUTER_API_KEY|OPENAI_API_KEY|"
    r"ANTHROPIC_API_KEY|VOYAGE_API_KEY)\b|"
    r"\bHM_(?:EMBEDDING|RECONSTRUCTION|CONSOLIDATION)_PROVIDER\b"
)
errors = []
visited = set()


def fail(path, message):
    errors.append(f"{path.relative_to(ROOT)}: {message}")


def load(path):
    data = yaml.load(path.read_text(), Loader=WorkflowLoader)
    if not isinstance(data, dict):
        raise ValueError("expected a YAML mapping")
    return data


def permissions(path, value, label):
    if not isinstance(value, dict) or any(level not in ("read", "none") for level in value.values()):
        fail(path, f"{label} must explicitly contain only read/none permissions")


def references(path, node):
    if isinstance(node, dict):
        if "uses" in node:
            target = node["uses"]
            if not isinstance(target, str):
                fail(path, "uses must be a literal pinned action or local path")
            elif target.startswith("./"):
                location = (ROOT / target).resolve()
                if not location.is_relative_to(ROOT):
                    fail(path, "local action escapes repository")
                elif location.is_file():
                    fail(path, "local reusable workflows need an explicit policy implementation")
                else:
                    found = [location / name for name in ("action.yml", "action.yaml")
                             if (location / name).is_file()]
                    if len(found) != 1:
                        fail(path, f"local action lacks a unique action manifest: {target}")
                    else:
                        inspect_action(found[0])
            elif not PIN.fullmatch(target) and not CONTAINER_PIN.fullmatch(target):
                fail(path, f"action is not pinned to a full commit/digest: {target}")
            if isinstance(target, str) and target.startswith("actions/checkout@"):
                options = node.get("with", {})
                if not isinstance(options, dict) or options.get("persist-credentials") is not False:
                    fail(path, "checkout must set persist-credentials: false")
        for value in node.values():
            references(path, value)
    elif isinstance(node, list):
        for value in node:
            references(path, value)


def inspect_action(path):
    if path in visited:
        return
    visited.add(path)
    data = load(path)
    if data.get("runs", {}).get("using") == "composite":
        references(path, data)
    else:
        fail(path, "only reviewable local composite actions are allowed")
    if PAID.search(yaml.dump(data)):
        fail(path, "local action includes live/provider credential configuration")


def inspect_workflow(path):
    data = load(path)
    permissions(path, data.get("permissions"), "workflow permissions")
    events = data.get("on")
    if isinstance(events, str):
        events = [events]
    if not isinstance(events, (list, dict)) or not events:
        fail(path, "workflow must declare its events")
        return
    if "pull_request_target" in events:
        fail(path, "pull_request_target is prohibited")
    if "workflow_run" in events:
        fail(path, "workflow_run requires a separately reviewed trust boundary")
    automatic = any(event != "workflow_dispatch" for event in events)
    if automatic and PAID.search(yaml.dump(data)):
        fail(path, "automatic workflow includes live/provider credential configuration")
    jobs = data.get("jobs")
    if not isinstance(jobs, dict) or not jobs:
        fail(path, "workflow must contain jobs")
        return
    for name, job in jobs.items():
        if not isinstance(job, dict):
            fail(path, f"job {name} is not a mapping")
            continue
        timeout = job.get("timeout-minutes")
        if type(timeout) is not int or not 1 <= timeout <= 360:
            fail(path, f"job {name} requires a literal timeout between 1 and 360 minutes")
        if "permissions" in job:
            permissions(path, job["permissions"], f"job {name} permissions")
        matrix = job.get("strategy", {}).get("matrix")
        if matrix is not None:
            if not isinstance(matrix, dict) or "${{" in yaml.dump(matrix):
                fail(path, f"job {name} uses an unbounded/dynamic matrix")
            else:
                combinations = 1
                extra = 0
                for dimension, values in matrix.items():
                    if not isinstance(values, list) or not values or len(values) > 256:
                        fail(path, f"job {name} matrix {dimension} must be a bounded nonempty list")
                        continue
                    if dimension == "include":
                        extra = len(values)
                    elif dimension != "exclude":
                        combinations *= len(values)
                if combinations + extra > 256:
                    fail(path, f"job {name} matrix can exceed 256 jobs")
        references(path, job)


def main():
    workflows = sorted(set((ROOT / ".github/workflows").glob("*.yml"))
                       | set((ROOT / ".github/workflows").glob("*.yaml")))
    if not workflows:
        raise ValueError("no workflow files found")
    for path in workflows:
        try:
            inspect_workflow(path)
        except (ValueError, TypeError, yaml.YAMLError) as error:
            fail(path, str(error))
    for pattern in ("action.yml", "action.yaml"):
        for path in (ROOT / ".github/actions").rglob(pattern):
            try:
                inspect_action(path)
            except (ValueError, TypeError, yaml.YAMLError) as error:
                fail(path, str(error))
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print(f"Workflow policy passed: {len(workflows)} workflows, {len(visited)} local actions; no hosted run implied.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
