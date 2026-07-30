use serde::Serialize;

// ---------------------------------------------------------------------------
// Entity summary (for list views)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct EntitySummary {
    pub id: String,
    pub entity_type: String,
    pub title: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// Entity detail (components + relationships + events + versions)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct ComponentData {
    pub component_type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct RelationshipInfo {
    pub id: String,
    pub relationship_type: String,
    pub source_id: String,
    pub target_id: String,
    pub source_title: String,
    pub target_title: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventInfo {
    pub id: String,
    pub event_type: String,
    pub timestamp: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionInfo {
    pub version: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EntityDetailResponse {
    pub id: String,
    pub entity_type: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub components: Vec<ComponentData>,
    pub outgoing_relationships: Vec<RelationshipInfo>,
    pub incoming_relationships: Vec<RelationshipInfo>,
    pub events: Vec<EventInfo>,
    pub versions: Vec<VersionInfo>,
}

// ---------------------------------------------------------------------------
// Import result
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct ImportErrorResponse {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct ImportResultResponse {
    pub created: usize,
    pub merged: usize,
    pub errors: Vec<ImportErrorResponse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportProgressItem {
    pub path: String,
    pub status: String,
    pub action: Option<String>,
    pub error: Option<String>,
    pub entity_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportProgressResponse {
    pub items: Vec<ImportProgressItem>,
    pub created: usize,
    pub merged: usize,
    pub errors: Vec<ImportErrorResponse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DirectoryPreviewResponse {
    pub file_count: usize,
    pub total_size_bytes: u64,
    pub formats: std::collections::HashMap<String, usize>,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ColumnSchemaResponse {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct StructuredPreviewResponse {
    pub columns: Vec<ColumnSchemaResponse>,
    pub sample_rows: Vec<Vec<serde_json::Value>>,
    pub total_rows: u64,
    pub format: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UndoImportResponse {
    pub removed_entities: Vec<String>,
    pub import_id: String,
}

// ---------------------------------------------------------------------------
// Search result
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct SearchResultResponse {
    pub entity_id: String,
    pub title: String,
    pub entity_type: String,
    pub snippet: String,
    pub score: f64,
}

// ---------------------------------------------------------------------------
// Graph view
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct GraphNodeResponse {
    pub id: String,
    pub title: String,
    pub entity_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphEdgeResponse {
    pub source: String,
    pub target: String,
    pub relationship_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphViewResponse {
    pub nodes: Vec<GraphNodeResponse>,
    pub edges: Vec<GraphEdgeResponse>,
}

// ---------------------------------------------------------------------------
// Tree view
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct TreeNodeResponse {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<TreeNodeResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TreeViewResponse {
    pub roots: Vec<TreeNodeResponse>,
}

// ---------------------------------------------------------------------------
// Table view
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct TableRowResponse {
    pub entity_id: String,
    pub entity_type: String,
    pub title: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableViewResponse {
    pub rows: Vec<TableRowResponse>,
    pub total: usize,
}

// ---------------------------------------------------------------------------
// Timeline view
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct TimelineItemResponse {
    pub entity_id: String,
    pub entity_type: String,
    pub title: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineViewResponse {
    pub items: Vec<TimelineItemResponse>,
}
