//! Policy Engine Module
//!
//! Implements a comprehensive policy engine with rule evaluation, content 
//! classification, and policy enforcement mechanisms. Works in coordination
//! with the content filtering system to enforce safety policies.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::{Arc, RwLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock as AsyncRwLock;
use async_trait::async_trait;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::filters::{
    ContentType, ContentCategory, ClassificationResult, FilterStrategy, 
    FilterResult, FilterError, ContentFilter
};

/// Policy engine error
#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("Invalid policy: {0}")]
    InvalidPolicy(String),
    
    #[error("Rule evaluation failed: {0}")]
    RuleEvaluationFailed(String),
    
    #[error("Policy not found: {0}")]
    PolicyNotFound(String),
    
    #[error("Rule not found: {0}")]
    RuleNotFound(String),
    
    #[error("Policy enforcement failed: {0}")]
    EnforcementFailed(String),
    
    #[error("Filter error: {0}")]
    FilterError(#[from] FilterError),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Policy scope enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolicyScope {
    /// Global policy (applies to all content)
    Global,
    /// Session-specific policy
    Session,
    /// User-specific policy
    User,
    /// Content type specific policy
    ContentType,
    /// Channel or source specific policy
    Source,
}

impl fmt::Display for PolicyScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Global => write!(f, "global"),
            Self::Session => write!(f, "session"),
            Self::User => write!(f, "user"),
            Self::ContentType => write!(f, "content_type"),
            Self::Source => write!(f, "source"),
        }
    }
}

/// Policy action to take when rules match
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolicyAction {
    /// Allow content
    Allow,
    /// Block content
    Block,
    /// Filter content using specified strategy
    Filter(FilterStrategy),
    /// Require additional review
    RequireReview,
    /// Log but take no action
    Log,
    /// Escalate to higher authority
    Escalate,
}

impl fmt::Display for PolicyAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Allow => write!(f, "allow"),
            Self::Block => write!(f, "block"),
            Self::Filter(strategy) => write!(f, "filter({})", strategy),
            Self::RequireReview => write!(f, "require_review"),
            Self::Log => write!(f, "log"),
            Self::Escalate => write!(f, "escalate"),
        }
    }
}

/// Condition operator for rule conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConditionOperator {
    /// Equal
    Equal,
    /// Not equal
    NotEqual,
    /// Greater than
    GreaterThan,
    /// Greater than or equal
    GreaterThanOrEqual,
    /// Less than
    LessThan,
    /// Less than or equal
    LessThanOrEqual,
    /// Contains
    Contains,
    /// Starts with
    StartsWith,
    /// Ends with
    EndsWith,
    /// In a list
    In,
    /// Not in a list
    NotIn,
    /// Regex match
    RegexMatch,
}

impl fmt::Display for ConditionOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Equal => write!(f, "="),
            Self::NotEqual => write!(f, "!="),
            Self::GreaterThan => write!(f, ">"),
            Self::GreaterThanOrEqual => write!(f, ">="),
            Self::LessThan => write!(f, "<"),
            Self::LessThanOrEqual => write!(f, "<="),
            Self::Contains => write!(f, "contains"),
            Self::StartsWith => write!(f, "startsWith"),
            Self::EndsWith => write!(f, "endsWith"),
            Self::In => write!(f, "in"),
            Self::NotIn => write!(f, "notIn"),
            Self::RegexMatch => write!(f, "regexMatch"),
        }
    }
}

/// Logical operator for combining conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LogicalOperator {
    /// AND - all conditions must be true
    And,
    /// OR - at least one condition must be true
    Or,
    /// NOT - negate the condition
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

/// Rule condition value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConditionValue {
    /// String value
    String(String),
    /// Numeric value
    Number(f64),
    /// Boolean value
    Boolean(bool),
    /// List of string values
    StringList(Vec<String>),
    /// List of numeric values
    NumberList(Vec<f64>),
    /// Map of string keys to string values
    Map(HashMap<String, String>),
}

impl fmt::Display for ConditionValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(s) => write!(f, "\"{}\"", s),
            Self::Number(n) => write!(f, "{}", n),
            Self::Boolean(b) => write!(f, "{}", b),
            Self::StringList(list) => {
                write!(f, "[")?;
                for (i, item) in list.iter().enumerate() {
                    if i > 0 { write!(f, ", ")? }
                    write!(f, "\"{}\"", item)?;
                }
                write!(f, "]")
            },
            Self::NumberList(list) => {
                write!(f, "[")?;
                for (i, item) in list.iter().enumerate() {
                    if i > 0 { write!(f, ", ")? }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            },
            Self::Map(map) => {
                write!(f, "{{")?;
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 { write!(f, ", ")? }
                    write!(f, "\"{}\": \"{}\"", k, v)?;
                }
                write!(f, "}}")
            },
        }
    }
}

/// Simple rule condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleCondition {
    /// Field to check
    pub field: String,
    /// Operator for the comparison
    pub operator: ConditionOperator,
    /// Value to compare against
    pub value: ConditionValue,
}

impl SimpleCondition {
    /// Create a new simple condition
    pub fn new(field: String, operator: ConditionOperator, value: ConditionValue) -> Self {
        Self {
            field,
            operator,
            value,
        }
    }
    
