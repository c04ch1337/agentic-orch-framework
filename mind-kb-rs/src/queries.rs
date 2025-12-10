//! Query Module for Mind Knowledge Base
//!
//! Provides advanced query mechanisms for knowledge retrieval from the graph database.
//! Implements query building, graph pattern matching, and traversal algorithms.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use log::{debug, info, warn, error};

use crate::graph_db::{
    GraphDb, GraphDbError, Entity, Relationship, PropertyValue, Properties,
    Path, Direction,
};

/// Query error types
#[derive(Debug, Error)]
pub enum QueryError {
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
    
    #[error("Graph database error: {0}")]
    GraphDbError(#[from] GraphDbError),
    
    #[error("Execution error: {0}")]
    ExecutionError(String),
    
    #[error("Timeout: {0}")]
    Timeout(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Comparison operators for property conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Contains,
    StartsWith,
    EndsWith,
    In,
    NotIn,
    Exists,
    NotExists,
}

impl fmt::Display for ComparisonOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Equal => write!(f, "="),
            Self::NotEqual => write!(f, "!="),
            Self::GreaterThan => write!(f, ">"),
            Self::GreaterThanOrEqual => write!(f, ">="),
            Self::LessThan => write!(f, "<"),
            Self::LessThanOrEqual => write!(f, "<="),
            Self::Contains => write!(f, "CONTAINS"),
            Self::StartsWith => write!(f, "STARTS WITH"),
            Self::EndsWith => write!(f, "ENDS WITH"),
            Self::In => write!(f, "IN"),
            Self::NotIn => write!(f, "NOT IN"),
            Self::Exists => write!(f, "EXISTS"),
            Self::NotExists => write!(f, "NOT EXISTS"),
        }
    }
}

/// Logical operators for combining conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogicalOperator {
    And,
    Or,
    Not,
}

impl fmt::Display for LogicalOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::And => write!(f, "AND"),
            Self::Or => write!(f, "OR"),
            Self::Not => write!(f, "NOT"),
        }
    }
}

/// Property condition for filtering entities and relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyCondition {
    /// Property key
    pub property: String,
    /// Comparison operator
    pub operator: ComparisonOperator,
    /// Value to compare against
    pub value: Option<PropertyValue>,
}

impl PropertyCondition {
    /// Create a new property condition
    pub fn new(property: String, operator: ComparisonOperator, value: Option<PropertyValue>) -> Self {
        Self {
            property,
            operator,
            value,
        }
    }
    
    /// Evaluate the condition against the given properties
    pub fn evaluate(&self, properties: &Properties) -> bool {
        match self.operator {
            // Existence operators don't need a value
            ComparisonOperator::Exists => properties.contains_key(&self.property),
            ComparisonOperator::NotExists => !properties.contains_key(&self.property),
            
            // All other operators need a value
            _ => {
                if let Some(prop_value) = properties.get(&self.property) {
                    if let Some(condition_value) = &self.value {
                        match self.operator {
                            ComparisonOperator::Equal => prop_value == condition_value,
                            ComparisonOperator::NotEqual => prop_value != condition_value,
                            ComparisonOperator::GreaterThan => compare_greater_than(prop_value, condition_value),
                            ComparisonOperator::GreaterThanOrEqual => {
                                prop_value == condition_value || compare_greater_than(prop_value, condition_value)
                            },
                            ComparisonOperator::LessThan => compare_less_than(prop_value, condition_value),
                            ComparisonOperator::LessThanOrEqual => {
                                prop_value == condition_value || compare_less_than(prop_value, condition_value)
                            },
                            ComparisonOperator::Contains => compare_contains(prop_value, condition_value),
                            ComparisonOperator::StartsWith => compare_starts_with(prop_value, condition_value),
                            ComparisonOperator::EndsWith => compare_ends_with(prop_value, condition_value),
                            ComparisonOperator::In => compare_in(prop_value, condition_value),
                            ComparisonOperator::NotIn => !compare_in(prop_value, condition_value),
                            ComparisonOperator::Exists => true,
                            ComparisonOperator::NotExists => false,
                        }
                    } else {
                        // Value is required for non-existence operators
                        false
                    }
                } else {
                    // Property doesn't exist
                    false
                }
            },
        }
    }
}

