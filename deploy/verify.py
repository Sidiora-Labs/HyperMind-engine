#!/usr/bin/env python3
"""Offline deployment validation: renders configuration, never provisions resources."""
import argparse
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tempfile
import tomllib
import yaml

ROOT = Path(__file__).resolve().parents[1]
DEPLOY = ROOT / "deploy"
RESULTS = []


def require(condition, message):
    if not condition:
        raise ValueError(message)


def run(*arguments, env=None):
    result = subprocess.run(arguments, cwd=ROOT, env=env, text=True, capture_output=True, timeout=120)
    if result.returncode:
        raise ValueError(f"{arguments[0]} failed: {result.stderr.strip()}")
    return result.stdout


def check(name, action):
    try:
        action()
        RESULTS.append({"check": name, "status": "passed"})
    except (OSError, ValueError, KeyError, TypeError, StopIteration, subprocess.TimeoutExpired) as error:
        RESULTS.append({"check": name, "status": "failed", "error": str(error)})


def compose():
    with tempfile.TemporaryDirectory(prefix="hypermind-config-check-") as directory:
        environment = dict(os.environ, HM_TLS_DIRECTORY=directory)
        prefix = ["docker", "compose", "-p", "hypermind-validation", "-f", str(DEPLOY / "compose.yaml")]
        base = json.loads(run(*prefix, "config", "--format", "json", env=environment))["services"]["hypermind"]
        require(not base.get("ports"), "base Compose must not publish ports")
        require(base["user"] == "65532:65532" and base["read_only"], "base Compose must remain non-root/read-only")
        require(base["platform"] == "linux/amd64", "current image requires explicit amd64 platform")
        require("ALL" in base["cap_drop"], "container capabilities must be dropped")
        require(any(v["type"] == "volume" and v["target"] == "/var/lib/hypermind" for v in base["volumes"]), "missing persistent state")
        require("--require-healthy" in base["healthcheck"]["test"], "health check must use authenticated doctor")
        remote = json.loads(run(*prefix, "-f", str(DEPLOY / "compose.mtls.yaml"), "config", "--format", "json", env=environment))["services"]["hypermind"]
        require({p["target"] for p in remote["ports"]} == {8443, 8444}, "unexpected exposed port")
        require(all(p["host_ip"] == "127.0.0.1" for p in remote["ports"]), "desktop remote ports must remain loopback-only")
        for flag in ("--tls-cert", "--tls-key", "--tls-client-ca"):
            require(flag in remote["command"], f"missing {flag}")
        require(not any("admin-bind" in argument for argument in remote["command"]), "admin endpoint must stay private")
        require(any(v["target"] == "/etc/hypermind/tls" and v["read_only"] for v in remote["volumes"]), "TLS mount must be read-only")


def helm():
    chart = str(DEPLOY / "helm/hypermind")
    run("helm", "lint", chart, "--strict")
    resources = list(yaml.safe_load_all(run("helm", "template", "hm-check", chart)))
    stateful = next(item for item in resources if item and item["kind"] == "StatefulSet")
    require(stateful["spec"]["replicas"] == 1, "only one state owner is allowed")
    pod = stateful["spec"]["template"]["spec"]
    require(pod["automountServiceAccountToken"] is False, "daemon does not need a Kubernetes API credential")
    require(pod["securityContext"]["runAsNonRoot"] and pod["securityContext"]["runAsUser"] == 65532, "Kubernetes must stay non-root")
    require(pod["nodeSelector"]["kubernetes.io/arch"] == "amd64", "current image requires amd64 scheduling")
    require(stateful["spec"]["volumeClaimTemplates"][0]["spec"]["accessModes"] == ["ReadWriteOnce"], "persistent single-writer claim required")
    for container in pod["initContainers"] + pod["containers"]:
        security = container["securityContext"]
        require(security["readOnlyRootFilesystem"] and not security["allowPrivilegeEscalation"], "container sandbox weakened")
        require("ALL" in security["capabilities"]["drop"], "container capabilities must be dropped")
    daemon = pod["containers"][0]
    require("--if-missing" in pod["initContainers"][0]["args"], "initialization must preserve identities")
    require("--require-healthy" in daemon["readinessProbe"]["exec"]["command"], "readiness must check actual daemon state")
    require(not any("admin-bind" in arg for arg in daemon["args"]), "admin listener must not be exposed")
    service = next(item for item in resources if item and item["kind"] == "Service")
    require(service["spec"].get("type", "ClusterIP") == "ClusterIP", "default Kubernetes service must stay internal")
    require({p["targetPort"] for p in service["spec"]["ports"]} == {"grpc", "rest"}, "unexpected service endpoint")


