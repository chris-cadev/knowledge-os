use knowledge_core::features::entity::Entity;

pub struct SearchIndex;

impl Default for SearchIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchIndex {
    pub fn new() -> Self {
        Self
    }

    pub async fn index(&self, _entity: &Entity) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: implement indexing
        Ok(())
    }

    pub async fn search(&self, _query: &str) -> Result<Vec<Entity>, Box<dyn std::error::Error>> {
        // TODO: implement search
        Ok(vec![])
    }
}
