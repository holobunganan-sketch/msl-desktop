[CmdletBinding()]
param()

# Runs an instrumented copy of the generated NSIS installer. All writable
# registry keys, shell folders, process names and data paths are test-specific.
# The production installer and the user's installed application are never run.
$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$generatedDir = Join-Path $projectRoot 'src-tauri\target\release\nsis\x64'
$compiler = Join-Path $env:LOCALAPPDATA 'tauri\NSIS\makensis.exe'
$source = Get-Content -LiteralPath (Join-Path $generatedDir 'installer.nsi') -Raw
if ($source -notmatch '!define PRODUCTNAME "MSL Desktop"' -or $source -notmatch 'MigrateLegacyProductShortcuts') {
    throw 'Build the updated release installer before running this test.'
}
$id = [Guid]::NewGuid().ToString('N')
$runtime = Join-Path ([IO.Path]::GetTempPath()) "msl-installer-name-$id"
if ($runtime.Contains('$') -or $runtime.Contains('"')) { throw 'Unsupported test directory characters.' }
$regRoot = "Software\MSLDesktopInstallerTests\$id"
$uninstallKey = "$regRoot\uninstall\msl-desktop"
$vendorKey = "$regRoot\vendor"
$install = Join-Path $runtime 'installed'
$programs = Join-Path $runtime 'start-menu'
$desktop = Join-Path $runtime 'desktop'
$appData = Join-Path $runtime 'appdata'
$localData = Join-Path $runtime 'localappdata'
$temp = Join-Path $runtime 'temp'
$tmp = Join-Path $runtime 'tmp'
$probeName = "msl-installer-probe-$id"
foreach ($dir in @($runtime,$install,$programs,$desktop,$appData,$localData,$temp,$tmp)) {
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
}
$replacements = [ordered]@{
    '$SMPROGRAMS' = $programs
    '$DESKTOP' = $desktop
    '$LOCALAPPDATA' = $localData
    '$APPDATA' = $appData
    '$TEMP' = $temp
}
foreach ($entry in $replacements.GetEnumerator()) { $source = $source.Replace($entry.Key, $entry.Value) }
function Replace-Define([string]$Text,[string]$Name,[string]$Value) {
    $pattern = '(?m)^!define ' + [regex]::Escape($Name) + ' "[^"\r\n]*"'
    if ([regex]::Matches($Text,$pattern).Count -ne 1) { throw "Unexpected define: $Name" }
    return [regex]::Replace($Text,$pattern,('!define ' + $Name + ' "' + $Value + '"'))
}
$source = Replace-Define $source 'UNINSTKEY' $uninstallKey
$source = Replace-Define $source 'MANUKEY' $vendorKey
$source = Replace-Define $source 'MAINBINARYNAME' $probeName
$source = Replace-Define $source 'BUNDLEID' "com.msl.installertest.$id"
$source = Replace-Define $source 'OUTFILE' (Join-Path $runtime 'setup-test.exe')
$source = Replace-Define $source 'INSTALLWEBVIEW2MODE' 'skip'
$source = $source.Replace('Software\Microsoft\Windows\CurrentVersion\Run', "$regRoot\run")
foreach ($write in [regex]::Matches($source,'(?m)^\s*(WriteReg\w+|DeleteReg\w+)\s+\w+\s+"(Software\\[^"\r\n]*)"')) {
    if (-not $write.Groups[2].Value.StartsWith($regRoot + '\')) { throw 'Unmapped literal registry write in test installer.' }
}
foreach ($include in @('utils.nsh','FileAssociation.nsh')) {
    Copy-Item -LiteralPath (Join-Path $generatedDir $include) -Destination (Join-Path $runtime $include)
}
$utf8 = [Text.UTF8Encoding]::new($false)
[IO.File]::WriteAllText((Join-Path $runtime 'installer-test.nsi'),$source,$utf8)
& $compiler /V2 (Join-Path $runtime 'installer-test.nsi')
if ($LASTEXITCODE -ne 0) { throw 'Isolated installer compilation failed.' }

$oldEnv = @{}
foreach ($name in @('APPDATA','LOCALAPPDATA','TEMP','TMP','MSL_ISOLATED_TEST')) {
    $oldEnv[$name] = [Environment]::GetEnvironmentVariable($name,'Process')
}
$registry = [Microsoft.Win32.RegistryKey]::OpenBaseKey('CurrentUser','Registry64')
$sentinelDir = Join-Path $appData 'MSLDesktop'
New-Item -ItemType Directory -Path $sentinelDir | Out-Null
$sentinel = Join-Path $sentinelDir 'keep-work-data.txt'
[IO.File]::WriteAllText($sentinel,'synthetic work data must survive',$utf8)
$sentinelHash = (Get-FileHash -LiteralPath $sentinel).Hash

function Run-Probe([string]$File,[string[]]$Arguments) {
    $p = Start-Process -FilePath $File -ArgumentList $Arguments -WorkingDirectory $runtime -WindowStyle Hidden -PassThru
    if (-not $p.WaitForExit(60000)) { Stop-Process -Id $p.Id; throw 'Test installer timed out.' }
    $p.Refresh()
    if ($p.ExitCode -ne 0) { throw "Test installer exited with $($p.ExitCode)" }
}
try {
    $env:APPDATA=$appData; $env:LOCALAPPDATA=$localData; $env:TEMP=$temp; $env:TMP=$tmp; $env:MSL_ISOLATED_TEST='1'
    # Seed an old installation using only this test's registry namespace.
    $key = $registry.CreateSubKey("$vendorKey\msl-desktop")
    $key.SetValue('', $install); $key.Dispose()
    $key = $registry.CreateSubKey($uninstallKey)
    $key.SetValue('DisplayName','msl-desktop')
    $key.SetValue('DisplayVersion','0.3.11')
    $key.SetValue('MainBinaryName', "$probeName.exe")
    $key.Dispose()
    Copy-Item -LiteralPath (Join-Path $projectRoot 'src-tauri\target\release\msl-desktop.exe') -Destination (Join-Path $install "$probeName.exe")
    $shell = New-Object -ComObject WScript.Shell
    foreach ($folder in @($programs,$desktop)) {
        $link = $shell.CreateShortcut((Join-Path $folder 'msl-desktop.lnk'))
        $link.TargetPath = Join-Path $install "$probeName.exe"
        $link.Save()
    }
    Run-Probe (Join-Path $runtime 'setup-test.exe') @('/S','/UPDATE')
    $key = $registry.OpenSubKey($uninstallKey)
    if ($null -eq $key -or $key.GetValue('DisplayName') -cne 'MSL Desktop') { throw 'Installed display name mismatch.' }
    $installedVersion = $key.GetValue('DisplayVersion'); $key.Dispose()
    foreach ($folder in @($programs,$desktop)) {
        $newPath = Join-Path $folder 'MSL Desktop.lnk'
        if (-not (Test-Path -LiteralPath $newPath)) { throw "New shortcut missing: $newPath" }
        if (Test-Path -LiteralPath (Join-Path $folder 'msl-desktop.lnk')) { throw 'Legacy shortcut was not migrated.' }
        if ($shell.CreateShortcut($newPath).TargetPath -ne (Join-Path $install "$probeName.exe")) { throw 'Wrong shortcut target.' }
    }
    if ((Get-FileHash -LiteralPath $sentinel).Hash -ne $sentinelHash) { throw 'Work data changed during upgrade.' }
    # Repeating the update must remain idempotent.
    Run-Probe (Join-Path $runtime 'setup-test.exe') @('/S','/UPDATE')
    if ((Get-ChildItem -LiteralPath $programs -Filter '*.lnk').Count -ne 1) { throw 'Duplicate start-menu entry.' }
    # Normal installation must not overwrite an unrelated same-name shortcut.
    $unrelatedTarget = Join-Path $runtime 'unrelated-application.exe'
    foreach ($folder in @($programs,$desktop)) {
        $link = $shell.CreateShortcut((Join-Path $folder 'MSL Desktop.lnk'))
        $link.TargetPath = $unrelatedTarget
        $link.Save()
    }
    Run-Probe (Join-Path $runtime 'setup-test.exe') @('/S')
    foreach ($folder in @($programs,$desktop)) {
        if ($shell.CreateShortcut((Join-Path $folder 'MSL Desktop.lnk')).TargetPath -ne $unrelatedTarget) {
            throw 'Unrelated same-name shortcut was overwritten.'
        }
    }
    if ((Get-FileHash -LiteralPath $sentinel).Hash -ne $sentinelHash) { throw 'Work data changed during normal installation.' }
    [pscustomobject]@{ Result='PASS'; DisplayName='MSL Desktop'; Version=$installedVersion; LegacyShortcutsMigrated=$true; RepeatedUpgrade=$true; UnrelatedShortcutsPreserved=$true; WorkDataUnchanged=$true; Runtime=$runtime } | ConvertTo-Json -Compress
}
finally {
    foreach ($name in $oldEnv.Keys) { [Environment]::SetEnvironmentVariable($name,$oldEnv[$name],'Process') }
    if ($regRoot -notmatch '^Software\\MSLDesktopInstallerTests\\[a-f0-9]{32}$') { throw 'Unsafe registry cleanup target.' }
    $registry.DeleteSubKeyTree($regRoot,$false)
    $registry.Dispose()
}
