use knowledge_core::ports::*;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use super::store::AppState;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChatSendResult {
    pub conversation_id: String,
    pub message_id: String,
    pub message: String,
    pub citations: Vec<CitationSource>,
    pub referenced_entities: Vec<String>,
}

#[tauri::command]
pub async fn chat_send(
    state: State<'_, AppState>,
    conversation_id: Option<String>,
    message: String,
    entity_refs: Vec<String>,
    source_toggles: Option<SourceToggles>,
    mode: Option<ResponseMode>,
) -> Result<ChatSendResult, String> {
    let entity_refs: Vec<Uuid> = entity_refs
        .iter()
        .filter_map(|s| Uuid::parse_str(s).ok())
        .collect();
    let toggles = source_toggles.unwrap_or_default();
    let mode = mode.unwrap_or(ResponseMode::Thinking);

    let conv_id = conversation_id.and_then(|s| Uuid::parse_str(&s).ok());

    let result = state
        .chat_pipeline
        .lock()
        .await
        .chat(conv_id, &message, &entity_refs, &toggles, mode)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ChatSendResult {
        conversation_id: result.conversation_id.to_string(),
        message_id: result.message_id.to_string(),
        message: result.message,
        citations: result.citations,
        referenced_entities: result
            .referenced_entities
            .iter()
            .map(|u| u.to_string())
            .collect(),
    })
}

#[tauri::command]
pub async fn chat_stream(
    app: AppHandle,
    state: State<'_, AppState>,
    conversation_id: Option<String>,
    message: String,
    entity_refs: Vec<String>,
    source_toggles: Option<SourceToggles>,
    mode: Option<ResponseMode>,
) -> Result<String, String> {
    let conv_id = conversation_id.and_then(|s| Uuid::parse_str(&s).ok());
    let entity_refs: Vec<Uuid> = entity_refs
        .iter()
        .filter_map(|s| Uuid::parse_str(s).ok())
        .collect();
    let toggles = source_toggles.unwrap_or_default();
    let mode = mode.unwrap_or(ResponseMode::Thinking);

    let handle = state
        .chat_pipeline
        .lock()
        .await
        .chat_stream(conv_id, &message, &entity_refs, &toggles, mode)
        .await
        .map_err(|e| e.to_string())?;

    let conversation_id = handle.conversation_id.to_string();
    let user_message_id = handle.user_message_id.to_string();

    let app_clone = app.clone();
    tokio::spawn(async move {
        use futures::StreamExt;
        use knowledge_derivation::features::chat::status::ChatStreamEvent;
        let mut stream = handle.stream;
        while let Some(event) = stream.next().await {
            match event {
                ChatStreamEvent::Status(s) => {
                    let _ = app_clone.emit("chat:status", &s);
                }
                ChatStreamEvent::Delta(d) => {
                    let _ = app_clone.emit("chat:delta", &d);
                }
                ChatStreamEvent::Done {
                    assistant_message_id,
                    citations,
                } => {
                    let _ = app_clone.emit(
                        "chat:done",
                        serde_json::json!({
                            "user_message_id": user_message_id,
                            "assistant_message_id": assistant_message_id.to_string(),
                            "citations": citations,
                        }),
                    );
                }
                ChatStreamEvent::Error(e) => {
                    let _ = app_clone.emit("chat:error", &e.to_string());
                }
            }
        }
    });

    Ok(conversation_id)
}

