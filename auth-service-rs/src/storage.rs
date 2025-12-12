// auth-service-rs/src/storage.rs
//
// Storage backend abstraction for the auth service
// Provides:
// - Entity storage and retrieval
// - Query support
// - Pagination
// - Multiple backend implementations (in-memory, PostgreSQL)
//
// Note: This uses JSON serialization for type erasure to make the trait dyn-compatible

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, de::DeserializeOwned};
use anyhow::{Result, anyhow, Context};
use async_trait::async_trait;
use tracing::{debug, error, info, warn};

/// Entity trait for storable objects
pub trait Entity: Serialize + DeserializeOwned + Clone + Send + Sync {
    /// Get the unique identifier for this entity
    fn get_id(&self) -> String;
    
    /// Get the entity type name (used for storage partitioning)
    fn get_entity_type() -> &'static str;
}

/// Storage backend trait - dyn-compatible version using JSON for type erasure
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Initialize the storage backend
    async fn initialize(&self) -> Result<()>;
    
    /// Check if the storage backend is healthy
    async fn is_healthy(&self) -> bool;
    
    /// Store an entity as JSON
    async fn store_json(&self, entity_type: &str, id: &str, data: serde_json::Value) -> Result<()>;
    
    /// Get an entity as JSON by ID
    async fn get_json(&self, entity_type: &str, id: &str) -> Result<serde_json::Value>;
    
    /// Delete an entity by ID
    async fn delete_by_id(&self, entity_type: &str, id: &str) -> Result<()>;
    
    /// List all entities of a type as JSON
    async fn list_json(&self, entity_type: &str) -> Result<Vec<serde_json::Value>>;
    
    /// Query entities with a filter
    async fn query_json(&self, entity_type: &str, query: &str) -> Result<Vec<serde_json::Value>>;
    
    /// Query entities with pagination
    async fn query_json_paged(
        &self,
        entity_type: &str,
        query: Option<&str>,
        page_size: usize,
        offset: usize,
        sort: Option<&str>,
    ) -> Result<Vec<serde_json::Value>>;
    
    /// Count entities matching a query
    async fn count(&self, entity_type: &str, query: Option<&str>) -> Result<usize>;
}

/// Extension trait for typed entity operations
#[async_trait]
pub trait StorageBackendExt: StorageBackend {
    /// Store a typed entity
    async fn store_entity<E: Entity>(&self, entity: &E) -> Result<()> {
        let entity_type = E::get_entity_type();
        let id = entity.get_id();
        let data = serde_json::to_value(entity)
            .context("Failed to serialize entity")?;
        self.store_json(entity_type, &id, data).await
    }
    
    /// Get a typed entity by ID
    async fn get_entity<E: Entity>(&self, id: &str) -> Result<E> {
        let entity_type = E::get_entity_type();
        let json = self.get_json(entity_type, id).await?;
        serde_json::from_value(json).context("Failed to deserialize entity")
    }
    
    /// Delete a typed entity by ID
    async fn delete_entity<E: Entity>(&self, id: &str) -> Result<()> {
        let entity_type = E::get_entity_type();
        self.delete_by_id(entity_type, id).await
    }
    
    /// List all typed entities
    async fn list_entities<E: Entity>(&self) -> Result<Vec<E>> {
        let entity_type = E::get_entity_type();
        let json_list = self.list_json(entity_type).await?;
        json_list
            .into_iter()
            .map(|json| serde_json::from_value(json).context("Failed to deserialize entity"))
            .collect()
    }
    
    /// Query typed entities
    async fn query_entities<E: Entity>(&self, query: &str) -> Result<Vec<E>> {
        let entity_type = E::get_entity_type();
        let json_list = self.query_json(entity_type, query).await?;
        json_list
            .into_iter()
            .map(|json| serde_json::from_value(json).context("Failed to deserialize entity"))
            .collect()
    }
    
    /// Query typed entities with pagination
    async fn query_entities_paged<E: Entity>(
        &self,
        query: Option<&str>,
        page_size: usize,
        offset: usize,
        sort: Option<&str>,
    ) -> Result<Vec<E>> {
        let entity_type = E::get_entity_type();
        let json_list = self.query_json_paged(entity_type, query, page_size, offset, sort).await?;
        json_list
            .into_iter()
            .map(|json| serde_json::from_value(json).context("Failed to deserialize entity"))
            .collect()
    }
    
    /// Count typed entities
    async fn count_entities<E: Entity>(&self, query: Option<&str>) -> Result<usize> {
        let entity_type = E::get_entity_type();
        self.count(entity_type, query).await
    }
}

