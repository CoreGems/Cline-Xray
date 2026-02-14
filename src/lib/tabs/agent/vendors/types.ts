// ============================================================================
// Inference Vendor Abstraction Layer
// ============================================================================
//
// This module defines the interface that all inference vendors must implement.
// To add a new vendor (e.g., OpenAI, Anthropic), create a new file that
// implements the InferenceVendor interface and register it in registry.ts.
//
// Architecture:
//   vendors/types.ts     - Interfaces & shared types (this file)
//   vendors/registry.ts  - Vendor registry (register/lookup)
//   vendors/gemini.ts    - Google Gemini implementation
//   vendors/openai.ts    - (future) OpenAI implementation
//   vendors/index.ts     - Re-exports

import type { AgentSettings } from '../../../../types';

/** A normalized model option for the UI (vendor-agnostic) */
export interface ModelOption {
  /** Unique model ID (e.g., "gemini-2.0-flash", "gpt-4o") */
  id: string;
  /** Human-readable display name */
  displayName: string;
  /** Optional description */
  description?: string;
  /** The vendor that owns this model */
  vendorId: string;
}

/** Branding information for vendor-specific UI styling */
export interface VendorBranding {
  /** Primary brand color (hex) */
  primaryColor: string;
  /** Gradient CSS string for fancy text (optional) */
  gradientCSS?: string;
  /** Small icon/emoji for inline display */
  icon: string;
  /** Vendor attribution line shown in empty state */
  poweredBy: string;
  /** Title for the empty chat state */
  emptyStateTitle: string;
  /** Decorative HTML for the empty state logo area (optional) */
  emptyStateLogoHTML?: string;
}

/** API connection info from the Tauri backend */
export interface VendorApiInfo {
  base_url: string;
  token: string;
}

/** Chat message format (vendor-agnostic) */
export interface VendorChatMessage {
  role: 'user' | 'model';
  content: string;
}

/** Chat response format (vendor-agnostic) */
export interface VendorChatResponse {
  response: string;
  history: VendorChatMessage[];
}

/**
 * Interface that all inference vendors must implement.
 *
 * Each vendor is responsible for:
 *  1. Fetching its model list from the backend API
 *  2. Normalizing models into ModelOption[]
 *  3. Sending chat messages through the backend
 *  4. Providing branding/styling info for the UI
 *
 * To add a new vendor:
 *  1. Create a new file (e.g., vendors/openai.ts)
 *  2. Implement this interface
 *  3. Register it in vendors/registry.ts
 */
export interface InferenceVendor {
  /** Unique vendor identifier (e.g., "gemini", "openai") */
  readonly id: string;

  /** Human-readable vendor name (e.g., "Google Gemini") */
  readonly name: string;

  /** Default model ID for this vendor */
  readonly defaultModel: string;

  /** UI branding configuration */
  readonly branding: VendorBranding;

  /**
   * Fetch available models from the backend API.
   * The vendor is responsible for calling the right endpoint and
   * normalizing the response into ModelOption[].
   *
   * @param apiInfo - Backend connection info (base_url + token)
   * @param settings - User's agent filter settings
   * @returns Normalized model list sorted by displayName
   */
  fetchModels(apiInfo: VendorApiInfo, settings: AgentSettings): Promise<ModelOption[]>;

  /**
   * Send a chat message through this vendor's backend endpoint.
   *
   * @param apiInfo - Backend connection info
   * @param message - The user's message text
   * @param history - Previous conversation messages
   * @param model - Model ID to use
   * @returns The AI response and updated history
   */
  sendChatMessage(
    apiInfo: VendorApiInfo,
    message: string,
    history: VendorChatMessage[],
    model: string,
  ): Promise<VendorChatResponse>;

  /**
   * Check if a given model ID belongs to this vendor.
   * Used for routing model selections to the correct vendor.
   */
  ownsModel(modelId: string): boolean;
}
