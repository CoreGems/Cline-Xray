# RUST_BACKEND_HOWTO.md

How to add a Rust/Axum REST backend with OpenAPI support to an existing UI-only application (like one with tabs).

---

## Overview

This guide explains the tech stack and approach for building a Rust backend that:
- Serves as a REST API for your UI
- Auto-generates OpenAPI specs with `utoipa`
- Runs as a Tauri desktop app (or standalone)
- Supports multiple API tiers (public vs admin)

---

## Tech Stack Summary

| Layer | Technology | Purpose |
|-------|------------|---------|
| **Framework** | [Tauri](https://tauri.app/) + Rust | Desktop app shell, IPC, native dialogs |
| **Web Server** | [Axum](https://github.com/tokio-rs/axum) | REST API routing, middleware |
| **Async Runtime** | [Tokio](https://tokio.rs/) | Async I/O, timers, channels |
| **OpenAPI** | [utoipa](https://github.com/juhaku/utoipa) v5+ | Auto-generate OpenAPI 3.1 specs from code |
| **Serialization** | [Serde](https://serde.rs/) | JSON request/response handling |
| **HTTP Middleware** | [tower-http](https://github.com/tower-rs/tower-http) | CORS, tracing, compression |
| **Auth** | Bearer tokens | Session-based authentication |
| **Logging** | [tracing](https://tracing.rs/) | Structured logging |
| **Frontend** | TypeScript/Vite | UI communicates via REST |

---

## Project Structure

```
your-app/
├── src/                          # Frontend (TypeScript)
│   └── ui/
│       ├── main.ts               # UI entry point
│       ├── api/
│       │   └── client.ts         # REST API client
│       └── components/           # UI components (tabs, etc.)
│
├── src-tauri/                    # Backend (Rust)
│   ├── Cargo.toml                # Rust dependencies
│   ├── tauri.conf.json           # Tauri configuration
│   └── src/
│       ├── main.rs               # Entry point, Tauri setup
│       ├── server.rs             # Axum router setup
│       ├── openapi.rs            # OpenAPI spec definitions
│       ├── state.rs              # Shared app state
│       └── api/
│           ├── mod.rs            # Module exports
│           ├── handlers.rs       # Route handlers
│           ├── middleware.rs     # Auth, logging middleware
│           └── routes.rs         # Route constants
│
├── .env                          # Environment variables
└── package.json                  # Frontend dependencies
```

---

## Step 1: Set Up Cargo.toml

Add these dependencies in `src-tauri/Cargo.toml`:

```toml
[package]
name = "your-app"
version = "0.1.0"
edition = "2021"

[dependencies]
# Tauri
tauri = { version = "1", features = ["shell-open", "dialog-open"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Async runtime
tokio = { version = "1", features = ["full"] }

# Web server
axum = { version = "0.7", features = ["macros"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }

# OpenAPI (v5+ for OpenAPI 3.1.0)
utoipa = { version = "5", features = ["axum_extras"] }

# Utilities
uuid = { version = "1", features = ["v4", "serde"] }
rand = "0.8"
anyhow = "1"
chrono = { version = "0.4", features = ["serde"] }
hex = "0.4"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Environment
dotenvy = "0.15"
```

---

## Step 2: Create the Application State

`src-tauri/src/state.rs`:

```rust
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::Instant;

pub struct AppState {
    pub auth_token: String,           // Generated at startup
    pub start_time: Instant,          // For uptime tracking
    pub api_base_url: RwLock<Option<String>>,
}

impl AppState {
    pub fn new(auth_token: String) -> Arc<Self> {
        Arc::new(Self {
            auth_token,
            start_time: Instant::now(),
            api_base_url: RwLock::new(None),
        })
    }

    pub fn verify_token(&self, token: &str) -> bool {
        self.auth_token == token
    }

    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}
```

---

## Step 3: Create Route Handlers

`src-tauri/src/api/handlers.rs`:

```rust
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::state::AppState;

// Response types with utoipa ToSchema for OpenAPI
#[derive(Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub uptime_secs: u64,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ItemsResponse {
    pub items: Vec<Item>,
    pub count: usize,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct Item {
    pub id: String,
    pub name: String,
}

// Request types
#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateItemRequest {
    pub name: String,
}

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service healthy", body = HealthResponse)
    ),
    tag = "system"
)]
pub async fn health_handler(
    State(state): State<Arc<AppState>>
) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        uptime_secs: state.uptime_secs(),
    })
}

/// List items endpoint
#[utoipa::path(
    get,
    path = "/items",
    responses(
        (status = 200, description = "List of items", body = ItemsResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "items"
)]
pub async fn list_items_handler() -> Json<ItemsResponse> {
    // Your business logic here
    Json(ItemsResponse {
        items: vec![],
        count: 0,
    })
}

/// Create item endpoint
#[utoipa::path(
    post,
    path = "/items",
    request_body = CreateItemRequest,
    responses(
        (status = 201, description = "Item created", body = Item)
    ),
    security(("bearerAuth" = [])),
    tag = "items"
)]
pub async fn create_item_handler(
    Json(req): Json<CreateItemRequest>
) -> Json<Item> {
    Json(Item {
        id: uuid::Uuid::new_v4().to_string(),
        name: req.name,
    })
}
```

---

## Step 4: Create Auth Middleware

`src-tauri/src/api/middleware.rs`:

```rust
use axum::{
    extract::State,
    http::{header, Request, StatusCode},
    middleware::Next,
    response::Response,
    body::Body,
};
use std::sync::Arc;
use crate::state::AppState;

/// Auth middleware - validates Bearer token
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let token = auth_header
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !state.verify_token(token) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
}
```

---

## Step 5: Define OpenAPI Specs

`src-tauri/src/openapi.rs`:

```rust
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};

/// Public OpenAPI spec (safe endpoints)
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Your App API",
        version = "1.0.0",
        description = "API documentation"
    ),
    paths(
        crate::api::handlers::health_handler,
        crate::api::handlers::list_items_handler,
        crate::api::handlers::create_item_handler,
    ),
    components(
        schemas(
            crate::api::handlers::HealthResponse,
            crate::api::handlers::Item,
            crate::api::handlers::ItemsResponse,
            crate::api::handlers::CreateItemRequest,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "system", description = "System endpoints"),
        (name = "items", description = "Item management")
    )
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        // Set OpenAPI 3.1.0 (required for OpenAI GPT Actions)
        openapi.openapi = utoipa::openapi::OpenApiVersion::Version31;
        
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearerAuth",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            );
        }
    }
}
```

---

## Step 6: Create the Axum Router

`src-tauri/src/server.rs`:

```rust
use crate::api::{handlers, middleware::auth_middleware};
use crate::openapi::ApiDoc;
use crate::state::AppState;
use axum::{
    extract::State,
    middleware,
    response::Json,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use utoipa::OpenApi;

pub fn create_router(state: Arc<AppState>) -> Router {
    // CORS - adjust for production
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Public routes (no auth)
    let public_routes = Router::new()
        .route("/health", get(handlers::health_handler))
        .route("/openapi.json", get(openapi_handler));

    // Protected routes (require auth)
    let protected_routes = Router::new()
        .route("/items", get(handlers::list_items_handler))
        .route("/items", post(handlers::create_item_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(cors)
        .with_state(state)
}

/// Serve OpenAPI spec as JSON
async fn openapi_handler() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
```

---

## Step 7: Wire Up main.rs with Tauri

`src-tauri/src/main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod openapi;
mod server;
mod state;

use anyhow::Result;
use state::AppState;
use std::sync::Arc;
use tauri::Manager;

/// API info returned to the UI (call once at startup)
#[derive(Clone, serde::Serialize)]
pub struct ApiInfo {
    pub base_url: String,
    pub token: String,
}

/// Tauri command: Get API connection info
#[tauri::command]
fn get_api_info(api_info: tauri::State<ApiInfo>) -> ApiInfo {
    (*api_info).clone()
}

fn main() {
    // Load .env
    dotenvy::dotenv().ok();

    // Initialize logging
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .setup(|app| {
            // Generate random auth token for this session
            let auth_token = generate_auth_token();
            
            // Create app state
            let state = AppState::new(auth_token.clone());

            // Start the REST server (loopback only!)
            let (base_url, _shutdown_tx) = start_server(state.clone())?;

            tracing::info!("REST API started at {}", base_url);

            // Store for UI access
            let api_info = ApiInfo {
                base_url,
                token: auth_token,
            };

            app.manage(state);
            app.manage(api_info);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_api_info])
        .run(tauri::generate_context!())
        .expect("error running app");
}

/// Generate a secure random auth token
fn generate_auth_token() -> String {
    use rand::Rng;
    let bytes: [u8; 32] = rand::thread_rng().gen();
    hex::encode(bytes)
}

/// Start the Axum REST server
/// SECURITY: Always binds to 127.0.0.1, never 0.0.0.0
fn start_server(state: Arc<AppState>) -> Result<(String, tokio::sync::oneshot::Sender<()>)> {
    use tokio::net::TcpListener;

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let runtime = tokio::runtime::Runtime::new()?;

    let (actual_addr, server_future) = runtime.block_on(async {
        // SECURITY: Bind to loopback only!
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let actual_addr = listener.local_addr()?;

        let app = server::create_router(state);

        let server = axum::serve(listener, app)
            .with_graceful_shutdown(async { let _ = shutdown_rx.await; });

        Ok::<_, anyhow::Error>((actual_addr, server))
    })?;

    // Run server in background thread
    std::thread::spawn(move || {
        runtime.block_on(async {
            if let Err(e) = server_future.await {
                tracing::error!("Server error: {}", e);
            }
        });
    });

    let base_url = format!("http://{}", actual_addr);
    Ok((base_url, shutdown_tx))
}
```

---

## Step 8: Create the UI API Client

`src/ui/api/client.ts`:

```typescript
// API connection info (obtained once at startup via Tauri IPC)
let apiInfo: { base_url: string; token: string } | null = null;

/**
 * Initialize API client - call ONCE at app startup
 */
export async function initializeApi(): Promise<void> {
  if (apiInfo) return;
  
  // Get connection info from Tauri backend
  const { invoke } = await import('@tauri-apps/api/tauri');
  apiInfo = await invoke<{ base_url: string; token: string }>('get_api_info');
  console.log('API initialized:', apiInfo.base_url);
}

/**
 * Generic request function
 */
async function request<T>(
  path: string, 
  options: { method?: string; body?: unknown; requiresAuth?: boolean } = {}
): Promise<T> {
  if (!apiInfo) throw new Error('API not initialized');
  
  const { method = 'GET', body, requiresAuth = true } = options;
  
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };
  
  if (requiresAuth) {
    headers['Authorization'] = `Bearer ${apiInfo.token}`;
  }
  
  const response = await fetch(`${apiInfo.base_url}${path}`, {
    method,
    headers,
    body: body ? JSON.stringify(body) : undefined,
  });
  
  if (!response.ok) {
    const text = await response.text();
    throw new Error(`API Error (${response.status}): ${text}`);
  }
  
  return response.json();
}

// Type definitions
export interface HealthResponse {
  status: string;
  uptime_secs: number;
}

export interface Item {
  id: string;
  name: string;
}

export interface ItemsResponse {
  items: Item[];
  count: number;
}

// API methods
export async function getHealth(): Promise<HealthResponse> {
  return request('/health', { requiresAuth: false });
}

export async function getItems(): Promise<ItemsResponse> {
  return request('/items');
}

export async function createItem(name: string): Promise<Item> {
  return request('/items', { method: 'POST', body: { name } });
}

// OpenAPI spec
export async function getOpenApiSpec(): Promise<unknown> {
  return request('/openapi.json', { requiresAuth: false });
}
```

---

## Step 9: Initialize in UI Entry Point

`src/ui/main.ts`:

```typescript
import { initializeApi } from './api/client';

async function main() {
  // Initialize API client FIRST
  await initializeApi();
  
  // Now render your app / tabs
  renderApp();
}

main().catch(console.error);
```

---

## Key Patterns

### 1. Auth Token Flow
```
Tauri startup
    │
    ├── Generate random token
    ├── Store in AppState
    ├── Pass to UI via get_api_info command
    │
UI startup
    │
    ├── Call invoke('get_api_info')
    ├── Store { base_url, token }
    └── Use token in Authorization header
```

### 2. Server Binding (Security)
```rust
// ✅ ALWAYS bind to loopback
TcpListener::bind("127.0.0.1:0")  // Ephemeral port

// ❌ NEVER bind to all interfaces
TcpListener::bind("0.0.0.0:3000")  // Security risk!
```

### 3. OpenAPI Generation
```rust
// Define on handler
#[utoipa::path(
    get,
    path = "/items",
    responses((status = 200, body = ItemsResponse)),
    tag = "items"
)]
pub async fn list_items_handler() -> Json<ItemsResponse> { ... }

// Register in OpenApi derive
#[derive(OpenApi)]
#[openapi(
    paths(list_items_handler),
    components(schemas(ItemsResponse))
)]
pub struct ApiDoc;
```

### 4. Split API (Optional - for GPT Actions)

If you need separate API specs for different consumers:

```rust
// GPT-safe spec (read-only, constrained)
#[derive(OpenApi)]
#[openapi(paths(list_items_handler))]
pub struct GptSafeApiDoc;

// Admin spec (full access)
#[derive(OpenApi)]
#[openapi(paths(list_items_handler, create_item_handler, delete_item_handler))]
pub struct AdminApiDoc;

// Routes
.route("/openapi.json", get(|| async { Json(GptSafeApiDoc::openapi()) }))
.route("/openapi-admin.json", get(|| async { Json(AdminApiDoc::openapi()) }))
```

---

## Browser Dev Mode (Optional)

For faster UI iteration without rebuilding Tauri:

`vite.config.ts`:
```typescript
export default defineConfig({
  define: {
    'import.meta.env.VITE_DEV_PORT': JSON.stringify(process.env.ORACLE_XRAY_DEV_PORT || '3030'),
    'import.meta.env.VITE_DEV_TOKEN': JSON.stringify(process.env.ORACLE_XRAY_LOCAL_TOKEN || ''),
  },
});
```

`src/ui/api/client.ts`:
```typescript
export async function initializeApi(): Promise<void> {
  if ('__TAURI_IPC__' in window) {
    // Tauri mode - get from backend
    const { invoke } = await import('@tauri-apps/api/tauri');
    apiInfo = await invoke('get_api_info');
  } else {
    // Browser dev mode - use env vars
    apiInfo = {
      base_url: `http://127.0.0.1:${import.meta.env.VITE_DEV_PORT}`,
      token: import.meta.env.VITE_DEV_TOKEN,
    };
  }
}
```

---

## Testing the API

PowerShell test script:
```powershell
# Health check (no auth)
Invoke-RestMethod -Uri "http://127.0.0.1:3030/health"

# With auth
$headers = @{ "Authorization" = "Bearer $env:ORACLE_XRAY_LOCAL_TOKEN" }
Invoke-RestMethod -Uri "http://127.0.0.1:3030/items" -Headers $headers

# OpenAPI spec
Invoke-RestMethod -Uri "http://127.0.0.1:3030/openapi.json"
```

---

## Summary Checklist

- [ ] Add Axum, Tokio, utoipa to Cargo.toml
- [ ] Create `state.rs` with AppState struct
- [ ] Create `api/handlers.rs` with `#[utoipa::path]` annotations
- [ ] Create `api/middleware.rs` with auth middleware
- [ ] Create `openapi.rs` with `#[derive(OpenApi)]`
- [ ] Create `server.rs` with router setup
- [ ] Wire up `main.rs` with Tauri setup and server start
- [ ] Create `src/ui/api/client.ts` with typed API methods
- [ ] Call `initializeApi()` in UI entry point
- [ ] Test with PowerShell/curl

---

## References

- [Axum Documentation](https://docs.rs/axum)
- [utoipa Documentation](https://docs.rs/utoipa)
- [Tauri Documentation](https://tauri.app/v1/guides/)
- [Tower HTTP Middleware](https://docs.rs/tower-http)