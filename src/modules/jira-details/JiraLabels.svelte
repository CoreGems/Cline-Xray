<script lang="ts">
  interface Props {
    labels: string[];
  }

  let { labels }: Props = $props();
</script>

{#if labels.length > 0}
  <div class="mb-4">
    <span class="text-xs text-gray-500 uppercase">Labels</span>
    <div class="flex flex-wrap gap-2 mt-1">
      {#each labels as label}
        <span class="px-2 py-1 text-xs bg-blue-100 text-blue-800 rounded">{label}</span>
      {/each}
    </div>
  </div>
{/if}