/// Compare if left is greater than right
fn compare_greater_than(left: &PropertyValue, right: &PropertyValue) -> bool {
    match (left, right) {
        (PropertyValue::Integer(l), PropertyValue::Integer(r)) => l > r,
        (PropertyValue::Float(l), PropertyValue::Float(r)) => l > r,
        (PropertyValue::Integer(l), PropertyValue::Float(r)) => (*l as f64) > *r,
        (PropertyValue::Float(l), PropertyValue::Integer(r)) => *l > (*r as f64),
        (PropertyValue::String(l), PropertyValue::String(r)) => l > r,
        _ => false,
    }
}

/// Compare if left is less than right
fn compare_less_than(left: &PropertyValue, right: &PropertyValue) -> bool {
    match (left, right) {
        (PropertyValue::Integer(l), PropertyValue::Integer(r)) => l < r,
        (PropertyValue::Float(l), PropertyValue::Float(r)) => l < r,
        (PropertyValue::Integer(l), PropertyValue::Float(r)) => (*l as f64) < *r,
        (PropertyValue::Float(l), PropertyValue::Integer(r)) => *l < (*r as f64),
        (PropertyValue::String(l), PropertyValue::String(r)) => l < r,
        _ => false,
    }
}

/// Compare if left contains right
fn compare_contains(left: &PropertyValue, right: &PropertyValue) -> bool {
    match (left, right) {
        (PropertyValue::String(l), PropertyValue::String(r)) => l.contains(r),
        (PropertyValue::List(l), r) => l.contains(r),
        _ => false,
    }
}

/// Compare if left starts with right
fn compare_starts_with(left: &PropertyValue, right: &PropertyValue) -> bool {
    match (left, right) {
        (PropertyValue::String(l), PropertyValue::String(r)) => l.starts_with(r),
        _ => false,
    }
}

/// Compare if left ends with right
fn compare_ends_with(left: &PropertyValue, right: &PropertyValue) -> bool {
    match (left, right) {
        (PropertyValue::String(l), PropertyValue::String(r)) => l.ends_with(r),
        _ => false,
    }
}

/// Compare if left is in right
fn compare_in(left: &PropertyValue, right: &PropertyValue) -> bool {
    match right {
        PropertyValue::List(items) => items.contains(left),
        _ => false,
    }
}

/// Filter expression for combining property conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterExpression {
    /// Single property condition
    Condition(PropertyCondition),
    /// Logical combination of sub-expressions
    Logical {
        operator: LogicalOperator,
        expressions: Vec<FilterExpression>,
    },
}

impl FilterExpression {
    /// Create a condition expression
    pub fn condition(property: String, operator: ComparisonOperator, value: Option<PropertyValue>) -> Self {
        Self::Condition(PropertyCondition::new(property, operator, value))
    }
    
    /// Create a logical AND expression
    pub fn and(expressions: Vec<FilterExpression>) -> Self {
        Self::Logical {
            operator: LogicalOperator::And,
            expressions,
        }
    }
    
    /// Create a logical OR expression
    pub fn or(expressions: Vec<FilterExpression>) -> Self {
        Self::Logical {
            operator: LogicalOperator::Or,
            expressions,
        }
    }
    
    /// Create a logical NOT expression
    pub fn not(expression: FilterExpression) -> Self {
        Self::Logical {
            operator: LogicalOperator::Not,
            expressions: vec![expression],
        }
    }
    
    /// Evaluate the expression against the given properties
    pub fn evaluate(&self, properties: &Properties) -> bool {
        match self {
            Self::Condition(condition) => condition.evaluate(properties),
            Self::Logical { operator, expressions } => match operator {
                LogicalOperator::And => expressions.iter().all(|expr| expr.evaluate(properties)),
                LogicalOperator::Or => expressions.iter().any(|expr| expr.evaluate(properties)),
                LogicalOperator::Not => {
                    debug_assert!(expressions.len() == 1);
                    !expressions[0].evaluate(properties)
                }
            },
        }
    }
}

/// Sort order for query results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    Ascending,
    Descending,
}

/// Sort specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortSpec {
    /// Property to sort by
    pub property: String,
    /// Sort order
    pub order: SortOrder,
}

impl SortSpec {
    /// Create a new sort specification
    pub fn new(property: String, order: SortOrder) -> Self {
        Self { property, order }
    }
}

/// Pagination settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    /// Offset for pagination (zero-based)
    pub offset: usize,
    /// Limit on number of results
    pub limit: Option<usize>,
}

