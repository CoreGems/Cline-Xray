// Activity Tab Utility Functions

/**
 * Format timestamp for display in HH:MM:SS format
 * @param timestamp - ISO timestamp string
 * @returns Formatted time string
 */
export function formatTimestamp(timestamp: string): string {
  try {
    const date = new Date(timestamp);
    return date.toLocaleTimeString('en-US', { 
      hour12: false, 
      hour: '2-digit', 
      minute: '2-digit', 
      second: '2-digit' 
    });
  } catch {
    return timestamp;
  }
}

/**
 * Get CSS color class for HTTP status code
 * @param statusCode - HTTP status code
 * @returns Tailwind CSS color class
 */
export function getStatusColor(statusCode: number): string {
  if (statusCode >= 200 && statusCode < 300) return 'text-green-600';
  if (statusCode >= 400 && statusCode < 500) return 'text-yellow-600';
  if (statusCode >= 500) return 'text-red-600';
  return 'text-gray-600';
}

/**
 * Get CSS color class for HTTP method
 * @param method - HTTP method (GET, POST, PUT, DELETE, etc.)
 * @returns Tailwind CSS color classes for background and text
 */
export function getMethodColor(method: string): string {
  switch (method.toUpperCase()) {
    case 'GET': return 'bg-blue-100 text-blue-800';
    case 'POST': return 'bg-green-100 text-green-800';
    case 'PUT': return 'bg-yellow-100 text-yellow-800';
    case 'DELETE': return 'bg-red-100 text-red-800';
    default: return 'bg-gray-100 text-gray-800';
  }
}
