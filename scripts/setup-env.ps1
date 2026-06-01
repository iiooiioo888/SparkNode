# ═══════════════════════════════════════════════════
#  SparkNode 開發環境一鍵配置
#
#  建議以「系統管理員」PowerShell 執行（可自動加入 Defender 排除）：
#    Set-ExecutionPolicy -Scope Process Bypass
#    cd E:\Jerry_python\SparkNode
#    .\scripts\setup-env.ps1
#
#  參數：
#    -RustRoot "E:\rust-toolchain"   Rust/Cargo 安裝目錄
#    -SkipRust                       跳過 Rust 安裝與 cargo check
#    -SkipDocker                     跳過 docker compose 與 migrations
#    -SkipDeps                       跳過 npm / pip 安裝
#    -SkipBuild                      跳過 cargo check
#    -NoUserEnv                      不寫入使用者永久環境變數
# ═══════════════════════════════════════════════════

[CmdletBinding()]
param(
    [string]$RustRoot = $(if ($env:SPARKNODE_RUST_ROOT) { $env:SPARKNODE_RUST_ROOT } else { "E:\rust-toolchain" }),
    [string]$ProjectRoot = "",
    [switch]$SkipRust,
    [switch]$SkipDocker,
    [switch]$SkipDeps,
    [switch]$SkipBuild,
    [switch]$NoUserEnv
)

$ErrorActionPreference = "Stop"

. "$PSScriptRoot\config.ps1"

if ($ProjectRoot) {
    $resolvedRoot = (Resolve-Path $ProjectRoot).Path
} else {
    $resolvedRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
}

$Paths = Initialize-SparkNodePaths -RustRoot $RustRoot -ProjectRoot $resolvedRoot

function Write-Step { param($n, $msg) Write-Host "`n=== $n. $msg ===" -ForegroundColor Cyan }
function Write-Ok   { param($msg) Write-Host "  [OK] $msg" -ForegroundColor Green }
function Write-Warn { param($msg) Write-Host "  [!!] $msg" -ForegroundColor Yellow }
function Write-Err  { param($msg) Write-Host "  [XX] $msg" -ForegroundColor Red }

Write-Host ""
Write-Host "SparkNode 環境配置" -ForegroundColor White
Write-Host "  專案: $($Paths.ProjectRoot)"
Write-Host "  Rust: $($Paths.RustRoot)"
Write-Host ""

# ── 0. 磁碟空間檢查 ─────────────────────────────
Write-Step 0 "檢查磁碟空間"
$cDrive = Get-PSDrive C -ErrorAction SilentlyContinue
if ($cDrive -and ($cDrive.Free / 1GB) -lt 2) {
    Write-Warn "C 槽可用空間不足 2GB（目前約 $([math]::Round($cDrive.Free/1GB,2)) GB）。Rust 將安裝在 $RustRoot"
}
$rustDriveName = [System.IO.Path]::GetPathRoot($RustRoot).TrimEnd('\', ':')
$rustDrive = Get-PSDrive $rustDriveName -ErrorAction SilentlyContinue
if ($rustDrive -and ($rustDrive.Free / 1GB) -lt 3) {
    Write-Err "${rustDriveName}: 槽空間不足 3GB，無法安裝工具鏈"
    exit 1
}
Write-Ok "磁碟檢查完成"

# ── 1. 建立目錄 ─────────────────────────────────
Write-Step 1 "建立目錄"
@($Paths.RustupHome, $Paths.CargoHome, $Paths.CargoTargetDir) | ForEach-Object {
    New-Item -ItemType Directory -Force -Path $_ | Out-Null
}
Write-Ok "Rust 目錄已就緒"

# ── 2. Defender 排除 ─────────────────────────────
Write-Step 2 "Windows Defender 排除（需管理員）"
Add-DefenderExclusionSafe -Paths $SparkNodeConfig.DefenderExclusions

# ── 3. .sql 寫入測試 ────────────────────────────
Write-Step 3 "測試防毒是否阻擋 .sql（sqlx 編譯需要）"
if (-not (Test-SqlFileWritable -Directory $Paths.RustRoot)) {
    Write-Err ".sql 檔案無法寫入 $RustRoot"
    Write-Host @"

請手動處理後重新執行本腳本：
  1. Windows 安全性 → 病毒與威脅防護 → 管理設定 → 排除項目
     加入：$($Paths.RustRoot) 與 $($Paths.ProjectRoot)
  2. 或暫時關閉「受控資料夾存取」

"@ -ForegroundColor Yellow
    exit 1
}
Write-Ok ".sql 寫入測試通過"

# ── 4. 工作階段 + 永久環境變數 ───────────────────
Write-Step 4 "設定環境變數"
Set-SparkNodeSessionEnv -Paths $Paths
if (-not $NoUserEnv) {
    Set-SparkNodeUserEnv -Paths $Paths
    Write-Ok "已寫入使用者環境變數（新終端機生效）"
} else {
    Write-Warn "略過永久環境變數（僅目前工作階段）"
}
Write-SparkNodeEnvScript -Paths $Paths
Write-Ok "已產生 $($Paths.EnvScript)"

# ── 5. 專案 .env ─────────────────────────────────
Write-Step 5 "專案 .env"
if (-not (Test-Path $Paths.EnvFile)) {
    if (Test-Path $Paths.EnvExample) {
        Copy-Item $Paths.EnvExample $Paths.EnvFile
        Write-Ok "已從 .env.example 建立 .env"
    } else {
        Write-Warn "找不到 .env.example"
    }
} else {
    Write-Ok ".env 已存在，未覆寫"
}

# ── 6. Rust 工具鏈 ───────────────────────────────
if (-not $SkipRust) {
    Write-Step 6 "安裝 / 更新 Rust stable (minimal)"
    if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
        Write-Err "找不到 rustup，請先安裝 https://rustup.rs 後重試"
        exit 1
    }
    rustup set profile minimal
    rustup toolchain install stable --force
    Write-Ok "rustc $(rustc -V)"
    Write-Ok "cargo $(cargo -V)"
} else {
    Write-Step 6 "跳過 Rust 安裝"
}

