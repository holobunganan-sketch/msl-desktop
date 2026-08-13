# measure-memory.ps1
#
# MSL Desktop 内存测量脚本（指南 §15 / §4）。
#
# 功能：
#   - 找到 MSL Desktop 主进程（默认 msl-desktop）；
#   - 递归统计其全部子进程（含 WebView2 子进程，按 ParentProcessId 归属）；
#   - 输出 Working Set 与 Private Memory（主进程 / 子进程 / 合计）；
#   - 通过主窗口标题标记当前状态：窗口打开 或 托盘常驻（窗口已销毁）；
#   - 每次测量追加一行到 results CSV，保留多次测量历史。
#
# 用法：
#   powershell -ExecutionPolicy Bypass -File .\scripts\measure-memory.ps1
#   powershell -ExecutionPolicy Bypass -File .\scripts\measure-memory.ps1 -Iterations 5 -IntervalMs 2000
#   powershell -ExecutionPolicy Bypass -File .\scripts\measure-memory.ps1 -Tag "10th-reopen"
#
# 输出文件：scripts\measure-results.csv（与脚本同目录，追加）

param(
    [string]$ProcessName = "msl-desktop",
    [int]$Iterations = 1,
    [int]$IntervalMs = 1000,
    [string]$Tag = "manual"
)

$ErrorActionPreference = "Stop"

# 递归收集某进程的全部后代进程
function Get-DescendantProcesses {
    param([int]$ParentId)

    $all = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue)
    $result = [System.Collections.Generic.List[object]]::new()
    $queue = [System.Collections.Generic.Queue[int]]::new()
    $queue.Enqueue($ParentId)

    while ($queue.Count -gt 0) {
        $currentId = $queue.Dequeue()
        foreach ($p in $all) {
            if ($p.ParentProcessId -eq $currentId) {
                $result.Add($p)
                $queue.Enqueue($p.ProcessId)
            }
        }
    }
    return $result
}

function Get-Measurement {
    $main = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue | Where-Object {
        $_.SessionId -eq (Get-Process -Id $PID).SessionId
    } | Select-Object -First 1

    if (-not $main) {
        return $null
    }

    $children = @(Get-DescendantProcesses -ParentId $main.Id)
    $childIds = @($children | ForEach-Object { $_.ProcessId })

    # 主进程内存（Get-Process 的 WorkingSet64 / PrivateMemorySize64）
    $mainWs = $main.WorkingSet64
    $mainPriv = $main.PrivateMemorySize64

    # 子进程内存（用 Get-Process 按 PID 批量取，避免 Win32_Process 的 WorkingSet 精度问题）
    $childWs = 0L
    $childPriv = 0L
    if ($childIds.Count -gt 0) {
        foreach ($proc in @(Get-Process -Id $childIds -ErrorAction SilentlyContinue)) {
            $childWs += $proc.WorkingSet64
            $childPriv += $proc.PrivateMemorySize64
        }
    }

    # 状态判定：主窗口标题存在 => 窗口打开；否则 => 托盘常驻。
    # （msl-desktop 主窗口标题固定为 "MSL Desktop"；窗口销毁后标题为空）
    $state = if ($main.MainWindowTitle -ne "") { "window-open" } else { "tray" }

    return [pscustomobject]@{
        Timestamp     = (Get-Date -Format "yyyy-MM-dd HH:mm:ss")
        Tag           = $Tag
        State         = $state
        MainPid       = $main.Id
        MainWindow    = $main.MainWindowTitle
        ChildCount    = $childIds.Count
        MainWsMB      = [math]::Round($mainWs / 1MB, 2)
        MainPrivMB    = [math]::Round($mainPriv / 1MB, 2)
        ChildWsMB     = [math]::Round($childWs / 1MB, 2)
        ChildPrivMB   = [math]::Round($childPriv / 1MB, 2)
        TotalWsMB     = [math]::Round(($mainWs + $childWs) / 1MB, 2)
        TotalPrivMB   = [math]::Round(($mainPriv + $childPriv) / 1MB, 2)
    }
}

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$csvPath = Join-Path $scriptDir "measure-results.csv"

# 首次运行写表头
if (-not (Test-Path $csvPath)) {
    "Timestamp,Tag,State,MainPid,MainWindow,ChildCount,MainWsMB,MainPrivMB,ChildWsMB,ChildPrivMB,TotalWsMB,TotalPrivMB" |
        Set-Content -Path $csvPath -Encoding UTF8
}

for ($i = 1; $i -le $Iterations; $i++) {
    $m = Get-Measurement
    if (-not $m) {
        Write-Host "[$i/$Iterations] 未找到进程 '$ProcessName'（可能未运行）" -ForegroundColor Yellow
    } else {
        $line = "{0},{1},{2},{3},{4},{5},{6},{7},{8},{9},{10},{11}" -f
            $m.Timestamp, $m.Tag, $m.State, $m.MainPid, $m.MainWindow, $m.ChildCount,
            $m.MainWsMB, $m.MainPrivMB, $m.ChildWsMB, $m.ChildPrivMB, $m.TotalWsMB, $m.TotalPrivMB
        Add-Content -Path $csvPath -Value $line -Encoding UTF8

        Write-Host "[$i/$Iterations] $($m.Timestamp)  state=$($m.State)  tag=$Tag"
        Write-Host ("   主进程  PID={0}  窗口='{1}'" -f $m.MainPid, $m.MainWindow)
        Write-Host ("   子进程数={0}" -f $m.ChildCount)
        Write-Host ("   Working Set   main={0} MB  child={1} MB  TOTAL={2} MB" -f $m.MainWsMB, $m.ChildWsMB, $m.TotalWsMB)
        Write-Host ("   Private Mem   main={0} MB  child={1} MB  TOTAL={2} MB" -f $m.MainPrivMB, $m.ChildPrivMB, $m.TotalPrivMB)
    }

    if ($i -lt $Iterations) {
        Start-Sleep -Milliseconds $IntervalMs
    }
}

Write-Host ""
Write-Host "结果已追加到: $csvPath"
