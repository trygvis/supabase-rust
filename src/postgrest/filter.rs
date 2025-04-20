//! Filter operations for PostgrestClient

/// Operator for filter expressions
#[derive(Debug, Clone, PartialEq)]
pub enum FilterOperator {
    /// Equal to
    Eq,
    
    /// Not equal to
    Neq,
    
    /// Greater than
    Gt,
    
    /// Greater than or equal to
    Gte,
    
    /// Less than
    Lt,
    
    /// Less than or equal to
    Lte,
    
    /// Like (case sensitive)
    Like,
    
    /// Like (case insensitive)
    ILike,
    
    /// Is
    Is,
    
    /// In a list of values
    In,
    
    /// Contains all values
    ContainsAll,
    
    /// Contained by another array
    ContainedBy,
    
    /// Ranges are adjacent
    Adjacent,
    
    /// Range overlap
    Overlap,
    
    /// Text search match
    TextSearch,
    
    /// Match a full-text search query
    FTS,
    
    /// Match a full-text search query with a specific configuration
    FTSWC,
    
    /// Match against a Postgres function
    Filter,
    
    /// Match against a Postgres function with a filter
    FilterWithFilter,
}

impl FilterOperator {
    /// Convert the operator to its string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            FilterOperator::Eq => "eq",
            FilterOperator::Neq => "neq",
            FilterOperator::Gt => "gt",
            FilterOperator::Gte => "gte",
            FilterOperator::Lt => "lt",
            FilterOperator::Lte => "lte",
            FilterOperator::Like => "like",
            FilterOperator::ILike => "ilike",
            FilterOperator::Is => "is",
            FilterOperator::In => "in",
            FilterOperator::ContainsAll => "cs",
            FilterOperator::ContainedBy => "cd",
            FilterOperator::Adjacent => "adj",
            FilterOperator::Overlap => "ov",
            FilterOperator::TextSearch => "ts",
            FilterOperator::FTS => "fts",
            FilterOperator::FTSWC => "ftswc",
            FilterOperator::Filter => "filter",
            FilterOperator::FilterWithFilter => "filterwithfilter",
        }
    }
}