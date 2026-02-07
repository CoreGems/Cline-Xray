# Test script for /agent/models endpoint
# Usage: .\scripts\test_agent_models.ps1
# Reads REST_API_TOKEN and REST_API_URL from .env file automatically

param(
    [string]$Token,
    [string]$BaseUrl
)

# Read .env file if it exists
$envFile = Join-Path $PSScriptRoot "..\\.env"
if (Test-Path $envFile) {
    Write-Host "Reading configuration from .env file..." -ForegroundColor Gray
    $envContent = Get-Content $envFile
    foreach ($line in $envContent) {
        if ($line -match "^\s*#" -or [string]::IsNullOrWhiteSpace($line)) { continue }
        if ($line -match "^([^=]+)=(.*)$") {
            $key = $matches[1].Trim()
            $value = $matches[2].Trim()
            
            if ($key -eq "REST_API_TOKEN" -and [string]::IsNullOrWhiteSpace($Token)) {
                $Token = $value
            }
            if ($key -eq "REST_API_URL" -and [string]::IsNullOrWhiteSpace($BaseUrl)) {
                $BaseUrl = $value
            }
        }
    }
}

# Default values if still not set
if ([string]::IsNullOrWhiteSpace($BaseUrl)) {
    $BaseUrl = "http://localhost:3030"
}

# Check if token is available
if ([string]::IsNullOrWhiteSpace($Token)) {
    Write-Host "Error: No token found. Please provide -Token parameter or set REST_API_TOKEN in .env file." -ForegroundColor Red
    exit 1
}

Write-Host "Testing GET /agent/models endpoint..." -ForegroundColor Cyan
Write-Host "Base URL: $BaseUrl" -ForegroundColor Gray

$headers = @{
    "Authorization" = "Bearer $Token"
    "Content-Type" = "application/json"
}

try {
    $response = Invoke-RestMethod -Uri "$BaseUrl/agent/models" -Method GET -Headers $headers
    
    Write-Host "`nSuccess! Retrieved $($response.total) models:" -ForegroundColor Green
    Write-Host "==========================================" -ForegroundColor Green
    
    foreach ($model in $response.models) {
        Write-Host "`nModel: $($model.name)" -ForegroundColor Yellow
        if ($model.displayName) {
            Write-Host "  Display Name: $($model.displayName)"
        }
        if ($model.description) {
            Write-Host "  Description: $($model.description)"
        }
        if ($model.inputTokenLimit) {
            Write-Host "  Input Token Limit: $($model.inputTokenLimit)"
        }
        if ($model.outputTokenLimit) {
            Write-Host "  Output Token Limit: $($model.outputTokenLimit)"
        }
        if ($model.supportedGenerationMethods -and $model.supportedGenerationMethods.Count -gt 0) {
            Write-Host "  Supported Methods: $($model.supportedGenerationMethods -join ', ')"
        }
    }
    
    Write-Host "`n==========================================" -ForegroundColor Green
    Write-Host "Total Models: $($response.total)" -ForegroundColor Green
}
catch {
    Write-Host "Error: $_" -ForegroundColor Red
    if ($_.Exception.Response) {
        $reader = New-Object System.IO.StreamReader($_.Exception.Response.GetResponseStream())
        $responseBody = $reader.ReadToEnd()
        Write-Host "Response Body: $responseBody" -ForegroundColor Red
    }
}
