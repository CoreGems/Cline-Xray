# Windows PowerShell Environment

This project is developed on **Windows 11 with PowerShell** as the default shell.

## Shell Syntax Rules

### DO NOT USE (Bash/Unix syntax):
- `&&` for command chaining (use `;` in PowerShell, or run commands separately)
- `||` for conditional execution
- `curl ... | jq` piping patterns (PowerShell has different piping semantics)
- `2>&1` redirection at end of command chains
- `export VAR=value` (use `$env:VAR = "value"`)
- Single quotes for variable interpolation (use double quotes)
- `~` for home directory (use `$env:USERPROFILE` or `$HOME`)

### DO USE (PowerShell-compatible):
- Separate commands on individual lines or use `;` as separator
- `--manifest-path` or `-C` flags for cargo/npm to specify directory
- `Invoke-WebRequest` or `curl.exe` (not curl alias) for HTTP requests
- `$env:VAR` for environment variables
- Full absolute paths without `~`

## Examples

### Running cargo from a different directory:
```powershell
# WRONG (bash syntax)
cd src-tauri && cargo check

# CORRECT (PowerShell)
cargo check --manifest-path c:\path\to\src-tauri\Cargo.toml
```

### Multiple commands:
```powershell
# WRONG (bash syntax)
npm install && npm run dev

# CORRECT (PowerShell - separate lines or semicolon)
npm install; npm run dev
# OR run as separate execute_command calls
```

### Environment variables:
```powershell
# WRONG (bash syntax)
export API_KEY=xyz

# CORRECT (PowerShell)
$env:API_KEY = "xyz"
```

## Package Manager Notes
- Use `npm` directly (available in PATH)
- Use `cargo` directly (available in PATH)
- Avoid scripts that assume bash availability
