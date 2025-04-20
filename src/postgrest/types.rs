//! Types for the PostgrestClient

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Count options for queries
#[derive(Debug, Clone, PartialEq)]
pub enum CountOption {
    /// Exact count
    Exact,
    
    /// Planned count (estimated)
    Planned,
    
    /// Estimated count
    Estimated,
}

impl CountOption {
    /// Convert the option to its string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            CountOption::Exact => "exact",
            CountOption::Planned => "planned",
            CountOption::Estimated => "estimated",
        }
    }
}

/// Options for returning data
#[derive(Debug, Clone, PartialEq)]
pub enum ReturnOption {
    /// Return headers only
    HeadersOnly,
    
    /// Return representation (the data)
    Representation,
    
    /// Return minimal data
    Minimal,
    
    /// Return no content
    NoContent,
}

impl ReturnOption {
    /// Convert the option to its string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ReturnOption::HeadersOnly => "headers-only",
            ReturnOption::Representation => "representation",
            ReturnOption::Minimal => "minimal",
            ReturnOption::NoContent => "no-content",
        }
    }
}

/// PostgreSQL data types
#[derive(Debug, Clone, PartialEq)]
pub enum PostgresType {
    /// Boolean
    Boolean,
    
    /// Integer
    Integer,
    
    /// Float
    Float,
    
    /// Text
    Text,
    
    /// Date
    Date,
    
    /// Time
    Time,
    
    /// Timestamp
    Timestamp,
    
    /// UUID
    Uuid,
    
    /// JSON
    Json,
    
    /// JSONB
    Jsonb,
    
    /// Array
    Array,
    
    /// Range
    Range,
}

impl PostgresType {
    /// Convert the type to its string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            PostgresType::Boolean => "boolean",
            PostgresType::Integer => "integer",
            PostgresType::Float => "float",
            PostgresType::Text => "text",
            PostgresType::Date => "date",
            PostgresType::Time => "time",
            PostgresType::Timestamp => "timestamp",
            PostgresType::Uuid => "uuid",
            PostgresType::Json => "json",
            PostgresType::Jsonb => "jsonb",
            PostgresType::Array => "array",
            PostgresType::Range => "range",
        }
    }
}