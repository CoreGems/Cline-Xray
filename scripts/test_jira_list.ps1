# test_jira_list.ps1 - Test the /jira/list REST API endpoint
# Usage: .\test_jira_list.ps1 [-Token <bearer_token>] [-Port <port>] [-Jql <jql_query>] [-MaxResults <number>]
#
# The bearer token is automatically read from the .env file if not provided.
# The Tauri app saves REST_API_URL and REST_API_TOKEN to the .env file on startup.

param(
    [string]$Token = "",
    [int]$Port = 0,  # If 0, will auto-discover or read from .env
    [string]$Jql = "",  # JQL query string, empty uses default
    [int]$MaxResults = 0  # 0 uses server default (100)
)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Testing Jira List REST API Endpoint" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Function to parse .env file
function Get-EnvValue {
    param([string]$Key, [string]$EnvPath = ".env")
    
    if (-not (Test-Path $EnvPath)) {
        return $null
    }
    
    $content = Get-Content $EnvPath
    foreach ($line in $content) {
        if ($line -match "^$Key=(.*)$") {
            return $Matches[1]
        }
    }
    return $null
}

# Try to load token from .env file if not provided
$envPath = ".env"

if ([string]::IsNullOrEmpty($Token)) {
    # Check environment variable first
    if (-not [string]::IsNullOrEmpty($env:REST_API_TOKEN)) {
        $Token = $env:REST_API_TOKEN
        Write-Host "Using token from REST_API_TOKEN environment variable" -ForegroundColor Yellow
    }
    # Try to read from .env file
    elseif (Test-Path $envPath) {
        $envToken = Get-EnvValue -Key "REST_API_TOKEN" -EnvPath $envPath
        if (-not [string]::IsNullOrEmpty($envToken)) {
            $Token = $envToken
            Write-Host "Loaded token from .env file" -ForegroundColor Green
        }
        
        # Also try to get the port from .env file
        if ($Port -eq 0) {
            $envUrl = Get-EnvValue -Key "REST_API_URL" -EnvPath $envPath
            if ($envUrl -match ":(\d+)$") {
                $Port = [int]$Matches[1]
                Write-Host "Using port $Port from .env file" -ForegroundColor Green
            }
        }
    }
}

# Still no token?
if ([string]::IsNullOrEmpty($Token)) {
    Write-Host "ERROR: Bearer token required!" -ForegroundColor Red
    Write-Host ""
    Write-Host "The token should be automatically saved when the Tauri app starts." -ForegroundColor Yellow
    Write-Host "Expected .env file with REST_API_TOKEN variable" -ForegroundColor Gray
    Write-Host ""
    Write-Host "If the token is missing, try:" -ForegroundColor Gray
    Write-Host "  1. Restart the Tauri app (npm run tauri dev)" -ForegroundColor Gray
    Write-Host "  2. Pass token manually: -Token <token>" -ForegroundColor Gray
    Write-Host "  3. Set environment variable: `$env:REST_API_TOKEN = '<token>'" -ForegroundColor Gray
    exit 1
}

