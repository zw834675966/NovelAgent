# NovelAgent AI quality gate — 机器可执行约束
# 用法（仓库根）:
#   pwsh -File scripts/ai-gate.ps1           # L0 必过
#   pwsh -File scripts/ai-gate.ps1 -Level L1 # + 供应链 / 死依赖
#   pwsh -File scripts/ai-gate.ps1 -Level L2 # + 覆盖率/变异（慢，可选）
#   pwsh -File scripts/ai-gate.ps1 -Level all
#
# AI DONE 定义：至少 L0 全绿。L1 工具未安装 → SKIP（不假绿，但报告缺失）。

param(
    [ValidateSet("L0", "L1", "L2", "all")]
    [string]$Level = "L0"
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

function Write-Step([string]$Name) {
    Write-Host ""
    Write-Host "==> $Name" -ForegroundColor Cyan
}

function Invoke-Required([string]$Name, [scriptblock]$Block) {
    Write-Step $Name
    & $Block
    if ($LASTEXITCODE -ne 0) {
        throw "GATE FAIL: $Name (exit $LASTEXITCODE)"
    }
    Write-Host "OK: $Name" -ForegroundColor Green
}

function Test-CargoTool([string]$Bin) {
    return [bool](Get-Command $Bin -ErrorAction SilentlyContinue)
}

function Invoke-Optional([string]$Name, [string]$Bin, [scriptblock]$Block) {
    Write-Step "$Name (optional)"
    if (-not (Test-CargoTool $Bin)) {
        Write-Host "SKIP: $Name — install: cargo install --locked $Bin" -ForegroundColor Yellow
        return
    }
    & $Block
    if ($LASTEXITCODE -ne 0) {
        throw "GATE FAIL: $Name (exit $LASTEXITCODE)"
    }
    Write-Host "OK: $Name" -ForegroundColor Green
}

$runL0 = $true
$runL1 = $Level -in @("L1", "all")
$runL2 = $Level -in @("L2", "all")

Write-Host "NovelAgent AI gate | level=$Level | root=$Root"

# --- L0: rustup 组件，无额外安装 ---
if ($runL0) {
    Invoke-Required "fmt" { cargo fmt --all --check }
    Invoke-Required "clippy" {
        cargo clippy --workspace --all-targets --all-features -- -D warnings
    }
    Invoke-Required "test" {
        if (Test-CargoTool "cargo-nextest") {
            # nextest defaults to exit 4 when 0 tests; empty starter crate is OK
            cargo nextest run --workspace --all-features --no-tests=pass
        } else {
            cargo test --workspace --all-features
        }
    }
}

# --- L1: 供应链 / 依赖卫生（AI 乱加 crate 的克星）---
if ($runL1) {
    Invoke-Optional "cargo-deny" "cargo-deny" {
        cargo deny check
    }
    # cargo-audit 与 deny advisories 重叠；有 deny 时 audit 作补充
    Invoke-Optional "cargo-audit" "cargo-audit" {
        cargo audit
    }
    # 未使用依赖：stable 上优先 machete；udeps 需 nightly
    Invoke-Optional "cargo-machete" "cargo-machete" {
        cargo machete
    }
}

# --- L2: 深度质量（慢；merge 前 / 大改行为时）---
if ($runL2) {
    Invoke-Optional "cargo-llvm-cov" "cargo-llvm-cov" {
        cargo llvm-cov --workspace --all-features --fail-under-lines 80
    }
    Invoke-Optional "cargo-mutants" "cargo-mutants" {
        cargo mutants --timeout 60 -- --all-features
    }
    Invoke-Optional "cargo-hack" "cargo-hack" {
        cargo hack check --feature-powerset --depth 2
    }
    # 仅当本 crate 作为库对外 semver 时有意义
    Invoke-Optional "cargo-semver-checks" "cargo-semver-checks" {
        cargo semver-checks check-release
    }
}

Write-Host ""
Write-Host "GATE PASS: level=$Level" -ForegroundColor Green
exit 0
