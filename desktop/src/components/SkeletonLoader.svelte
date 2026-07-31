<script lang="ts">
  let { variant, count = 5 }: {
    variant: "grid" | "table" | "list" | "card";
    count?: number;
  } = $props();
</script>

<div class="skeleton-loader skeleton-pulse" role="status" aria-label="Loading content">
  {#if variant === "grid"}
    <div class="skeleton-grid">
      {#each Array(count)}
        <div class="skeleton-grid-item">
          <div class="skeleton-block skeleton-block-lg"></div>
          <div class="skeleton-block skeleton-block-md"></div>
        </div>
      {/each}
    </div>
  {:else if variant === "table"}
    <div class="skeleton-table">
      <div class="skeleton-table-header">
        <div class="skeleton-block skeleton-block-sm"></div>
        <div class="skeleton-block skeleton-block-md"></div>
        <div class="skeleton-block skeleton-block-sm"></div>
        <div class="skeleton-block skeleton-block-sm"></div>
      </div>
      {#each Array(count)}
        <div class="skeleton-table-row">
          <div class="skeleton-block skeleton-block-xs"></div>
          <div class="skeleton-block skeleton-block-lg"></div>
          <div class="skeleton-block skeleton-block-sm"></div>
          <div class="skeleton-block skeleton-block-sm"></div>
        </div>
      {/each}
    </div>
  {:else if variant === "list"}
    <div class="skeleton-list">
      {#each Array(count)}
        <div class="skeleton-list-item">
          <div class="skeleton-block skeleton-block-xs"></div>
          <div class="skeleton-block skeleton-block-lg"></div>
        </div>
      {/each}
    </div>
  {:else if variant === "card"}
    <div class="skeleton-cards">
      {#each Array(count)}
        <div class="skeleton-card">
          <div class="skeleton-block skeleton-block-sm"></div>
          <div class="skeleton-block skeleton-block-lg"></div>
          <div class="skeleton-block skeleton-block-md"></div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .skeleton-loader {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
    width: 100%;
  }

  .skeleton-block {
    background: var(--border);
    border-radius: var(--radius-sm);
    height: 14px;
  }

  .skeleton-block-xs {
    width: 40px;
    height: 14px;
  }

  .skeleton-block-sm {
    width: 80px;
    height: 14px;
  }

  .skeleton-block-md {
    width: 60%;
    height: 14px;
  }

  .skeleton-block-lg {
    width: 100%;
    height: 14px;
  }

  .skeleton-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: var(--spacing-md);
  }

  .skeleton-grid-item {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
    padding: var(--spacing-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  .skeleton-table {
    display: flex;
    flex-direction: column;
  }

  .skeleton-table-header {
    display: grid;
    grid-template-columns: 60px 1fr 100px 100px;
    gap: var(--spacing-md);
    padding: var(--spacing-sm) var(--spacing-md);
    border-bottom: 1px solid var(--border);
    margin-bottom: var(--spacing-xs);
  }

  .skeleton-table-row {
    display: grid;
    grid-template-columns: 60px 1fr 100px 100px;
    gap: var(--spacing-md);
    padding: var(--spacing-sm) var(--spacing-md);
    border-bottom: 1px solid var(--border);
    align-items: center;
  }

  .skeleton-list {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
  }

  .skeleton-list-item {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    padding: var(--spacing-sm) var(--spacing-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  .skeleton-cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: var(--spacing-md);
  }

  .skeleton-card {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
    padding: var(--spacing-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }
</style>
