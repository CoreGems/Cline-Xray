// Agent Tab API Functions

import { invoke } from "@tauri-apps/api/core";
import type { ApiInfo, ChatMessage, ChatResponse, AgentModelsResponse } from "./types";

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
 * @param model - Optional model to use (e.g. "gemini-2.0-flash")
 * @returns The chat response with AI reply and updated history
 */
export async function sendChatMessage(
  message: string,
  history: ChatMessage[],
  model?: string
): Promise<ChatResponse> {
  const apiInfo = await getApiInfo();
  
  const body: Record<string, unknown> = { message, history };
  if (model) {
    body.model = model;
  }

  const response = await fetch(`${apiInfo.base_url}/agent/chat`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${apiInfo.token}`
    },
    body: JSON.stringify(body)
  });

  if (!response.ok) {
    const errorData = await response.json();
    throw new Error(errorData.error || `HTTP error ${response.status}`);
  }

  return await response.json();
}

/**
 * Get available agent models from the backend API
 * @returns List of available models
 */
export async function getAgentModels(): Promise<AgentModelsResponse> {
  const apiInfo = await getApiInfo();
  
  const response = await fetch(`${apiInfo.base_url}/agent/models`, {
    method: 'GET',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${apiInfo.token}`
    }
  });

  if (!response.ok) {
    const errorData = await response.json();
    throw new Error(errorData.error || `HTTP error ${response.status}`);
  }

  return await response.json();
}
