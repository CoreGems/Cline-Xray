// ============================================================================
// Google Gemini Vendor Implementation
// ============================================================================
//
// This module implements the InferenceVendor interface for Google Gemini.
// It fetches models from the /agent/models backend endpoint (which proxies
// to Google's generativelanguage API) and sends chat via /agent/chat.
//
// The backend currently serves Gemini-specific data; this vendor normalizes
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

/** Raw model shape returned by the /agent/models backend endpoint */
interface GeminiApiModel {
  name: string;
  displayName?: string;
  description?: string;
  inputTokenLimit?: number;
  outputTokenLimit?: number;
  supportedGenerationMethods?: string[];
}

/** Raw response from /agent/models */
interface GeminiModelsApiResponse {
  models: GeminiApiModel[];
  total: number;
}

// ---- Gemini vendor branding ------------------------------------------------

const geminiBranding: VendorBranding = {
  primaryColor: '#4285F4',
  gradientCSS: 'linear-gradient(to right, #4285F4, #8B5CF6, #EC4899)',
  icon: '✦',
  poweredBy: 'Powered by Google Gemini AI',
  emptyStateTitle: 'Chat with Gemini',
  emptyStateLogoHTML: [
    '<span style="color:#4285F4">G</span>',
    '<span style="color:#EA4335">o</span>',
    '<span style="color:#FBBC04">o</span>',
    '<span style="color:#4285F4">g</span>',
    '<span style="color:#34A853">l</span>',
    '<span style="color:#EA4335">e</span>',
  ].join(''),
};

// ---- Implementation --------------------------------------------------------

class GeminiVendor implements InferenceVendor {
  readonly id = 'gemini';
  readonly name = 'Google Gemini';
  readonly defaultModel = 'gemini-2.0-flash';
  readonly branding = geminiBranding;

  // Gemini model IDs always start with one of these prefixes
  private static readonly MODEL_PREFIXES = [
    'gemini-',
    'gemma-',
    'models/gemini-',
    'models/gemma-',
  ];

  ownsModel(modelId: string): boolean {
    return GeminiVendor.MODEL_PREFIXES.some((prefix) => modelId.startsWith(prefix));
  }

  async fetchModels(apiInfo: VendorApiInfo, settings: AgentSettings): Promise<ModelOption[]> {
    const response = await fetch(`${apiInfo.base_url}/agent/models`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${apiInfo.token}`,
      },
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        (errorData as any).error || `Gemini models HTTP error ${response.status}`,
      );
    }

    const data: GeminiModelsApiResponse = await response.json();

    // 1. Filter to models that support generateContent
    let models: ModelOption[] = (data.models || [])
      .filter(
        (m) => m.supportedGenerationMethods?.includes('generateContent'),
      )
      .map((m) => ({
        id: m.name.replace(/^models\//, ''),
        displayName: m.displayName || m.name.replace(/^models\//, ''),
        description: m.description,
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

    const response = await fetch(`${apiInfo.base_url}/agent/chat`, {
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
        (errorData as any).error || `Gemini chat HTTP error ${response.status}`,
      );
    }

    return await response.json();
  }
}

/** Singleton Gemini vendor instance */
export const geminiVendor = new GeminiVendor();
