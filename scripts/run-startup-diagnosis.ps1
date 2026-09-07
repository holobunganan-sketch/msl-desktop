[CmdletBinding()]
param(
    [int] $DebugPort = 9350,
    [ValidateSet('debug', 'release')]
    [string] $Profile = 'debug'
)

$ErrorActionPreference = 'Stop'

$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$diagnosisRoot = Join-Path $projectRoot '.test-runtime\startup-diagnosis'
$appData = Join-Path $diagnosisRoot 'appdata'
$localAppData = Join-Path $diagnosisRoot 'localappdata'
$temp = Join-Path $diagnosisRoot 'temp'
$artifacts = Join-Path $diagnosisRoot 'artifacts'
$executable = Join-Path $projectRoot "src-tauri\target\$Profile\msl-desktop.exe"

foreach ($path in @($appData, $localAppData, $temp, $artifacts)) {
    New-Item -ItemType Directory -Force -Path $path | Out-Null
}

$environment = @{
    APPDATA = $appData
    LOCALAPPDATA = $localAppData
    TEMP = $temp
    TMP = $temp
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$DebugPort"
}
$stdout = Join-Path $artifacts 'stdout.log'
$stderr = Join-Path $artifacts 'stderr.log'
$process = Start-Process `
    -FilePath $executable `
    -ArgumentList "--remote-debugging-port=$DebugPort" `
    -WorkingDirectory $projectRoot `
    -WindowStyle Hidden `
    -RedirectStandardOutput $stdout `
    -RedirectStandardError $stderr `
    -Environment $environment `
    -PassThru

Start-Sleep -Seconds 6
$process.Refresh()
$cdpAvailable = $false
try {
    $cdpAvailable = (Invoke-WebRequest "http://127.0.0.1:$DebugPort/json" -UseBasicParsing -TimeoutSec 2).StatusCode -eq 200
} catch {
    $cdpAvailable = $false
}

[pscustomobject]@{
    Pid = $process.Id
    HasExited = $process.HasExited
    ExitCode = if ($process.HasExited) { $process.ExitCode } else { $null }
    Stdout = $stdout
    StdoutBytes = if (Test-Path $stdout) { (Get-Item $stdout).Length } else { 0 }
    Stderr = $stderr
    StderrBytes = if (Test-Path $stderr) { (Get-Item $stderr).Length } else { 0 }
    CdpAvailable = $cdpAvailable
} | ConvertTo-Json -Compress
