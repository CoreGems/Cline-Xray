// API Tab Utility Functions

import { invoke } from "@tauri-apps/api/core";
import type { ApiEndpoint, ApiType } from './types';

// ============ API Info Types ============

/** API connection info from Tauri backend */
interface ApiInfo {
  base_url: string;
  token: string;
}

// ============ Session Cache for Endpoints ============

/** Cached endpoints (persists for session) */
let cachedEndpoints: ApiEndpoint[] | null = null;

// ============ Fetch Endpoints from OpenAPI ============

/**
 * Fetch API endpoints from both OpenAPI specs at runtime.
 * Merges /openapi.json (public) and /openapi_admin.json (admin).
 * 
 * Results are cached for the session to avoid repeated fetches.
 * 
 * @param forceRefresh - If true, bypass cache and fetch fresh data
 * @returns Promise<ApiEndpoint[]> - Merged list of all endpoints
 */
export async function fetchEndpointsFromOpenApi(forceRefresh = false): Promise<ApiEndpoint[]> {
  // Return cached data if available and not forcing refresh
  if (cachedEndpoints && !forceRefresh) {
    return cachedEndpoints;
  }

  // Get dynamic API base URL from Tauri backend
  const apiInfo: ApiInfo = await invoke('get_api_info');
  
  // Fetch both specs in parallel
  const [publicResp, adminResp] = await Promise.all([
    fetch(`${apiInfo.base_url}/openapi.json`),
    fetch(`${apiInfo.base_url}/openapi_admin.json`)
  ]);

  if (!publicResp.ok) {
    throw new Error(`Failed to fetch public OpenAPI spec: HTTP ${publicResp.status}`);
  }
  if (!adminResp.ok) {
    throw new Error(`Failed to fetch admin OpenAPI spec: HTTP ${adminResp.status}`);
  }

  const [publicSpec, adminSpec] = await Promise.all([
    publicResp.json(),
    adminResp.json()
  ]);

  // Parse and merge endpoints
  const publicEndpoints = parseOpenApiSpec(publicSpec, 'public');
  const adminEndpoints = parseOpenApiSpec(adminSpec, 'admin');

  // Merge and deduplicate (admin endpoints may override public)
  const endpointMap = new Map<string, ApiEndpoint>();
  
  // Add public endpoints first
  for (const ep of publicEndpoints) {
    const key = `${ep.method}:${ep.path}`;
    endpointMap.set(key, ep);
  }
  
  // Add admin endpoints (may override if same path exists)
  for (const ep of adminEndpoints) {
    const key = `${ep.method}:${ep.path}`;
    // Only add if not already in public spec, or override with admin version
    if (!endpointMap.has(key)) {
      endpointMap.set(key, ep);
    }
  }

  // Sort endpoints: public first, then admin, then alphabetically by path
  cachedEndpoints = Array.from(endpointMap.values()).sort((a, b) => {
    // Sort by apiType first (public before admin)
    if (a.apiType !== b.apiType) {
      return a.apiType === 'public' ? -1 : 1;
    }
    // Then by path alphabetically
    return a.path.localeCompare(b.path);
  });

  return cachedEndpoints;
}

/**
 * Parse an OpenAPI spec into ApiEndpoint objects
 * @param spec - OpenAPI JSON spec
 * @param apiType - Whether this is 'public' or 'admin' spec
 */
function parseOpenApiSpec(spec: any, apiType: ApiType): ApiEndpoint[] {
  const endpoints: ApiEndpoint[] = [];

  for (const [path, methods] of Object.entries(spec.paths || {})) {
    const methodsObj = methods as Record<string, any>;
    
    for (const [method, operation] of Object.entries(methodsObj)) {
      // Skip non-operation keys like 'parameters', 'servers', etc.
      if (!['get', 'post', 'put', 'delete', 'patch'].includes(method.toLowerCase())) {
        continue;
      }

      const op = operation as any;
      
      // Extract tags (default to ['untagged'] if none)
      const tags: string[] = op.tags || ['untagged'];
      
      // Determine if auth is required (has security requirements)
      const auth = Boolean(op.security?.length);

      endpoints.push({
        method: method.toUpperCase() as ApiEndpoint['method'],
        path,
        description: op.summary || op.description || '',
        tags,
        auth,
        apiType
      });
    }
  }

  return endpoints;
}

/**
 * Clear the cached endpoints (useful for testing or manual refresh)
 */
export function clearEndpointCache(): void {
  cachedEndpoints = null;
}

