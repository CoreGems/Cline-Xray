// Agent Tab API Functions

import { invoke } from "@tauri-apps/api/core";
import type { ApiInfo, ChatMessage, ChatResponse } from "./types";

/**
 * Get API connection info from the Tauri backend
 */
export async function getApiInfo(): Promise<ApiInfo> {
  return await invoke('get_api_info');
}

/**
 * Send a chat message to Gemini via the backend API
 * @param message - The user's message
 * @param history - Previous conversation history
 * @returns The chat response with AI reply and updated history
 */
export async function sendChatMessage(
  message: string,
  history: ChatMessage[]
): Promise<ChatResponse> {
  const apiInfo = await getApiInfo();
  
  const response = await fetch(`${apiInfo.base_url}/agent/chat`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${apiInfo.token}`
    },
    body: JSON.stringify({
      message,
      history
    })
  });

  if (!response.ok) {
    const errorData = await response.json();
    throw new Error(errorData.error || `HTTP error ${response.status}`);
  }

  return await response.json();
}
