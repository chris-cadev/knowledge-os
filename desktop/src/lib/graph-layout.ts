// === Knowledge OS — D3-force simulation factory for graph layout ===

import type { GraphNode, GraphEdge } from "./types.js";
import {
  forceSimulation,
  forceLink,
  forceManyBody,
  forceCenter,
  forceCollide,
  type SimulationNodeDatum,
  type SimulationLinkDatum,
} from "d3-force";

export interface SimulationNode extends GraphNode, SimulationNodeDatum {}

export interface SimulationLink extends SimulationLinkDatum<SimulationNode> {
  relationship_type: string;
  source: string | SimulationNode;
  target: string | SimulationNode;
}

export interface SimulationResult {
  /** Stop the simulation (for cleanup). */
  stop: () => void;
  /** Register a tick callback. Returns cleanup function. */
  onTick: (cb: () => void) => () => void;
  /** Get current positions of all nodes and resolved edge endpoints. */
  getPositions: () => { nodes: SimulationNode[]; edges: SimulationLink[] };
}

/**
 * Create a D3-force simulation for the given nodes and edges.
 *
 * Uses `forceLink`, `forceManyBody`, `forceCenter`, and `forceCollide`
 * to compute positions for graph visualization.
 *
 * For graphs with >500 nodes, the caller should offload to a Web Worker.
 * This implementation runs on the main thread with tick callbacks.
 */
export function startSimulation(
  nodes: GraphNode[],
  edges: GraphEdge[]
): SimulationResult {
  const simNodes: SimulationNode[] = nodes.map((n) => ({
    ...n,
    x: Math.random() * 600,
    y: Math.random() * 400,
    vx: 0,
    vy: 0,
  }));

  const simLinks: SimulationLink[] = edges.map((e) => ({
    source: e.source,
    target: e.target,
    relationship_type: e.relationship_type,
  }));

  // Create the force simulation
  const simulation = forceSimulation(simNodes)
    .force(
      "link",
      forceLink(simLinks)
        .id((d: SimulationNodeDatum) => (d as unknown as SimulationNode).id)
        .distance(100)
    )
    .force("charge", forceManyBody().strength(-300))
    .force("center", forceCenter(300, 250))
    .force("collide", forceCollide(30))
    .alphaDecay(0.02)
    .velocityDecay(0.3);

  // Resolve link source/target to node ids
  function resolveLinks(): SimulationLink[] {
    return simLinks.map((l) => ({
      ...l,
      source: typeof l.source === "object" ? (l.source as SimulationNode).id : l.source,
      target: typeof l.target === "object" ? (l.target as SimulationNode).id : l.target,
    }));
  }

  return {
    stop: () => simulation.stop(),
    onTick: (cb: () => void) => {
      simulation.on("tick", cb);
      return () => simulation.on("tick", null);
    },
    getPositions: () => ({
      nodes: simNodes,
      edges: resolveLinks(),
    }),
  };
}