/** An agent tool auto-generated from OpenAPI spec */
export interface AgentTool {
  /** Tool name derived from method + path (e.g., "get_jira_list") */
  name: string;
  /** HTTP method (GET, POST, etc.) */
  method: string;
  /** API path (e.g., "/jira/list") */
  path: string;
  /** Human-readable description from OpenAPI summary/description */
  description: string;
  /** Tool parameters extracted from OpenAPI spec */
  parameters: { name: string; type: string; required: boolean }[];
  /** Whether this endpoint requires authentication */
  auth: boolean;
}

// ============ OpenAPI Tool Fetcher ============

/** Paths to skip when generating tools (not useful as agent tools) */
const SKIP_PATHS = ['/agent/chat', '/openapi.json'];

/**
 * Fetch and parse /openapi.json into agent tool definitions.
 * 
 * Auto-generates tool names from HTTP method + path:
 *   GET /jira/list → "get_jira_list"
 *   GET /health → "get_health"
 * 
 * Skips /agent/chat (the agent itself) and /openapi.json (meta endpoint).
 */
export async function fetchToolsFromOpenApi(): Promise<AgentTool[]> {
  // Get dynamic API base URL from Tauri backend
  const apiInfo: ApiInfo = await invoke('get_api_info');
  
  const resp = await fetch(`${apiInfo.base_url}/openapi.json`);
  if (!resp.ok) {
    throw new Error(`Failed to fetch OpenAPI spec: HTTP ${resp.status}`);
  }
  const spec = await resp.json();
  const tools: AgentTool[] = [];

  for (const [path, methods] of Object.entries(spec.paths || {})) {
    if (SKIP_PATHS.includes(path)) continue;

    const methodsObj = methods as Record<string, any>;
    for (const [method, op] of Object.entries(methodsObj)) {
      const operation = op as any;
      // Build tool name: get_jira_list, get_health, get_agent_models
      const pathSlug = path.replace(/^\//, '').replace(/\//g, '_').replace(/[{}]/g, '');
      
      // Extract parameters from query params
      const params: { name: string; type: string; required: boolean }[] =
        (operation.parameters || []).map((p: any) => ({
          name: p.name,
          type: p.schema?.type || 'string',
          required: p.required || false,
        }));

      // Extract parameters from request body (for POST endpoints)
      if (operation.requestBody?.content?.['application/json']?.schema) {
        const bodySchema = operation.requestBody.content['application/json'].schema;
        // Handle $ref by looking up in components
        const resolvedSchema = bodySchema.$ref
          ? resolveRef(spec, bodySchema.$ref)
          : bodySchema;
        
        if (resolvedSchema?.properties) {
          const requiredFields: string[] = resolvedSchema.required || [];
          for (const [propName, propSchema] of Object.entries(resolvedSchema.properties as Record<string, any>)) {
            params.push({
              name: propName,
              type: propSchema.type || 'string',
              required: requiredFields.includes(propName),
            });
          }
        }
      }

      tools.push({
        name: `${method}_${pathSlug}`,
        method: method.toUpperCase(),
        path,
        description: operation.summary || operation.description || '',
        parameters: params,
        auth: !!operation.security?.length,
      });
    }
  }

  return tools;
}

/**
 * Resolve a JSON $ref pointer in the OpenAPI spec
 * e.g., "#/components/schemas/ChatRequest" → the actual schema object
 */
function resolveRef(spec: any, ref: string): any {
  if (!ref.startsWith('#/')) return null;
  const parts = ref.slice(2).split('/');
  let current = spec;
  for (const part of parts) {
    current = current?.[part];
  }
  return current;
}

// ============ CSS Utility Functions ============

/**
 * Get CSS color class for HTTP method
 * @param method - HTTP method (GET, POST, PUT, DELETE)
 * @returns Tailwind CSS color classes
 */
export function getMethodColor(method: string): string {
  switch (method) {
    case 'GET': return 'bg-green-100 text-green-800';
    case 'POST': return 'bg-blue-100 text-blue-800';
    case 'PUT': return 'bg-yellow-100 text-yellow-800';
    case 'DELETE': return 'bg-red-100 text-red-800';
    default: return 'bg-gray-100 text-gray-800';
  }
}

/**
 * Get CSS color class for API tag
 * @param tag - API tag (system, jira, agent, admin, etc.)
 * @returns Tailwind CSS color classes
 */
export function getTagColor(tag: string): string {
  switch (tag) {
    case 'system': return 'bg-purple-100 text-purple-700';
    case 'jira': return 'bg-indigo-100 text-indigo-700';
    case 'agent': return 'bg-emerald-100 text-emerald-700';
    case 'admin': return 'bg-pink-100 text-pink-700';
    case 'tools': return 'bg-cyan-100 text-cyan-700';
    case 'tool': return 'bg-amber-100 text-amber-700';  // AI agent tool-suitable
    default: return 'bg-gray-100 text-gray-700';
  }
}
