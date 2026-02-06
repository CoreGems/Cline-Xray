// API Tab - Endpoint Definitions

import type { ApiEndpoint } from './types';

/**
 * List of all available REST API endpoints
 */
export const endpoints: ApiEndpoint[] = [
  {
    method: 'GET',
    path: '/health',
    description: 'Health check endpoint - returns service status and uptime',
    tag: 'system',
    auth: false
  },
  {
    method: 'GET',
    path: '/openapi.json',
    description: 'OpenAPI specification in JSON format',
    tag: 'system',
    auth: false
  },
  {
    method: 'GET',
    path: '/access-logs',
    description: 'Get all HTTP access log entries',
    tag: 'system',
    auth: false
  },
  {
    method: 'DELETE',
    path: '/access-logs',
    description: 'Clear all HTTP access log entries',
    tag: 'system',
    auth: false
  },
  {
    method: 'GET',
    path: '/jira/list',
    description: 'List Jira issues based on JQL query',
    tag: 'jira',
    auth: true
  },
  {
    method: 'POST',
    path: '/agent/chat',
    description: 'Chat with Google Gemini AI - supports conversation history',
    tag: 'agent',
    auth: true
  }
];