# ── 7. Docker 與資料庫 ───────────────────────────
if (-not $SkipDocker) {
    Write-Step 7 "Docker 基礎設施"
    if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
        Write-Warn "未安裝 Docker，跳過 compose 與 migrations"
    } else {
        Push-Location $Paths.ProjectRoot
        try {
            docker compose up -d
            Write-Ok "docker compose up -d"
            Write-Host "  等待 PostgreSQL..." -ForegroundColor DarkGray
            Start-Sleep -Seconds 8
            $running = docker ps --filter "name=$($SparkNodeConfig.PostgresContainer)" --filter "status=running" -q
            if ($running) {
                Get-ChildItem $Paths.MigrationsDir -Filter "*.sql" | Sort-Object Name | ForEach-Object {
                    Write-Host "  執行 migration: $($_.Name)" -ForegroundColor DarkGray
                    Get-Content $_.FullName -Raw | docker exec -i $SparkNodeConfig.PostgresContainer psql -U spark -d sparknode 2>&1 | Out-Null
                }
                Write-Ok "資料庫 migrations 已執行"
            } else {
                Write-Warn "容器 $($SparkNodeConfig.PostgresContainer) 未運行，請稍後手動執行 migrations"
            }
        } finally {
            Pop-Location
        }
    }
} else {
    Write-Step 7 "跳過 Docker"
}

# ── 8. 前端 / Python 依賴 ────────────────────────
if (-not $SkipDeps) {
    Write-Step 8 "安裝前端與 Python 依賴"
    if ((Get-Command node -ErrorAction SilentlyContinue) -and (Test-Path $Paths.WebDir)) {
        Push-Location $Paths.WebDir
        npm install
        Pop-Location
        Write-Ok "web: npm install"
    } else {
        Write-Warn "跳過 npm（未安裝 Node 或無 web/）"
    }
    if ((Get-Command python -ErrorAction SilentlyContinue) -and (Test-Path $Paths.LlmDir)) {
        Push-Location $Paths.LlmDir
        python -m pip install -e . 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) { pip install -e . }
        Pop-Location
        Write-Ok "sp-llm-router: pip install -e ."
    } elseif ((Get-Command python3 -ErrorAction SilentlyContinue) -and (Test-Path $Paths.LlmDir)) {
        Push-Location $Paths.LlmDir
        python3 -m pip install -e .
        Pop-Location
        Write-Ok "sp-llm-router: pip install -e ."
    } else {
        Write-Warn "跳過 pip（未安裝 Python 或無 LLM 服務目錄）"
    }
} else {
    Write-Step 8 "跳過依賴安裝"
}

# ── 9. Cargo 編譯驗證 ────────────────────────────
if (-not $SkipRust -and -not $SkipBuild) {
    Write-Step 9 "驗證 Rust 編譯"
    Push-Location $Paths.CratesDir
    try {
        cargo update -p idna_adapter --precise 1.2.0 2>&1 | Out-Host
        cargo check -p sp-gateway
        Write-Ok "cargo check -p sp-gateway 通過"
    } catch {
        Write-Err "cargo check 失敗: $_"
        Pop-Location
        exit 1
    }
    Pop-Location
} else {
    Write-Step 9 "跳過 cargo check"
}

# ── 完成 ─────────────────────────────────────────
Write-Host ""
Write-Host "════════════════════════════════════════════" -ForegroundColor Green
Write-Host " SparkNode 環境配置完成" -ForegroundColor Green
Write-Host "════════════════════════════════════════════" -ForegroundColor Green
Write-Host ""
Write-Host "日常開發（新終端機請先執行）：" -ForegroundColor White
Write-Host "  . `"$($Paths.EnvScript)`"" -ForegroundColor Gray
Write-Host ""
Write-Host "啟動服務：" -ForegroundColor White
Write-Host "  cd `"$($Paths.CratesDir)`"; cargo run -p sp-gateway" -ForegroundColor Gray
Write-Host "  cd `"$($Paths.LlmDir)`"; python -m src.main" -ForegroundColor Gray
Write-Host "  cd `"$($Paths.WebDir)`"; npm run dev" -ForegroundColor Gray
Write-Host ""
Write-Host "端點：" -ForegroundColor White
Write-Host "  API     http://localhost:3001/api/v1/health" -ForegroundColor Gray
Write-Host "  LLM     http://localhost:8001/health" -ForegroundColor Gray
Write-Host "  前端    http://localhost:3000" -ForegroundColor Gray
Write-Host ""
