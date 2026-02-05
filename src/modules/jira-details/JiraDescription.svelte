<script lang="ts">
  import { renderDescription } from "./utils";

  interface Props {
    description: string | null;
  }

  let { description }: Props = $props();
</script>

<div class="mt-6 w-full">
  <h3 class="text-sm font-semibold text-gray-700 mb-2 uppercase">Description</h3>
  <div class="text-sm text-gray-700 p-4 bg-white rounded-lg border border-gray-200 w-full description-content">
    {@html renderDescription(description)}
  </div>
</div>
