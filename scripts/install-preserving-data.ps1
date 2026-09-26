[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Installer,
    [Parameter(Mandatory)][ValidatePattern('^\d+\.\d+\.\d+$')][string]$ExpectedVersion
)

# This command explicitly upgrades the real installation. It never starts the app.
$ErrorActionPreference = 'Stop'
if ($env:MSL_ISOLATED_TEST -eq '1') { throw 'Do not run the real upgrade from a test profile.' }
$package = (Resolve-Path -LiteralPath $Installer).Path
$registryPath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\msl-desktop'
$installed = Get-ItemProperty -LiteralPath $registryPath
$installDir = [IO.Path]::GetFullPath($installed.InstallLocation.Trim('"'))
$binary = Join-Path $installDir 'msl-desktop.exe'
if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) { throw 'Existing application binary was not found.' }
if (Get-Process -Name 'msl-desktop' -ErrorAction SilentlyContinue) { throw 'Close MSL Desktop, including the tray process, before upgrading.' }
$sourceBinary = Join-Path $PSScriptRoot '..\src-tauri\target\release\msl-desktop.exe'
if ((Get-Item -LiteralPath $sourceBinary).VersionInfo.ProductVersion -ne $ExpectedVersion) { throw 'Build version does not match the requested upgrade.' }
if ([IO.Path]::GetFileName($package) -cne "MSL Desktop_${ExpectedVersion}_x64-setup.exe") { throw 'Unexpected installer filename.' }

$dataDir = Join-Path $env:APPDATA 'MSLDesktop'
$backupRoot = Join-Path $env:LOCALAPPDATA ('MSLDesktopInstallBackups\v' + $ExpectedVersion + '-' + [DateTime]::Now.ToString('yyyyMMdd-HHmmss'))
if (Test-Path -LiteralPath $backupRoot) { throw 'Backup destination already exists.' }
New-Item -ItemType Directory -Path $backupRoot | Out-Null
function Data-Fingerprints {
    $result = @{}
    if (Test-Path -LiteralPath $dataDir) {
        foreach ($file in Get-ChildItem -LiteralPath $dataDir -File -Recurse) {
            $relative = [IO.Path]::GetRelativePath($dataDir,$file.FullName)
            $result[$relative] = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash
        }
    }
    return $result
}
$before = Data-Fingerprints
if (Test-Path -LiteralPath $dataDir) {
    $backupData = Join-Path $backupRoot 'data'
    Copy-Item -LiteralPath $dataDir -Destination $backupData -Recurse
    foreach ($relative in $before.Keys) {
        if ((Get-FileHash -LiteralPath (Join-Path $backupData $relative) -Algorithm SHA256).Hash -ne $before[$relative]) { throw 'Recovery copy verification failed.' }
    }
}
Copy-Item -LiteralPath $binary -Destination (Join-Path $backupRoot 'previous-msl-desktop.exe')

# /UPDATE preserves existing data and startup settings; omitting /R prevents launch.
$process = Start-Process -FilePath $package -ArgumentList @('/S','/UPDATE') -WindowStyle Hidden -PassThru
if (-not $process.WaitForExit(120000)) { throw 'Installer is still running; inspect it before retrying.' }
$process.Refresh()
if ($process.ExitCode -ne 0) { throw "Installer failed with exit code $($process.ExitCode). Recovery copy: $backupRoot" }
$current = Get-ItemProperty -LiteralPath $registryPath
if ($current.DisplayName -cne 'MSL Desktop' -or $current.DisplayVersion -ne $ExpectedVersion) { throw 'Installed application identity does not match the release.' }
& node (Join-Path $PSScriptRoot 'verify-installed-binary.mjs') $sourceBinary $binary
if ($LASTEXITCODE -ne 0) { throw 'Installed binary differs from the verified local build.' }
$after = Data-Fingerprints
if ($before.Count -ne $after.Count) { throw 'Data file count changed during installation; recovery copy retained.' }
foreach ($relative in $before.Keys) {
    if ($after[$relative] -ne $before[$relative]) { throw 'A data file changed during installation; recovery copy retained.' }
}
[pscustomobject]@{Result='PASS';DisplayName=$current.DisplayName;Version=$current.DisplayVersion;DataFilesUnchanged=$after.Count;BackupDirectory=$backupRoot;AppStarted=$false} | ConvertTo-Json -Compress
