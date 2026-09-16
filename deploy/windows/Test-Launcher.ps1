Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$launcher = Join-Path $PSScriptRoot 'hypermind.ps1'
$tokens = $null
$parseErrors = $null
$null = [System.Management.Automation.Language.Parser]::ParseFile($launcher, [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count -ne 0) { throw ($parseErrors | Out-String) }
foreach ($action in @('start', 'stop', 'status', 'logs', 'doctor', 'build')) {
    $result = & (Get-Process -Id $PID).Path -NoProfile -File $launcher $action -Render | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0 -or $result.executable -ne 'docker') { throw "Cannot render $action" }
    if ($result.project -ne 'hypermind') { throw 'Project identity is unstable.' }
    foreach ($step in $result.steps) {
        if ($step.arguments -contains '--volumes' -or $step.arguments -contains '-v' -or $step.arguments -contains 'down') { throw 'Destructive volume removal is forbidden.' }
    }
    if ($action -eq 'start') {
        if ($result.steps[0].condition -ne 'daemon_not_running') { throw 'Initialization may overlap a running writer.' }
        if ($result.steps[0].arguments -notcontains '--if-missing') { throw 'Configuration overwrite is forbidden.' }
        if ($result.steps[1].arguments -notcontains 'hypermind=1') { throw 'Single-writer scale is missing.' }
    }
}
Write-Output 'PowerShell syntax and all six real command renderings passed; no Docker/service execution performed.'
