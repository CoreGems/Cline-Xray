// API Tab Types

/** HTTP methods supported by the API */
export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'DELETE';

/** API type - public or admin (internal) */
export type ApiType = 'public' | 'admin';

/** An API endpoint definition */
export interface ApiEndpoint {
  method: HttpMethod;
  path: string;
  description: string;
  tags: string[];  // Changed from single tag to array of tags
  auth: boolean;
  apiType: ApiType;
}

/** Available subtabs in the API tab */
export type ApiSubTab = 'REST' | 'Tools';

/** Subtab definition */
export interface SubTabDefinition {
  id: ApiSubTab;
  label: string;
}
