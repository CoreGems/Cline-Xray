// Activity Tab API Functions

import { invoke } from "@tauri-apps/api/core";
import type { AccessLogEntry, InferenceLogEntry } from "./types";

/**
 * Fetch access logs from the Tauri backend
 * @returns Array of access log entries, newest first
 */
export async function fetchAccessLogs(): Promise<AccessLogEntry[]> {
  const logs = await invoke<AccessLogEntry[]>('get_access_logs');
  // Reverse to show newest first
  return logs.reverse();
}

/**
 * Clear all access logs in the backend
 */
export async function clearAccessLogs(): Promise<void> {
  await invoke('clear_access_logs');
}

/**
 * Fetch inference logs from the Tauri backend
 * @returns Array of inference log entries, newest first
 */
export async function fetchInferenceLogs(): Promise<InferenceLogEntry[]> {
  const logs = await invoke<InferenceLogEntry[]>('get_inference_logs');
  // Reverse to show newest first
  return logs.reverse();
}

/**
 * Clear all inference logs in the backend
 */
export async function clearInferenceLogs(): Promise<void> {
  await invoke('clear_inference_logs');
}
