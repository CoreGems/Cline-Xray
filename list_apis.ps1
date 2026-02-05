# list_apis.ps1 - Fetch OpenAPI spec and list available APIs
# Usage: .\list_apis.ps1 [-Port <port>]

param(
    [int]$Port = 0  # If 0, will auto-discover
)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "REST API - Available Endpoints" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

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
$openapiUrl = "$baseUrl/openapi.json"

Write-Host "Fetching OpenAPI spec from: $openapiUrl" -ForegroundColor Yellow
Write-Host ""

try {
    $spec = Invoke-RestMethod -Uri $openapiUrl -Method Get -ContentType "application/json"
    
    # Display API Info
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "API Information" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "  Title:       $($spec.info.title)" -ForegroundColor White
    Write-Host "  Version:     $($spec.info.version)" -ForegroundColor White
    Write-Host "  Description: $($spec.info.description)" -ForegroundColor White
    Write-Host "  OpenAPI:     $($spec.openapi)" -ForegroundColor White
    Write-Host ""
    
    # Display Available Endpoints
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "Available Endpoints" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    
    # Get paths and methods
    $paths = $spec.paths.PSObject.Properties
    
    foreach ($path in $paths) {
        $pathName = $path.Name
        $methods = $path.Value.PSObject.Properties
        
        foreach ($method in $methods) {
            $methodName = $method.Name.ToUpper()
            $endpoint = $method.Value
            
            # Determine if auth is required
            $authRequired = $false
            if ($endpoint.security -and $endpoint.security.Count -gt 0) {
                $authRequired = $true
            }
            
            # Get tag
            $tag = if ($endpoint.tags -and $endpoint.tags.Count -gt 0) { $endpoint.tags[0] } else { "general" }
            
            # Color code by method
            $methodColor = switch ($methodName) {
                "GET" { "Green" }
                "POST" { "Blue" }
                "PUT" { "Yellow" }
                "DELETE" { "Red" }
                default { "White" }
            }
            
            # Display endpoint
            Write-Host ""
            Write-Host "  [$methodName] " -ForegroundColor $methodColor -NoNewline
            Write-Host "$baseUrl$pathName" -ForegroundColor White
            
            if ($endpoint.summary) {
                Write-Host "    Summary: $($endpoint.summary)" -ForegroundColor Gray
            }
            
            Write-Host "    Tag: $tag" -ForegroundColor Gray
            
            if ($authRequired) {
                Write-Host "    Auth: " -ForegroundColor Gray -NoNewline
                Write-Host "Required (Bearer Token)" -ForegroundColor Yellow
            } else {
                Write-Host "    Auth: " -ForegroundColor Gray -NoNewline
                Write-Host "Not Required" -ForegroundColor Green
            }
            
            # Show parameters if any
            if ($endpoint.parameters -and $endpoint.parameters.Count -gt 0) {
                Write-Host "    Parameters:" -ForegroundColor Gray
                foreach ($param in $endpoint.parameters) {
                    $required = if ($param.required) { "(required)" } else { "(optional)" }
                    Write-Host "      - $($param.name): $($param.schema.type) $required" -ForegroundColor DarkGray
                    if ($param.description) {
                        Write-Host "        $($param.description)" -ForegroundColor DarkGray
                    }
                }
            }
            
            # Show responses
            if ($endpoint.responses) {
                Write-Host "    Responses:" -ForegroundColor Gray
                $responses = $endpoint.responses.PSObject.Properties
                foreach ($resp in $responses) {
                    $statusCode = $resp.Name
                    $statusColor = if ($statusCode -match "^2") { "Green" } elseif ($statusCode -match "^4") { "Yellow" } else { "Red" }
                    Write-Host "      ${statusCode}: " -ForegroundColor $statusColor -NoNewline
                    Write-Host "$($resp.Value.description)" -ForegroundColor DarkGray
                }
            }
        }
    }
    
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "Tags" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    
    if ($spec.tags) {
        foreach ($tag in $spec.tags) {
            Write-Host "  - $($tag.name): $($tag.description)" -ForegroundColor White
        }
    }
    
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "Security Schemes" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    
    if ($spec.components.securitySchemes) {
        $schemes = $spec.components.securitySchemes.PSObject.Properties
        foreach ($scheme in $schemes) {
            Write-Host "  - $($scheme.Name): $($scheme.Value.type) ($($scheme.Value.scheme))" -ForegroundColor White
        }
    }
    
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
    exit 1
}
