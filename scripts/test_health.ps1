# test_health.ps1 - Test the REST API health endpoint
# Usage: .\test_health.ps1 [-Port <port>] [-AutoDiscover]

param(
    [int]$Port = 0,  # If 0, will auto-discover
    [switch]$AutoDiscover
)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Testing REST API Health Endpoint" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Auto-discover port if not specified
if ($Port -eq 0 -or $AutoDiscover) {
    Write-Host "Auto-discovering REST API port..." -ForegroundColor Yellow
    
    # Get all listening ports on loopback
    $ports = netstat -an | Select-String "127.0.0.1:(\d+).*LISTENING" | ForEach-Object {
        [int]$_.Matches.Groups[1].Value
    } | Where-Object { $_ -gt 50000 } | Sort-Object -Descending
    
    foreach ($testPort in $ports) {
        try {
            $null = Invoke-RestMethod -Uri "http://127.0.0.1:$testPort/health" -TimeoutSec 1 -ErrorAction Stop
            Write-Host "Found REST API on port $testPort" -ForegroundColor Green
            $Port = $testPort
            break
        }
        catch {
            # Not this port
        }
    }
    
    if ($Port -eq 0) {
        Write-Host "FAILED!" -ForegroundColor Red
        Write-Host "Could not find REST API server on any port." -ForegroundColor Red
        Write-Host "Make sure the Tauri app is running!" -ForegroundColor Yellow
        exit 1
    }
    Write-Host ""
}

$baseUrl = "http://127.0.0.1:$Port"
$healthUrl = "$baseUrl/health"

Write-Host "URL: $healthUrl" -ForegroundColor Yellow
Write-Host ""

try {
    $response = Invoke-RestMethod -Uri $healthUrl -Method Get -ContentType "application/json"
    
    Write-Host "SUCCESS!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Response:" -ForegroundColor Cyan
    Write-Host "  Status: $($response.status)" -ForegroundColor White
    Write-Host "  Uptime: $($response.uptime_secs) seconds" -ForegroundColor White
    Write-Host ""
    
    # Also show raw JSON
    Write-Host "Raw JSON:" -ForegroundColor Cyan
    $response | ConvertTo-Json | Write-Host
}
catch {
    Write-Host "FAILED!" -ForegroundColor Red
    Write-Host ""
    Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
    
    if ($_.Exception.Response) {
        $statusCode = [int]$_.Exception.Response.StatusCode
        Write-Host "HTTP Status: $statusCode" -ForegroundColor Red
    }
    
    Write-Host ""
    Write-Host "Make sure the REST API server is running!" -ForegroundColor Yellow
    Write-Host "You can start it by running: npm run tauri dev" -ForegroundColor Yellow
    exit 1
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Also available endpoints:" -ForegroundColor Cyan
Write-Host "  GET $baseUrl/openapi.json - OpenAPI spec (no auth)" -ForegroundColor White
Write-Host "  GET $baseUrl/jira/list - List Jira issues (requires Bearer token)" -ForegroundColor White
Write-Host "========================================" -ForegroundColor Cyan
