use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::features::component::Component;
use crate::ports::MessageRole;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageContentData {
    pub role: MessageRole,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityRefsData {
    pub entity_ids: Vec<Uuid>,
}

impl Component {
    pub fn message_content(&self) -> Option<MessageContentData> {
        if self.component_type != crate::features::component::ComponentType::MessageContent {
            return None;
        }
        serde_json::from_value(self.data.clone()).ok()
    }

    pub fn entity_refs(&self) -> Option<EntityRefsData> {
        if self.component_type != crate::features::component::ComponentType::EntityRefs {
            return None;
        }
        serde_json::from_value(self.data.clone()).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::component::{Component, ComponentType};
    use uuid::Uuid;

    #[test]
    fn component_message_content_roundtrip() {
        let data = MessageContentData {
            role: MessageRole::Assistant,
            text: "Hello!".to_string(),
        };
        let value = serde_json::to_value(&data).unwrap();
        let comp = Component::new(Uuid::new_v4(), ComponentType::MessageContent, value);
        let json = serde_json::to_string(&comp).unwrap();
        let deserialized: Component = serde_json::from_str(&json).unwrap();
        let extracted = deserialized.message_content().unwrap();
        assert_eq!(extracted.role, MessageRole::Assistant);
        assert_eq!(extracted.text, "Hello!");
    }

    #[test]
    fn component_entity_refs_roundtrip() {
        let ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let data = EntityRefsData {
            entity_ids: ids.clone(),
        };
        let value = serde_json::to_value(&data).unwrap();
        let comp = Component::new(Uuid::new_v4(), ComponentType::EntityRefs, value);
        let json = serde_json::to_string(&comp).unwrap();
        let deserialized: Component = serde_json::from_str(&json).unwrap();
        let extracted = deserialized.entity_refs().unwrap();
        assert_eq!(extracted.entity_ids, ids);
    }

    #[test]
    fn component_wrong_type_returns_none() {
        let comp = Component::new(
            Uuid::new_v4(),
            ComponentType::Content,
            serde_json::json!("Some content"),
        );
        assert!(comp.message_content().is_none());
        assert!(comp.entity_refs().is_none());
    }

    #[test]
    fn message_role_serializes_lowercase() {
        let roles = [
            MessageRole::System,
            MessageRole::User,
            MessageRole::Assistant,
        ];
        let expected = ["\"system\"", "\"user\"", "\"assistant\""];
        for (role, expected_json) in roles.iter().zip(expected.iter()) {
            let json = serde_json::to_string(role).unwrap();
            assert_eq!(json, *expected_json);
        }
    }
}
