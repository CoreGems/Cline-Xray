// Agent Tab Types

/** A single chat message in the conversation */
export interface ChatMessage {
  /** Role of the message sender: "user" or "model" */
  role: 'user' | 'model';
  /** The content of the message */
  content: string;
}

/** Request body for the chat API */
export interface ChatRequest {
  /** The user's message to send to Gemini */
  message: string;
  /** Optional conversation history for context */
  history: ChatMessage[];
}

/** Response from the chat API */
export interface ChatResponse {
  /** The AI's response message */
  response: string;
  /** The updated conversation history */
  history: ChatMessage[];
}

/** API connection info from the backend */
export interface ApiInfo {
  base_url: string;
  token: string;
}

/** Available subtabs in the Agent tab */
export type AgentSubTab = 'Chat';

/** Subtab definition */
export interface SubTabDefinition {
  id: AgentSubTab;
  label: string;
}
