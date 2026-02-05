<script lang="ts">
  interface Props {
    components: string[];
  }

  let { components }: Props = $props();
</script>

{#if components.length > 0}
  <div class="mb-4">
    <span class="text-xs text-gray-500 uppercase">Components</span>
    <div class="flex flex-wrap gap-2 mt-1">
      {#each components as component}
        <span class="px-2 py-1 text-xs bg-purple-100 text-purple-800 rounded">{component}</span>
      {/each}
    </div>
  </div>
{/if}
