//! Graph Database Module
//!
//! Implements a graph database for knowledge representation with entities,
//! relationships, and traversal algorithms for connected knowledge.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use log::{debug, error, info, warn};
use async_trait::async_trait;
use uuid::Uuid;
use tokio::sync::Mutex as AsyncMutex;

/// Error types for Graph Database operations
#[derive(Debug, Error)]
pub enum GraphDbError {
    #[error("Entity not found: {0}")]
    EntityNotFound(String),
    
    #[error("Relationship not found: {0}")]
    RelationshipNotFound(String),
    
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    
    #[error("Transaction error: {0}")]
    TransactionError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Query error: {0}")]
    QueryError(String),
}

/// Entity property value types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropertyValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    DateTime(u64),
    List(Vec<PropertyValue>),
    Map(HashMap<String, PropertyValue>),
    Null,
}

/// Entity properties map
pub type Properties = HashMap<String, PropertyValue>;

/// Entity type representing a node in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Unique entity identifier
    pub id: String,
    /// Entity labels/types
    pub labels: Vec<String>,
    /// Entity properties
    pub properties: Properties,
    /// Creation timestamp
    pub created_at: u64,
    /// Last updated timestamp
    pub updated_at: u64,
}

impl Entity {
    /// Create a new entity with the given labels and properties
    pub fn new(labels: Vec<String>, properties: Properties) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        Self {
            id: Uuid::new_v4().to_string(),
            labels,
            properties,
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Get a property value by its key
    pub fn get_property(&self, key: &str) -> Option<&PropertyValue> {
        self.properties.get(key)
    }
    
    /// Set a property value
    pub fn set_property(&mut self, key: String, value: PropertyValue) {
        self.properties.insert(key, value);
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }
    
    /// Check if the entity has a specific label
    pub fn has_label(&self, label: &str) -> bool {
        self.labels.contains(&label.to_string())
    }
    
    /// Add a label to the entity
    pub fn add_label(&mut self, label: String) {
        if !self.labels.contains(&label) {
            self.labels.push(label);
            self.updated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
        }
    }
    
    /// Remove a label from the entity
    pub fn remove_label(&mut self, label: &str) {
        self.labels.retain(|l| l != label);
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }
}

/// Relationship type representing an edge in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Unique relationship identifier
    pub id: String,
    /// Relationship type
    pub rel_type: String,
    /// Source entity identifier
    pub from_id: String,
    /// Target entity identifier
    pub to_id: String,
    /// Relationship properties
    pub properties: Properties,
    /// Creation timestamp
    pub created_at: u64,
    /// Last updated timestamp
    pub updated_at: u64,
}

impl Relationship {
    /// Create a new relationship
    pub fn new(rel_type: String, from_id: String, to_id: String, properties: Properties) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        Self {
            id: Uuid::new_v4().to_string(),
            rel_type,
            from_id,
            to_id,
            properties,
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Get a property value by its key
    pub fn get_property(&self, key: &str) -> Option<&PropertyValue> {
        self.properties.get(key)
    }
    
    /// Set a property value
    pub fn set_property(&mut self, key: String, value: PropertyValue) {
        self.properties.insert(key, value);
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }
}

/// Query direction for traversal
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    /// Outgoing relationships
    Outgoing,
    /// Incoming relationships
    Incoming,
    /// Both directions
    Both,
}

/// Path in the graph, used for traversal results
#[derive(Debug, Clone)]
pub struct Path {
    /// Entities in the path
    pub entities: Vec<Entity>,
    /// Relationships in the path
    pub relationships: Vec<Relationship>,
}

/// Graph database implementation
pub struct GraphDb {
    /// Map of entities by ID
    entities: RwLock<HashMap<String, Entity>>,
    /// Map of relationships by ID
    relationships: RwLock<HashMap<String, Relationship>>,
    /// Index of relationships by source entity ID
    outgoing_relationships: RwLock<HashMap<String, Vec<String>>>,
    /// Index of relationships by target entity ID
    incoming_relationships: RwLock<HashMap<String, Vec<String>>>,
    /// Index of entities by label
    label_index: RwLock<HashMap<String, HashSet<String>>>,
}

