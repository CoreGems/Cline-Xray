$file = "c:\Users\alex_\mygems\Jira-Xray-Gem\src-tauri\src\api\handlers.rs"
$lines = Get-Content $file

# Find the FIRST occurrence of "// ============ Gemini Models Handler"
# This is the broken one. We need to remove from there to just before 
# "/// OpenAI chat completion request structures"
$startRemove = -1
$endRemove = -1

for ($i = 0; $i -lt $lines.Count; $i++) {
    if ($lines[$i] -match "// ============ Gemini Models Handler") {
        if ($startRemove -eq -1) {
            $startRemove = $i
            Write-Host "Found broken section start at line $($i+1)"
        }
    }
    if ($startRemove -ne -1 -and $endRemove -eq -1) {
        if ($lines[$i] -match "/// OpenAI chat completion request structures") {
            $endRemove = $i
            Write-Host "Found broken section end at line $($i+1)"
            break
        }
    }
}

if ($startRemove -ne -1 -and $endRemove -ne -1) {
    Write-Host "Removing lines $($startRemove+1) to $($endRemove)"
    $newLines = $lines[0..($startRemove-1)] + $lines[$endRemove..($lines.Count-1)]
    $newLines | Set-Content $file -Encoding UTF8
    Write-Host "Fixed! Removed $($endRemove - $startRemove) lines"
} else {
    Write-Host "Could not find broken section boundaries"
}