// Implement StorageBackendExt for all types that implement StorageBackend
impl<T: StorageBackend + ?Sized> StorageBackendExt for T {}

/// In-memory storage backend for testing and development
pub struct InMemoryStorage {
    data: Arc<RwLock<HashMap<String, HashMap<String, serde_json::Value>>>>,
}

impl InMemoryStorage {
    /// Create a new in-memory storage backend
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StorageBackend for InMemoryStorage {
    async fn initialize(&self) -> Result<()> {
        info!("In-memory storage initialized");
        Ok(())
    }
    
    async fn is_healthy(&self) -> bool {
        true
    }
    
    async fn store_json(&self, entity_type: &str, id: &str, data: serde_json::Value) -> Result<()> {
        let mut storage = self.data.write().await;
        let type_map = storage.entry(entity_type.to_string()).or_insert_with(HashMap::new);
        type_map.insert(id.to_string(), data);
        
        debug!("Stored entity {}:{}", entity_type, id);
        Ok(())
    }
    
    async fn get_json(&self, entity_type: &str, id: &str) -> Result<serde_json::Value> {
        let storage = self.data.read().await;
        let type_map = storage.get(entity_type)
            .ok_or_else(|| anyhow!("Entity type {} not found", entity_type))?;
        
        let data = type_map.get(id)
            .ok_or_else(|| anyhow!("Entity {}:{} not found", entity_type, id))?;
        
        Ok(data.clone())
    }
    
    async fn delete_by_id(&self, entity_type: &str, id: &str) -> Result<()> {
        let mut storage = self.data.write().await;
        if let Some(type_map) = storage.get_mut(entity_type) {
            type_map.remove(id);
            debug!("Deleted entity {}:{}", entity_type, id);
        }
        
        Ok(())
    }
    
    async fn list_json(&self, entity_type: &str) -> Result<Vec<serde_json::Value>> {
        let storage = self.data.read().await;
        let type_map = match storage.get(entity_type) {
            Some(map) => map,
            None => return Ok(Vec::new()),
        };
        
        Ok(type_map.values().cloned().collect())
    }
    
    async fn query_json(&self, entity_type: &str, _query: &str) -> Result<Vec<serde_json::Value>> {
        // For in-memory storage, we just return all entities
        // A real implementation would parse and apply the query
        self.list_json(entity_type).await
    }
    
    async fn query_json_paged(
        &self,
        entity_type: &str,
        _query: Option<&str>,
        page_size: usize,
        offset: usize,
        _sort: Option<&str>,
    ) -> Result<Vec<serde_json::Value>> {
        let all_entities = self.list_json(entity_type).await?;
        
        // Apply pagination
        let start = offset.min(all_entities.len());
        let end = (offset + page_size).min(all_entities.len());
        
        Ok(all_entities[start..end].to_vec())
    }
    
    async fn count(&self, entity_type: &str, _query: Option<&str>) -> Result<usize> {
        let storage = self.data.read().await;
        let count = storage.get(entity_type)
            .map(|m| m.len())
            .unwrap_or(0);
        
        Ok(count)
    }
}

/// PostgreSQL storage backend
pub struct PostgresStorage {
    pool: sqlx::PgPool,
}

impl PostgresStorage {
    /// Create a new PostgreSQL storage backend
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = sqlx::PgPool::connect(database_url)
            .await
            .context("Failed to connect to PostgreSQL")?;
        
        Ok(Self { pool })
    }
}

#[async_trait]
impl StorageBackend for PostgresStorage {
    async fn initialize(&self) -> Result<()> {
        // Create tables if they don't exist
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS entities (
                entity_type VARCHAR(255) NOT NULL,
                id VARCHAR(255) NOT NULL,
                data JSONB NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                PRIMARY KEY (entity_type, id)
            )
            "#
        )
        .execute(&self.pool)
        .await
        .context("Failed to create entities table")?;
        