impl GraphDb {
    /// Create a new graph database instance
    pub fn new() -> Self {
        Self {
            entities: RwLock::new(HashMap::new()),
            relationships: RwLock::new(HashMap::new()),
            outgoing_relationships: RwLock::new(HashMap::new()),
            incoming_relationships: RwLock::new(HashMap::new()),
            label_index: RwLock::new(HashMap::new()),
        }
    }
    
    /// Create a new entity in the graph
    pub fn create_entity(&self, labels: Vec<String>, properties: Properties) -> Result<Entity, GraphDbError> {
        let entity = Entity::new(labels.clone(), properties);
        
        // Add entity to storage
        {
            let mut entities = self.entities.write().unwrap();
            entities.insert(entity.id.clone(), entity.clone());
        }
        
        // Update label index
        {
            let mut label_index = self.label_index.write().unwrap();
            for label in labels {
                let entities_for_label = label_index.entry(label).or_insert_with(HashSet::new);
                entities_for_label.insert(entity.id.clone());
            }
        }
        
        info!("Created entity with ID: {}", entity.id);
        Ok(entity)
    }
    
    /// Get an entity by its ID
    pub fn get_entity(&self, entity_id: &str) -> Result<Entity, GraphDbError> {
        let entities = self.entities.read().unwrap();
        entities.get(entity_id)
            .cloned()
            .ok_or_else(|| GraphDbError::EntityNotFound(entity_id.to_string()))
    }
    
    /// Update an entity's properties
    pub fn update_entity(&self, entity_id: &str, properties: Properties) -> Result<Entity, GraphDbError> {
        let mut entities = self.entities.write().unwrap();
        
        match entities.get_mut(entity_id) {
            Some(entity) => {
                // Update properties
                for (key, value) in properties {
                    entity.set_property(key, value);
                }
                
                Ok(entity.clone())
            },
            None => Err(GraphDbError::EntityNotFound(entity_id.to_string())),
        }
    }
    
    /// Delete an entity and all its relationships
    pub fn delete_entity(&self, entity_id: &str) -> Result<(), GraphDbError> {
        // Get all relationships for this entity
        let related_rel_ids = {
            let outgoing = self.outgoing_relationships.read().unwrap();
            let incoming = self.incoming_relationships.read().unwrap();
            
            let mut rel_ids = HashSet::new();
            
            if let Some(rels) = outgoing.get(entity_id) {
                rel_ids.extend(rels.iter().cloned());
            }
            
            if let Some(rels) = incoming.get(entity_id) {
                rel_ids.extend(rels.iter().cloned());
            }
            
            rel_ids
        };
        
        // Delete all relationships first
        for rel_id in related_rel_ids {
            self.delete_relationship(&rel_id)?;
        }
        
        // Remove from label index
        {
            let entity = self.get_entity(entity_id)?;
            let mut label_index = self.label_index.write().unwrap();
            
            for label in &entity.labels {
                if let Some(entities_for_label) = label_index.get_mut(label) {
                    entities_for_label.remove(entity_id);
                    
                    // Clean up empty label entries
                    if entities_for_label.is_empty() {
                        label_index.remove(label);
                    }
                }
            }
        }
        
        // Delete the entity
        {
            let mut entities = self.entities.write().unwrap();
            if entities.remove(entity_id).is_none() {
                return Err(GraphDbError::EntityNotFound(entity_id.to_string()));
            }
        }
        
        info!("Deleted entity with ID: {}", entity_id);
        Ok(())
    }
    
