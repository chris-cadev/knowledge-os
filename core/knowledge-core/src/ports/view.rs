use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::error::StorageError;
use super::event::Event;
use crate::features::entity::{Entity, EntityType};
use crate::features::relationship::RelationshipType;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Default)]
pub struct ViewFilter {
    pub entity_types: Option<Vec<EntityType>>,
    pub tags: Option<Vec<String>>,
    pub relationship_types: Option<Vec<RelationshipType>>,
    pub max_depth: Option<u32>,
    pub max_results: Option<usize>,
    pub start_entity_id: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<SortOrder>,
    pub search_query: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ViewOutput {
    Tree(TreeData),
    Graph(GraphData),
    Table(TableData),
    Timeline(TimelineData),
}

#[derive(Debug, Clone, Serialize)]
pub struct TreeData {
    pub roots: Vec<TreeNode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TreeNode {
    pub entity: Entity,
    pub label: String,
    pub children: Vec<TreeNode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphNode {
    pub entity: Entity,
    pub label: String,
    pub node_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphEdge {
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableData {
    pub columns: Vec<TableColumn>,
    pub rows: Vec<TableRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableColumn {
    pub name: String,
    pub sortable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableRow {
    pub cells: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineData {
    pub entries: Vec<TimelineEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineEntry {
    pub entity: Entity,
    pub timestamp: String,
    pub label: String,
}

#[async_trait]
pub trait ViewAdapter: Send + Sync {
    fn name(&self) -> &str;
    async fn render(&self, filter: &ViewFilter) -> Result<ViewOutput, StorageError>;
    async fn on_event(&self, event: &Event) -> Result<(), StorageError>;
}

#[async_trait]
pub trait EventNotifier: Send + Sync {
    async fn notify(&self, event: &Event) -> Result<(), StorageError>;
}

pub struct ViewRegistry {
    views: HashMap<String, Box<dyn ViewAdapter>>,
}

impl ViewRegistry {
    pub fn new() -> Self {
        Self {
            views: HashMap::new(),
        }
    }

    pub fn register(&mut self, view: Box<dyn ViewAdapter>) {
        let name = view.name().to_string();
        self.views.insert(name, view);
    }

    pub async fn render(
        &self,
        name: &str,
        filter: &ViewFilter,
    ) -> Result<ViewOutput, StorageError> {
        self.views
            .get(name)
            .ok_or_else(|| StorageError::Internal(format!("view '{}' not found", name)))?
            .render(filter)
            .await
    }

    pub fn list_views(&self) -> Vec<String> {
        self.views.keys().cloned().collect()
    }
}

impl Default for ViewRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventNotifier for ViewRegistry {
    async fn notify(&self, event: &Event) -> Result<(), StorageError> {
        for view in self.views.values() {
            view.on_event(event).await?;
        }
        Ok(())
    }
}
