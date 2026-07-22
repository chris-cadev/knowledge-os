use knowledge_core::features::entity::{Entity, EntityType};

pub struct Importer;

impl Default for Importer {
    fn default() -> Self {
        Self::new()
    }
}

impl Importer {
    pub fn new() -> Self {
        Self
    }

    pub async fn import(&self, _source: &str) -> Result<Entity, Box<dyn std::error::Error>> {
        // TODO: implement import pipeline
        let entity = Entity::new(EntityType::Note);
        Ok(entity)
    }
}