    /// Create a new relationship between entities
    pub fn create_relationship(
        &self, 
        rel_type: String, 
        from_id: String, 
        to_id: String,
        properties: Properties
    ) -> Result<Relationship, GraphDbError> {
        // Ensure both entities exist
        {
            let entities = self.entities.read().unwrap();
            if !entities.contains_key(&from_id) {
                return Err(GraphDbError::EntityNotFound(from_id));
            }
            if !entities.contains_key(&to_id) {
                return Err(GraphDbError::EntityNotFound(to_id));
            }
        }
        
        // Create relationship
        let relationship = Relationship::new(rel_type, from_id.clone(), to_id.clone(), properties);
        
        // Store relationship
        {
            let mut relationships = self.relationships.write().unwrap();
            relationships.insert(relationship.id.clone(), relationship.clone());
        }
        
        // Update indices
        {
            let mut outgoing = self.outgoing_relationships.write().unwrap();
            let outgoing_rels = outgoing.entry(from_id).or_insert_with(Vec::new);
            outgoing_rels.push(relationship.id.clone());
        }
        
        {
            let mut incoming = self.incoming_relationships.write().unwrap();
            let incoming_rels = incoming.entry(to_id).or_insert_with(Vec::new);
            incoming_rels.push(relationship.id.clone());
        }
        
        info!("Created relationship with ID: {}", relationship.id);
        Ok(relationship)
    }
    
    /// Get a relationship by its ID
    pub fn get_relationship(&self, relationship_id: &str) -> Result<Relationship, GraphDbError> {
        let relationships = self.relationships.read().unwrap();
        relationships.get(relationship_id)
            .cloned()
            .ok_or_else(|| GraphDbError::RelationshipNotFound(relationship_id.to_string()))
    }
    
    /// Update a relationship's properties
    pub fn update_relationship(&self, relationship_id: &str, properties: Properties) -> Result<Relationship, GraphDbError> {
        let mut relationships = self.relationships.write().unwrap();
        
        match relationships.get_mut(relationship_id) {
            Some(relationship) => {
                // Update properties
                for (key, value) in properties {
                    relationship.set_property(key, value);
                }
                
                Ok(relationship.clone())
            },
            None => Err(GraphDbError::RelationshipNotFound(relationship_id.to_string())),
        }
    }
    
    /// Delete a relationship
    pub fn delete_relationship(&self, relationship_id: &str) -> Result<(), GraphDbError> {
        // Get the relationship first
        let relationship = self.get_relationship(relationship_id)?;
        let from_id = relationship.from_id.clone();
        let to_id = relationship.to_id.clone();
        
        // Remove from indices
        {
            let mut outgoing = self.outgoing_relationships.write().unwrap();
            if let Some(rels) = outgoing.get_mut(&from_id) {
                rels.retain(|id| id != relationship_id);
                
                // Clean up empty entries
                if rels.is_empty() {
                    outgoing.remove(&from_id);
                }
            }
        }
        
        {
            let mut incoming = self.incoming_relationships.write().unwrap();
            if let Some(rels) = incoming.get_mut(&to_id) {
                rels.retain(|id| id != relationship_id);
                
                // Clean up empty entries
                if rels.is_empty() {
                    incoming.remove(&to_id);
                }
            }
        }
        
        // Delete the relationship
        {
            let mut relationships = self.relationships.write().unwrap();
            if relationships.remove(relationship_id).is_none() {
                return Err(GraphDbError::RelationshipNotFound(relationship_id.to_string()));
            }
        }
        
        info!("Deleted relationship with ID: {}", relationship_id);
        Ok(())
    }
    
    /// Find entities by label
    pub fn find_entities_by_label(&self, label: &str) -> Vec<Entity> {
        // Get entity IDs for the label
        let entity_ids = {
            let label_index = self.label_index.read().unwrap();
            match label_index.get(label) {
                Some(ids) => ids.clone(),
                None => return Vec::new(),
            }
        };
        
        // Get entities
        let entities = self.entities.read().unwrap();
        entity_ids.into_iter()
            .filter_map(|id| entities.get(&id).cloned())
            .collect()
    }
    
    /// Find entities by property value
    pub fn find_entities_by_property(&self, property_key: &str, property_value: &PropertyValue) -> Vec<Entity> {
        let entities = self.entities.read().unwrap();
        entities.values()
            .filter(|entity| {
                match entity.get_property(property_key) {
                    Some(value) => value == property_value,
                    None => false,
                }
            })
            .cloned()
            .collect()
    }
    