impl Pagination {
    /// Create a new pagination with the given offset and limit
    pub fn new(offset: usize, limit: Option<usize>) -> Self {
        Self { offset, limit }
    }
    
    /// Create pagination for the first N results
    pub fn first(n: usize) -> Self {
        Self {
            offset: 0,
            limit: Some(n),
        }
    }
}

/// Entity matcher for pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMatcher {
    /// Variable name to bind the matched entity
    pub variable: String,
    /// Labels to match (any of these)
    pub labels: Option<Vec<String>>,
    /// Property filter expression
    pub filter: Option<FilterExpression>,
}

impl EntityMatcher {
    /// Create a new entity matcher
    pub fn new(variable: String, labels: Option<Vec<String>>, filter: Option<FilterExpression>) -> Self {
        Self {
            variable,
            labels,
            filter,
        }
    }
    
    /// Check if an entity matches this matcher
    pub fn matches(&self, entity: &Entity) -> bool {
        // Check labels if specified
        if let Some(ref labels) = self.labels {
            if !labels.iter().any(|label| entity.has_label(label)) {
                return false;
            }
        }
        
        // Check property filter if specified
        if let Some(ref filter) = self.filter {
            if !filter.evaluate(&entity.properties) {
                return false;
            }
        }
        
        true
    }
}

/// Relationship matcher for pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipMatcher {
    /// Variable name to bind the matched relationship
    pub variable: Option<String>,
    /// Relationship types to match (any of these)
    pub types: Option<Vec<String>>,
    /// Direction of the relationship
    pub direction: Direction,
    /// Property filter expression
    pub filter: Option<FilterExpression>,
}

impl RelationshipMatcher {
    /// Create a new relationship matcher
    pub fn new(
        variable: Option<String>,
        types: Option<Vec<String>>,
        direction: Direction,
        filter: Option<FilterExpression>,
    ) -> Self {
        Self {
            variable,
            types,
            direction,
            filter,
        }
    }
    
    /// Check if a relationship matches this matcher
    pub fn matches(&self, relationship: &Relationship) -> bool {
        // Check relationship type if specified
        if let Some(ref types) = self.types {
            if !types.contains(&relationship.rel_type) {
                return false;
            }
        }
        
        // Check property filter if specified
        if let Some(ref filter) = self.filter {
            if !filter.evaluate(&relationship.properties) {
                return false;
            }
        }
        
        true
    }
}

/// Pattern element for building traversal patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternElement {
    /// Entity pattern
    Entity(EntityMatcher),
    /// Relationship pattern
    Relationship(RelationshipMatcher),
}

/// Pattern for graph traversal and matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    /// Sequence of pattern elements
    pub elements: Vec<PatternElement>,
}

impl Pattern {
    /// Create a new empty pattern
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
        }
    }
    
    /// Add an entity matcher to the pattern
    pub fn add_entity(&mut self, matcher: EntityMatcher) -> &mut Self {
        self.elements.push(PatternElement::Entity(matcher));
        self
    }
    
    /// Add a relationship matcher to the pattern
    pub fn add_relationship(&mut self, matcher: RelationshipMatcher) -> &mut Self {
        self.elements.push(PatternElement::Relationship(matcher));
        self
    }
}

impl Default for Pattern {
    fn default() -> Self {
        Self::new()
    }
}

/// Result projection specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Projection {
    /// Return the entire graph match
    All,
    /// Return specific variables
    Variables(Vec<String>),
    /// Return specific properties of variables
    Properties(HashMap<String, Vec<String>>),
    /// Return a custom projection with expressions
    Expression(String),
}

/// Query builder for constructing graph queries
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    /// Patterns to match
    patterns: Vec<Pattern>,
    /// Filter expressions to apply after pattern matching
    filters: Vec<FilterExpression>,
    /// Sorting specifications
    sort: Vec<SortSpec>,
    /// Pagination settings
    pagination: Option<Pagination>,
    /// Projection specification
    projection: Projection,
}