#[tauri::command]
pub async fn chat_search_entities(
    state: State<'_, AppState>,
    prefix: String,
) -> Result<Vec<EntitySearchResult>, String> {
    let query = SearchQuery {
        query: prefix,
        entity_type: None,
        tag: None,
    };
    let store = &*state.store;
    let results = SearchIndex::search(store, &query)
        .await
        .map_err(|e| e.to_string())?;

    let mut output = Vec::new();
    for r in results {
        let entity = EntityRepository::get(store, r.entity_id)
            .await
            .map_err(|e| e.to_string())?;
        if let Some(entity) = entity {
            use knowledge_core::ports::ComponentRepository;
            let components = ComponentRepository::get(store, entity.id)
                .await
                .map_err(|e| e.to_string())?;
            let title = components
                .iter()
                .find(|c| {
                    c.component_type == knowledge_core::features::component::ComponentType::Title
                })
                .and_then(|c| c.data.get("name").and_then(|v| v.as_str()))
                .unwrap_or("Untitled")
                .to_string();
            let preview = components
                .iter()
                .find(|c| {
                    c.component_type == knowledge_core::features::component::ComponentType::Content
                })
                .and_then(|c| c.data.as_str())
                .unwrap_or("")
                .chars()
                .take(200)
                .collect();
            output.push(EntitySearchResult {
                id: entity.id.to_string(),
                entity_type: entity.entity_type.to_string(),
                title,
                preview,
            });
        }
    }
    Ok(output)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EntitySearchResult {
    pub id: String,
    pub entity_type: String,
    pub title: String,
    pub preview: String,
}

#[tauri::command]
pub async fn chat_list_conversations(
    state: State<'_, AppState>,
) -> Result<Vec<ConversationSummaryResponse>, String> {
    let store = &*state.store;
    use knowledge_core::ports::EntityRepository;
    let entities = EntityRepository::find_by_type(store, "Conversation")
        .await
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for entity in entities {
        use knowledge_core::ports::ComponentRepository;
        let components = ComponentRepository::get(store, entity.id)
            .await
            .map_err(|e| e.to_string())?;
        let title = components
            .iter()
            .find(|c| c.component_type == knowledge_core::features::component::ComponentType::Title)
            .and_then(|c| c.data.get("name").and_then(|v| v.as_str()))
            .unwrap_or("Untitled")
            .to_string();
        let last_message_preview = components
            .iter()
            .find(|c| {
                c.component_type
                    == knowledge_core::features::component::ComponentType::MessageContent
            })
            .and_then(|c| {
                c.data
                    .get("content")
                    .and_then(|v| v.as_str())
                    .map(|s| s.chars().take(100).collect::<String>())
            });

        results.push(ConversationSummaryResponse {
            id: entity.id.to_string(),
            title,
            message_count: 0,
            last_message_preview,
            last_message_at: None,
            created_at: entity.created_at.to_rfc3339(),
            updated_at: entity.updated_at.to_rfc3339(),
        });
    }
    Ok(results)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ConversationSummaryResponse {
    pub id: String,
    pub title: String,
    pub message_count: usize,
    pub last_message_preview: Option<String>,
    pub last_message_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[tauri::command]
pub async fn chat_delete_conversation(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<(), String> {
    let id = Uuid::parse_str(&conversation_id).map_err(|e| e.to_string())?;
    use knowledge_core::ports::EntityRepository;
    let store = &*state.store;
    let mut entity = EntityRepository::get(store, id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "conversation not found".to_string())?;
    entity.archive();
    EntityRepository::save(store, &entity)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn chat_rename_conversation(
    state: State<'_, AppState>,
    conversation_id: String,
    title: String,
) -> Result<(), String> {
    let id = Uuid::parse_str(&conversation_id).map_err(|e| e.to_string())?;
    let store = &*state.store;
    use knowledge_core::ports::ComponentRepository;
    let components = ComponentRepository::get(store, id)
        .await
        .map_err(|e| e.to_string())?;
    if let Some(title_comp) = components
        .iter()
        .find(|c| c.component_type == knowledge_core::features::component::ComponentType::Title)
    {
        ComponentRepository::update_data(store, title_comp.id, serde_json::json!({"name": title}))
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn chat_stop_stream(
    state: State<'_, AppState>,
    _conversation_id: String,
) -> Result<(), String> {
    let _ = state;
    Ok(())
}

#[tauri::command]
pub async fn chat_send_feedback(
    state: State<'_, AppState>,
    feedback: ResponseFeedback,
) -> Result<(), String> {
    let store = &*state.store;
    use knowledge_core::features::component::{Component, ComponentType};
    use knowledge_core::ports::ComponentRepository;
    let component = Component::new(
        feedback.message_id,
        ComponentType::Provenance,
        serde_json::to_value(&feedback).map_err(|e| e.to_string())?,
    );
    ComponentRepository::save(store, &component)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