    /// Get related entities (neighbors) for an entity
    pub fn get_related_entities(
        &self, 
        entity_id: &str, 
        direction: Direction,
        rel_types: Option<Vec<String>>
    ) -> Result<Vec<(Relationship, Entity)>, GraphDbError> {
        let mut results = Vec::new();
        
        // Check if entity exists
        if !self.entities.read().unwrap().contains_key(entity_id) {
            return Err(GraphDbError::EntityNotFound(entity_id.to_string()));
        }
        
        let relationships = self.relationships.read().unwrap();
        let entities = self.entities.read().unwrap();
        
        // Get outgoing relationships if needed
        if direction == Direction::Outgoing || direction == Direction::Both {
            let outgoing = self.outgoing_relationships.read().unwrap();
            if let Some(rel_ids) = outgoing.get(entity_id) {
                for rel_id in rel_ids {
                    if let Some(rel) = relationships.get(rel_id) {
                        // Filter by relationship type if specified
                        if let Some(ref types) = rel_types {
                            if !types.contains(&rel.rel_type) {
                                continue;
                            }
                        }
                        
                        if let Some(target_entity) = entities.get(&rel.to_id) {
                            results.push((rel.clone(), target_entity.clone()));
                        }
                    }
                }
            }
        }
        
        // Get incoming relationships if needed
        if direction == Direction::Incoming || direction == Direction::Both {
            let incoming = self.incoming_relationships.read().unwrap();
            if let Some(rel_ids) = incoming.get(entity_id) {
                for rel_id in rel_ids {
                    if let Some(rel) = relationships.get(rel_id) {
                        // Filter by relationship type if specified
                        if let Some(ref types) = rel_types {
                            if !types.contains(&rel.rel_type) {
                                continue;
                            }
                        }
                        
                        if let Some(source_entity) = entities.get(&rel.from_id) {
                            results.push((rel.clone(), source_entity.clone()));
                        }
                    }
                }
            }
        }
        
        Ok(results)
    }
    
