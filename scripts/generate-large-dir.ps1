# generate-large-dir.ps1
#
# 生成 10,000 文件的大目录，用于 Stage 3 大目录性能测试（指南 §4.4 / §17）。
#
# 用法：
#   powershell -ExecutionPolicy Bypass -File .\scripts\generate-large-dir.ps1
#   默认输出到 %TEMP%\msl-large-dir，可用 -Output 覆盖
#   powershell -ExecutionPolicy Bypass -File .\scripts\generate-large-dir.ps1 -Output D:\perf-dir -Count 10000

param(
    [string]$Output = (Join-Path $env:TEMP "msl-large-dir"),
    [int]$Count = 10000
)

$ErrorActionPreference = "Stop"

if (Test-Path $Output) {
    Write-Host "删除旧目录: $Output"
    Remove-Item -Recurse -Force $Output
}
New-Item -ItemType Directory -Path $Output -Force | Out-Null

# 生成 50 个子目录，每个约 Count/50 个文件
$dirs = 50
$perDir = [math]::Ceiling($Count / $dirs)

Write-Host "生成 $Count 个文件到 $Output ..."
$total = 0
for ($d = 0; $d -lt $dirs; $d++) {
    $dir = Join-Path $Output ("dir-{0:D2}" -f $d)
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
    for ($f = 0; $f -lt $perDir -and $total -lt $Count; $f++) {
        $file = Join-Path $dir ("file-{0:D4}.txt" -f $f)
        Set-Content -Path $file -Value ("dummy {0}" -f $total) -Encoding ASCII
        $total++
    }
}

Write-Host ("完成：{0} 个文件，{1} 个目录" -f $total, $dirs)