    /// Evaluate the condition against a context
    pub fn evaluate(&self, context: &HashMap<String, ConditionValue>) -> Result<bool, PolicyError> {
        if let Some(context_value) = context.get(&self.field) {
            match (self.operator, context_value, &self.value) {
                // Equal
                (ConditionOperator::Equal, ConditionValue::String(cv), ConditionValue::String(v)) => {
                    Ok(cv == v)
                },
                (ConditionOperator::Equal, ConditionValue::Number(cv), ConditionValue::Number(v)) => {
                    Ok((cv - v).abs() < std::f64::EPSILON)
                },
                (ConditionOperator::Equal, ConditionValue::Boolean(cv), ConditionValue::Boolean(v)) => {
                    Ok(cv == v)
                },
                
                // Not Equal
                (ConditionOperator::NotEqual, ConditionValue::String(cv), ConditionValue::String(v)) => {
                    Ok(cv != v)
                },
                (ConditionOperator::NotEqual, ConditionValue::Number(cv), ConditionValue::Number(v)) => {
                    Ok((cv - v).abs() >= std::f64::EPSILON)
                },
                (ConditionOperator::NotEqual, ConditionValue::Boolean(cv), ConditionValue::Boolean(v)) => {
                    Ok(cv != v)
                },
                
                // Greater Than
                (ConditionOperator::GreaterThan, ConditionValue::Number(cv), ConditionValue::Number(v)) => {
                    Ok(cv > v)
                },
                (ConditionOperator::GreaterThan, ConditionValue::String(cv), ConditionValue::String(v)) => {
                    Ok(cv > v)
                },
                
                // Greater Than or Equal
                (ConditionOperator::GreaterThanOrEqual, ConditionValue::Number(cv), ConditionValue::Number(v)) => {
                    Ok(cv >= v)
                },
                (ConditionOperator::GreaterThanOrEqual, ConditionValue::String(cv), ConditionValue::String(v)) => {
                    Ok(cv >= v)
                },
                
                // Less Than
                (ConditionOperator::LessThan, ConditionValue::Number(cv), ConditionValue::Number(v)) => {
                    Ok(cv < v)
                },
                (ConditionOperator::LessThan, ConditionValue::String(cv), ConditionValue::String(v)) => {
                    Ok(cv < v)
                },
                
                // Less Than or Equal
                (ConditionOperator::LessThanOrEqual, ConditionValue::Number(cv), ConditionValue::Number(v)) => {
                    Ok(cv <= v)
                },
                (ConditionOperator::LessThanOrEqual, ConditionValue::String(cv), ConditionValue::String(v)) => {
                    Ok(cv <= v)
                },
                
                // Contains
                (ConditionOperator::Contains, ConditionValue::String(cv), ConditionValue::String(v)) => {
                    Ok(cv.contains(v))
                },
                
                // Starts With
                (ConditionOperator::StartsWith, ConditionValue::String(cv), ConditionValue::String(v)) => {
                    Ok(cv.starts_with(v))
                },
                
                // Ends With
                (ConditionOperator::EndsWith, ConditionValue::String(cv), ConditionValue::String(v)) => {
                    Ok(cv.ends_with(v))
                },
                
                // In
                (ConditionOperator::In, ConditionValue::String(cv), ConditionValue::StringList(v)) => {
                    Ok(v.contains(cv))
                },
                (ConditionOperator::In, ConditionValue::Number(cv), ConditionValue::NumberList(v)) => {
                    Ok(v.iter().any(|x| (x - cv).abs() < std::f64::EPSILON))
                },
                
                // Not In
                (ConditionOperator::NotIn, ConditionValue::String(cv), ConditionValue::StringList(v)) => {
                    Ok(!v.contains(cv))
                },
                (ConditionOperator::NotIn, ConditionValue::Number(cv), ConditionValue::NumberList(v)) => {
                    Ok(!v.iter().any(|x| (x - cv).abs() < std::f64::EPSILON))
                },
                
                // Regex Match
                (ConditionOperator::RegexMatch, ConditionValue::String(cv), ConditionValue::String(v)) => {
                    match regex::Regex::new(v) {
                        Ok(re) => Ok(re.is_match(cv)),
                        Err(e) => Err(PolicyError::RuleEvaluationFailed(
                            format!("Invalid regex pattern: {}", e)
                        )),
                    }
                },
                
                // Unsupported combinations
                _ => Err(PolicyError::RuleEvaluationFailed(
                    format!("Unsupported operator/value combination: {:?} {:?} {:?}", 
                        self.operator, context_value, self.value)
                )),
            }
        } else {
            // Field not found in context
            Ok(false)
        }
    }
}

/// Composite rule condition (logical combination of conditions)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeCondition {
    /// Logical operator
    pub operator: LogicalOperator,
    /// List of conditions
    pub conditions: Vec<Condition>,
}

impl CompositeCondition {
    /// Create a new composite condition
    pub fn new(operator: LogicalOperator, conditions: Vec<Condition>) -> Self {
        Self {
            operator,
            conditions,
        }
    }
    
    /// Create an AND condition
    pub fn and(conditions: Vec<Condition>) -> Self {
        Self::new(LogicalOperator::And, conditions)
    }
    
    /// Create an OR condition
    pub fn or(conditions: Vec<Condition>) -> Self {
        Self::new(LogicalOperator::Or, conditions)
    }
    
    /// Create a NOT condition
    pub fn not(condition: Condition) -> Self {
        Self::new(LogicalOperator::Not, vec![condition])
    }
    