impl QueryBuilder {
    /// Create a new query builder
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
            filters: Vec::new(),
            sort: Vec::new(),
            pagination: None,
            projection: Projection::All,
        }
    }
    
    /// Add a pattern to match
    pub fn pattern(&mut self, pattern: Pattern) -> &mut Self {
        self.patterns.push(pattern);
        self
    }
    
    /// Add a filter expression
    pub fn filter(&mut self, filter: FilterExpression) -> &mut Self {
        self.filters.push(filter);
        self
    }
    
    /// Add a sorting specification
    pub fn sort_by(&mut self, sort: SortSpec) -> &mut Self {
        self.sort.push(sort);
        self
    }
    
    /// Set pagination
    pub fn paginate(&mut self, pagination: Pagination) -> &mut Self {
        self.pagination = Some(pagination);
        self
    }
    
    /// Set projection
    pub fn project(&mut self, projection: Projection) -> &mut Self {
        self.projection = projection;
        self
    }
    
    /// Build the query
    pub fn build(self) -> Query {
        Query {
            patterns: self.patterns,
            filters: self.filters,
            sort: self.sort,
            pagination: self.pagination,
            projection: self.projection,
        }
    }
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Path query for finding paths between entities
#[derive(Debug, Clone)]
pub struct PathQuery {
    /// Source entity ID or matcher
    pub source: EntityMatcher,
    /// Target entity ID or matcher
    pub target: EntityMatcher,
    /// Relationship types to traverse
    pub rel_types: Option<Vec<String>>,
    /// Direction of traversal
    pub direction: Direction,
    /// Maximum path length
    pub max_length: usize,
    /// Filter for intermediate nodes
    pub node_filter: Option<FilterExpression>,
    /// Filter for relationships
    pub rel_filter: Option<FilterExpression>,
}

impl PathQuery {
    /// Create a new path query
    pub fn new(
        source: EntityMatcher,
        target: EntityMatcher,
        rel_types: Option<Vec<String>>,
        direction: Direction,
        max_length: usize,
    ) -> Self {
        Self {
            source,
            target,
            rel_types,
            direction,
            max_length,
            node_filter: None,
            rel_filter: None,
        }
    }
    
    /// Add a filter for intermediate nodes
    pub fn with_node_filter(&mut self, filter: FilterExpression) -> &mut Self {
        self.node_filter = Some(filter);
        self
    }
    
    /// Add a filter for relationships
    pub fn with_rel_filter(&mut self, filter: FilterExpression) -> &mut Self {
        self.rel_filter = Some(filter);
        self
    }
}

/// Subgraph query for extracting a subgraph
#[derive(Debug, Clone)]
pub struct SubgraphQuery {
    /// Center entity matcher
    pub center: EntityMatcher,
    /// Maximum depth of traversal
    pub max_depth: usize,
    /// Relationship types to traverse
    pub rel_types: Option<Vec<String>>,
    /// Direction of traversal
    pub direction: Direction,
    /// Filter for entities to include
    pub entity_filter: Option<FilterExpression>,
    /// Filter for relationships to include
    pub rel_filter: Option<FilterExpression>,
}

impl SubgraphQuery {
    /// Create a new subgraph query
    pub fn new(
        center: EntityMatcher,
        max_depth: usize,
        direction: Direction,
    ) -> Self {
        Self {
            center,
            max_depth,
            rel_types: None,
            direction,
            entity_filter: None,
            rel_filter: None,
        }
    }
    
    /// Add a filter for entities
    pub fn with_entity_filter(&mut self, filter: FilterExpression) -> &mut Self {
        self.entity_filter = Some(filter);
        self
    }
    
    /// Add a filter for relationships
    pub fn with_rel_filter(&mut self, filter: FilterExpression) -> &mut Self {
        self.rel_filter = Some(filter);
        self
    }
    
    /// Set relationship types to traverse
    pub fn with_rel_types(&mut self, types: Vec<String>) -> &mut Self {
        self.rel_types = Some(types);
        self
    }
}

/// Complete query specification
#[derive(Debug, Clone)]
pub struct Query {
    /// Patterns to match
    pub patterns: Vec<Pattern>,
    /// Filter expressions to apply after pattern matching
    pub filters: Vec<FilterExpression>,
    /// Sorting specifications
    pub sort: Vec<SortSpec>,
    /// Pagination settings
    pub pagination: Option<Pagination>,
    /// Projection specification
    pub projection: Projection,
}

/// Query result row with variable bindings
#[derive(Debug, Clone)]
pub struct QueryResultRow {
    /// Entity bindings by variable name
    pub entities: HashMap<String, Entity>,
    /// Relationship bindings by variable name
    pub relationships: HashMap<String, Relationship>,
}

