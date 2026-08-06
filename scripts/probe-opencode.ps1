# Connectivity probe for OpenCode Go (no secrets printed).
$ErrorActionPreference = 'Continue'

try {
    $r = Invoke-WebRequest -Uri 'https://opencode.ai' -Method Head -TimeoutSec 20 -UseBasicParsing
    Write-Output ("site_status={0}" -f $r.StatusCode)
} catch {
    Write-Output ("site_err={0}" -f $_.Exception.Message)
}

# Load key length only from .env for POST test
$key = $null
if (Test-Path '.env') {
    $line = Get-Content '.env' | Where-Object { $_ -match '^OPENCODE_GO_API_KEY=' } | Select-Object -First 1
    if ($line) {
        $key = ($line -split '=', 2)[1].Trim()
    }
}

if (-not $key) {
    Write-Output 'POST skipped: no OPENCODE_GO_API_KEY'
    exit 0
}

Write-Output ("key_len={0}" -f $key.Length)

$body = @{
    model = 'deepseek-v4-flash'
    messages = @(@{ role = 'user'; content = 'Reply with exactly: ok' })
    max_tokens = 16
} | ConvertTo-Json -Depth 5

try {
    $headers = @{ Authorization = "Bearer $key"; 'Content-Type' = 'application/json' }
    $resp = Invoke-RestMethod -Uri 'https://opencode.ai/zen/go/v1/chat/completions' -Method Post -Headers $headers -Body $body -TimeoutSec 90
    $json = $resp | ConvertTo-Json -Depth 8 -Compress
    Write-Output ("raw_len={0}" -f $json.Length)
    # Structure only: keys + content length (no full dump if huge)
    if ($resp.choices) {
        $msg = $resp.choices[0].message
        $content = [string]$msg.content
        Write-Output ("content_len={0}" -f $content.Length)
        if ($content.Length -gt 0) {
            Write-Output ("content_preview={0}" -f $content.Substring(0, [Math]::Min(120, $content.Length)))
        }
        $props = ($msg | Get-Member -MemberType NoteProperty | ForEach-Object { $_.Name }) -join ','
        Write-Output ("message_props={0}" -f $props)
        if ($msg.PSObject.Properties.Name -contains 'reasoning_content') {
            $rc = [string]$msg.reasoning_content
            Write-Output ("reasoning_len={0}" -f $rc.Length)
        }
    } else {
        Write-Output ("no_choices keys={0}" -f (($resp | Get-Member -MemberType NoteProperty | ForEach-Object Name) -join ','))
        Write-Output ("raw_head={0}" -f $json.Substring(0, [Math]::Min(400, $json.Length)))
    }
    Write-Output 'chat_ok=True'
} catch {
    Write-Output ("chat_err={0}" -f $_.Exception.Message)
    if ($_.ErrorDetails.Message) {
        $msg = $_.ErrorDetails.Message
        if ($msg.Length -gt 300) { $msg = $msg.Substring(0, 300) }
        Write-Output ("chat_body={0}" -f $msg)
    }
}
