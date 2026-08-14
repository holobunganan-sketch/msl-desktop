# defender-scan.ps1 — 用 Microsoft Defender 扫描产物（指南 §26 / §12）
#
# 扫描对象：Release exe 与 NSIS setup.exe。
# 自定义扫描需要管理员权限；无权限时输出指引。
#
# 用法（管理员 PowerShell）：
#   powershell -ExecutionPolicy Bypass -File .\scripts\defender-scan.ps1
# 参数：
#   -ExePath  : 默认 src-tauri\target\release\msl-desktop.exe
#   -SetupPath: 默认 src-tauri\target\release\bundle\nsis\*-setup.exe（自动展开）
#   -Quiet    : 仅输出结论行

param(
    [string]$ExePath = "",
    [string]$SetupPath = "",
    [switch]$Quiet
)

$ErrorActionPreference = "Continue"
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Split-Path -Parent $scriptDir
if (-not $ExePath) { $ExePath = Join-Path $root "src-tauri\target\release\msl-desktop.exe" }
if (-not $SetupPath) {
    $nsisDir = Join-Path $root "src-tauri\target\release\bundle\nsis"
    $SetupPath = (Get-ChildItem (Join-Path $nsisDir "*-setup.exe") -ErrorAction SilentlyContinue | Select-Object -First 1).FullName
}

# 管理员检测
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

function Invoke-DefenderScan {
    param([string]$Path)
    if (-not (Test-Path $Path)) {
        Write-Host "[SKIP] 文件不存在: $Path"
        return $null
    }
    $scan = $null
    if (Get-Command Start-MpScan -ErrorAction SilentlyContinue) {
        try {
            $result = Start-MpScan -ScanType CustomScan -ScanPath $Path
            $scan = $result
        } catch {
            $scan = "Start-MpScan failed: $_"
        }
    } else {
        # 回退：MpCmdRun.exe
        $mp = "C:\Program Files\Windows Defender\MpCmdRun.exe"
        if (Test-Path $mp) {
            $out = & $mp -Scan -ScanType 3 -File $Path 2>&1
            $scan = ($out | Out-String)
        } else {
            $scan = "MpCmdRun not found"
        }
    }
    return $scan
}

Write-Host "=== Defender Scan ==="
Write-Host "Admin: $isAdmin"
Write-Host "Exe:   $ExePath"
Write-Host "Setup: $SetupPath"
if (-not $isAdmin) {
    Write-Host "警告: 自定义扫描需要管理员权限。请以管理员身份重新运行本脚本。"
}

foreach ($p in @($ExePath, $SetupPath)) {
    if (-not $p -or -not (Test-Path $p)) {
        Write-Host "[SKIP] 不存在: $p"
        continue
    }
    Write-Host "--- 扫描: $p ---"
    $out = Invoke-DefenderScan -Path $p
    if ($out) {
        if ($out -is [string]) { Write-Host $out }
        else { $out | Format-List * | Out-String | Write-Host }
    }
    Write-Host "[DONE] $p"
}

Write-Host ""
Write-Host "=== 判定 ==="
Write-Host "若输出含 Threat/检测（ThreatName），按指南 §26：立即停止安装验证、记录 detection name、检查依赖与打包行为；"
Write-Host "不得通过排除项强行忽略后宣布通过。"
Write-Host "若仅 SmartScreen '未知发布者' 提示，属 unsigned local build 预期限制，可继续人工验证。"
