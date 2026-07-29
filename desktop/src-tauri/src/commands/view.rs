use knowledge_core::features::entity::EntityType;
use knowledge_core::ports::{ViewAdapter, ViewFilter};
use knowledge_derivation::features::view::{
    graph::GraphViewAdapter, table::TableViewAdapter, timeline::TimelineViewAdapter,
    tree::TreeViewAdapter,
};
use tauri::State;

use super::response::*;
use super::store::{AppState, StoreWrapper};

/// Render the graph view (nodes + edges).
#[tauri::command]
pub async fn get_graph_view(
    state: State<'_, AppState>,
    start_id: Option<String>,
    depth: Option<u32>,
    entity_type: Option<String>,
) -> Result<GraphViewResponse, String> {
    let store = state.store.clone();

    let adapter = GraphViewAdapter::new(
        Box::new(StoreWrapper(store.clone())),
        Box::new(StoreWrapper(store.clone())),
        Box::new(StoreWrapper(store.clone())),
        Box::new(StoreWrapper(store.clone())),
    );

    let entity_types = entity_type.map(|t| vec![EntityType::new(&t)]);

    let filter = ViewFilter {
        start_entity_id: start_id,
        max_depth: depth,
        entity_types,
        ..Default::default()
    };

    let output = adapter.render(&filter).await.map_err(|e| e.to_string())?;

    match output {
        knowledge_core::ports::ViewOutput::Graph(data) => {
            let nodes: Vec<GraphNodeResponse> = data
                .nodes
                .into_iter()
                .map(|n| GraphNodeResponse {
                    id: n.entity.id.to_string(),
                    title: n.label,
                    entity_type: n.node_type,
                })
                .collect();

            let edges: Vec<GraphEdgeResponse> = data
                .edges
                .into_iter()
                .map(|e| GraphEdgeResponse {
                    source: e.source_id,
                    target: e.target_id,
                    relationship_type: e.relationship_type,
                })
                .collect();

            Ok(GraphViewResponse { nodes, edges })
        }
        other => Err(format!("unexpected view output: {:?}", other)),
    }
}

/// Render the tree view (hierarchical by entity type).
#[tauri::command]
pub async fn get_tree_view(
    state: State<'_, AppState>,
    entity_type: Option<String>,
) -> Result<TreeViewResponse, String> {
    let store = state.store.clone();

    let adapter = TreeViewAdapter::new(
        Box::new(StoreWrapper(store.clone())),
        Box::new(StoreWrapper(store.clone())),
        None,
    );

    let entity_types = entity_type.map(|t| vec![EntityType::new(&t)]);

    let filter = ViewFilter {
        entity_types,
        ..Default::default()
    };

    let output = adapter.render(&filter).await.map_err(|e| e.to_string())?;

    match output {
        knowledge_core::ports::ViewOutput::Tree(data) => {
            let roots = convert_tree_nodes(data.roots);
            Ok(TreeViewResponse { roots })
        }
        other => Err(format!("unexpected view output: {:?}", other)),
    }
}

fn convert_tree_nodes(nodes: Vec<knowledge_core::ports::TreeNode>) -> Vec<TreeNodeResponse> {
    nodes
        .into_iter()
        .map(|n| {
            let count = if n.children.is_empty() {
                None
            } else {
                Some(n.children.len())
            };

            TreeNodeResponse {
                label: n.label,
                entity_id: Some(n.entity.id.to_string()),
                entity_type: Some(n.entity.entity_type.to_string()),
                children: if n.children.is_empty() {
                    None
                } else {
                    Some(convert_tree_nodes(n.children))
                },
                count,
            }
        })
        .collect()
}

/// Render the table view (sortable columns).
#[tauri::command]
pub async fn get_table_view(
    state: State<'_, AppState>,
    sort: Option<String>,
    entity_type: Option<String>,
) -> Result<TableViewResponse, String> {
    let store = state.store.clone();

    let adapter = TableViewAdapter::new(
        Box::new(StoreWrapper(store.clone())),
        Box::new(StoreWrapper(store.clone())),
    );

    let entity_types = entity_type.map(|t| vec![EntityType::new(&t)]);

    let filter = ViewFilter {
        entity_types,
        sort_by: sort,
        ..Default::default()
    };

    let output = adapter.render(&filter).await.map_err(|e| e.to_string())?;

    match output {
        knowledge_core::ports::ViewOutput::Table(data) => {
            let rows: Vec<TableRowResponse> = data
                .rows
                .into_iter()
                .map(|r| {
                    let tags_str = r.cells.get(3).cloned().unwrap_or_default();
                    TableRowResponse {
                        entity_id: r.cells.first().cloned().unwrap_or_default(),
                        entity_type: r.cells.get(1).cloned().unwrap_or_default(),
                        title: r.cells.get(2).cloned().unwrap_or_default(),
                        tags: tags_str
                            .split(", ")
                            .filter(|s| !s.is_empty())
                            .map(String::from)
                            .collect(),
                        created_at: r.cells.get(4).cloned().unwrap_or_default(),
                        updated_at: r.cells.get(5).cloned().unwrap_or_default(),
                    }
                })
                .collect();

            let total = rows.len();
            Ok(TableViewResponse { rows, total })
        }
        other => Err(format!("unexpected view output: {:?}", other)),
    }
}

/// Render the timeline view (chronological).
#[tauri::command]
pub async fn get_timeline_view(
    state: State<'_, AppState>,
    entity_type: Option<String>,
) -> Result<TimelineViewResponse, String> {
    let store = state.store.clone();

    let adapter = TimelineViewAdapter::new(
        Box::new(StoreWrapper(store.clone())),
        Box::new(StoreWrapper(store.clone())),
    );

    let entity_types = entity_type.map(|t| vec![EntityType::new(&t)]);

    let filter = ViewFilter {
        entity_types,
        ..Default::default()
    };

    let output = adapter.render(&filter).await.map_err(|e| e.to_string())?;

    match output {
        knowledge_core::ports::ViewOutput::Timeline(data) => {
            let items: Vec<TimelineItemResponse> = data
                .entries
                .into_iter()
                .map(|e| TimelineItemResponse {
                    entity_id: e.entity.id.to_string(),
                    entity_type: e.entity.entity_type.to_string(),
                    title: e.label,
                    created_at: e.timestamp,
                })
                .collect();

            Ok(TimelineViewResponse { items })
        }
        other => Err(format!("unexpected view output: {:?}", other)),
    }
}