    /// Evaluate the composite condition against a context
    pub fn evaluate(&self, context: &HashMap<String, ConditionValue>) -> Result<bool, PolicyError> {
        match self.operator {
            LogicalOperator::And => {
                for condition in &self.conditions {
                    if !condition.evaluate(context)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            },
            LogicalOperator::Or => {
                for condition in &self.conditions {
                    if condition.evaluate(context)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            },
            LogicalOperator::Not => {
                if self.conditions.len() != 1 {
                    return Err(PolicyError::RuleEvaluationFailed(
                        "NOT operator requires exactly one condition".to_string()
                    ));
                }
                Ok(!self.conditions[0].evaluate(context)?)
            },
        }
    }
}

/// Rule condition (simple or composite)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Condition {
    /// Simple condition
    Simple(SimpleCondition),
    /// Composite condition
    Composite(CompositeCondition),
}

impl Condition {
    /// Evaluate the condition against a context
    pub fn evaluate(&self, context: &HashMap<String, ConditionValue>) -> Result<bool, PolicyError> {
        match self {
            Self::Simple(condition) => condition.evaluate(context),
            Self::Composite(condition) => condition.evaluate(context),
        }
    }
}

/// Policy rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Rule ID
    pub id: String,
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: String,
    /// Rule condition
    pub condition: Condition,
    /// Rule action
    pub action: PolicyAction,
    /// Rule priority (higher number = higher priority)
    pub priority: i32,
    /// Whether the rule is enabled
    pub enabled: bool,
    /// Rule tags
    pub tags: HashSet<String>,
    /// Rule version
    pub version: String,
    /// Rule creation timestamp
    pub created_at: u64,
    /// Rule last updated timestamp
    pub updated_at: u64,
}

impl Rule {
    /// Create a new rule
    pub fn new(
        id: Option<String>,
        name: String,
        description: String,
        condition: Condition,
        action: PolicyAction,
        priority: i32,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        Self {
            id: id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            name,
            description,
            condition,
            action,
            priority,
            enabled: true,
            tags: HashSet::new(),
            version: "1.0.0".to_string(),
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Check if the rule matches the given context
    pub fn matches(&self, context: &HashMap<String, ConditionValue>) -> Result<bool, PolicyError> {
        if !self.enabled {
            return Ok(false);
        }
        
        self.condition.evaluate(context)
    }
    
    /// Add a tag to the rule
    pub fn add_tag(&mut self, tag: String) -> &mut Self {
        self.tags.insert(tag);
        self
    }
    
    /// Update the rule's version
    pub fn update_version(&mut self, version: String) -> &mut Self {
        self.version = version;
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self
    }
}

/// Policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Policy ID
    pub id: String,
    /// Policy name
    pub name: String,
    /// Policy description
    pub description: String,
    /// Policy scope
    pub scope: PolicyScope,
    /// Policy scope ID (for scoped policies)
    pub scope_id: Option<String>,
    /// List of rule IDs in this policy
    pub rule_ids: Vec<String>,
    /// Whether the policy is enabled
    pub enabled: bool,
    /// Policy priority (higher number = higher priority)
    pub priority: i32,
    /// Policy version
    pub version: String,
    /// Policy creation timestamp
    pub created_at: u64,
    /// Policy last updated timestamp
    pub updated_at: u64,
}

impl Policy {
    /// Create a new policy
    pub fn new(
        id: Option<String>,
        name: String,
        description: String,
        scope: PolicyScope,
        scope_id: Option<String>,
        priority: i32,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        Self {
            id: id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            name,
            description,
            scope,
            scope_id,
            rule_ids: Vec::new(),
            enabled: true,
            priority,
            version: "1.0.0".to_string(),
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Add a rule to the policy
    pub fn add_rule(&mut self, rule_id: String) -> &mut Self {
        if !self.rule_ids.contains(&rule_id) {
            self.rule_ids.push(rule_id);
            self.updated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
        }
        self
    }
    
    /// Remove a rule from the policy
    pub fn remove_rule(&mut self, rule_id: &str) -> &mut Self {
        self.rule_ids.retain(|id| id != rule_id);
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self
    }
    
    /// Update the policy's version
    pub fn update_version(&mut self, version: String) -> &mut Self {
        self.version = version;
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self
    }
    
    /// Set policy enabled status
    pub fn set_enabled(&mut self, enabled: bool) -> &mut Self {
        self.enabled = enabled;
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self
    }
}

/// Evaluation context for policy evaluation
#[derive(Debug, Clone)]
pub struct EvaluationContext {
    /// Content to evaluate
    pub content: String,
    /// Content type
    pub content_type: ContentType,
    /// Classification result if available
    pub classification: Option<ClassificationResult>,
    /// User ID if available
    pub user_id: Option<String>,
    /// Session ID if available
    pub session_id: Option<String>,
    /// Source ID if available
    pub source_id: Option<String>,
    /// Additional context data
    pub context_data: HashMap<String, ConditionValue>,
}

impl EvaluationContext {
    /// Create a new evaluation context
    pub fn new(content: String, content_type: ContentType) -> Self {
        Self {
            content,
            content_type,
            classification: None,
            user_id: None,
            session_id: None,
            source_id: None,
            context_data: HashMap::new(),
        }
    }
    
    /// Set classification result
    pub fn with_classification(mut self, classification: ClassificationResult) -> Self {
        self.classification = Some(classification);
        self
    }
    
    /// Set user ID
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }
    
    /// Set session ID
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }
    
    /// Set source ID
    pub fn with_source_id(mut self, source_id: String) -> Self {
        self.source_id = Some(source_id);
        self
    }
    
    /// Add context data
    pub fn with_context(mut self, key: String, value: ConditionValue) -> Self {
        self.context_data.insert(key, value);
        self
    }
    
