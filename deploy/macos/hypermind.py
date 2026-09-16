#!/usr/bin/env python3
import argparse
import contextlib
import fcntl
import json
import os
from pathlib import Path
import platform
import plistlib
import re
import signal
import stat
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
MANAGED = "Managed by HyperMind deploy/macos/hypermind.py"


def arguments(argv=None):
    parser = argparse.ArgumentParser(description="HyperMind macOS user-agent and Docker lifecycle")
    parser.add_argument("command", choices=("build", "render", "install", "start", "stop", "restart", "status", "logs", "doctor", "uninstall", "supervise", "docker"))
    parser.add_argument("--state", type=Path, default=Path.home() / "Library/Application Support/HyperMind")
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/hm")
    parser.add_argument("--launch-agents", type=Path, default=Path.home() / "Library/LaunchAgents")
    parser.add_argument("--label", default="ag.centra.hypermind")
    parser.add_argument("--replace", action="store_true", help="explicitly back up and replace an existing plist, never the config")
    parser.add_argument("--action", choices=("start", "stop", "status", "logs", "doctor", "build"), default="doctor", help="Docker operation")
    parser.add_argument("--project", default="hypermind")
    parser.add_argument("--build", action="store_true", help="build the Docker image before starting")
    parser.add_argument("--render-plan", action="store_true", help="print Docker arguments without contacting Docker")
    value = parser.parse_args(argv)
    for field in ("state", "binary", "launch_agents"):
        setattr(value, field, getattr(value, field).expanduser().absolute())
    if not re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9_.-]*", value.label):
        parser.error("invalid launchd label")
    if not re.fullmatch(r"[a-z0-9][a-z0-9_-]*", value.project):
        parser.error("invalid Compose project name")
    return value


def plist_bytes(args):
    return plistlib.dumps({
        "Label": args.label,
        "Comment": MANAGED,
        "ProgramArguments": [sys.executable, str(Path(__file__).resolve()), "supervise", "--state", str(args.state), "--binary", str(args.binary)],
        "WorkingDirectory": str(args.state),
        "StandardOutPath": str(args.state / "logs/stdout.log"),
        "StandardErrorPath": str(args.state / "logs/stderr.log"),
        "RunAtLoad": True,
        "KeepAlive": True,
        "ThrottleInterval": 10,
        "ExitTimeOut": 35,
        "Umask": 0o077,
        "ProcessType": "Background",
    }, fmt=plistlib.FMT_XML, sort_keys=False)


def private_directory(path):
    if path.is_symlink():
        raise RuntimeError(f"refusing symlink state directory: {path}")
    path.mkdir(mode=0o700, parents=True, exist_ok=True)
    if path.stat().st_uid != os.getuid():
        raise RuntimeError(f"directory is owned by another user: {path}")
    path.chmod(0o700)


def private_file(path, create=False):
    if path.is_symlink():
        raise RuntimeError(f"refusing symlink private file: {path}")
    if create and not path.exists():
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        os.close(descriptor)
    info = path.stat()
    if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or stat.S_IMODE(info.st_mode) & 0o077:
        raise RuntimeError(f"file must be owned by this user and private (0600): {path}")


def install_plist(path, data, replace=False):
    if path.is_symlink():
        raise RuntimeError(f"refusing symlink plist: {path}")
    if path.exists():
        if path.read_bytes() == data:
            private_file(path)
            return
        if not replace:
            raise RuntimeError(f"existing plist preserved; use --replace explicitly after review: {path}")
        backup = path.with_name(path.name + f".{time.time_ns()}.backup")
        path.rename(backup)
        print(f"Previous plist preserved at {backup}", file=sys.stderr)
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, "wb") as stream:
        stream.write(data)