/// Query result set
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Result rows
    pub rows: Vec<QueryResultRow>,
    /// Total count before pagination
    pub total_count: usize,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Query metadata
    pub metadata: HashMap<String, String>,
}

/// Query executor for running queries on graph databases
pub struct QueryExecutor {
    /// Graph database instance
    graph_db: Arc<GraphDb>,
}

impl QueryExecutor {
    /// Create a new query executor
    pub fn new(graph_db: Arc<GraphDb>) -> Self {
        Self { graph_db }
    }
    
    /// Execute a query
    pub fn execute(&self, query: &Query) -> Result<QueryResult, QueryError> {
        let start_time = std::time::Instant::now();
        
        // First, find initial matches from the patterns
        let mut matches = self.match_patterns(&query.patterns)?;
        
        // Apply filters
        if !query.filters.is_empty() {
            matches = self.apply_filters(matches, &query.filters);
        }
        
        // Save total count before pagination
        let total_count = matches.len();
        
        // Apply sorting
        if !query.sort.is_empty() {
            self.sort_results(&mut matches, &query.sort);
        }
        
        // Apply pagination
        let paginated_matches = if let Some(pagination) = &query.pagination {
            let start = pagination.offset;
            let end = if let Some(limit) = pagination.limit {
                pagination.offset + limit
            } else {
                matches.len()
            };
            
            if start >= matches.len() {
                vec![]
            } else {
                matches[start..end.min(matches.len())].to_vec()
            }
        } else {
            matches
        };
        
        // Apply projection
        let result_rows = self.apply_projection(paginated_matches, &query.projection)?;
        
        // Calculate execution time
        let execution_time = start_time.elapsed();
        
        let mut metadata = HashMap::new();
        metadata.insert("pattern_count".to_string(), query.patterns.len().to_string());
        metadata.insert("filter_count".to_string(), query.filters.len().to_string());
        
        Ok(QueryResult {
            rows: result_rows,
            total_count,
            execution_time_ms: execution_time.as_millis() as u64,
            metadata,
        })
    }
    
    /// Execute a path query
    pub fn execute_path_query(&self, query: &PathQuery) -> Result<Vec<Path>, QueryError> {
        // Find matching source entities
        let source_entities = self.find_matching_entities(&query.source)?;
        if source_entities.is_empty() {
            return Ok(Vec::new());
        }
        
        // Find matching target entities
        let target_entities = self.find_matching_entities(&query.target)?;
        if target_entities.is_empty() {
            return Ok(Vec::new());
        }
        
        let mut all_paths = Vec::new();
        
        // Find paths between each source and target
        for source in &source_entities {
            for target in &target_entities {
                let paths = self.graph_db.find_paths(
                    &source.id,
                    &target.id,
                    query.max_length,
                    query.rel_types.clone(),
                    query.direction,
                )?;
                
                // Filter paths based on provided filters
                for path in paths {
                    let mut include_path = true;
                    
                    // Apply node filter if present
                    if let Some(ref node_filter) = query.node_filter {
                        // Skip endpoints when filtering intermediate nodes
                        for i in 1..path.entities.len() - 1 {
                            if !node_filter.evaluate(&path.entities[i].properties) {
                                include_path = false;
                                break;
                            }
                        }
                    }
                    
                    // Apply relationship filter if present
                    if include_path && query.rel_filter.is_some() {
                        let rel_filter = query.rel_filter.as_ref().unwrap();
                        for rel in &path.relationships {
                            if !rel_filter.evaluate(&rel.properties) {
                                include_path = false;
                                break;
                            }
                        }
                    }
                    
                    if include_path {
                        all_paths.push(path);
                    }
                }
            }
        }
        
        Ok(all_paths)
    }
    
    /// Execute a subgraph query
    pub fn execute_subgraph_query(&self, query: &SubgraphQuery) -> Result<(Vec<Entity>, Vec<Relationship>), QueryError> {
        // Find matching center entities
        let center_entities = self.find_matching_entities(&query.center)?;
        if center_entities.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }
        
        let mut all_entities = HashSet::new();
        let mut all_relationships = HashSet::new();
        