    /// Build a full context map for rule evaluation
    pub fn build_context_map(&self) -> HashMap<String, ConditionValue> {
        let mut context = self.context_data.clone();
        
        // Add standard context fields
        context.insert("content".to_string(), ConditionValue::String(self.content.clone()));
        context.insert("content_type".to_string(), ConditionValue::String(self.content_type.to_string()));
        context.insert("content_length".to_string(), ConditionValue::Number(self.content.len() as f64));
        
        if let Some(user_id) = &self.user_id {
            context.insert("user_id".to_string(), ConditionValue::String(user_id.clone()));
        }
        
        if let Some(session_id) = &self.session_id {
            context.insert("session_id".to_string(), ConditionValue::String(session_id.clone()));
        }
        
        if let Some(source_id) = &self.source_id {
            context.insert("source_id".to_string(), ConditionValue::String(source_id.clone()));
        }
        
        // Add classification data if available
        if let Some(classification) = &self.classification {
            // Add primary category
            context.insert(
                "primary_category".to_string(), 
                ConditionValue::String(classification.primary_category.to_string())
            );
            
            // Add all category confidences
            for (category, confidence) in &classification.categories {
                context.insert(
                    format!("category_{}_confidence", category), 
                    ConditionValue::Number(*confidence as f64)
                );
            }
            
            // Add metadata
            for (key, value) in &classification.metadata {
                context.insert(
                    format!("classification_{}", key), 
                    ConditionValue::String(value.clone())
                );
            }
        }
        
        context
    }
}

/// Evaluation result for policy evaluation
#[derive(Debug, Clone)]
pub struct PolicyEvaluationResult {
    /// Final action to take
    pub action: PolicyAction,
    /// Matching policies
    pub matching_policies: Vec<Policy>,
    /// Matching rules
    pub matching_rules: Vec<Rule>,
    /// Evaluation context
    pub context: EvaluationContext,
    /// Filter result if available
    pub filter_result: Option<FilterResult>,
    /// Explanation for the decision
    pub explanation: String,
    /// Evaluation time in milliseconds
    pub evaluation_time_ms: u64,
    /// Additional result metadata
    pub metadata: HashMap<String, String>,
}

impl PolicyEvaluationResult {
    /// Create a new policy evaluation result
    pub fn new(
        action: PolicyAction,
        matching_policies: Vec<Policy>,
        matching_rules: Vec<Rule>,
        context: EvaluationContext,
        explanation: String,
        evaluation_time_ms: u64,
    ) -> Self {
        Self {
            action,
            matching_policies,
            matching_rules,
            context,
            filter_result: None,
            explanation,
            evaluation_time_ms,
            metadata: HashMap::new(),
        }
    }
    
    /// Add filter result
    pub fn with_filter_result(mut self, filter_result: FilterResult) -> Self {
        self.filter_result = Some(filter_result);
        self
    }
    
    /// Add metadata
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
    
    /// Check if action is to block content
    pub fn is_blocked(&self) -> bool {
        matches!(self.action, PolicyAction::Block) || 
        (matches!(self.action, PolicyAction::Filter(strategy)) && 
         strategy == FilterStrategy::Block)
    }
    
    /// Check if action requires human review
    pub fn requires_review(&self) -> bool {
        matches!(self.action, PolicyAction::RequireReview)
    }
}

/// Policy engine implementation
pub struct PolicyEngine {
    /// Content filter
    filter: Arc<ContentFilter>,
    /// Map of rules by ID
    rules: RwLock<HashMap<String, Rule>>,
    /// Map of policies by ID
    policies: RwLock<HashMap<String, Policy>>,
    /// Map of policies by scope
    policies_by_scope: RwLock<HashMap<PolicyScope, HashMap<String, HashSet<String>>>>,
    /// Global policies (applied to all content)
    global_policies: RwLock<HashSet<String>>,
}

impl PolicyEngine {
    /// Create a new policy engine
    pub fn new(filter: Arc<ContentFilter>) -> Self {
        Self {
            filter,
            rules: RwLock::new(HashMap::new()),
            policies: RwLock::new(HashMap::new()),
            policies_by_scope: RwLock::new(HashMap::new()),
            global_policies: RwLock::new(HashSet::new()),
        }
    }
    
    /// Create a default policy engine
    pub fn default_engine() -> Result<Self, PolicyError> {
        let filter = ContentFilter::default_filter()
            .map_err(PolicyError::FilterError)?;
            
        let engine = Self::new(Arc::new(filter));
        
        // Create default rules and policies
        engine.initialize_defaults()?;
        
        Ok(engine)
    }
    
