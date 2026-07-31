<script lang="ts">
  import { getState } from "../lib/state.svelte.js";
  import { onMount, onDestroy } from "svelte";
  import { getGraphView, listEntities } from "../lib/api.js";
  import { startSimulation } from "../lib/graph-layout.js";
  import { navigateTo } from "../lib/router.svelte.js";
  import { getEntityTypeColor } from "../lib/theme.svelte.js";
  import EntityPicker from "../components/EntityPicker.svelte";
  import TypeFilterDropdown from "../components/TypeFilterDropdown.svelte";
  import type { GraphNode, GraphEdge, EntitySummary } from "../lib/types.js";

  const app = getState();

  let nodes = $state<GraphNode[]>([]);
  let edges = $state<GraphEdge[]>([]);
  let loading = $state(false);

  let startId = $state(app.selectedEntityId || "");
  let focusTitle = $state("");
  let depth = $state(2);
  let entityTypeFilter = $state("");
  let availableTypes = $state<string[]>([]);

  let selectedNode = $state<GraphNode | null>(null);

  let svgEl = $state<SVGSVGElement | null>(null);
  let svgGroup = $state<SVGGElement | null>(null);

  let transformX = $state(0);
  let transformY = $state(0);
  let transformScale = $state(1);
  let isPanning = $state(false);
  let panStart = { x: 0, y: 0 };

  let sim: ReturnType<typeof startSimulation> | null = null;
  let animationId: number | null = null;

  let legendTypes = $derived(
    [...new Set(nodes.map((n) => n.entity_type))].sort()
  );

  onMount(async () => {
    try {
      const entities = await listEntities();
      availableTypes = [...new Set(entities.map((e) => e.entity_type))].sort();
    } catch {
      availableTypes = [];
    }

    if (app.selectedEntityId) {
      startId = app.selectedEntityId;
      await loadGraph();
    } else {
      await loadGraph();
    }
  });

  onDestroy(() => {
    cleanupSimulation();
  });

  function cleanupSimulation() {
    if (sim) {
      sim.stop();
      sim = null;
    }
    if (animationId !== null) {
      cancelAnimationFrame(animationId);
      animationId = null;
    }
  }

  async function loadGraph() {
    loading = true;
    cleanupSimulation();

    try {
      const data = await getGraphView(
        startId || undefined,
        depth,
        entityTypeFilter || undefined
      );
      nodes = data.nodes;
      edges = data.edges;
      selectedNode = null;

      if (startId && !focusTitle) {
        const startNode = nodes.find((n) => n.id === startId);
        if (startNode) focusTitle = startNode.title;
      }

      if (nodes.length > 0) {
        sim = startSimulation(nodes, edges);
        sim.onTick(() => {
          if (animationId === null) {
            animationId = requestAnimationFrame(() => {
              animationId = null;
              updateSvgPositions();
            });
          }
        });
      }
    } catch (e) {
      app.statusMessage = `Failed to load graph: ${e}`;
    } finally {
      loading = false;
    }
  }

  function handleEntitySelect(entity: EntitySummary) {
    startId = entity.id;
    focusTitle = entity.title;
    loadGraph();
  }

  function updateSvgPositions() {
    if (!svgGroup || !sim) return;

    const positions = sim.getPositions();

    for (const n of positions.nodes) {
      const el = svgGroup.querySelector(`[data-node-id="${n.id}"]`) as SVGGElement | null;
      if (el) {
        el.setAttribute("transform", `translate(${n.x},${n.y})`);
      }
    }

    for (const e of positions.edges) {
      const lineEl = svgGroup.querySelector(`[data-edge-src="${e.source}"][data-edge-tgt="${e.target}"]`) as SVGLineElement | null;
      const srcNode = positions.nodes.find((n) => n.id === e.source);
      const tgtNode = positions.nodes.find((n) => n.id === e.target);

      if (lineEl && srcNode && tgtNode) {
        lineEl.setAttribute("x1", String(srcNode.x));
        lineEl.setAttribute("y1", String(srcNode.y));
        lineEl.setAttribute("x2", String(tgtNode.x));
        lineEl.setAttribute("y2", String(tgtNode.y));
      }

      if (srcNode && tgtNode) {
        const labelEl = svgGroup.querySelector(`[data-edge-label-src="${e.source}"][data-edge-label-tgt="${e.target}"]`) as SVGTextElement | null;
        if (labelEl) {
          const midX = (srcNode.x + tgtNode.x) / 2;
          const midY = (srcNode.y + tgtNode.y) / 2;
          labelEl.setAttribute("x", String(midX));
          labelEl.setAttribute("y", String(midY - 4));
        }
      }
    }
  }

  function onSvgMouseDown(e: MouseEvent) {
    const target = e.target as SVGElement;
    if (target === svgEl || target.classList.contains("canvas-bg")) {
      isPanning = true;
      panStart = { x: e.clientX - transformX, y: e.clientY - transformY };
    }
  }

  function onSvgMouseMove(e: MouseEvent) {
    if (isPanning) {
      transformX = e.clientX - panStart.x;
      transformY = e.clientY - panStart.y;
    }
  }

  function onSvgMouseUp() {
    isPanning = false;
  }

  function onSvgWheel(e: WheelEvent) {
    e.preventDefault();
    const delta = e.deltaY > 0 ? 0.9 : 1.1;
    transformScale = Math.max(0.1, Math.min(10, transformScale * delta));
  }

  function zoomIn() {
    transformScale = Math.min(10, transformScale * 1.3);
  }

  function zoomOut() {
    transformScale = Math.max(0.1, transformScale * 0.7);
  }

  function resetZoom() {
    transformScale = 1;
    transformX = 0;
    transformY = 0;
  }

  function onNodeClick(node: GraphNode) {
    selectedNode = node;
  }

  function onNodeDoubleClick(node: GraphNode) {
    startId = node.id;
    focusTitle = node.title;
    loadGraph();
  }
