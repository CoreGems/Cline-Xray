// Agent Tab Types

/** A single chat message in the conversation */
export interface ChatMessage {
  /** Role of the message sender: "user" or "model" */
  role: 'user' | 'model';
  /** The content of the message */
  content: string;
}

/** A saved chat session */
export interface ChatSession {
  /** Unique identifier for the session */
  id: string;
  /** Title of the chat session (first message preview or default) */
  title: string;
  /** Messages in this session */
  messages: ChatMessage[];
  /** Timestamp when the session was created */
  createdAt: number;
  /** Timestamp when the session was last updated */
  updatedAt: number;
}

/** A single message in an agent chat conversation */
export interface AgentChatMessage {
  /** Role of the message sender: "user" or "agent" */
  role: 'user' | 'agent';
  /** The content of the message */
  content: string;
  /** Optional timestamp */
  timestamp?: number;
  /** ID of the agent that responded (for agent messages) */
  agentId?: string;
  /** Name of the agent at the time of response (in case agent is deleted) */
  agentName?: string;
  /** Color of the agent at the time of response */
  agentColor?: string;
}

/** A saved agent chat session */
export interface AgentChatSession {
  /** Unique identifier for the session */
  id: string;
  /** Title of the agent chat session (first message preview or default) */
  title: string;
  /** Messages in this session */
  messages: AgentChatMessage[];
  /** ID of the agent used in this session */
  agentId?: string;
  /** Timestamp when the session was created */
  createdAt: number;
  /** Timestamp when the session was last updated */
  updatedAt: number;
}

/** Agent definition */
export interface AgentDefinition {
  /** Unique identifier for the agent */
  id: string;
  /** Display name of the agent */
  name: string;
  /** Description of what the agent does */
  description: string;
  /** System prompt/instructions for the agent */
  systemPrompt: string;
  /** Color for UI (hex or tailwind color) */
  color: string;
  /** Default model to use for this agent */
  defaultModel?: string;
  /** Icon name (optional) */
  icon?: string;
  /** Whether this is a built-in agent */
  isBuiltIn?: boolean;
  /** Timestamp when the agent was created */
  createdAt: number;
  /** Timestamp when the agent was last updated */
  updatedAt: number;
}

/** Agent model from the API */
export interface AgentModel {
  /** Model name (e.g., "models/gemini-2.5-flash") */
  name: string;
  /** Display name (e.g., "Gemini 2.5 Flash") */
  displayName?: string;
  /** Description of the model */
  description?: string;
  /** Input token limit */
  inputTokenLimit?: number;
  /** Output token limit */
  outputTokenLimit?: number;
  /** Supported generation methods */
  supportedGenerationMethods?: string[];
}

/** Response from the /agent/models API */
export interface AgentModelsResponse {
  /** List of available models */
  models: AgentModel[];
  /** Total count of models */
  total: number;
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
export type AgentSubTab = 'Chat' | 'Agent Chat';

/** Subtab definition */
export interface SubTabDefinition {
  id: AgentSubTab;
  label: string;
}