    /// Initialize default rules and policies
    fn initialize_defaults(&self) -> Result<(), PolicyError> {
        // Create default rules
        
        // Rule to block malicious content
        let malicious_rule = Rule::new(
            Some("default-malicious-rule".to_string()),
            "Block Malicious Content".to_string(),
            "Blocks any content classified as malicious".to_string(),
            Condition::Simple(SimpleCondition::new(
                "primary_category".to_string(),
                ConditionOperator::Equal,
                ConditionValue::String("malicious".to_string()),
            )),
            PolicyAction::Block,
            100,
        );
        
        // Rule to filter explicit content
        let explicit_rule = Rule::new(
            Some("default-explicit-rule".to_string()),
            "Filter Explicit Content".to_string(),
            "Filters any content classified as explicit".to_string(),
            Condition::Simple(SimpleCondition::new(
                "primary_category".to_string(),
                ConditionOperator::Equal,
                ConditionValue::String("explicit".to_string()),
            )),
            PolicyAction::Filter(FilterStrategy::Replace),
            90,
        );
        
        // Rule to mark security content
        let security_rule = Rule::new(
            Some("default-security-rule".to_string()),
            "Mark Security Content".to_string(),
            "Marks any content with security concerns".to_string(),
            Condition::Simple(SimpleCondition::new(
                "primary_category".to_string(),
                ConditionOperator::Equal,
                ConditionValue::String("security_concern".to_string()),
            )),
            PolicyAction::Filter(FilterStrategy::Mark),
            80,
        );
                
        // Rule to sanitize HTML content
        let html_rule = Rule::new(
            Some("default-html-rule".to_string()),
            "Sanitize HTML Content".to_string(),
            "Sanitizes any HTML content".to_string(),
            Condition::Simple(SimpleCondition::new(
                "content_type".to_string(),
                ConditionOperator::Equal,
                ConditionValue::String("html".to_string()),
            )),
            PolicyAction::Filter(FilterStrategy::Sanitize),
            70,
        );
        
        // Default policy combining these rules
        let default_policy = Policy::new(
            Some("default-safety-policy".to_string()),
            "Default Safety Policy".to_string(),
            "Default policy for enforcing content safety".to_string(),
            PolicyScope::Global,
            None,
            100,
        );
        
        // Add rules
        self.add_rule(malicious_rule)?;
        self.add_rule(explicit_rule)?;
        self.add_rule(security_rule)?;
        self.add_rule(html_rule)?;
        
        // Add rules to policy
        let mut policy = default_policy;
        policy.add_rule("default-malicious-rule".to_string());
        policy.add_rule("default-explicit-rule".to_string());
        policy.add_rule("default-security-rule".to_string());
        policy.add_rule("default-html-rule".to_string());
        
        // Add policy
        self.add_policy(policy)?;
        
        Ok(())
    }
    
    /// Add a rule
    pub fn add_rule(&self, rule: Rule) -> Result<(), PolicyError> {
        let mut rules = self.rules.write().unwrap();
        rules.insert(rule.id.clone(), rule);
        Ok(())
    }
    
    /// Get a rule by ID
    pub fn get_rule(&self, rule_id: &str) -> Result<Rule, PolicyError> {
        let rules = self.rules.read().unwrap();
        rules.get(rule_id)
            .cloned()
            .ok_or_else(|| PolicyError::RuleNotFound(rule_id.to_string()))
    }
    
    /// Update a rule
    pub fn update_rule(&self, rule: Rule) -> Result<(), PolicyError> {
        let mut rules = self.rules.write().unwrap();
        if !rules.contains_key(&rule.id) {
            return Err(PolicyError::RuleNotFound(rule.id.clone()));
        }
        
        rules.insert(rule.id.clone(), rule);
        Ok(())
    }
    
    /// Remove a rule
    pub fn remove_rule(&self, rule_id: &str) -> Result<(), PolicyError> {
        let mut rules = self.rules.write().unwrap();
        if !rules.contains_key(rule_id) {
            return Err(PolicyError::RuleNotFound(rule_id.to_string()));
        }
        
        rules.remove(rule_id);
        
        // Also remove from policies
        let mut policies = self.policies.write().unwrap();
        for policy in policies.values_mut() {
            policy.remove_rule(rule_id);
        }
        
        Ok(())
    }
    
    /// Add a policy
    pub fn add_policy(&self, policy: Policy) -> Result<(), PolicyError> {
        let mut policies = self.policies.write().unwrap();
        
        // Update scope indexes
        if policy.scope == PolicyScope::Global {
            let mut global_policies = self.global_policies.write().unwrap();
            global_policies.insert(policy.id.clone());
        } else {
            let mut policies_by_scope = self.policies_by_scope.write().unwrap();
            let scope_id = policy.scope_id.clone().unwrap_or_default();
            let scope_policies = policies_by_scope
                .entry(policy.scope)
                .or_insert_with(HashMap::new);
                
            let policy_ids = scope_policies
                .entry(scope_id)
                .or_insert_with(HashSet::new);
                
            policy_ids.insert(policy.id.clone());
        }
        
        policies.insert(policy.id.clone(), policy);
        Ok(())
    }
    
    /// Get a policy by ID
    pub fn get_policy(&self, policy_id: &str) -> Result<Policy, PolicyError> {
        let policies = self.policies.read().unwrap();
        policies.get(policy_id)
            .cloned()
            .ok_or_else(|| PolicyError::PolicyNotFound(policy_id.to_string()))
    }
    
    /// Update a policy
    pub fn update_policy(&self, policy: Policy) -> Result<(), PolicyError> {
        let mut policies = self.policies.write().unwrap();
        if !policies.contains_key(&policy.id) {
            return Err(PolicyError::PolicyNotFound(policy.id.clone()));
        }
        
        // Get the old policy
        let old_policy = policies.get(&policy.id).unwrap();
        
        // Check if scope changed
        if old_policy.scope != policy.scope || old_policy.scope_id != policy.scope_id {
            // Remove from old scope index
            if old_policy.scope == PolicyScope::Global {
                let mut global_policies = self.global_policies.write().unwrap();
                global_policies.remove(&policy.id);
            } else {
                let mut policies_by_scope = self.policies_by_scope.write().unwrap();
                if let Some(scope_map) = policies_by_scope.get_mut(&old_policy.scope) {
                    let old_scope_id = old_policy.scope_id.clone().unwrap_or_default();
                    if let Some(policy_ids) = scope_map.get_mut(&old_scope_id) {
                        policy_ids.remove(&policy.id);
                    }
                }
            }
            
            // Add to new scope index
            if policy.scope == PolicyScope::Global {
                let mut global_policies = self.global_policies.write().unwrap();
                global_policies.insert(policy.id.clone());
            } else {
                let mut policies_by_scope = self.policies_by_scope.write().unwrap();
                let scope_id = policy.scope_id.clone().unwrap_or_default();
                let scope_policies = policies_by_scope
                    .entry(policy.scope)
                    .or_insert_with(HashMap::new);
                    
                let policy_ids = scope_policies
                    .entry(scope_id)
                    .or_insert_with(HashSet::new);
                    
                policy_ids.insert(policy.id.clone());
            }
        }
        
        policies.insert(policy.id.clone(), policy);
        Ok(())
    }
    
