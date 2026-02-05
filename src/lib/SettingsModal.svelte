<script lang="ts">
  import type { JiraSettings } from "../types";

  interface Props {
    settings: JiraSettings;
    apiToken: string;
    onSave: (settings: JiraSettings, apiToken: string) => void;
    onClose: () => void;
  }

  let { settings, apiToken, onSave, onClose }: Props = $props();

  let baseUrl = $state("");
  let email = $state("");
  let token = $state("");
  let defaultJql = $state("");

  $effect(() => {
    baseUrl = settings.baseUrl;
    email = settings.email;
    defaultJql = settings.defaultJql;
    token = apiToken;
  });

  function handleSave() {
    onSave(
      {
        baseUrl: baseUrl.trim(),
        email: email.trim(),
        defaultJql: defaultJql.trim(),
      },
      token.trim()
    );
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
  <div class="bg-white rounded-lg shadow-xl w-full max-w-md mx-4">
    <div class="flex items-center justify-between p-4 border-b border-gray-200">
      <h2 class="text-lg font-semibold text-gray-900">Settings</h2>
      <button
        onclick={onClose}
        class="p-1 text-gray-400 hover:text-gray-600 rounded-md transition-colors"
        aria-label="Close settings"
      >
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
        </svg>
      </button>
    </div>

    <div class="p-4 space-y-4">
      <div>
        <label for="baseUrl" class="block text-sm font-medium text-gray-700 mb-1">
          Jira Base URL
        </label>
        <input
          id="baseUrl"
          type="url"
          bind:value={baseUrl}
          placeholder="https://your-domain.atlassian.net"
          class="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
        />
      </div>

      <div>
        <label for="email" class="block text-sm font-medium text-gray-700 mb-1">
          Email Address
        </label>
        <input
          id="email"
          type="email"
          bind:value={email}
          placeholder="your.email@company.com"
          class="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
        />
      </div>

      <div>
        <label for="token" class="block text-sm font-medium text-gray-700 mb-1">
          API Token
        </label>
        <input
          id="token"
          type="password"
          bind:value={token}
          placeholder="Your Jira API token"
          class="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
        />
        <p class="mt-1 text-xs text-gray-500">
          Generate at <a href="https://id.atlassian.com/manage-profile/security/api-tokens" target="_blank" class="text-blue-600 hover:underline">Atlassian Account Settings</a>
        </p>
      </div>

      <div>
        <label for="defaultJql" class="block text-sm font-medium text-gray-700 mb-1">
          Default JQL Query
        </label>
        <textarea
          id="defaultJql"
          bind:value={defaultJql}
          placeholder="assignee = currentUser() ORDER BY updated DESC"
          rows="2"
          class="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent resize-none"
        ></textarea>
      </div>
    </div>

    <div class="flex justify-end gap-3 p-4 border-t border-gray-200">
      <button
        onclick={onClose}
        class="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-offset-2 transition-colors"
      >
        Cancel
      </button>
      <button
        onclick={handleSave}
        class="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 transition-colors"
      >
        Save
      </button>
    </div>
  </div>
</div>