        // Extract subgraph for each center entity
        for center in &center_entities {
            let (entities, relationships) = self.graph_db.export_subgraph(
                &center.id,
                query.max_depth,
            )?;
            
            // Apply entity filter if present
            let filtered_entities = if let Some(ref filter) = query.entity_filter {
                entities.into_iter()
                    .filter(|e| filter.evaluate(&e.properties))
                    .collect::<Vec<_>>()
            } else {
                entities
            };
            
            // Apply relationship filter if present
            let filtered_relationships = if let Some(ref filter) = query.rel_filter {
                relationships.into_iter()
                    .filter(|r| filter.evaluate(&r.properties))
                    .collect::<Vec<_>>()
            } else {
                relationships
            };
            
            // Add to results, avoiding duplicates
            for entity in filtered_entities {
                all_entities.insert(entity.id.clone());
            }
            
            for rel in filtered_relationships {
                all_relationships.insert(rel.id.clone());
            }
        }
        
        // Retrieve complete entity and relationship objects
        let result_entities = all_entities.into_iter()
            .filter_map(|id| match self.graph_db.get_entity(&id) {
                Ok(e) => Some(e),
                Err(_) => None,
            })
            .collect();
            
        let result_relationships = all_relationships.into_iter()
            .filter_map(|id| match self.graph_db.get_relationship(&id) {
                Ok(r) => Some(r),
                Err(_) => None,
            })
            .collect();
        
        Ok((result_entities, result_relationships))
    }
    
    // Helper methods
    
    /// Find entities matching an entity matcher
    fn find_matching_entities(&self, matcher: &EntityMatcher) -> Result<Vec<Entity>, QueryError> {
        let mut candidates = Vec::new();
        
        // If labels specified, use them for initial filtering
        if let Some(ref labels) = matcher.labels {
            for label in labels {
                let mut label_entities = self.graph_db.find_entities_by_label(label);
                candidates.append(&mut label_entities);
            }
        } else {
            // No labels specified, must scan all entities
            let mut all_entities = Vec::new();
            for i in 0..self.graph_db.entity_count() {
                if let Ok(entity) = self.graph_db.get_entity(&i.to_string()) {
                    all_entities.push(entity);
                }
            }
            candidates = all_entities;
        }
        
        // Apply property filter if specified
        if let Some(ref filter) = matcher.filter {
            candidates.retain(|entity| filter.evaluate(&entity.properties));
        }
        
        Ok(candidates)
    }
    
    /// Match patterns against the graph database
    fn match_patterns(&self, patterns: &[Pattern]) -> Result<Vec<QueryResultRow>, QueryError> {
        if patterns.is_empty() {
            return Err(QueryError::InvalidQuery("No patterns specified".to_string()));
        }
        
        let mut results = Vec::new();
        
        for pattern in patterns {
            let pattern_results = self.match_single_pattern(pattern)?;
            results.extend(pattern_results);
        }
        
        Ok(results)
    }
    
    /// Match a single pattern against the graph database
    fn match_single_pattern(&self, pattern: &Pattern) -> Result<Vec<QueryResultRow>, QueryError> {
        if pattern.elements.is_empty() {
            return Ok(Vec::new());
        }
        
        // Start with initial bindings based on the first element
        let mut bindings = match &pattern.elements[0] {
            PatternElement::Entity(matcher) => {
                let entities = self.find_matching_entities(matcher)?;
                
                entities.into_iter().map(|entity| {
                    let mut row = QueryResultRow {
                        entities: HashMap::new(),
                        relationships: HashMap::new(),
                    };
                    row.entities.insert(matcher.variable.clone(), entity);
                    row
                }).collect::<Vec<_>>()
            },
            PatternElement::Relationship(_) => {
                return Err(QueryError::InvalidQuery("Pattern cannot start with a relationship".to_string()));
            }
        };
        
        if bindings.is_empty() {
            return Ok(Vec::new());
        }
        
        // Iterate through the remaining elements
        for i in 1..pattern.elements.len() {
            bindings = self.extend_bindings(bindings, &pattern.elements[i])?;
            
            if bindings.is_empty() {
                // No matches, we can stop early
                return Ok(Vec::new());
            }
        }
        
        Ok(bindings)
    }
    
    /// Extend bindings with a new pattern element
    fn extend_bindings(&self, bindings: Vec<QueryResultRow>, element: &PatternElement) -> Result<Vec<QueryResultRow>, QueryError> {
        let mut new_bindings = Vec::new();
        
        match element {
            PatternElement::Entity(matcher) => {
                // Find bound entity in previous element
                for mut row in bindings {
                    // In a real implementation, we would use the relationship from the previous element
                    // to find connected entities, but for simplicity, we'll just find all matching entities
                    let entities = self.find_matching_entities(matcher)?;
                    
                    for entity in entities {
                        let mut new_row = row.clone();
                        new_row.entities.insert(matcher.variable.clone(), entity);
                        new_bindings.push(new_row);
                    }
                }
            },
            PatternElement::Relationship(matcher) => {
                // Find relationships between previously bound entities
                // This is a simplified implementation
                for row in bindings {
                    // In a real implementation, we would use the entities from previous elements
                    // to find relationships between them based on the matcher
                    new_bindings.push(row);
                }
            }
        }
        
        Ok(new_bindings)
    }
    
    /// Apply filter expressions to result rows
    fn apply_filters(&self, rows: Vec<QueryResultRow>, filters: &[FilterExpression]) -> Vec<QueryResultRow> {
        rows.into_iter().filter(|row| {
            filters.iter().all(|filter| {
                // In a real implementation, we would evaluate the filter against the entities
                // and relationships in the row, but for simplicity we'll just return true
                true
            })
        }).collect()
    }
    
    /// Sort result rows
    fn sort_results(&self, rows: &mut Vec<QueryResultRow>, sort_specs: &[SortSpec]) {
        // In a real implementation, we would sort based on the sort specifications
    }
    
    /// Apply projection to result rows
    fn apply_projection(&self, rows: Vec<QueryResultRow>, projection: &Projection) -> Result<Vec<QueryResultRow>, QueryError> {
        match projection {
            Projection::All => Ok(rows),
            Projection::Variables(variables) => {
                let projected_rows = rows.into_iter().map(|mut row| {
                    let mut projected_row = QueryResultRow {
                        entities: HashMap::new(),
                        relationships: HashMap::new(),
                    };
                    
                    for var in variables {
                        if let Some(entity) = row.entities.remove(var) {
                            projected_row.entities.insert(var.clone(), entity);
                        }
                        
                        if let Some(rel) = row.relationships.remove(var) {
                            projected_row.relationships.insert(var.clone(), rel);
                        }
                    }
                    
                    projected_row
                }).collect();
                
                Ok(projected_rows)
            },
            Projection::Properties(prop_map) => {
                // In a real implementation, we would project only the specified properties
                Ok(rows)
            },
            Projection::Expression(_) => {
                // In a real implementation, we would evaluate the expression
                Ok(rows)
            },
        }
    }
}

