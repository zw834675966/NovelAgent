# Report presence of keys only (never print values).
if (-not (Test-Path '.env')) {
    Write-Output 'no .env'
    exit 0
}
$names = @('OPENCODE_GO_API_KEY', 'COHERE_API_KEY')
foreach ($name in $names) {
    $line = Get-Content '.env' | Where-Object { $_ -match ("^" + [regex]::Escape($name) + "=") } | Select-Object -First 1
    if (-not $line) {
        Write-Output ("{0} set=False" -f $name)
        continue
    }
    $parts = $line -split '=', 2
    $len = 0
    if ($parts.Count -gt 1) {
        $len = $parts[1].Trim().Length
    }
    Write-Output ("{0} set={1} len={2}" -f $name, ($len -gt 0), $len)
}
