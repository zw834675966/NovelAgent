# 安装 NovelAgent AI 门禁 L1/L2 可选工具（需网络）
# 用法: pwsh -File scripts/install-ai-tools.ps1
# 优先 cargo-binstall（快）；否则 cargo install --locked

$ErrorActionPreference = "Stop"

$tools = @(
    "cargo-deny",
    "cargo-machete",
    "cargo-nextest",
    "cargo-audit",
    "cargo-llvm-cov",
    "cargo-mutants",
    "cargo-hack"
)

$useBinstall = [bool](Get-Command cargo-binstall -ErrorAction SilentlyContinue)

foreach ($t in $tools) {
    if (Get-Command $t -ErrorAction SilentlyContinue) {
        Write-Host "already: $t"
        continue
    }
    Write-Host "install: $t"
    if ($useBinstall) {
        cargo binstall -y $t
    } else {
        cargo install --locked $t
    }
}

Write-Host "done. Run: pwsh -File scripts/ai-gate.ps1 -Level L1"
