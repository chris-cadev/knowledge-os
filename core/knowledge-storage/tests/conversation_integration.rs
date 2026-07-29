use knowledge_core::features::component::Component;
use knowledge_core::features::component::ComponentType;
use knowledge_core::features::entity::Entity;
use knowledge_core::features::entity::EntityType;
use knowledge_core::features::relationship::Relationship;
use knowledge_core::features::relationship::RelationshipType;
use knowledge_core::ports::*;
use knowledge_storage::adapters::sqlite::SqliteStore;
use uuid::Uuid;

fn test_store() -> SqliteStore {
    SqliteStore::new(":memory:").unwrap()
}

async fn create_conversation(
    store: &SqliteStore,
    title: &str,
    messages: Vec<(&str, &str)>,
) -> Uuid {
    let conv = Entity::new(EntityType::new("Conversation"));
    EntityRepository::save(store, &conv).await.unwrap();

    let title_comp = Component::new(
        conv.id,
        ComponentType::Title,
        serde_json::json!(title),
    );
    ComponentRepository::save(store, &title_comp).await.unwrap();

    for (role, text) in messages {
        let msg = Entity::new(EntityType::new("Message"));
        EntityRepository::save(store, &msg).await.unwrap();

        let content_comp = Component::new(
            msg.id,
            ComponentType::MessageContent,
            serde_json::json!({
                "role": role,
                "text": text,
            }),
        );
        ComponentRepository::save(store, &content_comp).await.unwrap();

        let rel = Relationship::new(conv.id, msg.id, RelationshipType::HasMessage);
        RelationshipRepository::save(store, &rel).await.unwrap();
    }

    conv.id
}

#[tokio::test]
async fn list_conversations_returns_empty_for_no_data() {
    let store = test_store();
    let conversations = ConversationRepository::list_conversations(&store)
        .await
        .unwrap();
    assert!(conversations.is_empty());
}

#[tokio::test]
async fn list_conversations_sorts_by_recency() {
    let store = test_store();

    let conv1_id = create_conversation(&store, "Old conversation", vec![("user", "hello")]).await;
    let conv2_id = create_conversation(&store, "Recent conversation", vec![("user", "hi")]).await;

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let mut conv2 = EntityRepository::get(&store, conv2_id)
        .await
        .unwrap()
        .unwrap();
    conv2.touch();
    EntityRepository::save(&store, &conv2).await.unwrap();

    let conversations = ConversationRepository::list_conversations(&store)
        .await
        .unwrap();
    assert_eq!(conversations.len(), 2);
    assert_eq!(conversations[0].id, conv2_id);
    assert_eq!(conversations[1].id, conv1_id);
}

#[tokio::test]
async fn list_conversations_excludes_archived() {
    let store = test_store();
    let conv_id = create_conversation(&store, "To archive", vec![("user", "test")]).await;

    ConversationRepository::archive_conversation(&store, conv_id)
        .await
        .unwrap();

    let conversations = ConversationRepository::list_conversations(&store)
        .await
        .unwrap();
    assert!(conversations.is_empty());
}

#[tokio::test]
async fn get_conversation_loads_messages_ordered() {
    let store = test_store();
    let conv_id = create_conversation(
        &store,
        "Test",
        vec![("user", "first"), ("assistant", "second"), ("user", "third")],
    )
    .await;

    let detail = ConversationRepository::get_conversation(&store, conv_id)
        .await
        .unwrap()
        .expect("conversation should exist");

    assert_eq!(detail.title, "Test");
    assert_eq!(detail.messages.len(), 3);
    assert_eq!(detail.messages[0].text, "first");
    assert_eq!(detail.messages[1].text, "second");
    assert_eq!(detail.messages[2].text, "third");
}

#[tokio::test]
async fn rename_conversation_updates_title() {
    let store = test_store();
    let conv_id = create_conversation(&store, "Old name", vec![("user", "hello")]).await;

    ConversationRepository::rename_conversation(&store, conv_id, "New name")
        .await
        .unwrap();

    let detail = ConversationRepository::get_conversation(&store, conv_id)
        .await
        .unwrap()
        .expect("conversation should exist");
    assert_eq!(detail.title, "New name");
}

#[tokio::test]
async fn archive_conversation_marks_inactive() {
    let store = test_store();
    let conv_id = create_conversation(&store, "Test", vec![("user", "hello")]).await;

    ConversationRepository::archive_conversation(&store, conv_id)
        .await
        .unwrap();

    let conversations = ConversationRepository::list_conversations(&store)
        .await
        .unwrap();
    assert!(conversations.is_empty());

    let detail = ConversationRepository::get_conversation(&store, conv_id)
        .await
        .unwrap();
    assert!(detail.is_none());
}

#[tokio::test]
async fn archive_conversation_cascades_to_messages() {
    let store = test_store();
    let conv_id = create_conversation(
        &store,
        "Test",
        vec![("user", "msg1"), ("assistant", "msg2")],
    )
    .await;

    ConversationRepository::archive_conversation(&store, conv_id)
        .await
        .unwrap();

    let detail = ConversationRepository::get_conversation(&store, conv_id)
        .await
        .unwrap();
    assert!(detail.is_none());
}

#[tokio::test]
async fn get_conversation_returns_none_for_missing() {
    let store = test_store();
    let result = ConversationRepository::get_conversation(&store, Uuid::new_v4())
        .await
        .unwrap();
    assert!(result.is_none());
}