def cloud_files():
    fly = tomllib.loads((DEPLOY / "fly/fly.toml").read_text())
    require(fly["build"]["args"] == {"HM_RUNTIME_UID": "0", "HM_RUNTIME_GID": "0"}, "Fly volume ownership exception must be explicit")
    require(any(m["destination"] == "/var/lib/hypermind" for m in fly["mounts"]), "Fly persistent mount missing")
    for service in fly["services"]:
        require(service["protocol"] == "tcp", "Fly must use raw TCP")
        require(all(p.get("handlers", []) == [] for p in service["ports"]), "Fly proxy must not terminate mTLS")
    command = fly["experimental"]["cmd"]
    require("--init-if-missing" in command and "--tls-from-env" in command, "Fly bootstrap/TLS contract missing")
    render = yaml.safe_load((DEPLOY / "render/render.yaml").read_text())["services"]
    require(len(render) == 1, "Render must have one owner")
    service = render[0]
    require(service["type"] == "pserv" and service["numInstances"] == 1, "Render must be one private service")
    require(service["autoDeployTrigger"] == "off", "Render automatic redeploy setting must be the string off")
    require(service["disk"]["mountPath"] == "/var/lib/hypermind", "Render persistent disk missing")
    variables = {v["key"]: v for v in service["envVars"]}
    for key in ("HM_TLS_CERT_PEM", "HM_TLS_KEY_PEM", "HM_TLS_CLIENT_CA_PEM"):
        require(variables[key].get("sync") is False and "value" not in variables[key], "TLS secrets must be supplied at runtime")
    for key in ("HM_RUNTIME_UID", "HM_RUNTIME_GID"):
        require(variables[key]["value"] == "0", "Render volume ownership exception must be explicit")
    railway = (DEPLOY / "railway/.railway/railway.ts").read_text()
    for contract in ('from "railway/iac"', 'RAILWAY_RUN_UID: "0"', 'requiredMountPath: "/var/lib/hypermind"', 'overlapSeconds: 0', 'tcp: [8443]', '--init-if-missing', '--tls-from-env'):
        require(contract in railway, f"Railway source contract missing: {contract}")
    package = json.loads((DEPLOY / "railway/package.json").read_text())
    lock = json.loads((DEPLOY / "railway/package-lock.json").read_text())
    require(package["dependencies"]["railway"] == lock["packages"]["node_modules/railway"]["version"], "Railway SDK must match its lockfile")
    dockerfile = (DEPLOY / "Dockerfile").read_text()
    for contract in ("ARG HM_RUNTIME_UID=65532", "ARG HM_RUNTIME_GID=65532", "@sha256:"):
        require(contract in dockerfile, f"default image contract missing: {contract}")


def desktop_files():
    for script in ("linux/install.sh", "tls/create-dev-certs.sh"):
        run("bash", "-n", str(DEPLOY / script))
    plist = plistlib.loads(run(sys.executable, str(DEPLOY / "macos/hypermind.py"), "render", "--state", "/tmp/hm deployment & state").encode())
    require("/tmp/hm deployment & state" in plist["ProgramArguments"], "macOS paths must survive XML/argument escaping")
    require(plist["Umask"] == 0o077, "macOS launch agent must create private files")
    unit = (DEPLOY / "hypermind.service").read_text()
    for contract in ("User=hypermind", "Group=hypermind", "UMask=0077", "StateDirectoryMode=0700", "NoNewPrivileges=yes", "KillSignal=SIGINT"):
        require(contract in unit, f"systemd contract missing: {contract}")
    for path in ("docker/README.md", "windows/README.md", "macos/README.md", "linux/README.md", "kubernetes/README.md", "fly/README.md", "railway/README.md", "render/README.md"):
        require((DEPLOY / path).is_file(), f"missing platform guide: {path}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--require-tools", action="store_true", help="fail instead of skipping missing Compose/Helm")
    options = parser.parse_args()
    check("cloud manifest and secret contracts (offline)", cloud_files)
    check("desktop render and shell/unit contracts (offline)", desktop_files)
    for tool, action in (("docker", compose), ("helm", helm)):
        if shutil.which(tool):
            check(f"{tool} configuration rendering (offline)", action)
        else:
            RESULTS.append({"check": f"{tool} configuration rendering", "status": "failed" if options.require_tools else "skipped", "error": f"{tool} is not installed"})
    print(json.dumps({"scope": "offline configuration validation, not cloud or native-OS qualification", "checks": RESULTS}, indent=2))
    return int(any(result["status"] == "failed" for result in RESULTS))


if __name__ == "__main__":
    raise SystemExit(main())
