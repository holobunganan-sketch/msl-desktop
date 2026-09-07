[CmdletBinding()]
param(
    [ValidateSet('debug', 'release')]
    [string] $Profile = 'debug',
    [int] $DebugPort = 0,
    [string] $ExePath,
    [ValidatePattern('^[a-zA-Z0-9_-]+$')]
    [string] $RuntimeName = 'luna-ai-secretary',
    [switch] $DryRun
)

$ErrorActionPreference = 'Stop'

function Resolve-CheckedPath([string] $Path, [string] $Root) {
    $full = [IO.Path]::GetFullPath($Path)
    $rootWithSep = $Root.TrimEnd('\') + '\'
    if ($full -ne $Root -and -not $full.StartsWith($rootWithSep, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Path is outside project root: $full"
    }
    return $full
}

$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$runtimeRoot = Resolve-CheckedPath (Join-Path $projectRoot ".test-runtime\$RuntimeName") $projectRoot
$appData = Resolve-CheckedPath (Join-Path $runtimeRoot 'appdata') $projectRoot
$localAppData = Resolve-CheckedPath (Join-Path $runtimeRoot 'localappdata') $projectRoot
$temp = Resolve-CheckedPath (Join-Path $runtimeRoot 'temp') $projectRoot
$tmp = $temp
$workspace = Resolve-CheckedPath (Join-Path $runtimeRoot 'workspace') $projectRoot
$artifacts = Resolve-CheckedPath (Join-Path $runtimeRoot 'artifacts') $projectRoot
$mockProvider = Resolve-CheckedPath (Join-Path $runtimeRoot 'mock-provider') $projectRoot
$cacheRoot = Resolve-CheckedPath (Join-Path $localAppData 'MSLDesktop\cache') $projectRoot

foreach ($directory in @($appData, $localAppData, $temp, $workspace, $artifacts, $mockProvider)) {
    New-Item -ItemType Directory -Force -Path $directory | Out-Null
}

if (-not $ExePath) {
    $ExePath = Join-Path $projectRoot "src-tauri\target\$Profile\msl-desktop.exe"
}
$exe = Resolve-CheckedPath $ExePath $projectRoot
$targetRoot = Resolve-CheckedPath (Join-Path $projectRoot 'src-tauri\target') $projectRoot
$targetPrefix = $targetRoot.TrimEnd('\') + '\'
if (-not $exe.StartsWith($targetPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Executable must be under src-tauri\target: $exe"
}

$argumentList = @()
if ($DebugPort -gt 0) {
    $argumentList += "--remote-debugging-port=$DebugPort"
}

if ($DryRun) {
    [pscustomobject]@{
        ProjectRoot = $projectRoot
        AppData = $appData
        LocalAppData = $localAppData
        Temp = $temp
        Tmp = $tmp
        Workspace = $workspace
        Artifacts = $artifacts
        MockProvider = $mockProvider
        Executable = $exe
        ExpectedDatabase = (Join-Path $appData 'MSLDesktop\msl-desktop.db')
        ExpectedCacheRoot = $cacheRoot
        Arguments = ($argumentList -join ' ')
        WouldStart = (Test-Path -LiteralPath $exe)
    } | ConvertTo-Json -Compress
    exit 0
}

if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
    throw "Executable does not exist: $exe"
}

$previousAppData = $env:APPDATA
$previousLocalAppData = $env:LOCALAPPDATA
$previousTemp = $env:TEMP
$previousTmp = $env:TMP
$previousWebviewArgs = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
$previousWebviewData = $env:WEBVIEW2_USER_DATA_FOLDER
$previousTestFlag = $env:MSL_ISOLATED_TEST
$env:APPDATA = $appData
$env:LOCALAPPDATA = $localAppData
$env:TEMP = $temp
$env:TMP = $tmp
$env:MSL_ISOLATED_TEST = '1'
$env:WEBVIEW2_USER_DATA_FOLDER = Join-Path $localAppData 'MSLDesktop\test-webview'
try {
    if ($DebugPort -gt 0) { $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$DebugPort" }
    $process = Start-Process -FilePath $exe -ArgumentList $argumentList -WorkingDirectory $projectRoot -WindowStyle Hidden -PassThru
    [pscustomobject]@{
        Pid = $process.Id
        Executable = $exe
        IsolatedAppData = $appData
        IsolatedLocalAppData = $localAppData
        IsolatedTemp = $temp
        IsolatedTmp = $tmp
        IsolatedDatabase = (Join-Path $appData 'MSLDesktop\msl-desktop.db')
        IsolatedCacheRoot = $cacheRoot
        Workspace = $workspace
        Artifacts = $artifacts
        MockProvider = $mockProvider
    } | ConvertTo-Json -Compress
}
finally {
    $env:APPDATA = $previousAppData
    $env:LOCALAPPDATA = $previousLocalAppData
    $env:TEMP = $previousTemp
    $env:TMP = $previousTmp
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $previousWebviewArgs
    $env:WEBVIEW2_USER_DATA_FOLDER = $previousWebviewData
    $env:MSL_ISOLATED_TEST = $previousTestFlag
}