</script>

<div class="graph-container">
  <div class="traversal-controls">
    <div class="picker-wrapper">
      <EntityPicker onSelect={handleEntitySelect} placeholder="Search entity to explore..." />
    </div>
    <label class="control-label">
      Depth:
      <input
        type="range"
        min="1"
        max="5"
        bind:value={depth}
        class="depth-slider"
        aria-label="Graph depth"
      />
      <span class="depth-value">{depth}</span>
    </label>
    <TypeFilterDropdown
      value={entityTypeFilter}
      options={availableTypes}
      onchange={(v) => { entityTypeFilter = v; loadGraph(); }}
    />
    <button class="btn btn-primary" onclick={loadGraph} disabled={loading}>
      <span class="material-symbols-outlined">explore</span>
      Explore
    </button>
  </div>

  <div class="canvas-wrapper">
    {#if loading}
      <div class="loading-overlay">
        <span class="material-symbols-outlined spinning">sync</span>
        <p>Loading graph...</p>
      </div>
    {/if}

    {#if nodes.length === 0 && !loading}
      <div class="empty-state">
        <span class="material-symbols-outlined empty-icon">bubble_chart</span>
        <p>Search for an entity above to explore its knowledge graph connections. Double-click any node to re-center the graph on it.</p>
      </div>
    {/if}

    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <svg
      class="graph-svg"
      bind:this={svgEl}
      onmousedown={onSvgMouseDown}
      onmousemove={onSvgMouseMove}
      onmouseup={onSvgMouseUp}
      onmouseleave={onSvgMouseUp}
      onwheel={onSvgWheel}
      role="img"
      aria-label="Knowledge graph visualization"
    >
      <rect class="canvas-bg" width="100%" height="100%" />

      <g
        bind:this={svgGroup}
        transform="translate({transformX},{transformY}) scale({transformScale})"
      >
        {#each edges as edge (edge.source + edge.target)}
          <line
            data-edge-src={edge.source}
            data-edge-tgt={edge.target}
            class="graph-edge"
            x1="0"
            y1="0"
            x2="0"
            y2="0"
            stroke="var(--color-outline-variant)"
            stroke-width="1.5"
          />
          <text
            data-edge-label-src={edge.source}
            data-edge-label-tgt={edge.target}
            class="edge-label"
            x="0"
            y="0"
            text-anchor="middle"
            fill="var(--text-secondary)"
            font-size="9"
            font-family="var(--font-sans)"
          >
            {edge.relationship_type}
          </text>
        {/each}

        {#each nodes as node (node.id)}
          <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
          <g
            data-node-id={node.id}
            class="graph-node"
            class:selected={selectedNode?.id === node.id}
            onclick={() => onNodeClick(node)}
            ondblclick={() => onNodeDoubleClick(node)}
            onkeydown={(e) => { if (e.key === 'Enter') onNodeClick(node); }}
            transform="translate(0,0)"
            role="button"
            tabindex="0"
            aria-label={node.title}
          >
            <circle
              r="16"
              fill={getEntityTypeColor(node.entity_type)}
              stroke="var(--color-surface)"
              stroke-width="2"
              class="node-circle"
            />
            <text
              text-anchor="middle"
              dy="30"
              fill="var(--color-on-surface)"
              font-size="11"
              font-family="var(--font-sans)"
            >
              {node.title.length > 15 ? node.title.slice(0, 15) + "..." : node.title}
            </text>
          </g>
        {/each}
      </g>
    </svg>

    <div class="zoom-controls">
      <button class="zoom-btn" onclick={zoomIn} title="Zoom in" aria-label="Zoom in">
        <span class="material-symbols-outlined">add</span>
      </button>
      <button class="zoom-btn" onclick={zoomOut} title="Zoom out" aria-label="Zoom out">
        <span class="material-symbols-outlined">remove</span>
      </button>
      <button class="zoom-btn" onclick={resetZoom} title="Reset zoom" aria-label="Reset zoom">
        <span class="material-symbols-outlined">center_focus_strong</span>
      </button>
    </div>

    {#if startId}
      <div class="breadcrumb">
        <span class="material-symbols-outlined">my_location</span>
        <span class="text-muted">Focus: {focusTitle || startId.slice(0, 8) + "..."}</span>
        <span class="text-muted">Depth: {depth}</span>
        <span class="text-muted">{nodes.length} nodes</span>
      </div>
    {/if}

    {#if legendTypes.length > 0}
      <div class="legend">
        <span class="legend-title">Legend</span>
        {#each legendTypes as type}
          <span class="legend-item">
            <span class="legend-dot" style="background: {getEntityTypeColor(type)}"></span>
            {type}
          </span>
        {/each}
      </div>
    {/if}
  </div>

  {#if selectedNode}
    <div class="entity-inspector">
      <div class="inspector-header">
        <h3>Entity</h3>
        <button class="close-btn" onclick={() => (selectedNode = null)} aria-label="Close inspector">
          <span class="material-symbols-outlined">close</span>
        </button>
      </div>
      <div class="inspector-body">
        <div class="inspector-row">
          <span class="label">Type</span>
          <span class="type-badge" style="background: {getEntityTypeColor(selectedNode.entity_type)}">{selectedNode.entity_type}</span>
        </div>
        <div class="inspector-row">
          <span class="label">Title</span>
          <span>{selectedNode.title}</span>
        </div>
        <div class="inspector-row">
          <span class="label">ID</span>
          <span class="mono">{selectedNode.id.slice(0, 12)}...</span>
        </div>
      </div>
      <div class="inspector-actions">
        <button class="btn btn-small" onclick={() => { const n = selectedNode!; navigateTo("detail", n.id); }}>
          <span class="material-symbols-outlined">open_in_new</span>
          View Details
        </button>
        <button class="btn btn-small" onclick={() => { const n = selectedNode!; startId = n.id; focusTitle = n.title; loadGraph(); }}>
          <span class="material-symbols-outlined">explore</span>
          Explore from Here
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .graph-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    position: relative;
  }

  /* Traversal Controls */
  .traversal-controls {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    padding: var(--spacing-md);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    margin-bottom: var(--spacing-md);
    flex-wrap: wrap;
  }

  .picker-wrapper {
    min-width: 200px;
    max-width: 320px;
    flex: 1;
  }

  .control-label {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    font-size: var(--font-size-body-sm);
    color: var(--text-secondary);
  }

  .depth-slider {
    width: 80px;
  }

  .depth-value {
    font-weight: 600;
    color: var(--text-primary);
    min-width: 16px;
  }

  .btn {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    padding: var(--spacing-xs) var(--spacing-md);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body-sm);
    font-weight: 500;
    cursor: pointer;
    transition: background var(--transition-fast);
  }

  .btn-primary {
    background: var(--accent);
    color: white;
    border: none;
  }

  .btn-primary:hover {
    background: var(--accent-hover);
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-small {
    padding: var(--spacing-xs) var(--spacing-sm);
    background: var(--bg-card);
    border: 1px solid var(--border);
    color: var(--text-primary);
    font-size: var(--font-size-body-sm);
  }

  .btn-small:hover {
    background: var(--bg-secondary);
  }

  /* Canvas */
  .canvas-wrapper {
    flex: 1;
    position: relative;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--color-surface-container-low);
    min-height: 400px;
  }

  .graph-svg {
    width: 100%;
    height: 100%;
    cursor: grab;
    display: block;
  }

  .graph-svg:active {
    cursor: grabbing;
  }

  .canvas-bg {
    fill: var(--color-surface-container-low);
  }

  :global(.graph-node) {
    cursor: pointer;
  }

  :global(.graph-node:hover .node-circle) {
    stroke-width: 3;
    stroke: var(--accent);
  }

  :global(.graph-node.selected .node-circle) {
    stroke-width: 3;
    stroke: var(--accent);
    filter: brightness(1.2);
  }

  .edge-label {
    pointer-events: none;
    user-select: none;
    opacity: 0.7;
  }

  .loading-overlay,
  .empty-state {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--spacing-md);
    pointer-events: none;
  }

  .spinning {
    animation: spin 1s linear infinite;
    font-size: 36px;
    width: 36px;
    height: 36px;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .empty-icon {
    font-size: 48px;
    width: 48px;
    height: 48px;
    color: var(--text-secondary);
  }

  .empty-state p {
    max-width: 300px;
    text-align: center;
    color: var(--text-secondary);
    font-size: var(--font-size-body-sm);
  }

  /* Zoom Controls */
  .zoom-controls {
    position: absolute;
    bottom: var(--spacing-md);
    right: var(--spacing-md);
    display: flex;
    flex-direction: column;
    gap: 2px;
    z-index: 10;
  }

  .zoom-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    cursor: pointer;
    transition: background var(--transition-fast);
  }

  .zoom-btn:hover {
    background: var(--bg-secondary);
  }

  /* Breadcrumb */
  .breadcrumb {
    position: absolute;
    top: var(--spacing-sm);
    left: var(--spacing-sm);
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-xs) var(--spacing-sm);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-sm);
    z-index: 10;
  }

  /* Legend */
  .legend {
    position: absolute;
    top: var(--spacing-sm);
    right: var(--spacing-sm);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
    padding: var(--spacing-sm);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-sm);
    z-index: 10;
  }

  .legend-title {
    font-weight: 600;
    margin-bottom: var(--spacing-xs);
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
  }

  .legend-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  /* Entity Inspector */
  .entity-inspector {
    position: absolute;
    bottom: var(--spacing-md);
    left: var(--spacing-md);
    width: 280px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
    z-index: 10;
  }

  .inspector-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--spacing-sm) var(--spacing-md);
    border-bottom: 1px solid var(--border);
  }

  .inspector-header h3 {
    font-size: var(--font-size-body-md);
    font-weight: 600;
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    cursor: pointer;
  }

  .close-btn:hover {
    background: var(--bg-secondary);
    color: var(--text-primary);
  }

  .inspector-body {
    padding: var(--spacing-sm) var(--spacing-md);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
  }

  .inspector-row {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    font-size: var(--font-size-body-sm);
  }

  .label {
    color: var(--text-secondary);
    min-width: 40px;
  }

  .type-badge {
    display: inline-block;
    padding: 1px 6px;
    color: white;
    border-radius: var(--radius-sm);
    font-size: 10px;
    font-weight: 500;
  }

  .mono {
    font-family: var(--font-mono);
    font-size: var(--font-size-code-md);
  }

  .inspector-actions {
    display: flex;
    gap: var(--spacing-xs);
    padding: var(--spacing-sm) var(--spacing-md);
    border-top: 1px solid var(--border);
  }
</style>
