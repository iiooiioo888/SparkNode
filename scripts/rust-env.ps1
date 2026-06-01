# 載入 SparkNode 開發環境（目前 PowerShell 工作階段）
# 用法:  . .\scripts\rust-env.ps1

$envScript = Join-Path $PSScriptRoot "sparknode.env.ps1"

if (-not (Test-Path $envScript)) {
    Write-Host "尚未產生 sparknode.env.ps1，請先執行: .\scripts\setup-env.ps1" -ForegroundColor Yellow
    . "$PSScriptRoot\config.ps1"
    $root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
    $rustRoot = if ($env:SPARKNODE_RUST_ROOT) { $env:SPARKNODE_RUST_ROOT } else { "E:\rust-toolchain" }
    $paths = Initialize-SparkNodePaths -RustRoot $rustRoot -ProjectRoot $root
    Set-SparkNodeSessionEnv -Paths $paths
    Write-Host "已載入臨時環境（建議執行 setup-env.ps1 完成完整配置）" -ForegroundColor Yellow
    return
}

. $envScript