    /// Remove a policy
    pub fn remove_policy(&self, policy_id: &str) -> Result<(), PolicyError> {
        let mut policies = self.policies.write().unwrap();
        if !policies.contains_key(policy_id) {
            return Err(PolicyError::PolicyNotFound(policy_id.to_string()));
        }
        
        // Get the policy before removing it
        let policy = policies.get(policy_id).unwrap();
        
        // Remove from scope index
        if policy.scope == PolicyScope::Global {
            let mut global_policies = self.global_policies.write().unwrap();
            global_policies.remove(policy_id);
        } else {
            let mut policies_by_scope = self.policies_by_scope.write().unwrap();
            if let Some(scope_map) = policies_by_scope.get_mut(&policy.scope) {
                let scope_id = policy.scope_id.clone().unwrap_or_default();
                if let Some(policy_ids) = scope_map.get_mut(&scope_id) {
                    policy_ids.remove(policy_id);
                }
            }
        }
        
        policies.remove(policy_id);
        Ok(())
    }
    
    /// Get policies applicable to a context
    fn get_applicable_policies(&self, context: &EvaluationContext) -> Result<Vec<Policy>, PolicyError> {
        let mut applicable_policies = Vec::new();
        let policies = self.policies.read().unwrap();
        
        // Add global policies
        let global_policies = self.global_policies.read().unwrap();
        for policy_id in global_policies.iter() {
            if let Some(policy) = policies.get(policy_id) {
                if policy.enabled {
                    applicable_policies.push(policy.clone());
                }
            }
        }
        
        // Add content type policies
        let policies_by_scope = self.policies_by_scope.read().unwrap();
        if let Some(content_type_map) = policies_by_scope.get(&PolicyScope::ContentType) {
            let content_type_str = context.content_type.to_string();
            if let Some(policy_ids) = content_type_map.get(&content_type_str) {
                for policy_id in policy_ids {
                    if let Some(policy) = policies.get(policy_id) {
                        if policy.enabled {
                            applicable_policies.push(policy.clone());
                        }
                    }
                }
            }
        }
        
        // Add user policies if user ID is present
        if let Some(user_id) = &context.user_id {
            if let Some(user_map) = policies_by_scope.get(&PolicyScope::User) {
                if let Some(policy_ids) = user_map.get(user_id) {
                    for policy_id in policy_ids {
                        if let Some(policy) = policies.get(policy_id) {
                            if policy.enabled {
                                applicable_policies.push(policy.clone());
                            }
                        }
                    }
                }
            }
        }
        
        // Add session policies if session ID is present
        if let Some(session_id) = &context.session_id {
            if let Some(session_map) = policies_by_scope.get(&PolicyScope::Session) {
                if let Some(policy_ids) = session_map.get(session_id) {
                    for policy_id in policy_ids {
                        if let Some(policy) = policies.get(policy_id) {
                            if policy.enabled {
                                applicable_policies.push(policy.clone());
                            }
                        }
                    }
                }
            }
        }
        
        // Add source policies if source ID is present
        if let Some(source_id) = &context.source_id {
            if let Some(source_map) = policies_by_scope.get(&PolicyScope::Source) {
                if let Some(policy_ids) = source_map.get(source_id) {
                    for policy_id in policy_ids {
                        if let Some(policy) = policies.get(policy_id) {
                            if policy.enabled {
                                applicable_policies.push(policy.clone());
                            }
                        }
                    }
                }
            }
        }
        
        // Sort by priority (higher priority first)
        applicable_policies.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        Ok(applicable_policies)
    }
    
    /// Evaluate a context against policies
    pub async fn evaluate(&self, mut context: EvaluationContext) -> Result<PolicyEvaluationResult, PolicyError> {
        let start_time = Instant::now();
        
        // If classification is not provided, classify the content
        if context.classification.is_none() {
            let classification = self.filter.filter_content(&context.content, context.content_type)
                .await?
                .classification;
                
            context.classification = Some(classification);
        }
        
        // Build evaluation context
        let context_map = context.build_context_map();
        
        // Get applicable policies
        let applicable_policies = self.get_applicable_policies(&context)?;
        
        let mut matching_policies = Vec::new();
        let mut matching_rules = Vec::new();
        let mut highest_priority_action = None;
        let mut highest_priority = -1;
        
        let rules = self.rules.read().unwrap();
        
        // Evaluate policies
        for policy in applicable_policies {
            if !policy.enabled {
                continue;
            }
            
            let mut policy_matched = false;
            
            // Evaluate rules in the policy
            for rule_id in &policy.rule_ids {
                if let Some(rule) = rules.get(rule_id) {
                    if !rule.enabled {
                        continue;
                    }
                    
                    // Evaluate rule
                    if rule.matches(&context_map)? {
                        policy_matched = true;
                        
                        // Check if this rule has higher priority
                        if rule.priority > highest_priority {
                            highest_priority = rule.priority;
                            highest_priority_action = Some(rule.action);
                        }
                        
                        matching_rules.push(rule.clone());
                    }
                }
            }
            
            if policy_matched {
                matching_policies.push(policy.clone());
            }
        }
        
        // Determine final action
        let (final_action, explanation) = if let Some(action) = highest_priority_action {
            (action, format!("Action determined by highest priority rule ({})", highest_priority))
        } else {
            // Default to allowing if no rules matched
            (PolicyAction::Allow, "No matching rules found".to_string())
        };
        
        let result = PolicyEvaluationResult::new(
            final_action,
            matching_policies,
            matching_rules,
            context,
            explanation,
            start_time.elapsed().as_millis() as u64,
        );
        
        Ok(result)
    }
    
