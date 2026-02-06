// API Tab Types

/** HTTP methods supported by the API */
export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'DELETE';

/** An API endpoint definition */
export interface ApiEndpoint {
  method: HttpMethod;
  path: string;
  description: string;
  tag: string;
  auth: boolean;
}

/** Available subtabs in the API tab */
export type ApiSubTab = 'REST';

/** Subtab definition */
export interface SubTabDefinition {
  id: ApiSubTab;
  label: string;
}
