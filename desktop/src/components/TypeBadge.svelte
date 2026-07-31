<script lang="ts">
  import { getEntityTypeColor } from "../lib/theme.svelte.js";

  let { type }: { type: string } = $props();
</script>

<span class="type-badge" style="background: {getEntityTypeColor(type)}">{type}</span>

<style>
  .type-badge {
    display: inline-block;
    padding: 2px 8px;
    color: white;
    border-radius: var(--radius-sm);
    font-size: var(--font-size-sm);
    font-weight: 500;
    white-space: nowrap;
  }
</style>
