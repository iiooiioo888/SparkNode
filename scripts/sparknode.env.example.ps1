# 範例：複製為 sparknode.env.ps1，或執行 setup-env.ps1 自動產生
# 用法:  . .\scripts\sparknode.env.ps1

$env:SPARKNODE_ROOT      = "E:\Jerry_python\SparkNode"
$env:SPARKNODE_RUST_ROOT = "E:\rust-toolchain"
$env:RUSTUP_HOME         = "$env:SPARKNODE_RUST_ROOT\.rustup"
$env:CARGO_HOME          = "$env:SPARKNODE_RUST_ROOT\.cargo"
$env:CARGO_TARGET_DIR    = "$env:SPARKNODE_RUST_ROOT\target"

$env:PATH = "$env:CARGO_HOME\bin;$env:USERPROFILE\.cargo\bin;" + $env:PATH

Write-Host "SparkNode 環境已載入（範例檔，請執行 setup-env.ps1 產生正式設定）" -ForegroundColor Yellow
