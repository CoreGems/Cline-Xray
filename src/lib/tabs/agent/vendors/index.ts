// ============================================================================
// Vendors Module - Public API
// ============================================================================
//
// Import this module to access the vendor registry and all registered vendors.
// Vendors are auto-registered on first import of this module.
//
// Usage:
//   import { vendorRegistry, geminiVendor } from './vendors';
//
// To add a new vendor:
//   1. Create vendors/myvendor.ts implementing InferenceVendor
//   2. Import and register it below
//   3. That's it! The registry, ModelSelector, and ChatSubtab will pick it up.

// --- Re-export types --------------------------------------------------------
export type {
  InferenceVendor,
  ModelOption,
  VendorApiInfo,
  VendorBranding,
  VendorChatMessage,
  VendorChatResponse,
} from './types';

// --- Re-export registry -----------------------------------------------------
export { vendorRegistry } from './registry';

// --- Import and register all vendor implementations -------------------------
import { vendorRegistry } from './registry';
import { geminiVendor } from './gemini';
import { openaiVendor } from './openai';

// Register vendors (order determines default tab ordering in UI)
vendorRegistry.register(geminiVendor);
vendorRegistry.register(openaiVendor);

// --- Re-export individual vendor instances for direct access ----------------
export { geminiVendor } from './gemini';
export { openaiVendor } from './openai';
