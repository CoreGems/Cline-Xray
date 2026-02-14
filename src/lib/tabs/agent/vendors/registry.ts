// ============================================================================
// Vendor Registry
// ============================================================================
//
// Central registry for all inference vendors. Vendors register themselves
// here and consumers can look up vendors by ID or find the vendor that
// owns a particular model.
//
// Usage:
//   import { vendorRegistry } from './vendors';
//   const allModels = await vendorRegistry.fetchAllModels(apiInfo, settings);
//   const vendor = vendorRegistry.getVendorForModel('gemini-2.0-flash');

import type { AgentSettings } from '../../../../types';
import type { InferenceVendor, ModelOption, VendorApiInfo } from './types';

class VendorRegistry {
  private vendors: Map<string, InferenceVendor> = new Map();

  /**
   * Register a new inference vendor.
   * Call this at module load time for each vendor implementation.
   */
  register(vendor: InferenceVendor): void {
    if (this.vendors.has(vendor.id)) {
      console.warn(`[VendorRegistry] Vendor "${vendor.id}" already registered, overwriting.`);
    }
    this.vendors.set(vendor.id, vendor);
  }

  /**
   * Get a vendor by its unique ID.
   */
  getVendor(vendorId: string): InferenceVendor | undefined {
    return this.vendors.get(vendorId);
  }

  /**
   * Get the vendor that owns a given model ID.
   * Returns undefined if no vendor claims the model.
   */
  getVendorForModel(modelId: string): InferenceVendor | undefined {
    for (const vendor of this.vendors.values()) {
      if (vendor.ownsModel(modelId)) {
        return vendor;
      }
    }
    return undefined;
  }

  /**
   * Get all registered vendors.
   */
  getAllVendors(): InferenceVendor[] {
    return Array.from(this.vendors.values());
  }

  /**
   * Get all registered vendor IDs.
   */
  getVendorIds(): string[] {
    return Array.from(this.vendors.keys());
  }

  /**
   * Fetch models from ALL registered vendors and merge into a single list.
   * Models are tagged with their vendorId so the UI can group/filter by vendor.
   *
   * @param apiInfo - Backend connection info
   * @param settings - User's agent filter settings
   * @returns Combined model list from all vendors, sorted by displayName
   */
  async fetchAllModels(
    apiInfo: VendorApiInfo,
    settings: AgentSettings,
  ): Promise<ModelOption[]> {
    const results = await Promise.allSettled(
      this.getAllVendors().map(async (vendor) => {
        try {
          return await vendor.fetchModels(apiInfo, settings);
        } catch (err) {
          console.error(`[VendorRegistry] Failed to fetch models from "${vendor.id}":`, err);
          return [] as ModelOption[];
        }
      }),
    );

    const allModels: ModelOption[] = [];
    for (const result of results) {
      if (result.status === 'fulfilled') {
        allModels.push(...result.value);
      }
    }

    // Sort combined list alphabetically by display name
    return allModels.sort((a, b) => a.displayName.localeCompare(b.displayName));
  }

  /**
   * Get the default model ID (from the first registered vendor).
   * Useful for initial selection when no preference is stored.
   */
  getDefaultModel(): string {
    const first = this.getAllVendors()[0];
    return first?.defaultModel ?? '';
  }

  /**
   * Get fallback models (one per vendor) for when API calls fail.
   */
  getFallbackModels(): ModelOption[] {
    return this.getAllVendors().map((v) => ({
      id: v.defaultModel,
      displayName: v.name,
      vendorId: v.id,
    }));
  }
}

/** Singleton vendor registry instance */
export const vendorRegistry = new VendorRegistry();
