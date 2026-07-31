<script lang="ts">
  let { value, options, onchange }: {
    value: string;
    options: string[];
    onchange: (value: string) => void;
  } = $props();

  function handleChange(e: Event) {
    const selected = (e.target as HTMLSelectElement).value;
    onchange(selected);
  }
</script>

<select class="filter-select" {value} onchange={handleChange}>
  <option value="">All types</option>
  {#each options as opt}
    <option value={opt} selected={opt === value}>{opt}</option>
  {/each}
</select>

<style>
  .filter-select {
    padding: var(--spacing-sm) var(--spacing-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    color: var(--text-primary);
    font-size: var(--font-size-body-sm);
    cursor: pointer;
  }

  .filter-select:focus {
    outline: none;
    border-color: var(--accent);
  }
</style>
