[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [ValidateSet('start', 'stop', 'status', 'logs', 'doctor', 'build')]
    [string]$Action = 'doctor',
    [ValidatePattern('^[a-z0-9][a-z0-9_-]*$')]
    [string]$Project = 'hypermind',
    [switch]$Build,
    [switch]$Render
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$composeFile = Join-Path (Split-Path -Parent $PSScriptRoot) 'compose.yaml'
$composeFile = (Resolve-Path -LiteralPath $composeFile).Path
$prefix = @('compose', '--project-name', $Project, '--file', $composeFile)

function Get-CommandPlan {
    param([string]$Operation)
    $steps = [System.Collections.Generic.List[object]]::new()
    switch ($Operation) {
        'build' { $steps.Add(@{ arguments = @('build', 'hypermind'); condition = 'always' }) }
        'start' {
            if ($Build) { $steps.Add(@{ arguments = @('build', 'hypermind'); condition = 'always' }) }
            $steps.Add(@{ arguments = @('run', '--rm', '--no-deps', 'hypermind', 'init', '--path', '/var/lib/hypermind', '--if-missing'); condition = 'daemon_not_running' })
            $steps.Add(@{ arguments = @('up', '--detach', '--wait', '--scale', 'hypermind=1', 'hypermind'); condition = 'always' })
        }
        'stop' { $steps.Add(@{ arguments = @('stop', '--timeout', '30', 'hypermind'); condition = 'always' }) }
        'status' { $steps.Add(@{ arguments = @('ps', '--all', 'hypermind'); condition = 'always' }) }
        'logs' { $steps.Add(@{ arguments = @('logs', '--follow', '--tail', '100', 'hypermind'); condition = 'always' }) }
        'doctor' { $steps.Add(@{ arguments = @('exec', '--no-TTY', 'hypermind', '/usr/local/bin/hm', 'doctor', '--config', '/var/lib/hypermind/hypermind.conf', '--require-healthy', '--json'); condition = 'always' }) }
    }
    return ,$steps
}

$plan = Get-CommandPlan $Action
if ($Render) {
    @{ executable = 'docker'; prefix = $prefix; steps = @($plan.ToArray()); project = $Project } | ConvertTo-Json -Depth 8
    exit 0
}

if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
    throw 'Install and start Docker Desktop with the WSL 2 Linux-container backend; see deploy/windows/README.md.'
}
& docker compose version
if ($LASTEXITCODE -ne 0) { throw 'Docker Compose v2 is required.' }
$serverOS = & docker info --format '{{.OSType}}'
if ($LASTEXITCODE -ne 0 -or "$serverOS".Trim() -ne 'linux') {
    throw 'Docker Desktop must be running in Linux-container mode.'
}

if ($Action -eq 'doctor' -and $env:OS -eq 'Windows_NT') {
    & wsl.exe --version
    if ($LASTEXITCODE -ne 0) { throw 'WSL is unavailable; follow the Microsoft WSL installation guide.' }
}

foreach ($step in $plan) {
    if ($step.condition -eq 'daemon_not_running') {
        $running = & docker @prefix ps --status running --quiet hypermind
        if ($LASTEXITCODE -ne 0) { throw 'Cannot determine whether the daemon already owns the persistent volume.' }
        if ($running) { continue }
    }
    $arguments = @($prefix) + @($step.arguments)
    & docker @arguments
    if ($LASTEXITCODE -ne 0) { throw "Docker action '$Action' failed with exit code $LASTEXITCODE." }
}
