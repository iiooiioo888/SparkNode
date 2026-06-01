# SparkNode 腳本共用設定（被 setup-env.ps1 / rust-env.ps1 載入）

$Script:SparkNodeConfig = @{
    # 專案根目錄（自動推算，可被環境變數 SPARKNODE_ROOT 覆寫）
    ProjectRoot = if ($env:SPARKNODE_ROOT) { $env:SPARKNODE_ROOT } else { (Resolve-Path (Join-Path $PSScriptRoot "..")).Path }

    # Rust 工具鏈根目錄（預設 E 槽，避免 C 槽空間不足）
    RustRoot = if ($env:SPARKNODE_RUST_ROOT) { $env:SPARKNODE_RUST_ROOT } else { "E:\rust-toolchain" }

    # Defender 排除路徑
    DefenderExclusions = @()

    # PostgreSQL Docker 容器名
    PostgresContainer = "sp-postgres"
}

function Initialize-SparkNodePaths {
    param([string]$RustRoot, [string]$ProjectRoot)

    $script:Paths = @{
        ProjectRoot     = $ProjectRoot
        RustRoot        = $RustRoot
        RustupHome      = Join-Path $RustRoot ".rustup"
        CargoHome       = Join-Path $RustRoot ".cargo"
        CargoTargetDir  = Join-Path $RustRoot "target"
        CratesDir       = Join-Path $ProjectRoot "crates"
        WebDir          = Join-Path $ProjectRoot "web"
        LlmDir          = Join-Path $ProjectRoot "services\sp-llm-router"
        MigrationsDir   = Join-Path $ProjectRoot "migrations"
        EnvFile         = Join-Path $ProjectRoot ".env"
        EnvExample      = Join-Path $ProjectRoot ".env.example"
        EnvScript       = Join-Path $ProjectRoot "scripts\sparknode.env.ps1"
    }

    if ($SparkNodeConfig.DefenderExclusions.Count -eq 0) {
        $SparkNodeConfig.DefenderExclusions = @($RustRoot, $ProjectRoot)
    }

    return $script:Paths
}

function Set-SparkNodeSessionEnv {
    param($Paths)

    $env:SPARKNODE_ROOT       = $Paths.ProjectRoot
    $env:SPARKNODE_RUST_ROOT  = $Paths.RustRoot
    $env:RUSTUP_HOME          = $Paths.RustupHome
    $env:CARGO_HOME           = $Paths.CargoHome
    $env:CARGO_TARGET_DIR     = $Paths.CargoTargetDir

    $cargoBin = Join-Path $Paths.CargoHome "bin"
    $legacyBin = Join-Path $env:USERPROFILE ".cargo\bin"
    $pathParts = @($cargoBin)
    if (Test-Path $legacyBin) { $pathParts += $legacyBin }
    $pathParts += $env:PATH
    $env:PATH = ($pathParts -join ";")

    # 從 .env 載入到目前工作階段（若存在）
    if (Test-Path $Paths.EnvFile) {
        Get-Content $Paths.EnvFile | ForEach-Object {
            if ($_ -match '^\s*#' -or $_ -match '^\s*$') { return }
            if ($_ -match '^\s*([^=]+)=(.*)$') {
                $name = $Matches[1].Trim()
                $value = $Matches[2].Trim().Trim('"').Trim("'")
                Set-Item -Path "Env:$name" -Value $value -ErrorAction SilentlyContinue
            }
        }
    }
}

function Set-SparkNodeUserEnv {
    param($Paths)

    [Environment]::SetEnvironmentVariable("SPARKNODE_ROOT", $Paths.ProjectRoot, "User")
    [Environment]::SetEnvironmentVariable("SPARKNODE_RUST_ROOT", $Paths.RustRoot, "User")
    [Environment]::SetEnvironmentVariable("RUSTUP_HOME", $Paths.RustupHome, "User")
    [Environment]::SetEnvironmentVariable("CARGO_HOME", $Paths.CargoHome, "User")
    [Environment]::SetEnvironmentVariable("CARGO_TARGET_DIR", $Paths.CargoTargetDir, "User")

    $cargoBin = Join-Path $Paths.CargoHome "bin"
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($null -eq $userPath) { $userPath = "" }
    if ($userPath -notlike "*$cargoBin*") {
        [Environment]::SetEnvironmentVariable("Path", "$cargoBin;$userPath", "User")
    }
}

function Test-SqlFileWritable {
    param([string]$Directory)
    $testFile = Join-Path $Directory "sparknode-write-test.sql"
    try {
        Set-Content -Path $testFile -Value "-- sparknode env test" -Force -ErrorAction Stop
        Remove-Item $testFile -Force -ErrorAction Stop
        return $true
    } catch {
        return $false
    }
}

function Add-DefenderExclusionSafe {
    param([string[]]$Paths)
    $isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    if (-not $isAdmin) {
        Write-Host "  [略過] 非管理員，無法自動加入 Defender 排除" -ForegroundColor Yellow
        return
    }
    foreach ($p in $Paths) {
        try {
            Add-MpPreference -ExclusionPath $p -ErrorAction Stop
            Write-Host "  [OK] Defender 已排除: $p" -ForegroundColor Green
        } catch {
            Write-Host "  [警告] 無法排除 $p : $_" -ForegroundColor Yellow
        }
    }
}

function Write-SparkNodeEnvScript {
    param($Paths)

    $content = @"
# 由 scripts/setup-env.ps1 自動產生 — 請用 dot-source 載入
#   . "$($Paths.EnvScript)"

`$env:SPARKNODE_ROOT      = "$($Paths.ProjectRoot)"
`$env:SPARKNODE_RUST_ROOT = "$($Paths.RustRoot)"
`$env:RUSTUP_HOME         = "$($Paths.RustupHome)"
`$env:CARGO_HOME          = "$($Paths.CargoHome)"
`$env:CARGO_TARGET_DIR    = "$($Paths.CargoTargetDir)"

`$cargoBin = Join-Path `$env:CARGO_HOME "bin"
`$legacyBin = Join-Path `$env:USERPROFILE ".cargo\bin"
`$env:PATH = "`$cargoBin;`$legacyBin;" + `$env:PATH

# 載入專案 .env
`$dotenv = "$($Paths.EnvFile)"
if (Test-Path `$dotenv) {
    Get-Content `$dotenv | ForEach-Object {
        if (`$_ -match '^\s*#' -or `$_ -match '^\s*`$') { return }
        if (`$_ -match '^\s*([^=]+)=(.*)`$') {
            Set-Item -Path "Env:`$(`$Matches[1].Trim())" -Value `$Matches[2].Trim().Trim('"').Trim("'") -ErrorAction SilentlyContinue
        }
    }
}

Write-Host "SparkNode 環境已載入 @ `$env:SPARKNODE_ROOT" -ForegroundColor DarkGray
if (Get-Command rustc -ErrorAction SilentlyContinue) {
    Write-Host "  rustc `$(rustc -V 2>`$null)" -ForegroundColor DarkGray
}
"@
    Set-Content -Path $Paths.EnvScript -Value $content -Encoding UTF8
}