    /// Find all paths between two entities (BFS traversal)
    pub fn find_paths(
        &self,
        start_id: &str,
        end_id: &str,
        max_depth: usize,
        rel_types: Option<Vec<String>>,
        direction: Direction,
    ) -> Result<Vec<Path>, GraphDbError> {
        // Check if entities exist
        {
            let entities = self.entities.read().unwrap();
            if !entities.contains_key(start_id) {
                return Err(GraphDbError::EntityNotFound(start_id.to_string()));
            }
            if !entities.contains_key(end_id) {
                return Err(GraphDbError::EntityNotFound(end_id.to_string()));
            }
        }
        
        let mut paths = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        
        // Initialize with start node
        let start_entity = self.get_entity(start_id)?;
        let initial_path = Path {
            entities: vec![start_entity],
            relationships: vec![],
        };
        
        queue.push_back((initial_path, 0));
        visited.insert(start_id.to_string());
        
        while let Some((current_path, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            
            let current_entity_id = &current_path.entities.last().unwrap().id;
            
            // If we've reached the end, add the path to results
            if current_entity_id == end_id {
                paths.push(current_path.clone());
                continue;
            }
            
            // Find next steps
            let related = self.get_related_entities(current_entity_id, direction, rel_types.clone())?;
            
            for (rel, entity) in related {
                // Skip visited nodes to prevent cycles
                if visited.contains(&entity.id) {
                    continue;
                }
                
                // Create new path
                let mut new_path = current_path.clone();
                new_path.entities.push(entity.clone());
                new_path.relationships.push(rel.clone());
                
                // Add to queue
                queue.push_back((new_path, depth + 1));
                visited.insert(entity.id.clone());
            }
        }
        
        Ok(paths)
    }
    
    /// Get entity count
    pub fn entity_count(&self) -> usize {
        self.entities.read().unwrap().len()
    }
    
    /// Get relationship count
    pub fn relationship_count(&self) -> usize {
        self.relationships.read().unwrap().len()
    }
    
    /// Export a subgraph around an entity (for visualization or analysis)
    pub fn export_subgraph(&self, center_id: &str, max_depth: usize) -> Result<(Vec<Entity>, Vec<Relationship>), GraphDbError> {
        let mut entity_ids = HashSet::new();
        let mut relationship_ids = HashSet::new();
        let mut queue = VecDeque::new();
        
        // Check if start entity exists
        if !self.entities.read().unwrap().contains_key(center_id) {
            return Err(GraphDbError::EntityNotFound(center_id.to_string()));
        }
        
        // Start BFS from center entity
        entity_ids.insert(center_id.to_string());
        queue.push_back((center_id.to_string(), 0));
        
        while let Some((current_id, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            
            // Get all relationships for current entity (both directions)
            let related = self.get_related_entities(&current_id, Direction::Both, None)?;
            
            for (rel, entity) in related {
                // Add relationship and entity to result
                relationship_ids.insert(rel.id.clone());
                
                if !entity_ids.contains(&entity.id) {
                    entity_ids.insert(entity.id.clone());
                    queue.push_back((entity.id.clone(), depth + 1));
                }
            }
        }
        
        // Collect entities and relationships
        let entities = {
            let all_entities = self.entities.read().unwrap();
            entity_ids.iter()
                .filter_map(|id| all_entities.get(id).cloned())
                .collect()
        };
        
        let relationships = {
            let all_relationships = self.relationships.read().unwrap();
            relationship_ids.iter()
                .filter_map(|id| all_relationships.get(id).cloned())
                .collect()
        };
        
        Ok((entities, relationships))
    }
}

/// Default implementation
impl Default for GraphDb {
    fn default() -> Self {
        Self::new()
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_graph() -> GraphDb {
        let graph = GraphDb::new();
        
        // Create entities
        let person1 = graph.create_entity(
            vec!["Person".to_string()],
            {
                let mut props = HashMap::new();
                props.insert("name".to_string(), PropertyValue::String("Alice".to_string()));
                props.insert("age".to_string(), PropertyValue::Integer(30));
                props
            }
        ).unwrap();
        
        let person2 = graph.create_entity(
            vec!["Person".to_string()],
            {
                let mut props = HashMap::new();
                props.insert("name".to_string(), PropertyValue::String("Bob".to_string()));
                props.insert("age".to_string(), PropertyValue::Integer(25));
                props
            }
        ).unwrap();
        
        let person3 = graph.create_entity(
            vec!["Person".to_string()],
            {
                let mut props = HashMap::new();
                props.insert("name".to_string(), PropertyValue::String("Charlie".to_string()));
                props.insert("age".to_string(), PropertyValue::Integer(35));
                props
            }
        ).unwrap();
        
        // Create relationships
        graph.create_relationship(
            "KNOWS".to_string(),
            person1.id,
            person2.id,
            {
                let mut props = HashMap::new();
                props.insert("since".to_string(), PropertyValue::Integer(2020));
                props
            }
        ).unwrap();
        
        graph.create_relationship(
            "KNOWS".to_string(),
            person2.id,
            person3.id,
            {
                let mut props = HashMap::new();
                props.insert("since".to_string(), PropertyValue::Integer(2019));
                props
            }
        ).unwrap();
        
        graph
    }
    
    #[test]
    fn test_entity_operations() {
        let graph = GraphDb::new();
        
        // Create entity
        let entity = graph.create_entity(
            vec!["Person".to_string()],
            {
                let mut props = HashMap::new();
                props.insert("name".to_string(), PropertyValue::String("Test".to_string()));
                props
            }
        ).unwrap();
        
        // Get entity
        let retrieved = graph.get_entity(&entity.id).unwrap();
        assert_eq!(retrieved.id, entity.id);
        assert_eq!(retrieved.labels[0], "Person");
        
        // Update entity
        let updated = graph.update_entity(
            &entity.id,
            {
                let mut props = HashMap::new();
                props.insert("name".to_string(), PropertyValue::String("Updated".to_string()));
                props.insert("age".to_string(), PropertyValue::Integer(30));
                props
            }
        ).unwrap();
        
        assert_eq!(updated.id, entity.id);
        assert_eq!(
            updated.get_property("name").unwrap(),
            &PropertyValue::String("Updated".to_string())
        );
        assert_eq!(
            updated.get_property("age").unwrap(),
            &PropertyValue::Integer(30)
        );
        
        // Delete entity
        graph.delete_entity(&entity.id).unwrap();
        assert!(graph.get_entity(&entity.id).is_err());
    }
    
    #[test]
    fn test_relationship_operations() {
        let graph = GraphDb::new();
        
        // Create entities
        let entity1 = graph.create_entity(
            vec!["Person".to_string()],
            {
                let mut props = HashMap::new();
                props.insert("name".to_string(), PropertyValue::String("Alice".to_string()));
                props
            }
        ).unwrap();
        
        let entity2 = graph.create_entity(
            vec!["Person".to_string()],
            {
                let mut props = HashMap::new();
                props.insert("name".to_string(), PropertyValue::String("Bob".to_string()));
                props
            }
        ).unwrap();
        
        // Create relationship
        let relationship = graph.create_relationship(
            "KNOWS".to_string(),
            entity1.id.clone(),
            entity2.id.clone(),
            {
                let mut props = HashMap::new();
                props.insert("since".to_string(), PropertyValue::Integer(2020));
                props
            }
        ).unwrap();
        
        // Get relationship
        let retrieved = graph.get_relationship(&relationship.id).unwrap();
        assert_eq!(retrieved.id, relationship.id);
        assert_eq!(retrieved.rel_type, "KNOWS");
        assert_eq!(retrieved.from_id, entity1.id);
        assert_eq!(retrieved.to_id, entity2.id);
        
        // Update relationship
        let updated = graph.update_relationship(
            &relationship.id,
            {
                let mut props = HashMap::new();
                props.insert("since".to_string(), PropertyValue::Integer(2022));
                props.insert("strength".to_string(), PropertyValue::Float(0.75));
                props
            }
        ).unwrap();
        
        assert_eq!(updated.id, relationship.id);
        assert_eq!(
            updated.get_property("since").unwrap(),
            &PropertyValue::Integer(2022)
        );
        assert_eq!(
            updated.get_property("strength").unwrap(),
            &PropertyValue::Float(0.75)
        );
        
        // Delete relationship
        graph.delete_relationship(&relationship.id).unwrap();
        assert!(graph.get_relationship(&relationship.id).is_err());
    }
    
    #[test]
    fn test_entity_queries() {
        let graph = create_test_graph();
        
        // Find by label
        let people = graph.find_entities_by_label("Person");
        assert_eq!(people.len(), 3);
        
        // Find by property
        let alice = graph.find_entities_by_property("name", &PropertyValue::String("Alice".to_string()));
        assert_eq!(alice.len(), 1);
        assert_eq!(
            alice[0].get_property("name").unwrap(),
            &PropertyValue::String("Alice".to_string())
        );
    }
    
    #[test]
    fn test_traversal() {
        let graph = create_test_graph();
        
        // Find people that Alice knows
        let alice = graph.find_entities_by_property("name", &PropertyValue::String("Alice".to_string()))[0].clone();
        
        let alice_knows = graph.get_related_entities(&alice.id, Direction::Outgoing, Some(vec!["KNOWS".to_string()])).unwrap();
        assert_eq!(alice_knows.len(), 1);
        assert_eq!(
            alice_knows[0].1.get_property("name").unwrap(),
            &PropertyValue::String("Bob".to_string())
        );
        
        // Find path from Alice to Charlie
        let charlie = graph.find_entities_by_property("name", &PropertyValue::String("Charlie".to_string()))[0].clone();
        
        let paths = graph.find_paths(&alice.id, &charlie.id, 2, None, Direction::Outgoing).unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].entities.len(), 3); // Alice -> Bob -> Charlie
        assert_eq!(paths[0].relationships.len(), 2);
    }
    
    #[test]
    fn test_subgraph_export() {
        let graph = create_test_graph();
        
        // Get Alice
        let alice = graph.find_entities_by_property("name", &PropertyValue::String("Alice".to_string()))[0].clone();
        
        // Export subgraph centered on Alice with depth 2
        let (entities, relationships) = graph.export_subgraph(&alice.id, 2).unwrap();
        
        assert_eq!(entities.len(), 3); // Alice, Bob, Charlie
        assert_eq!(relationships.len(), 2); // Alice-KNOWS->Bob, Bob-KNOWS->Charlie
    }
}