# Jira Viewer - Migration & Setup Guide

This guide outlines what has been implemented and how to set up the development environment on a new machine.

---

## What Has Been Done

### Project Structure
The application is built with:
- **Tauri 2.x** - Desktop runtime (Rust backend)
- **Svelte 5** - Frontend framework
- **TypeScript** - Type-safe JavaScript
- **Tailwind CSS 4** - Styling

### Implemented Features

#### 1. Left Pane - Issue List (`src/lib/LeftPane.svelte`)
- Displays list of Jira issues from JQL query
- Shows for each issue:
  - Issue key (e.g., PROJ-123) in blue
  - Status with color-coded badges (green=done, blue=in progress, red=blocked)
  - Truncated summary
  - Assignee name
  - Last updated date
  - Priority
- Click to select an issue
- Loading spinner while fetching
- Empty state when no issues found

#### 2. Main Pane - Issue Details (`src/lib/MainPane.svelte`)
- Displays full details when an issue is selected
- Shows:
  - Issue key and status badge
  - Full summary as title
  - Resolution status (if resolved)
  - Metadata grid: Type, Priority, Assignee, Reporter, Created, Updated
  - Labels (as tags)
  - Components (as tags)
  - Full description with basic Jira markup rendering
- Loading spinner while fetching details
- Empty state with "Select an issue" message

#### 3. Top Bar (`src/lib/TopBar.svelte`)
- JQL search input field
- Search button
- Refresh button
- Settings gear icon

#### 4. Settings Modal (`src/lib/SettingsModal.svelte`)
- Configure Jira Base URL
- Email address input
- API Token input (stored securely in OS keychain)
- Default JQL query
- Save/Cancel buttons

#### 5. Main App (`src/App.svelte`)
- State management connecting all components
- Issue selection handling
- Error display (toast-style at bottom-right)
- Configuration check on startup
- Welcome screen for unconfigured state

#### 6. Rust Backend (`src-tauri/src/lib.rs`)
- **`is_configured`** - Check if credentials are set up
- **`get_settings`** - Retrieve saved settings
- **`save_settings`** - Save settings and API token to keychain
- **`list_issues`** - Search Jira with JQL, return issue summaries
- **`get_issue`** - Get full issue details with in-memory caching
- **Secure Storage** - API tokens stored in OS keychain (Windows Credential Manager)
- **Error Handling** - Custom error types with proper serialization

#### 7. TypeScript Types (`src/types.ts`)
- `IssueSummary` - For list view
- `IssueDetails` - For detail view
- `JiraSettings` - Configuration
- `SearchResult` - Search response wrapper

---

## Prerequisites for New Machine

### Required Software

1. **Node.js 18+**
   - Download: https://nodejs.org/
   - Verify: `node --version` and `npm --version`

2. **Rust (stable)**
   - Download: https://rustup.rs/
   - Run `rustup-init.exe` (Windows) or `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` (Unix)
   - Verify: `rustc --version` and `cargo --version`

3. **Visual Studio Build Tools (Windows only)**
   - Download: https://visualstudio.microsoft.com/visual-cpp-build-tools/
   - Select "Desktop development with C++" workload
   - Required for compiling native Rust dependencies

4. **WebView2 Runtime (Windows 10/11)**
   - Usually pre-installed on Windows 10/11
   - If missing: https://developer.microsoft.com/en-us/microsoft-edge/webview2/

### Optional but Recommended

- **Visual Studio Code** with extensions:
  - Svelte for VS Code
  - Rust Analyzer
  - Tauri (official extension)

---

## Setup Steps

### 1. Clone/Copy the Project
```bash
# Copy the jira-viewer folder to your new machine
```

### 2. Install Node Dependencies
```bash
cd jira-viewer
npm install
```

### 3. Verify Rust Installation
```bash
rustc --version
# Should output: rustc 1.XX.X (hash date)

cargo --version
# Should output: cargo 1.XX.X (hash date)
```

### 4. Run in Development Mode
```bash
npm run tauri dev
```

This will:
- Start the Vite dev server for frontend hot-reload
- Compile the Rust backend (first run takes ~2-5 minutes)
- Open the Tauri desktop window

### 5. Configure Jira Credentials (in app)
1. Click "Configure Settings" button
2. Enter:
   - **Jira Base URL**: `https://your-domain.atlassian.net`
   - **Email**: Your Atlassian account email
   - **API Token**: Generate from https://id.atlassian.com/manage-profile/security/api-tokens
   - **Default JQL**: e.g., `assignee = currentUser() AND resolution = Unresolved ORDER BY updated DESC`
3. Click Save

---

## Build for Production

```bash
npm run tauri build
```

Output locations:
- Windows: `src-tauri/target/release/jira-viewer.exe`
- Installer: `src-tauri/target/release/bundle/msi/` or `nsis/`

---

## Troubleshooting

### "Cannot read properties of undefined (reading 'invoke')"
- **Cause**: App is running in browser instead of Tauri window
- **Fix**: Run with `npm run tauri dev`, not just `npm run dev`

### "failed to run 'cargo metadata' command"
- **Cause**: Rust/Cargo not installed or not in PATH
- **Fix**: Install Rust via rustup, then restart terminal

### "LINK : fatal error LNK1181: cannot open input file 'windows.0.52.0.lib'"
- **Cause**: Missing Windows SDK or VS Build Tools
- **Fix**: Install Visual Studio Build Tools with C++ workload

### Keyring errors on first run
- **Cause**: No credentials saved yet
- **Fix**: Normal on first run - configure settings in the app

### API returns 401 Unauthorized
- **Cause**: Invalid or expired API token
- **Fix**: Generate new token at https://id.atlassian.com/manage-profile/security/api-tokens

---

## Project File Structure

```
jira-viewer/
├── src/                          # Frontend (Svelte + TypeScript)
│   ├── lib/
│   │   ├── TopBar.svelte        # Search bar, refresh, settings
│   │   ├── LeftPane.svelte      # Issue list
│   │   ├── MainPane.svelte      # Issue details
│   │   └── SettingsModal.svelte # Configuration dialog
│   ├── App.svelte               # Main app, state management
│   ├── main.ts                  # Entry point
│   ├── types.ts                 # TypeScript interfaces
│   └── app.css                  # Global styles + Tailwind
├── src-tauri/                   # Backend (Rust)
│   ├── src/
│   │   ├── lib.rs              # Jira client, commands, models
│   │   └── main.rs             # Entry point
│   ├── Cargo.toml              # Rust dependencies
│   └── tauri.conf.json         # Tauri configuration
├── package.json                 # npm dependencies & scripts
├── vite.config.ts              # Vite configuration
├── tsconfig.json               # TypeScript configuration
└── index.html                  # HTML entry point
```

---

## Jira API Endpoints Used

| Endpoint | Purpose |
|----------|---------|
| `GET /rest/api/3/search?jql=...` | Search issues with JQL |
| `GET /rest/api/3/issue/{key}` | Get single issue details |

Both endpoints require Basic Auth with `email:api_token` base64-encoded.

---

## Next Steps / Future Enhancements

Potential improvements:
- [ ] Persist settings to disk (currently in-memory only)
- [ ] Add issue comments display
- [ ] Add ability to transition issues
- [ ] Add attachment viewing
- [ ] Add worklog support
- [ ] Dark/Light theme toggle
- [ ] Multiple Jira instance support
- [ ] Keyboard shortcuts
- [ ] Issue search/filter within list
