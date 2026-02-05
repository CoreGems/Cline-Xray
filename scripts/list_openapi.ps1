# list_openapi.ps1 - Fetch and list all REST APIs from OpenAPI spec
# Usage: .\list_openapi.ps1 [-Port <port>]

param(
    [int]$Port = 0  # If 0, will auto-discover
)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "REST API Endpoints from OpenAPI Spec" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Auto-discover port if not specified
if ($Port -eq 0) {
    Write-Host "Auto-discovering REST API port..." -ForegroundColor Yellow
    
    # Get all listening ports on loopback (high ports typically used by the app)
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
$openapiUrl = "$baseUrl/openapi.json"

Write-Host "Fetching OpenAPI spec from: $openapiUrl" -ForegroundColor Yellow
Write-Host ""

try {
    $spec = Invoke-RestMethod -Uri $openapiUrl -Method Get -ContentType "application/json"
    
    Write-Host "API Information:" -ForegroundColor Cyan
    Write-Host "  Title:   $($spec.info.title)" -ForegroundColor White
    Write-Host "  Version: $($spec.info.version)" -ForegroundColor White
    Write-Host "  OpenAPI: $($spec.openapi)" -ForegroundColor White
    Write-Host ""
    
    # Parse and list all endpoints
    Write-Host "Available Endpoints:" -ForegroundColor Cyan
    Write-Host "----------------------------------------" -ForegroundColor Gray
    
    $apiCount = 0
    
    foreach ($path in $spec.paths.PSObject.Properties) {
        $pathName = $path.Name
        $pathMethods = $path.Value
        
        foreach ($method in $pathMethods.PSObject.Properties) {
            $methodName = $method.Name.ToUpper()
            $methodDetails = $method.Value
            
            $apiCount++
            
            # Color code by method
            $methodColor = switch ($methodName) {
                "GET"    { "Green" }
                "POST"   { "Blue" }
                "PUT"    { "Yellow" }
                "DELETE" { "Red" }
                "PATCH"  { "Magenta" }
                default  { "White" }
            }
            
            # Check if auth required
            $authRequired = $false
            if ($methodDetails.security) {
                $authRequired = $true
            }
            
            # Get tag
            $tag = ""
            if ($methodDetails.tags -and $methodDetails.tags.Count -gt 0) {
                $tag = $methodDetails.tags[0]
            }
            
            # Get summary/description
            $summary = ""
            if ($methodDetails.summary) {
                $summary = $methodDetails.summary
            } elseif ($methodDetails.description) {
                $summary = $methodDetails.description
            }
            
            # Display endpoint
            Write-Host ""
            Write-Host -NoNewline "  "
            Write-Host -NoNewline $methodName.PadRight(8) -ForegroundColor $methodColor
            Write-Host -NoNewline $pathName -ForegroundColor White
            
            if ($authRequired) {
                Write-Host -NoNewline " " 
                Write-Host -NoNewline "[AUTH]" -ForegroundColor DarkYellow
            }
            
            if ($tag) {
                Write-Host -NoNewline " "
                Write-Host -NoNewline "($tag)" -ForegroundColor DarkCyan
            }
            
            Write-Host ""
            
            if ($summary) {
                Write-Host "           $summary" -ForegroundColor Gray
            }
            
            # Show response codes
            if ($methodDetails.responses) {
                $responseCodes = ($methodDetails.responses.PSObject.Properties | ForEach-Object { $_.Name }) -join ", "
                Write-Host "           Responses: $responseCodes" -ForegroundColor DarkGray
            }
        }
    }
    
    Write-Host ""
    Write-Host "----------------------------------------" -ForegroundColor Gray
    Write-Host "Total: $apiCount endpoint(s)" -ForegroundColor Cyan
    Write-Host ""
    
    # Show tags info
    if ($spec.tags) {
        Write-Host "Tags:" -ForegroundColor Cyan
        foreach ($tag in $spec.tags) {
            Write-Host "  - $($tag.name): $($tag.description)" -ForegroundColor White
        }
        Write-Host ""
    }
    
    Write-Host "Base URL: $baseUrl" -ForegroundColor Yellow
    Write-Host ""
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
