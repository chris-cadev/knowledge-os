// === Knowledge OS — TypeScript type definitions ===
// Mirrors the Rust types from knowledge-core and knowledge-storage

export interface EntitySummary {
  id: string;
  entity_type: string;
  title: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface ComponentData {
  component_type: string;
  data: any;
}

export interface RelationshipInfo {
  id: string;
  relationship_type: string;
  source_id: string;
  target_id: string;
  source_title: string;
  target_title: string;
  is_active: boolean;
}

export interface EntityDetail {
  id: string;
  entity_type: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
  components: ComponentData[];
  outgoing_relationships: RelationshipInfo[];
  incoming_relationships: RelationshipInfo[];
  events: EventInfo[];
  versions: VersionInfo[];
}

export interface EventInfo {
  id: string;
  event_type: string;
  timestamp: string;
  data: any;
}

export interface VersionInfo {
  version: number;
  created_at: string;
}

export interface ImportResult {
  created: number;
  merged: number;
  errors: ImportError[];
}

export interface ImportError {
  path: string;
  message: string;
}

export interface ImportProgressItem {
  path: string;
  status: string;
  action?: string;
  error?: string;
  entity_id?: string;
}

export interface ImportProgressResult {
  items: ImportProgressItem[];
  created: number;
  merged: number;
  errors: ImportError[];
}

export interface DirectoryPreview {
  file_count: number;
  total_size_bytes: number;
  formats: Record<string, number>;
  files: string[];
}

export interface ColumnSchemaInfo {
  name: string;
  data_type: string;
  nullable: boolean;
}

export interface StructuredPreview {
  columns: ColumnSchemaInfo[];
  sample_rows: any[][];
  total_rows: number;
  format: string;
}

export interface UndoResult {
  removed_entities: string[];
  import_id: string;
}

export interface SearchResult {
  entity_id: string;
  title: string;
  entity_type: string;
  snippet: string;
  score: number;
}

export interface GraphNode {
  id: string;
  title: string;
  entity_type: string;
  x?: number;
  y?: number;
}

export interface GraphEdge {
  source: string;
  target: string;
  relationship_type: string;
}

export interface GraphOutput {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export interface TreeNode {
  label: string;
  children?: TreeNode[];
  entity_id?: string;
  entity_type?: string;
  count?: number;
}

export interface TreeOutput {
  roots: TreeNode[];
}

export interface TableRow {
  entity_id: string;
  entity_type: string;
  title: string;
  tags: string[];
  created_at: string;
  updated_at: string;
}

export interface TableOutput {
  rows: TableRow[];
  total: number;
}

export interface TimelineItem {
  entity_id: string;
  entity_type: string;
  title: string;
  created_at: string;
}

export interface TimelineOutput {
  items: TimelineItem[];
}

export type View =
  | "dashboard"
  | "browse"
  | "detail"
  | "graph"
  | "tree"
  | "table"
  | "timeline"
  | "import"
  | "search";
