[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateRange(1, 2147483647)]
    [int] $ProcessId,
    [switch] $DryRun
)

$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$targetRoot = (Resolve-Path (Join-Path $projectRoot 'src-tauri\target')).Path
$targetPrefix = $targetRoot.TrimEnd('\') + '\'
$process = Get-Process -Id $ProcessId -ErrorAction SilentlyContinue
if (-not $process) {
    [pscustomobject]@{ Pid = $ProcessId; Stopped = $false; Reason = 'not-running' } | ConvertTo-Json -Compress
    exit 0
}

$path = $null
try { $path = $process.Path } catch { }
if (-not $path) {
    throw "Cannot verify executable path for PID $ProcessId"
}
$fullPath = [IO.Path]::GetFullPath($path)
if (-not $fullPath.StartsWith($targetPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to stop PID $ProcessId outside project target: $fullPath"
}
if (-not ([IO.Path]::GetFileName($fullPath) -ieq 'msl-desktop.exe')) {
    throw "Refusing to stop non-MSL executable: $fullPath"
}

if ($DryRun) {
    [pscustomobject]@{ Pid = $ProcessId; Executable = $fullPath; WouldStop = $true } | ConvertTo-Json -Compress
    exit 0
}

Stop-Process -Id $ProcessId
[pscustomobject]@{ Pid = $ProcessId; Executable = $fullPath; Stopped = $true } | ConvertTo-Json -Compress
