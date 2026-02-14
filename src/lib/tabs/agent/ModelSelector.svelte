<script lang="ts">
  // ============================================================================
  // ModelSelector - Vendor-tabbed model picker
  // ============================================================================
  //
  // A model selector with vendor family tabs at the top of the dropdown.
  // Clicking a vendor tab filters the model list to that vendor's models.
  // The trigger button shows the vendor icon + selected model name.
  //
  // Props:
  //   selectedModel (bindable) - currently selected model ID
  //   storageKey               - localStorage key for persisting selection
  //   onModelsLoaded           - callback when models finish loading

  import { getApiInfo } from './api';
  import { vendorRegistry } from './vendors';
  import type { ModelOption, InferenceVendor } from './vendors';
  import type { AgentSettings } from '../../../types';
  import { DEFAULT_AGENT_SETTINGS } from '../../../types';

  const AGENT_SETTINGS_KEY = 'agent-settings';

  // ---- Props ----------------------------------------------------------------

  interface Props {
    selectedModel: string;
    storageKey?: string;
    onModelsLoaded?: (models: ModelOption[]) => void;
  }

  let {
    selectedModel = $bindable(''),
    storageKey = 'agent-chat-selected-model',
    onModelsLoaded,
  }: Props = $props();

  // ---- Internal state -------------------------------------------------------

  let availableModels: ModelOption[] = $state([]);
  let modelsLoading = $state(false);
  let modelsError: string | null = $state(null);
  let showDropdown = $state(false);
  /** Which vendor tab is active in the dropdown (null = auto from selected model) */
  let activeVendorTab: string | null = $state(null);

  // ---- Helpers --------------------------------------------------------------

  function getAgentSettings(): AgentSettings {
    try {
      const stored = localStorage.getItem(AGENT_SETTINGS_KEY);
      if (stored) return JSON.parse(stored) as AgentSettings;
    } catch (e) {
      console.error('Failed to load agent settings:', e);
    }
    return DEFAULT_AGENT_SETTINGS;
  }

  /** Get the currently selected model object */
  function currentModel(): ModelOption | undefined {
    return availableModels.find((m) => m.id === selectedModel);
  }

  /** Get the vendor for the currently selected model */
  function currentVendor(): InferenceVendor | undefined {
    return vendorRegistry.getVendorForModel(selectedModel);
  }

  /** Get the display name for the trigger button */
  function currentModelDisplay(): string {
    const model = currentModel();
    return model?.displayName || selectedModel || 'Select a model';
  }

  /** Get the effective active vendor tab ID */
  function effectiveVendorTab(): string {
    if (activeVendorTab) return activeVendorTab;
    const vendor = currentVendor();
    if (vendor) return vendor.id;
    const allVendors = vendorRegistry.getAllVendors();
    return allVendors[0]?.id ?? '';
  }

  /** Group models by vendor */
  function modelsByVendor(): Map<string, { vendor: InferenceVendor; models: ModelOption[] }> {
    const groups = new Map<string, { vendor: InferenceVendor; models: ModelOption[] }>();

    for (const model of availableModels) {
      const vendor = vendorRegistry.getVendor(model.vendorId);
      if (!vendor) continue;

      if (!groups.has(model.vendorId)) {
        groups.set(model.vendorId, { vendor, models: [] });
      }
      groups.get(model.vendorId)!.models.push(model);
    }

    return groups;
  }

  /** Get models for the active vendor tab */
  function activeTabModels(): ModelOption[] {
    const tabId = effectiveVendorTab();
    const groups = modelsByVendor();
    return groups.get(tabId)?.models ?? [];
  }

  // ---- Model loading --------------------------------------------------------

  /** Get the display name for a model ID (used by parent components) */
  export function getModelDisplayName(modelId: string): string {
    const model = availableModels.find((m) => m.id === modelId);
    return model?.displayName || modelId || 'AI';
  }

  export async function loadModels() {
    modelsLoading = true;
    modelsError = null;

    try {
      const apiInfo = await getApiInfo();
      const settings = getAgentSettings();
      const models = await vendorRegistry.fetchAllModels(apiInfo, settings);
      availableModels = models;

      // If selected model isn't in the list, fall back to default
      if (models.length > 0 && !models.find((m) => m.id === selectedModel)) {
        const defaultModel = vendorRegistry.getDefaultModel();
        const match = models.find((m) => m.id === defaultModel);
        selectedModel = match ? match.id : models[0].id;
        if (storageKey) localStorage.setItem(storageKey, selectedModel);
      }

      onModelsLoaded?.(models);
    } catch (e) {
      console.error('Failed to load models:', e);
      modelsError = e instanceof Error ? e.message : 'Failed to load models';

      if (availableModels.length === 0) {
        availableModels = vendorRegistry.getFallbackModels();
      }
    } finally {
      modelsLoading = false;
    }
  }

  // ---- Actions --------------------------------------------------------------

  function selectModel(modelId: string) {
    selectedModel = modelId;
    showDropdown = false;
    activeVendorTab = null; // reset to auto-follow
    if (storageKey) localStorage.setItem(storageKey, modelId);
  }

  function switchVendorTab(vendorId: string) {
    activeVendorTab = vendorId;
  }

  function openDropdown() {
    activeVendorTab = null; // reset to follow current model's vendor
    showDropdown = true;
    // Auto-load models if they haven't been loaded yet
    if (availableModels.length === 0 && !modelsLoading) {
      loadModels();
    }
  }

  // ---- Derived state --------------------------------------------------------

  const vendors = $derived(vendorRegistry.getAllVendors());
  const tabModels = $derived(activeTabModels());
  const tabVendorId = $derived(effectiveVendorTab());
  const selectedVendor = $derived(currentVendor());
  const modelCount = $derived.by(() => {
    const groups = modelsByVendor();
    const counts: Record<string, number> = {};
    for (const [id, group] of groups) {
      counts[id] = group.models.length;
    }
    return counts;
  });
