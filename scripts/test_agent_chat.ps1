# test_agent_chat.ps1 - Test the /agent/chat REST API endpoint (Gemini AI)
# Usage: .\test_agent_chat.ps1 [-Message <message>] [-Token <bearer_token>] [-Port <port>]
#
# The bearer token is automatically read from the .env file if not provided.
# The Tauri app saves REST_API_URL and REST_API_TOKEN to the .env file on startup.

param(
    [string]$Message = "Hello! What can you help me with?",
    [string]$Token = "",
    [int]$Port = 0  # If 0, will auto-discover or read from .env
)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Testing Agent Chat REST API Endpoint" -ForegroundColor Cyan
Write-Host "(Google Gemini AI)" -ForegroundColor DarkCyan
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
$chatUrl = "$baseUrl/agent/chat"

Write-Host "URL: $chatUrl" -ForegroundColor Yellow
Write-Host "Auth: Bearer token ($(($Token.Substring(0, [Math]::Min(8, $Token.Length))))...)" -ForegroundColor Yellow
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Your Message:" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  $Message" -ForegroundColor White
Write-Host ""

try {
    $headers = @{
        "Authorization" = "Bearer $Token"
        "Content-Type" = "application/json"
    }
    
    # Build request body
    $body = @{
        message = $Message
        history = @()
    } | ConvertTo-Json
    
    Write-Host "Sending request to Gemini AI..." -ForegroundColor Yellow
    Write-Host ""
    
    $startTime = Get-Date
    $response = Invoke-RestMethod -Uri $chatUrl -Method Post -Headers $headers -Body $body
    $endTime = Get-Date
    $duration = ($endTime - $startTime).TotalSeconds
    
    Write-Host "SUCCESS!" -ForegroundColor Green
    Write-Host "Response time: $([math]::Round($duration, 2)) seconds" -ForegroundColor Gray
    Write-Host ""
    
    # Display Gemini's response
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "Gemini's Response:" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host $response.response -ForegroundColor White
    Write-Host ""
    
    # Show conversation history count
    if ($response.history -and $response.history.Count -gt 0) {
        Write-Host "========================================" -ForegroundColor Cyan
        Write-Host "Conversation History: $($response.history.Count) messages" -ForegroundColor Cyan
        Write-Host "========================================" -ForegroundColor Cyan
        foreach ($msg in $response.history) {
            $roleColor = if ($msg.role -eq "user") { "Blue" } else { "Green" }
            $roleLabel = if ($msg.role -eq "user") { "You" } else { "Gemini" }
            $preview = $msg.content
            if ($preview.Length -gt 60) {
                $preview = $preview.Substring(0, 60) + "..."
            }
            Write-Host "  [$roleLabel]: $preview" -ForegroundColor $roleColor
        }
        Write-Host ""
    }
    
    # Show tip for raw JSON
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "Tip: To see raw JSON response:" -ForegroundColor Gray
    Write-Host '  $response | ConvertTo-Json -Depth 10' -ForegroundColor Gray
    Write-Host ""
}
catch {
    Write-Host "FAILED!" -ForegroundColor Red
    Write-Host ""
    Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
    
    if ($_.Exception.Response) {
        $statusCode = [int]$_.Exception.Response.StatusCode
        Write-Host "HTTP Status: $statusCode" -ForegroundColor Red
        
        # Try to get response body for more details
        try {
            $reader = New-Object System.IO.StreamReader($_.Exception.Response.GetResponseStream())
            $responseBody = $reader.ReadToEnd()
            $reader.Close()
            
            if ($responseBody) {
                $errorJson = $responseBody | ConvertFrom-Json
                if ($errorJson.error) {
                    Write-Host "API Error: $($errorJson.error)" -ForegroundColor Red
                }
            }
        }
        catch {
            # Ignore error reading response body
        }
        
        if ($statusCode -eq 401) {
            Write-Host ""
            Write-Host "Authentication failed! Check your bearer token." -ForegroundColor Yellow
            Write-Host "The token is generated fresh each time the Tauri app starts." -ForegroundColor Gray
        }
        elseif ($statusCode -eq 400) {
            Write-Host ""
            Write-Host "Gemini API key may not be configured!" -ForegroundColor Yellow
            Write-Host "Set GEMINI_API_KEY in your .env file." -ForegroundColor Gray
            Write-Host "Get a key at: https://aistudio.google.com/app/apikey" -ForegroundColor Gray
        }
    }
    
    Write-Host ""
    Write-Host "Make sure:" -ForegroundColor Yellow
    Write-Host "  1. The REST API server is running (npm run tauri dev)" -ForegroundColor Gray
    Write-Host "  2. GEMINI_API_KEY is set in .env file" -ForegroundColor Gray
    exit 1
}
