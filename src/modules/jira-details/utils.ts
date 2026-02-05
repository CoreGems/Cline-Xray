// Utility functions for Jira Details components

export function getStatusClass(statusCategory: string): string {
  switch (statusCategory.toLowerCase()) {
    case "done":
      return "status-done";
    case "in progress":
    case "indeterminate":
      return "status-in-progress";
    case "blocked":
      return "status-blocked";
    default:
      return "status-todo";
  }
}

export function formatDate(dateString: string): string {
  const date = new Date(dateString);
  return date.toLocaleDateString();
}

export function formatDateTime(dateString: string): string {
  const date = new Date(dateString);
  return date.toLocaleString();
}

export function truncate(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;
  return text.substring(0, maxLength) + "...";
}

export function renderDescription(description: string | null): string {
  if (!description) return "<p class='text-gray-400 italic'>No description</p>";
  // Basic Jira markup to HTML conversion
  // Note: whitespace-pre-wrap CSS handles newlines, so no need for <br> replacement
  return description
    .replace(/\*(\w+)\*/g, "<strong>$1</strong>")
    .replace(/_(\w+)_/g, "<em>$1</em>");
}
