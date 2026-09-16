# Windows: Docker Desktop with WSL 2

This launcher runs the Linux daemon in Docker Desktop. It does not claim native
Windows daemon support. Install WSL 2 and Docker Desktop, enable the Linux-container
backend, and start Docker Desktop. Microsoft documents `wsl --install` and the
required restart; check current Windows/WSL requirements in Docker's installer
guide. [Microsoft WSL installation](https://learn.microsoft.com/en-us/windows/wsl/install),
[Docker Desktop for Windows](https://docs.docker.com/desktop/setup/install/windows-install/).

From the repository root in PowerShell:

```powershell
wsl --version
wsl --list --verbose
docker compose version
pwsh -NoProfile -File .\deploy\windows\hypermind.ps1 start -Build
pwsh -NoProfile -File .\deploy\windows\hypermind.ps1 doctor
pwsh -NoProfile -File .\deploy\windows\hypermind.ps1 status
pwsh -NoProfile -File .\deploy\windows\hypermind.ps1 logs
pwsh -NoProfile -File .\deploy\windows\hypermind.ps1 stop
```

Use PowerShell 7 (`pwsh`); do not run the launcher elevated. If your execution
policy blocks a downloaded checkout, inspect and explicitly unblock the trusted
script rather than globally weakening the execution policy. Image builds require
network access to the pinned base images and package registries; no model-provider
call is part of this launcher.

`start` initializes only when no daemon container is running, with
`hm init --path /var/lib/hypermind --if-missing`, and then waits for one healthy
daemon. Existing configuration, keys and credentials are preserved. `stop` sends
the image's SIGINT stop signal and retains the container and named volume. The
launcher has no volume-deletion operation. Do not run a second daemon or attach
another writer to the same volume.

The stable Compose project name is `hypermind`; use `-Project other-name`
consistently for an isolated second installation with its own named volume.
Changing project names does not migrate data. Docker documents how project names
isolate Compose resources and that named-volume deletion is a separate opt-in
action. [Compose project names](https://docs.docker.com/compose/how-tos/project-name/),
[Compose teardown and volumes](https://docs.docker.com/reference/cli/docker/compose/down/).

No TCP endpoint or capability token is exposed by default. Keep provider secrets
out of image build arguments and source control. Follow the parent
[deployment guide](../README.md) for authenticated remote listeners and backups.
The current image is `linux/amd64`; Windows Arm emulation is not qualified.

To inspect exact arguments without Docker or service changes:

```powershell
pwsh -NoProfile -File .\deploy\windows\hypermind.ps1 start -Build -Render
pwsh -NoProfile -File .\deploy\windows\Test-Launcher.ps1
```

The syntax and command-render test can run under PowerShell on Linux. Actual
Windows Docker Desktop/WSL execution remains untested in this Linux workspace;
rendering checks are not an end-to-end Windows deployment qualification.