    /// Evaluate and enforce a policy
    pub async fn evaluate_and_enforce(&self, context: EvaluationContext) -> Result<PolicyEvaluationResult, PolicyError> {
        // First, evaluate the policy
        let mut result = self.evaluate(context).await?;
        
        // Then, enforce the action
        match result.action {
            PolicyAction::Allow => {
                // No additional action needed
            },
            PolicyAction::Block => {
                // Block the content by setting an empty filter result
                let filter_result = FilterResult::new(
                    result.context.content.clone(),
                    String::new(),
                    true,
                    FilterStrategy::Block,
                    result.context.classification.clone().unwrap(),
                    result.evaluation_time_ms,
                    "Content blocked by policy".to_string(),
                );
                
                result = result.with_filter_result(filter_result);
            },
            PolicyAction::Filter(strategy) => {
                // Apply the filter with the specified strategy
                let filter_result = self.filter.filter_content(
                    &result.context.content,
                    result.context.content_type,
                ).await?;
                
                result = result.with_filter_result(filter_result);
            },
            PolicyAction::RequireReview => {
                // Mark for review (implementation specific)
                result = result.with_metadata("review_required", "true");
            },
            PolicyAction::Log => {
                // Just log (implementation specific)
                info!("Policy log action: Content logged but allowed");
                result = result.with_metadata("logged", "true");
            },
            PolicyAction::Escalate => {
                // Escalate (implementation specific)
                warn!("Policy escalation: Content requires attention");
                result = result.with_metadata("escalated", "true");
                result = result.with_metadata("review_priority", "high");
            },
        }
        
        Ok(result)
    }
}

/// Filter policy builder for creating common policies
pub struct PolicyBuilder;

impl PolicyBuilder {
    /// Create a new safe content policy
    pub fn create_safe_content_policy() -> Policy {
        // Rule to allow safe content
        let rule = Rule::new(
            Some("safe-content-rule".to_string()),
            "Allow Safe Content".to_string(),
            "Allows content that is classified as safe".to_string(),
            Condition::Simple(SimpleCondition::new(
                "primary_category".to_string(),
                ConditionOperator::Equal,
                ConditionValue::String("safe".to_string()),
            )),
            PolicyAction::Allow,
            100,
        );
        
        // Create policy
        let mut policy = Policy::new(
            Some("safe-content-policy".to_string()),
            "Safe Content Policy".to_string(),
            "Policy to allow safe content".to_string(),
            PolicyScope::Global,
            None,
            100,
        );
        
        policy.add_rule(rule.id.clone());
        
        policy
    }
    
    /// Create a malicious content blocking policy
    pub fn create_malicious_content_policy() -> Policy {
        // Rule to block malicious content
        let rule = Rule::new(
            Some("block-malicious-rule".to_string()),
            "Block Malicious Content".to_string(),
            "Blocks content classified as malicious".to_string(),
            Condition::Simple(SimpleCondition::new(
                "primary_category".to_string(),
                ConditionOperator::Equal,
                ConditionValue::String("malicious".to_string()),
            )),
            PolicyAction::Block,
            200,
        );
        
        // Create policy
        let mut policy = Policy::new(
            Some("malicious-content-policy".to_string()),
            "Malicious Content Policy".to_string(),
            "Policy to block malicious content".to_string(),
            PolicyScope::Global,
            None,
            200,
        );
        
        policy.add_rule(rule.id.clone());
        
        policy
    }
    
    /// Create a sensitive content filtering policy
    pub fn create_sensitive_content_policy() -> Policy {
        // Create rules
        let explicit_rule = Rule::new(
            Some("filter-explicit-rule".to_string()),
            "Filter Explicit Content".to_string(),
            "Filters content classified as explicit".to_string(),
            Condition::Simple(SimpleCondition::new(
                "primary_category".to_string(),
                ConditionOperator::Equal,
                ConditionValue::String("explicit".to_string()),
            )),
            PolicyAction::Filter(FilterStrategy::Replace),
            150,
        );
        
        let privacy_rule = Rule::new(
            Some("filter-privacy-rule".to_string()),
            "Filter Privacy Concerns".to_string(),
            "Filters content with privacy concerns".to_string(),
            Condition::Simple(SimpleCondition::new(
                "primary_category".to_string(),
                ConditionOperator::Equal,
                ConditionValue::String("privacy_concern".to_string()),
            )),
            PolicyAction::Filter(FilterStrategy::Sanitize),
            140,
        );
        
        // Create policy
        let mut policy = Policy::new(
            Some("sensitive-content-policy".to_string()),
            "Sensitive Content Policy".to_string(),
            "Policy to filter sensitive content".to_string(),
            PolicyScope::Global,
            None,
            150,
        );
        
        policy.add_rule(explicit_rule.id.clone());
        policy.add_rule(privacy_rule.id.clone());
        
        policy
    }
    