/// Query result formatter for converting results to different formats
pub struct QueryFormatter;

impl QueryFormatter {
    /// Format a query result as JSON
    pub fn to_json(result: &QueryResult) -> Result<String, QueryError> {
        let json = serde_json::to_string_pretty(
            &result.rows.iter().map(|row| {
                let mut map = serde_json::Map::new();
                
                for (var, entity) in &row.entities {
                    map.insert(var.clone(), serde_json::to_value(entity).unwrap());
                }
                
                for (var, rel) in &row.relationships {
                    map.insert(var.clone(), serde_json::to_value(rel).unwrap());
                }
                
                serde_json::Value::Object(map)
            }).collect::<Vec<_>>()
        ).map_err(|e| QueryError::SerializationError(e.to_string()))?;
        
        Ok(json)
    }
    
    /// Format path results as JSON
    pub fn paths_to_json(paths: &[Path]) -> Result<String, QueryError> {
        let json = serde_json::to_string_pretty(paths)
            .map_err(|e| QueryError::SerializationError(e.to_string()))?;
            
        Ok(json)
    }
    
    /// Format a subgraph as JSON
    pub fn subgraph_to_json(
        entities: &[Entity],
        relationships: &[Relationship],
    ) -> Result<String, QueryError> {
        let mut map = serde_json::Map::new();
        
        map.insert(
            "entities".to_string(),
            serde_json::to_value(entities).map_err(|e| QueryError::SerializationError(e.to_string()))?,
        );
        
        map.insert(
            "relationships".to_string(),
            serde_json::to_value(relationships).map_err(|e| QueryError::SerializationError(e.to_string()))?,
        );
        
        let json = serde_json::to_string_pretty(&serde_json::Value::Object(map))
            .map_err(|e| QueryError::SerializationError(e.to_string()))?;
            
        Ok(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    
    fn create_test_graph() -> Arc<GraphDb> {
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
        
        // Create relationship
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
        
        Arc::new(graph)
    }
    
    #[test]
    fn test_property_conditions() {
        let mut props = HashMap::new();
        props.insert("name".to_string(), PropertyValue::String("Alice".to_string()));
        props.insert("age".to_string(), PropertyValue::Integer(30));
        
        // Test Equal
        let cond1 = PropertyCondition::new(
            "name".to_string(),
            ComparisonOperator::Equal,
            Some(PropertyValue::String("Alice".to_string())),
        );
        assert!(cond1.evaluate(&props));
        
        // Test NotEqual
        let cond2 = PropertyCondition::new(
            "name".to_string(),
            ComparisonOperator::NotEqual,
            Some(PropertyValue::String("Bob".to_string())),
        );
        assert!(cond2.evaluate(&props));
        
        // Test GreaterThan
        let cond3 = PropertyCondition::new(
            "age".to_string(),
            ComparisonOperator::GreaterThan,
            Some(PropertyValue::Integer(25)),
        );
        assert!(cond3.evaluate(&props));
        
        // Test LessThan
        let cond4 = PropertyCondition::new(
            "age".to_string(),
            ComparisonOperator::LessThan,
            Some(PropertyValue::Integer(40)),
        );
        assert!(cond4.evaluate(&props));
        
        // Test Exists
        let cond5 = PropertyCondition::new(
            "age".to_string(),
            ComparisonOperator::Exists,
            None,
        );
        assert!(cond5.evaluate(&props));
        
        // Test NotExists
        let cond6 = PropertyCondition::new(
            "location".to_string(),
            ComparisonOperator::NotExists,
            None,
        );
        assert!(cond6.evaluate(&props));
    }
    
    #[test]
    fn test_filter_expressions() {
        let mut props = HashMap::new();
        props.insert("name".to_string(), PropertyValue::String("Alice".to_string()));
        props.insert("age".to_string(), PropertyValue::Integer(30));
        
        // Test AND
        let expr1 = FilterExpression::and(vec![
            FilterExpression::condition(
                "name".to_string(),
                ComparisonOperator::Equal,
                Some(PropertyValue::String("Alice".to_string())),
            ),
            FilterExpression::condition(
                "age".to_string(),
                ComparisonOperator::GreaterThan,
                Some(PropertyValue::Integer(25)),
            ),
        ]);
        assert!(expr1.evaluate(&props));
        
        // Test OR
        let expr2 = FilterExpression::or(vec![
            FilterExpression::condition(
                "name".to_string(),
                ComparisonOperator::Equal,
                Some(PropertyValue::String("Bob".to_string())),
            ),
            FilterExpression::condition(
                "age".to_string(),
                ComparisonOperator::GreaterThan,
                Some(PropertyValue::Integer(25)),
            ),
        ]);
        assert!(expr2.evaluate(&props));
        
        // Test NOT
        let expr3 = FilterExpression::not(
            FilterExpression::condition(
                "name".to_string(),
                ComparisonOperator::Equal,
                Some(PropertyValue::String("Bob".to_string())),
            ),
        );
        assert!(expr3.evaluate(&props));
    }
    
    #[test]
    fn test_entity_matcher() {
        let graph = create_test_graph();
        
        // Find Alice
        let alice = graph.find_entities_by_property(
            "name",
            &PropertyValue::String("Alice".to_string()),
        )[0].clone();
        
        // Create matcher for Person with name Alice
        let matcher = EntityMatcher::new(
            "p".to_string(),
            Some(vec!["Person".to_string()]),
            Some(FilterExpression::condition(
                "name".to_string(),
                ComparisonOperator::Equal,
                Some(PropertyValue::String("Alice".to_string())),
            )),
        );
        
        assert!(matcher.matches(&alice));
    }
    
    #[test]
    fn test_query_builder() {
        let graph = create_test_graph();
        let executor = QueryExecutor::new(graph); 
        
        // Create a pattern
        let mut pattern = Pattern::new();
        pattern.add_entity(EntityMatcher::new(
            "p".to_string(),
            Some(vec!["Person".to_string()]),
            Some(FilterExpression::condition(
                "name".to_string(),
                ComparisonOperator::Equal,
                Some(PropertyValue::String("Alice".to_string())),
            )),
        ));
        
        // Build a query
        let query = QueryBuilder::new()
            .pattern(pattern)
            .paginate(Pagination::first(10))
            .build();
        
        // Execute the query
        let result = executor.execute(&query).unwrap();
        
        // Should have found Alice
        assert_eq!(result.rows.len(), 1);
    }
}