@contextlib.contextmanager
def writer_lock(state):
    private_directory(state)
    descriptor = os.open(state / ".launcher.lock", os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    try:
        private_file(state / ".launcher.lock")
        try:
            fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise RuntimeError("another managed process owns this state directory") from error
        yield
    finally:
        os.close(descriptor)


def supervise(args):
    private_file(args.state / "hypermind.conf")
    with writer_lock(args.state):
        child = None
        stopping = False

        def stop(_number, _frame):
            nonlocal stopping
            if not stopping:
                stopping = True
                if child is not None and child.poll() is None:
                    os.killpg(child.pid, signal.SIGINT)

        signal.signal(signal.SIGTERM, stop)
        signal.signal(signal.SIGINT, stop)
        child = subprocess.Popen([str(args.binary), "serve", "--config", str(args.state / "hypermind.conf")], start_new_session=True)
        if stopping and child.poll() is None:
            os.killpg(child.pid, signal.SIGINT)
        while child.poll() is None:
            try:
                child.wait(timeout=30 if stopping else 0.25)
            except subprocess.TimeoutExpired:
                if stopping:
                    os.killpg(child.pid, signal.SIGKILL)
                    child.wait()
                    return 1
        return 0 if stopping else child.returncode


def docker_plan(args):
    prefix = ["docker", "compose", "--project-name", args.project, "--file", str(ROOT / "deploy/compose.yaml")]
    steps = []
    if args.action == "build" or (args.action == "start" and args.build):
        steps.append({"arguments": ["build", "hypermind"], "condition": "always"})
    if args.action == "start":
        steps.extend([
            {"arguments": ["run", "--rm", "--no-deps", "hypermind", "init", "--path", "/var/lib/hypermind", "--if-missing"], "condition": "daemon_not_running"},
            {"arguments": ["up", "--detach", "--wait", "--scale", "hypermind=1", "hypermind"], "condition": "always"},
        ])
    elif args.action != "build":
        commands = {
            "stop": ["stop", "--timeout", "30", "hypermind"],
            "status": ["ps", "--all", "hypermind"],
            "logs": ["logs", "--follow", "--tail", "100", "hypermind"],
            "doctor": ["exec", "--no-TTY", "hypermind", "/usr/local/bin/hm", "doctor", "--config", "/var/lib/hypermind/hypermind.conf", "--require-healthy", "--json"],
        }
        steps.append({"arguments": commands[args.action], "condition": "always"})
    return {"prefix": prefix, "steps": steps, "project": args.project}


def docker(args):
    plan = docker_plan(args)
    if args.render_plan:
        print(json.dumps(plan, indent=2))
        return 0
    subprocess.run(["docker", "compose", "version"], check=True)
    server = subprocess.run(["docker", "info", "--format", "{{.OSType}}"], check=True, text=True, capture_output=True)
    if server.stdout.strip() != "linux":
        raise RuntimeError("Docker Desktop must use Linux containers")
    for step in plan["steps"]:
        if step["condition"] == "daemon_not_running":
            running = subprocess.run(plan["prefix"] + ["ps", "--status", "running", "--quiet", "hypermind"], check=True, text=True, capture_output=True)
            if running.stdout.strip():
                continue
        subprocess.run(plan["prefix"] + step["arguments"], check=True)
    return 0


def main(argv=None):
    args = arguments(argv)
    os.umask(0o077)
    if args.command == "render":
        sys.stdout.buffer.write(plist_bytes(args))
        return 0
    if args.command == "docker":
        return docker(args)
    if platform.system() != "Darwin":
        raise RuntimeError("native lifecycle requires macOS; render and Docker plans are portable")
    if os.getuid() == 0:
        raise RuntimeError("run this user agent as your login user, not root or sudo")
    if args.command == "build":
        subprocess.run(["cargo", "build", "--locked", "--release", "-p", "hm-cli", "--features", "portable"], cwd=ROOT, check=True)
        return 0
    if args.command == "supervise":
        return supervise(args)
    domain = f"gui/{os.getuid()}"
    service = f"{domain}/{args.label}"
    plist = args.launch_agents / f"{args.label}.plist"

    def loaded():
        return subprocess.run(["launchctl", "print", service], capture_output=True).returncode == 0

    def managed():
        private_file(plist)
        if plistlib.loads(plist.read_bytes()).get("Comment") != MANAGED:
            raise RuntimeError("refusing to manage an existing custom plist; review it before explicit --replace installation")

    if args.command == "install":
        if loaded():
            raise RuntimeError("stop the existing user agent before installing or replacing its plist")
        data = plist_bytes(args)
        if plist.exists() and plist.read_bytes() != data and not args.replace:
            raise RuntimeError("existing custom plist preserved; --replace is required")
        if not args.binary.is_file() or not os.access(args.binary, os.X_OK):
            raise RuntimeError("build hm first or specify an executable --binary")
        with writer_lock(args.state):
            subprocess.run([str(args.binary), "init", "--path", str(args.state), "--if-missing"], check=True)
            private_file(args.state / "hypermind.conf")
            private_directory(args.state / "logs")
            for name in ("stdout.log", "stderr.log"):
                private_file(args.state / "logs" / name, create=True)
            args.launch_agents.mkdir(mode=0o700, parents=True, exist_ok=True)
            install_plist(plist, data, args.replace)
        print(f"Installed {plist}; start it explicitly with the start command.")
        return 0
    if args.command == "status":
        return subprocess.run(["launchctl", "print", service]).returncode
    if args.command == "doctor":
        return subprocess.run([str(args.binary), "doctor", "--config", str(args.state / "hypermind.conf"), "--require-healthy", "--json"]).returncode
    if args.command == "logs":
        for name in ("stdout.log", "stderr.log"):
            private_file(args.state / "logs" / name)
        return subprocess.run(["tail", "-n", "100", "-f", str(args.state / "logs/stdout.log"), str(args.state / "logs/stderr.log")]).returncode
    managed()
    if args.command in ("stop", "restart", "uninstall") and loaded():
        subprocess.run(["launchctl", "bootout", service], check=True)
    if args.command == "uninstall":
        disabled = plist.with_name(plist.name + f".{time.time_ns()}.disabled")
        plist.rename(disabled)
        print(f"Agent disabled; plist preserved at {disabled}. State and logs remain at {args.state}.")
    if args.command in ("start", "restart") and not loaded():
        if plist.read_bytes() != plist_bytes(args):
            raise RuntimeError("installed paths differ; use the same --state/--binary/--label options used at installation")
        subprocess.run(["launchctl", "enable", service], check=True)
        subprocess.run(["launchctl", "bootstrap", domain, str(plist)], check=True)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, ValueError, subprocess.CalledProcessError) as error:
        print(f"hypermind: {error}", file=sys.stderr)
        raise SystemExit(1)