    /// Create a content type specific policy
    pub fn create_content_type_policy(content_type: ContentType, strategy: FilterStrategy) -> Policy {
        // Rule for the content type
        let rule = Rule::new(
            Some(format!("filter-{}-rule", content_type)),
            format!("Filter {} Content", content_type),
            format!("Applies {} filtering to {} content", strategy, content_type),
            Condition::Simple(SimpleCondition::new(
                "content_type".to_string(),
                ConditionOperator::Equal,
                ConditionValue::String(content_type.to_string()),
            )),
            PolicyAction::Filter(strategy),
            100,
        );
        
        // Create policy
        let mut policy = Policy::new(
            Some(format!("{}-content-policy", content_type)),
            format!("{} Content Policy", content_type),
            format!("Policy for {} content", content_type),
            PolicyScope::ContentType,
            Some(content_type.to_string()),
            100,
        );
        
        policy.add_rule(rule.id.clone());
        
        policy
    }
    
    /// Create a user-specific policy
    pub fn create_user_policy(user_id: &str, action: PolicyAction) -> Policy {
        // Rule for the user
        let rule = Rule::new(
            Some(format!("user-{}-rule", user_id)),
            format!("User {} Rule", user_id),
            format!("Rule for user {}", user_id),
            Condition::Simple(SimpleCondition::new(
                "user_id".to_string(),
                ConditionOperator::Equal,
                ConditionValue::String(user_id.to_string()),
            )),
            action,
            100,
        );
        
        // Create policy
        let mut policy = Policy::new(
            Some(format!("user-{}-policy", user_id)),
            format!("User {} Policy", user_id),
            format!("Policy for user {}", user_id),
            PolicyScope::User,
            Some(user_id.to_string()),
            100,
        );
        
        policy.add_rule(rule.id.clone());
        
        policy
    }
}

// Context builder for creating evaluation contexts
#[derive(Clone)]
pub struct ContextBuilder {
    content: String,
    content_type: ContentType,
    classification: Option<ClassificationResult>,
    user_id: Option<String>,
    session_id: Option<String>,
    source_id: Option<String>,
    context_data: HashMap<String, ConditionValue>,
}

impl ContextBuilder {
    pub fn new(content: String, content_type: ContentType) -> Self {
        Self {
            content,
            content_type,
            classification: None,
            user_id: None,
            session_id: None,
            source_id: None,
            context_data: HashMap::new(),
        }
    }
    
    pub fn with_classification(mut self, classification: ClassificationResult) -> Self {
        self.classification = Some(classification);
        self
    }
    
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }
    
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }
    
    pub fn with_source_id(mut self, source_id: String) -> Self {
        self.source_id = Some(source_id);
        self
    }
    
    pub fn with_context(mut self, key: String, value: ConditionValue) -> Self {
        self.context_data.insert(key, value);
        self
    }
    
    pub fn build(self) -> EvaluationContext {
        EvaluationContext {
            content: self.content,
            content_type: self.content_type,
            classification: self.classification,
            user_id: self.user_id,
            session_id: self.session_id,
            source_id: self.source_id,
            context_data: self.context_data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_condition_evaluation() {
        let condition = SimpleCondition::new(
            "category".to_string(),
            ConditionOperator::Equal,
            ConditionValue::String("safe".to_string()),
        );
        
        let mut context = HashMap::new();
        context.insert("category".to_string(), ConditionValue::String("safe".to_string()));
        
        assert!(condition.evaluate(&context).unwrap());
        
        context.insert("category".to_string(), ConditionValue::String("unsafe".to_string()));
        assert!(!condition.evaluate(&context).unwrap());
    }
    
    #[test]
    fn test_composite_condition_evaluation() {
        let condition = CompositeCondition::and(vec![
            Condition::Simple(SimpleCondition::new(
                "category".to_string(),
                ConditionOperator::Equal,
                ConditionValue::String("safe".to_string()),
            )),
            Condition::Simple(SimpleCondition::new(
                "content_type".to_string(),
                ConditionOperator::Equal,
                ConditionValue::String("text".to_string()),
            )),
        ]);
        
        let mut context = HashMap::new();
        context.insert("category".to_string(), ConditionValue::String("safe".to_string()));
        context.insert("content_type".to_string(), ConditionValue::String("text".to_string()));
        
        assert!(condition.evaluate(&context).unwrap());
        
        context.insert("content_type".to_string(), ConditionValue::String("html".to_string()));
        assert!(!condition.evaluate(&context).unwrap());
    }
    
    #[test]
    fn test_rule_matching() {
        let rule = Rule::new(
            Some("test-rule".to_string()),
            "Test Rule".to_string(),
            "A rule for testing".to_string(),
            Condition::Simple(SimpleCondition::new(
                "category".to_string(),
                ConditionOperator::Equal,
                ConditionValue::String("safe".to_string()),
            )),
            PolicyAction::Allow,
            100,
        );
        
        let mut context = HashMap::new();
        context.insert("category".to_string(), ConditionValue::String("safe".to_string()));
        
        assert!(rule.matches(&context).unwrap());
        
        context.insert("category".to_string(), ConditionValue::String("unsafe".to_string()));
        assert!(!rule.matches(&context).unwrap());
    }
    
    #[tokio::test]
    async fn test_policy_builder() {
        let safe_policy = PolicyBuilder::create_safe_content_policy();
        let malicious_policy = PolicyBuilder::create_malicious_content_policy();
        let html_policy = PolicyBuilder::create_content_type_policy(ContentType::Html, FilterStrategy::Sanitize);
        
        assert_eq!(safe_policy.name, "Safe Content Policy");
        assert_eq!(safe_policy.rule_ids.len(), 1);
        assert_eq!(safe_policy.scope, PolicyScope::Global);
        
        assert_eq!(malicious_policy.name, "Malicious Content Policy");
        assert_eq!(malicious_policy.rule_ids.len(), 1);
        
        assert_eq!(html_policy.scope, PolicyScope::ContentType);
        assert_eq!(html_policy.scope_id, Some("html".to_string()));
    }
}