# Auto-discover port if not specified
if ($Port -eq 0) {
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
$jiraListUrl = "$baseUrl/jira/list"

# Build query parameters
$queryParams = @()
if (-not [string]::IsNullOrEmpty($Jql)) {
    $queryParams += "jql=$([System.Uri]::EscapeDataString($Jql))"
}
if ($MaxResults -gt 0) {
    $queryParams += "max_results=$MaxResults"
}
if ($queryParams.Count -gt 0) {
    $jiraListUrl += "?" + ($queryParams -join "&")
}

Write-Host "URL: $jiraListUrl" -ForegroundColor Yellow
Write-Host "Auth: Bearer token (${Token.Substring(0, [Math]::Min(8, $Token.Length))}...)" -ForegroundColor Yellow
Write-Host ""

try {
    $headers = @{
        "Authorization" = "Bearer $Token"
        "Content-Type" = "application/json"
    }
    
    $response = Invoke-RestMethod -Uri $jiraListUrl -Method Get -Headers $headers
    
    Write-Host "SUCCESS!" -ForegroundColor Green
    Write-Host ""
    
    # Display summary
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "Query Results" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "  JQL: $($response.jql)" -ForegroundColor White
    Write-Host "  Total: $($response.total) issues" -ForegroundColor White
    Write-Host "  Returned: $($response.issues.Count) issues" -ForegroundColor White
    Write-Host ""
    
    # Display issues in table format
    if ($response.issues.Count -gt 0) {
        Write-Host "========================================" -ForegroundColor Cyan
        Write-Host "Issues" -ForegroundColor Cyan
        Write-Host "========================================" -ForegroundColor Cyan
        Write-Host ""
        
        # Table header
        $keyWidth = 15
        $statusWidth = 20
        $priorityWidth = 10
        $summaryWidth = 50
        
        Write-Host ("  {0,-$keyWidth} {1,-$statusWidth} {2,-$priorityWidth} {3}" -f "KEY", "STATUS", "PRIORITY", "SUMMARY") -ForegroundColor Gray
        Write-Host ("  {0,-$keyWidth} {1,-$statusWidth} {2,-$priorityWidth} {3}" -f ("-" * $keyWidth), ("-" * $statusWidth), ("-" * $priorityWidth), ("-" * $summaryWidth)) -ForegroundColor Gray
        
        foreach ($issue in $response.issues) {
            # Color code by status category
            $statusColor = switch ($issue.statusCategory) {
                "Done" { "Green" }
                "In Progress" { "Yellow" }
                "To Do" { "Cyan" }
                default { "White" }
            }
            
            # Priority color
            $priorityColor = switch ($issue.priority) {
                "Highest" { "Red" }
                "High" { "DarkRed" }
                "Medium" { "Yellow" }
                "Low" { "Green" }
                "Lowest" { "DarkGreen" }
                default { "White" }
            }
            
            # Truncate summary if too long
            $summary = $issue.summary
            if ($summary.Length -gt $summaryWidth) {
                $summary = $summary.Substring(0, $summaryWidth - 3) + "..."
            }
            
            Write-Host "  " -NoNewline
            Write-Host ("{0,-$keyWidth}" -f $issue.key) -ForegroundColor Cyan -NoNewline
            Write-Host " " -NoNewline
            Write-Host ("{0,-$statusWidth}" -f $issue.status) -ForegroundColor $statusColor -NoNewline
            Write-Host " " -NoNewline
            Write-Host ("{0,-$priorityWidth}" -f $issue.priority) -ForegroundColor $priorityColor -NoNewline
            Write-Host " $summary" -ForegroundColor White
        }
        
        Write-Host ""
        
        # Show assignee breakdown
        Write-Host "========================================" -ForegroundColor Cyan
        Write-Host "Assignees" -ForegroundColor Cyan
        Write-Host "========================================" -ForegroundColor Cyan
        $assignees = $response.issues | Group-Object -Property assignee | Sort-Object -Property Count -Descending
        foreach ($assignee in $assignees) {
            $name = if ([string]::IsNullOrEmpty($assignee.Name)) { "(Unassigned)" } else { $assignee.Name }
            Write-Host "  $name : $($assignee.Count) issues" -ForegroundColor White
        }
        Write-Host ""
        
        # Show status breakdown
        Write-Host "========================================" -ForegroundColor Cyan
        Write-Host "Status Breakdown" -ForegroundColor Cyan
        Write-Host "========================================" -ForegroundColor Cyan
        $statuses = $response.issues | Group-Object -Property status | Sort-Object -Property Count -Descending
        foreach ($status in $statuses) {
            Write-Host "  $($status.Name) : $($status.Count) issues" -ForegroundColor White
        }
        Write-Host ""
    }
    else {
        Write-Host "No issues found matching the query." -ForegroundColor Yellow
        Write-Host ""
    }
    
    # Show raw JSON option
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "Tip: To see raw JSON, pipe to ConvertTo-Json:" -ForegroundColor Gray
    Write-Host "  (Invoke-RestMethod ... ) | ConvertTo-Json -Depth 10" -ForegroundColor Gray
    Write-Host ""
}
catch {
    Write-Host "FAILED!" -ForegroundColor Red
    Write-Host ""
    Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
    
    if ($_.Exception.Response) {
        $statusCode = [int]$_.Exception.Response.StatusCode
        Write-Host "HTTP Status: $statusCode" -ForegroundColor Red
        
        if ($statusCode -eq 401) {
            Write-Host ""
            Write-Host "Authentication failed! Check your bearer token." -ForegroundColor Yellow
            Write-Host "The token is generated fresh each time the Tauri app starts." -ForegroundColor Gray
        }
    }
    
    Write-Host ""
    Write-Host "Make sure the REST API server is running!" -ForegroundColor Yellow
    exit 1
}