        info!("PostgreSQL storage initialized");
        Ok(())
    }
    
    async fn is_healthy(&self) -> bool {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .is_ok()
    }
    
    async fn store_json(&self, entity_type: &str, id: &str, data: serde_json::Value) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO entities (entity_type, id, data, updated_at)
            VALUES ($1, $2, $3, NOW())
            ON CONFLICT (entity_type, id) 
            DO UPDATE SET data = $3, updated_at = NOW()
            "#
        )
        .bind(entity_type)
        .bind(id)
        .bind(&data)
        .execute(&self.pool)
        .await
        .context("Failed to store entity")?;
        
        debug!("Stored entity {}:{}", entity_type, id);
        Ok(())
    }
    
    async fn get_json(&self, entity_type: &str, id: &str) -> Result<serde_json::Value> {
        let row: (serde_json::Value,) = sqlx::query_as(
            "SELECT data FROM entities WHERE entity_type = $1 AND id = $2"
        )
        .bind(entity_type)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .context("Entity not found")?;
        
        Ok(row.0)
    }
    
    async fn delete_by_id(&self, entity_type: &str, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM entities WHERE entity_type = $1 AND id = $2")
            .bind(entity_type)
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete entity")?;
        
        debug!("Deleted entity {}:{}", entity_type, id);
        Ok(())
    }
    
    async fn list_json(&self, entity_type: &str) -> Result<Vec<serde_json::Value>> {
        let rows: Vec<(serde_json::Value,)> = sqlx::query_as(
            "SELECT data FROM entities WHERE entity_type = $1"
        )
        .bind(entity_type)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list entities")?;
        
        Ok(rows.into_iter().map(|r| r.0).collect())
    }
    
    async fn query_json(&self, entity_type: &str, _query: &str) -> Result<Vec<serde_json::Value>> {
        // For simplicity, just list all entities
        // A real implementation would parse the query and build SQL
        self.list_json(entity_type).await
    }
    
    async fn query_json_paged(
        &self,
        entity_type: &str,
        _query: Option<&str>,
        page_size: usize,
        offset: usize,
        sort: Option<&str>,
    ) -> Result<Vec<serde_json::Value>> {
        let order_by = sort.unwrap_or("id ASC");
        
        let sql = format!(
            "SELECT data FROM entities WHERE entity_type = $1 ORDER BY {} LIMIT $2 OFFSET $3",
            order_by
        );
        
        let rows: Vec<(serde_json::Value,)> = sqlx::query_as(&sql)
            .bind(entity_type)
            .bind(page_size as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .context("Failed to query entities")?;
        
        Ok(rows.into_iter().map(|r| r.0).collect())
    }
    
    async fn count(&self, entity_type: &str, _query: Option<&str>) -> Result<usize> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM entities WHERE entity_type = $1"
        )
        .bind(entity_type)
        .fetch_one(&self.pool)
        .await
        .context("Failed to count entities")?;
        
        Ok(row.0 as usize)
    }
}

/// Create a storage backend based on configuration
pub async fn create_storage_backend(config: &StorageConfig) -> Result<Arc<dyn StorageBackend>> {
    match config.backend_type.as_str() {
        "memory" | "in-memory" => {
            info!("Using in-memory storage backend");
            Ok(Arc::new(InMemoryStorage::new()))
        }
        "postgres" | "postgresql" => {
            let url = config.connection_string.as_ref()
                .ok_or_else(|| anyhow!("PostgreSQL connection string required"))?;
            info!("Using PostgreSQL storage backend");
            Ok(Arc::new(PostgresStorage::new(url).await?))
        }
        _ => {
            Err(anyhow!("Unknown storage backend type: {}", config.backend_type))
        }
    }
}

/// Storage configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub backend_type: String,
    pub connection_string: Option<String>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend_type: "memory".to_string(),
            connection_string: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestEntity {
        id: String,
        name: String,
        value: i32,
    }
    
    impl Entity for TestEntity {
        fn get_id(&self) -> String {
            self.id.clone()
        }
        
        fn get_entity_type() -> &'static str {
            "test_entity"
        }
    }
    
    #[tokio::test]
    async fn test_in_memory_storage() {
        let storage = InMemoryStorage::new();
        storage.initialize().await.unwrap();
        
        // Store an entity
        let entity = TestEntity {
            id: "test-1".to_string(),
            name: "Test Entity".to_string(),
            value: 42,
        };
        
        storage.store_entity(&entity).await.unwrap();
        
        // Retrieve the entity
        let retrieved: TestEntity = storage.get_entity("test-1").await.unwrap();
        assert_eq!(retrieved.name, "Test Entity");
        assert_eq!(retrieved.value, 42);
        
        // List entities
        let all: Vec<TestEntity> = storage.list_entities().await.unwrap();
        assert_eq!(all.len(), 1);
        
        // Delete the entity
        storage.delete_entity::<TestEntity>("test-1").await.unwrap();
        
        // Verify deletion
        let result: Result<TestEntity> = storage.get_entity("test-1").await;
        assert!(result.is_err());
    }
}
