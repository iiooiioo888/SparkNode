# 相容舊指令：轉呼叫 setup-env.ps1（僅 Rust + 編譯）
# 建議改用: .\scripts\setup-env.ps1

Write-Host "fix-rust-toolchain.ps1 已合併至 setup-env.ps1，正在轉接..." -ForegroundColor Yellow
& "$PSScriptRoot\setup-env.ps1" -SkipDocker -SkipDeps @args
