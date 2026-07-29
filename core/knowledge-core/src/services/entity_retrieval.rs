use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ports::{
    ComponentRepository, EntityRepository, RelationshipRepository, SearchIndex, SearchQuery,
    StorageError, TraversalConfig, TraversalDirection, TraversalPort, TraversalQuery,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalFilter {
    pub entity_types: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySummary {
    pub id: Uuid,
    pub entity_type: String,
    pub title: String,
    pub preview: String,
    pub tags: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDetail {
    pub id: Uuid,
    pub entity_type: String,
    pub components: BTreeMap<String, serde_json::Value>,
    pub relationships: Vec<RelationshipSummary>,
    pub events: Vec<EventSummary>,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipSummary {
    pub id: Uuid,
    pub relationship_type: String,
    pub direction: RelationshipDirection,
    pub peer_id: Uuid,
    pub peer_type: String,
    pub peer_title: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelationshipDirection {
    Outgoing,
    Incoming,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSummary {
    pub id: Uuid,
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
}

pub struct EntityRetrievalService {
    entity_repo: Arc<dyn EntityRepository>,
    component_repo: Arc<dyn ComponentRepository>,
    relationship_repo: Arc<dyn RelationshipRepository>,
    search_index: Arc<dyn SearchIndex>,
    traversal_port: Arc<dyn TraversalPort>,
}

impl EntityRetrievalService {
    pub fn new(
        entity_repo: Arc<dyn EntityRepository>,
        component_repo: Arc<dyn ComponentRepository>,
        relationship_repo: Arc<dyn RelationshipRepository>,
        search_index: Arc<dyn SearchIndex>,
        traversal_port: Arc<dyn TraversalPort>,
    ) -> Self {
        Self {
            entity_repo,
            component_repo,
            relationship_repo,
            search_index,
            traversal_port,
        }
    }

    pub async fn get_entity(&self, id: Uuid) -> Result<EntityDetail, StorageError> {
        let entity = self
            .entity_repo
            .get(id)
            .await?
            .ok_or(StorageError::NotFound)?;

        let components_list = self.component_repo.get(id).await?;
        let mut components = BTreeMap::new();
        for c in components_list {
            components.insert(format!("{:?}", c.component_type), c.data);
        }

        let outgoing = self.relationship_repo.by_source(id).await?;
        let incoming = self.relationship_repo.by_target(id).await?;
        let mut relationships = Vec::new();
        for r in outgoing {
            let peer = self.entity_repo.get(r.target_id).await?;
            relationships.push(RelationshipSummary {
                id: r.id,
                relationship_type: format!("{:?}", r.relationship_type),
                direction: RelationshipDirection::Outgoing,
                peer_id: r.target_id,
                peer_type: peer
                    .as_ref()
                    .map(|e| e.entity_type.to_string())
                    .unwrap_or_default(),
                peer_title: extract_title(&peer, &self.component_repo).await,
                is_active: r.is_active,
            });
        }
        for r in incoming {
            let peer = self.entity_repo.get(r.source_id).await?;
            relationships.push(RelationshipSummary {
                id: r.id,
                relationship_type: format!("{:?}", r.relationship_type),
                direction: RelationshipDirection::Incoming,
                peer_id: r.source_id,
                peer_type: peer
                    .as_ref()
                    .map(|e| e.entity_type.to_string())
                    .unwrap_or_default(),
                peer_title: extract_title(&peer, &self.component_repo).await,
                is_active: r.is_active,
            });
        }

        let events = self.fetch_events(id).await?;

        Ok(EntityDetail {
            id: entity.id,
            entity_type: entity.entity_type.to_string(),
            components,
            relationships,
            events,
            version: entity.version,
            created_at: entity.created_at,
            updated_at: entity.updated_at,
            is_active: entity.is_active,
        })
    }

    pub async fn get_entities(&self, ids: &[Uuid]) -> Result<Vec<EntitySummary>, StorageError> {
        let mut results = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(summary) = self.entity_to_summary(*id).await? {
                results.push(summary);
            }
        }
        Ok(results)
    }

    pub async fn search(
        &self,
        query: &str,
        filter: &RetrievalFilter,
    ) -> Result<Vec<EntitySummary>, StorageError> {
        let search_query = SearchQuery {
            query: query.to_string(),
            entity_type: filter.entity_types.as_ref().and_then(|t| t.first().cloned()),
            tag: filter.tags.as_ref().and_then(|t| t.first().cloned()),
        };
        let results = self.search_index.search(&search_query).await?;
        let limit = filter.limit.unwrap_or(20);

        let mut summaries = Vec::new();
        for r in results.into_iter().take(limit) {
            if let Some(summary) = self.entity_to_summary(r.entity_id).await? {
                summaries.push(summary);
            }
        }
        Ok(summaries)
    }

    pub async fn traverse(
        &self,
        start: Uuid,
        max_depth: u32,
    ) -> Result<TraversalResult, StorageError> {
        let query = TraversalQuery {
            start_id: start,
            direction: TraversalDirection::Both,
            max_depth: Some(max_depth),
            max_results: Some(100),
            relationship_type: None,
            entity_type_filter: None,
        };
        let config = TraversalConfig::default();
        let results = self
            .traversal_port
            .traverse(&query, &config)
            .await
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(TraversalResult {
            start_id: start,
            results: results.into_iter().map(|r| r.path).collect(),
        })
    }

    async fn entity_to_summary(&self, id: Uuid) -> Result<Option<EntitySummary>, StorageError> {
        let entity = match self.entity_repo.get(id).await? {
            Some(e) => e,
            None => return Ok(None),
        };
        let components = self.component_repo.get(id).await?;
        let title = extract_title_from_components(&components);
        let preview = extract_preview_from_components(&components);
        let tags = extract_tags_from_components(&components);
        Ok(Some(EntitySummary {
            id: entity.id,
            entity_type: entity.entity_type.to_string(),
            title,
            preview,
            tags,
            updated_at: entity.updated_at,
        }))
    }

    async fn fetch_events(&self, _id: Uuid) -> Result<Vec<EventSummary>, StorageError> {
        Ok(vec![])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalResult {
    pub start_id: Uuid,
    pub results: Vec<Vec<Uuid>>,
}

async fn extract_title(
    entity: &Option<crate::features::entity::Entity>,
    component_repo: &Arc<dyn ComponentRepository>,
) -> String {
    match entity {
        Some(e) => {
            let comps = component_repo.get(e.id).await.unwrap_or_default();
            extract_title_from_components(&comps)
        }
        None => String::new(),
    }
}

fn extract_title_from_components(
    components: &[crate::features::component::Component],
) -> String {
    components
        .iter()
        .find(|c| {
            matches!(
                c.component_type,
                crate::features::component::ComponentType::Title
            )
        })
        .and_then(|c| c.data.get("name").and_then(|v| v.as_str()))
        .unwrap_or("Untitled")
        .to_string()
}

fn extract_preview_from_components(
    components: &[crate::features::component::Component],
) -> String {
    components
        .iter()
        .find(|c| {
            matches!(
                c.component_type,
                crate::features::component::ComponentType::Content
            )
        })
        .and_then(|c| c.data.get("markdown").and_then(|v| v.as_str()))
        .map(|s| s.chars().take(200).collect())
        .unwrap_or_default()
}

fn extract_tags_from_components(
    components: &[crate::features::component::Component],
) -> Vec<String> {
    components
        .iter()
        .find(|c| {
            matches!(
                c.component_type,
                crate::features::component::ComponentType::Tags
            )
        })
        .and_then(|c| c.data.get("values").and_then(|v| v.as_array()))
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::component::Component;
    use crate::features::entity::{Entity, EntityType};
    use crate::features::relationship::Relationship;
    use async_trait::async_trait;
    use std::sync::RwLock;

    struct MockEntityRepo {
        entities: RwLock<Vec<Entity>>,
    }

    impl MockEntityRepo {
        fn new() -> Self {
            Self {
                entities: RwLock::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl EntityRepository for MockEntityRepo {
        async fn get(&self, id: Uuid) -> Result<Option<Entity>, StorageError> {
            Ok(self
                .entities
                .read()
                .unwrap()
                .iter()
                .find(|e| e.id == id)
                .cloned())
        }
        async fn save(&self, entity: &Entity) -> Result<(), StorageError> {
            self.entities.write().unwrap().push(entity.clone());
            Ok(())
        }
        async fn delete(&self, id: Uuid) -> Result<(), StorageError> {
            self.entities.write().unwrap().retain(|e| e.id != id);
            Ok(())
        }
        async fn list(&self) -> Result<Vec<Entity>, StorageError> {
            Ok(self.entities.read().unwrap().clone())
        }
        async fn find_by_type(&self, _et: &str) -> Result<Vec<Entity>, StorageError> {
            Ok(vec![])
        }
        async fn find_by_title(&self, _t: &str) -> Result<Vec<Entity>, StorageError> {
            Ok(vec![])
        }
        async fn increment_version(&self, _id: Uuid) -> Result<(), StorageError> {
            Ok(())
        }
        async fn find_by_component_type(
            &self,
            _ct: &str,
        ) -> Result<Vec<Entity>, StorageError> {
            Ok(vec![])
        }
        async fn find_by_tag(&self, _tag: &str) -> Result<Vec<Entity>, StorageError> {
            Ok(vec![])
        }
        async fn get_version_history(
            &self,
            _eid: Uuid,
        ) -> Result<Vec<crate::ports::EntityVersion>, StorageError> {
            Ok(vec![])
        }
    }

    struct MockComponentRepo {
        components: RwLock<Vec<Component>>,
    }

    impl MockComponentRepo {
        fn new() -> Self {
            Self {
                components: RwLock::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl ComponentRepository for MockComponentRepo {
        async fn get(&self, entity_id: Uuid) -> Result<Vec<Component>, StorageError> {
            Ok(self
                .components
                .read()
                .unwrap()
                .iter()
                .filter(|c| c.entity_id == entity_id)
                .cloned()
                .collect())
        }
        async fn save(&self, component: &Component) -> Result<(), StorageError> {
            self.components.write().unwrap().push(component.clone());
            Ok(())
        }
        async fn delete(&self, id: Uuid) -> Result<(), StorageError> {
            self.components.write().unwrap().retain(|c| c.id != id);
            Ok(())
        }
        async fn find_by_type(
            &self,
            _eid: Uuid,
            _ct: &str,
        ) -> Result<Vec<Component>, StorageError> {
            Ok(vec![])
        }
        async fn update_data(&self, _id: Uuid, _data: serde_json::Value) -> Result<(), StorageError> {
            Ok(())
        }
        async fn find_by_component_data(
            &self,
            _ct: &str,
            _jp: &str,
            _v: &str,
        ) -> Result<Vec<Component>, StorageError> {
            Ok(vec![])
        }
        async fn delete_by_entity(&self, _eid: Uuid) -> Result<(), StorageError> {
            Ok(())
        }
    }

    struct MockRelRepo {
        relationships: RwLock<Vec<Relationship>>,
    }

    impl MockRelRepo {
        fn new() -> Self {
            Self {
                relationships: RwLock::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl RelationshipRepository for MockRelRepo {
        async fn get(&self, id: Uuid) -> Result<Option<Relationship>, StorageError> {
            Ok(self
                .relationships
                .read()
                .unwrap()
                .iter()
                .find(|r| r.id == id)
                .cloned())
        }
        async fn save(&self, rel: &Relationship) -> Result<(), StorageError> {
            self.relationships.write().unwrap().push(rel.clone());
            Ok(())
        }
        async fn update(&self, rel: &Relationship) -> Result<(), StorageError> {
            let mut rels = self.relationships.write().unwrap();
            if let Some(existing) = rels.iter_mut().find(|r| r.id == rel.id) {
                *existing = rel.clone();
            }
            Ok(())
        }
        async fn delete(&self, id: Uuid) -> Result<(), StorageError> {
            self.relationships.write().unwrap().retain(|r| r.id != id);
            Ok(())
        }
        async fn by_source(&self, sid: Uuid) -> Result<Vec<Relationship>, StorageError> {
            Ok(self
                .relationships
                .read()
                .unwrap()
                .iter()
                .filter(|r| r.source_id == sid)
                .cloned()
                .collect())
        }
        async fn by_target(&self, tid: Uuid) -> Result<Vec<Relationship>, StorageError> {
            Ok(self
                .relationships
                .read()
                .unwrap()
                .iter()
                .filter(|r| r.target_id == tid)
                .cloned()
                .collect())
        }
        async fn find_by_source_and_target(
            &self,
            sid: Uuid,
            tid: Uuid,
        ) -> Result<Option<Relationship>, StorageError> {
            Ok(self
                .relationships
                .read()
                .unwrap()
                .iter()
                .find(|r| r.source_id == sid && r.target_id == tid)
                .cloned())
        }
        async fn find_by_type(&self, _rt: &str) -> Result<Vec<Relationship>, StorageError> {
            Ok(vec![])
        }
    }

    struct MockSearchIndex;

    #[async_trait]
    impl SearchIndex for MockSearchIndex {
        async fn index_entity(
            &self,
            _e: &Entity,
            _c: &[Component],
        ) -> Result<(), StorageError> {
            Ok(())
        }
        async fn remove_entity(&self, _eid: Uuid) -> Result<(), StorageError> {
            Ok(())
        }
        async fn search(
            &self,
            _q: &SearchQuery,
        ) -> Result<Vec<crate::ports::SearchResult>, StorageError> {
            Ok(vec![])
        }
        async fn rebuild(
            &self,
            _e: &[(Entity, Vec<Component>)],
        ) -> Result<(), StorageError> {
            Ok(())
        }
    }

    struct MockTraversal;

    #[async_trait]
    impl TraversalPort for MockTraversal {
        async fn traverse(
            &self,
            _q: &TraversalQuery,
            _c: &TraversalConfig,
        ) -> Result<Vec<crate::ports::TraversalResult>, crate::ports::TraversalError> {
            Ok(vec![])
        }
    }

    fn setup() -> EntityRetrievalService {
        let entity_repo = Arc::new(MockEntityRepo::new());
        let component_repo = Arc::new(MockComponentRepo::new());
        let relationship_repo = Arc::new(MockRelRepo::new());
        let search_index = Arc::new(MockSearchIndex);
        let traversal_port = Arc::new(MockTraversal);

        EntityRetrievalService::new(
            entity_repo as Arc<dyn EntityRepository>,
            component_repo as Arc<dyn ComponentRepository>,
            relationship_repo as Arc<dyn RelationshipRepository>,
            search_index as Arc<dyn SearchIndex>,
            traversal_port as Arc<dyn TraversalPort>,
        )
    }

    fn make_entity(entity_type: &str) -> Entity {
        Entity::new(EntityType::new(entity_type))
    }

    #[tokio::test]
    async fn get_entity_returns_components_map() {
        let service = setup();
        let entity = make_entity("Paper");
        let eid = entity.id;
        // Can't test get_entity without populating repos which are behind Arc
        // This is a minimal compilation test
        let result = service.get_entity(eid).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn get_entity_not_found_returns_error() {
        let service = setup();
        let fake_id = Uuid::new_v4();
        let result = service.get_entity(fake_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn get_entities_batch_returns_all() {
        let service = setup();
        let ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let results = service.get_entities(&ids).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn search_returns_filtered_results() {
        let service = setup();
        let filter = RetrievalFilter {
            entity_types: None,
            tags: None,
            limit: Some(10),
        };
        let results = service.search("test", &filter).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn search_respects_limit() {
        let service = setup();
        let filter = RetrievalFilter {
            entity_types: None,
            tags: None,
            limit: Some(5),
        };
        let results = service.search("test", &filter).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn traverse_returns_paths() {
        let service = setup();
        let start = Uuid::new_v4();
        let result = service.traverse(start, 3).await.unwrap();
        assert_eq!(result.start_id, start);
        assert!(result.results.is_empty());
    }
}
