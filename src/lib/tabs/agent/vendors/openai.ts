// ============================================================================
// OpenAI Vendor Implementation
// ============================================================================
//
// This module implements the InferenceVendor interface for OpenAI.
// It fetches models from the /agent/openai/models backend endpoint (which
// proxies to OpenAI's API) and sends chat via /agent/openai/chat.
//
// The backend serves OpenAI-specific data; this vendor normalizes
// it into the vendor-agnostic ModelOption format.

import type { AgentSettings } from '../../../../types';
import { getVendorSettings } from '../../../../types';
import type {
  InferenceVendor,
  ModelOption,
  VendorApiInfo,
  VendorBranding,
  VendorChatMessage,
  VendorChatResponse,
} from './types';

/** Raw model shape returned by the /agent/openai/models backend endpoint */
interface OpenAIApiModel {
  id: string;
  object?: string;
  created?: number;
  owned_by?: string;
}

/** Raw response from /agent/openai/models */
interface OpenAIModelsApiResponse {
  models: OpenAIApiModel[];
  total: number;
}

// ---- OpenAI vendor branding ------------------------------------------------

const openaiBranding: VendorBranding = {
  primaryColor: '#10A37F',
  gradientCSS: 'linear-gradient(to right, #10A37F, #1A7F64)',
  icon: '◆',
  poweredBy: 'Powered by OpenAI',
  emptyStateTitle: 'Chat with OpenAI',
  emptyStateLogoHTML: '<span style="color:#10A37F;font-weight:700;font-size:1.2em">OpenAI</span>',
};

// ---- Implementation --------------------------------------------------------

class OpenAIVendor implements InferenceVendor {
  readonly id = 'openai';
  readonly name = 'OpenAI';
  readonly defaultModel = 'gpt-4o-mini';
  readonly branding = openaiBranding;

  // OpenAI model IDs typically start with these prefixes
  private static readonly MODEL_PREFIXES = [
    'gpt-',
    'o1-',
    'o3-',
    'o4-',
    'chatgpt-',
  ];

  ownsModel(modelId: string): boolean {
    return OpenAIVendor.MODEL_PREFIXES.some((prefix) => modelId.startsWith(prefix));
  }

  async fetchModels(apiInfo: VendorApiInfo, settings: AgentSettings): Promise<ModelOption[]> {
    const response = await fetch(`${apiInfo.base_url}/agent/openai/models`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${apiInfo.token}`,
      },
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        (errorData as any).error || `OpenAI models HTTP error ${response.status}`,
      );
    }

    const data: OpenAIModelsApiResponse = await response.json();

    // 1. Normalize models into ModelOption format
    let models: ModelOption[] = (data.models || []).map((m) => ({
      id: m.id,
      displayName: formatModelDisplayName(m.id),
      description: m.owned_by ? `Owned by ${m.owned_by}` : undefined,
      vendorId: this.id,
    }));

    // 2. Apply keyword filtering from per-vendor settings
    const vendorSettings = getVendorSettings(settings, this.id);
    if (vendorSettings.filterEnabled && vendorSettings.excludeKeywords.length > 0) {
      models = models.filter((model) => {
        const searchText =
          `${model.id} ${model.displayName} ${model.description || ''}`.toLowerCase();
        return !vendorSettings.excludeKeywords.some((keyword: string) =>
          searchText.includes(keyword.toLowerCase()),
        );
      });
    }

    // 3. Sort alphabetically
    return models.sort((a, b) => a.displayName.localeCompare(b.displayName));
  }

  async sendChatMessage(
    apiInfo: VendorApiInfo,
    message: string,
    history: VendorChatMessage[],
    model: string,
  ): Promise<VendorChatResponse> {
    const body: Record<string, unknown> = { message, history };
    if (model) {
      body.model = model;
    }

    const response = await fetch(`${apiInfo.base_url}/agent/openai/chat`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${apiInfo.token}`,
      },
      body: JSON.stringify(body),
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        (errorData as any).error || `OpenAI chat HTTP error ${response.status}`,
      );
    }

    return await response.json();
  }
}

// ---- Helpers ---------------------------------------------------------------

/** Convert model ID to a nicer display name */
function formatModelDisplayName(modelId: string): string {
  // Map of known model IDs to display names
  const displayNames: Record<string, string> = {
    'gpt-4o': 'GPT-4o',
    'gpt-4o-mini': 'GPT-4o Mini',
    'gpt-4-turbo': 'GPT-4 Turbo',
    'gpt-4': 'GPT-4',
    'gpt-3.5-turbo': 'GPT-3.5 Turbo',
    'o1-preview': 'o1 Preview',
    'o1-mini': 'o1 Mini',
    'o3-mini': 'o3 Mini',
    'chatgpt-4o-latest': 'ChatGPT-4o Latest',
  };

  // Check for exact match first
  if (displayNames[modelId]) {
    return displayNames[modelId];
  }

  // Check for prefix matches (e.g., "gpt-4o-2024-08-06")
  for (const [key, name] of Object.entries(displayNames)) {
    if (modelId.startsWith(key + '-')) {
      const suffix = modelId.slice(key.length + 1);
      return `${name} (${suffix})`;
    }
  }

  // Fallback: capitalize segments
  return modelId
    .split('-')
    .map((s) => s.charAt(0).toUpperCase() + s.slice(1))
    .join(' ');
}

/** Singleton OpenAI vendor instance */
export const openaiVendor = new OpenAIVendor();
