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

export interface ConnectionInfo {
  server_version: string;
  database_name: string;
  reachable: boolean;
  latency_ms: number;
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

export interface EntitySearchResult {
  id: string;
  entity_type: string;
  title: string;
  preview: string;
}

export interface ChatMessage {
  role: "system" | "user" | "assistant";
  content: string;
  entity_refs: string[];
}

export interface Citation {
  number: number;
  entity_id: string;
  entity_type: string;
  title: string;
  snippet: string;
}

export interface ChatSendResult {
  conversation_id: string;
  message_id: string;
  message: string;
  citations: Citation[];
  referenced_entities: string[];
}

export interface ChatDelta {
  delta: string;
  citation?: number;
  status?: ProcessingStatus;
  finished: boolean;
}

export interface ProcessingStatus {
  Searching?: { detail: string };
  ReadingEntities?: { count: number };
  Generating?: null;
}

export interface ConversationSummary {
  id: string;
  title: string;
  message_count: number;
  last_message_preview: string | null;
  last_message_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface MessageDetail {
  id: string;
  role: string;
  text: string;
  entity_refs: string[];
  citations?: Citation[];
  feedback?: MessageFeedback | null;
  created_at: string;
}

export interface MessageFeedback {
  message_id: string;
  rating: FeedbackRating;
  reason?: string | null;
  comment?: string | null;
}

export interface ConversationDetail {
  id: string;
  title: string;
  messages: MessageDetail[];
  created_at: string;
  updated_at: string;
}

export interface ChatState {
  conversations: ConversationSummary[];
  currentConversationId: string | null;
  messages: MessageDisplay[];
  inputText: string;
  mode: "fast" | "thinking";
  knowledgeGraph: boolean;
  webSearch: boolean;
  streaming: boolean;
  processingStatus: ProcessingStatus | null;
}

export interface MessageDisplay {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  citations?: Citation[];
  referencedEntities?: string[];
  feedback?: FeedbackRating;
  timestamp: string;
}

export type FeedbackRating = "thumbs_up" | "thumbs_down";

export interface ProviderConfig {
  provider_kind: string;
  model: string;
  base_url: string | null;
  api_key: string | null;
}

export interface ProviderStatus {
  provider: string;
  model: string;
  base_url: string;
  reachable: boolean;
  latency_ms: number;
}

export interface TestResult {
  success: boolean;
  latency_ms: number;
  error: string | null;
}

export interface OcrProviderStatus {
  backend: string;
  model: string;
  base_url: string;
  reachable: boolean;
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
  | "search"
  | "chat"
  | "settings";