</script>

<!-- Model Selector Button + Dropdown -->
<div class="relative">
  <!-- Trigger Button -->
  <button
    onclick={() => showDropdown ? (showDropdown = false) : openDropdown()}
    disabled={modelsLoading && showDropdown}
    class="inline-flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium rounded-md border border-gray-300 bg-white text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-1 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
    title={modelsError ? `Error: ${modelsError}` : `Model: ${selectedModel}`}
  >
    <!-- Vendor icon -->
    {#if selectedVendor}
      <span class="flex-shrink-0 text-sm" style="color: {selectedVendor.branding.primaryColor}">{selectedVendor.branding.icon}</span>
    {:else}
      <svg class="w-3.5 h-3.5 text-purple-500 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"></path>
      </svg>
    {/if}
    {#if modelsLoading}
      <span class="text-gray-400">Loading...</span>
    {:else}
      <span class="max-w-[200px] truncate">{currentModelDisplay()}</span>
    {/if}
    <!-- Chevron -->
    <svg class="w-3 h-3 text-gray-400 flex-shrink-0 transition-transform {showDropdown ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path>
    </svg>
  </button>

  <!-- Dropdown (opens upward) -->
  {#if showDropdown}
    <!-- Backdrop -->
    <div
      class="fixed inset-0 z-10"
      onclick={() => (showDropdown = false)}
      onkeydown={(e) => e.key === 'Escape' && (showDropdown = false)}
      role="button"
      tabindex="-1"
    ></div>

    <div class="absolute bottom-full left-0 mb-1 w-80 bg-white border border-gray-200 rounded-lg shadow-lg z-20 overflow-hidden">
      {#if modelsLoading && availableModels.length === 0}
        <!-- Loading state -->
        <div class="px-4 py-6 text-center">
          <svg class="w-5 h-5 mx-auto mb-2 text-gray-400 animate-spin" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
          <div class="text-xs text-gray-500">Loading models...</div>
        </div>
      {:else if availableModels.length === 0}
        <!-- Empty state -->
        <div class="px-4 py-6 text-center">
          <div class="text-xs text-gray-400 mb-2">No models available.</div>
          <button onclick={loadModels} class="text-xs text-blue-600 hover:text-blue-700 underline">Retry loading</button>
        </div>
      {:else}
        <!-- Vendor Tabs -->
        <div class="flex border-b border-gray-200 bg-gray-50">
          {#each vendors as vendor (vendor.id)}
            {@const isActive = tabVendorId === vendor.id}
            {@const count = modelCount[vendor.id] ?? 0}
            <button
              onclick={() => switchVendorTab(vendor.id)}
              class="flex-1 flex items-center justify-center gap-1.5 px-3 py-2 text-xs font-medium transition-colors relative
                {isActive
                  ? 'text-gray-900 bg-white'
                  : 'text-gray-500 hover:text-gray-700 hover:bg-gray-100'}"
              title="{vendor.name} ({count} models)"
            >
              <span class="text-sm" style="color: {vendor.branding.primaryColor}">{vendor.branding.icon}</span>
              <span class="truncate">{vendor.name}</span>
              {#if count > 0}
                <span class="text-[9px] px-1 py-0.5 rounded-full {isActive ? 'bg-blue-100 text-blue-700' : 'bg-gray-200 text-gray-500'}">{count}</span>
              {/if}
              <!-- Active indicator bar -->
              {#if isActive}
                <div class="absolute bottom-0 left-2 right-2 h-0.5 rounded-full" style="background-color: {vendor.branding.primaryColor}"></div>
              {/if}
            </button>
          {/each}
        </div>

        <!-- Model List for active vendor tab -->
        <div class="max-h-64 overflow-y-auto">
          {#if tabModels.length === 0}
            <div class="px-4 py-6 text-center text-xs text-gray-400">
              No models available for this vendor.
            </div>
          {:else}
            {#each tabModels as model (model.id)}
              <button
                onclick={() => selectModel(model.id)}
                class="w-full text-left px-3 py-2 text-xs hover:bg-blue-50 transition-colors flex items-center gap-2
                  {selectedModel === model.id ? 'bg-blue-50 text-blue-700' : 'text-gray-700'}"
                title={model.description || model.id}
              >
                <span class="w-4 flex-shrink-0 text-blue-500">
                  {#if selectedModel === model.id}
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                    </svg>
                  {/if}
                </span>
                <div class="flex-1 min-w-0">
                  <div class="font-medium truncate">{model.displayName}</div>
                  <div class="text-[10px] text-gray-400 truncate">{model.id}</div>
                </div>
              </button>
            {/each}
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>

{#if modelsError}
  <span class="text-[10px] text-amber-600" title={modelsError}>⚠</span>
{/if}

<!-- Refresh models button -->
<button
  onclick={loadModels}
  disabled={modelsLoading}
  class="p-1 text-gray-400 hover:text-gray-600 rounded transition-colors disabled:opacity-50"
  title="Refresh models list"
>
  <svg class="w-3.5 h-3.5 {modelsLoading ? 'animate-spin' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
  </svg>
</button>
