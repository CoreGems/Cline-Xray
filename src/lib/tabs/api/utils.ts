// API Tab Utility Functions

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
 * @param tag - API tag (system, jira, agent, etc.)
 * @returns Tailwind CSS color classes
 */
export function getTagColor(tag: string): string {
  switch (tag) {
    case 'system': return 'bg-purple-100 text-purple-700';
    case 'jira': return 'bg-indigo-100 text-indigo-700';
    case 'agent': return 'bg-emerald-100 text-emerald-700';
    default: return 'bg-gray-100 text-gray-700';
  }
}
