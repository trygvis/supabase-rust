"""
use sea_orm_migration::prelude::Alias;
use std::fmt;

/// Represents the SQL command type for an RLS policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RlsCommand {
    All,
    Select,
    Insert,
    Update,
    Delete,
}

impl fmt::Display for RlsCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RlsCommand::All => write!(f, "ALL"),
            RlsCommand::Select => write!(f, "SELECT"),
            RlsCommand::Insert => write!(f, "INSERT"),
            RlsCommand::Update => write!(f, "UPDATE"),
            RlsCommand::Delete => write!(f, "DELETE"),
        }
    }
}

/// Represents the target role for an RLS policy.
/// Use `RlsRole::Custom("my_role".to_string())` for non-standard roles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RlsRole {
    Authenticated,
    Anon,
    ServiceRole,
    Public, // Represents the 'public' role in PostgreSQL
    Custom(String),
}

impl fmt::Display for RlsRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RlsRole::Authenticated => write!(f, "authenticated"),
            RlsRole::Anon => write!(f, "anon"),
            RlsRole::ServiceRole => write!(f, "service_role"),
            RlsRole::Public => write!(f, "public"),
            RlsRole::Custom(role) => write!(f, ""{}"", role), // Quote custom roles if needed
        }
    }
}

/// Represents a complete RLS policy definition.
#[derive(Debug, Clone)]
pub struct RlsPolicy {
    /// Unique name for the policy.
    pub name: String,
    /// The table this policy applies to (use `Alias::new("table_name")`).
    pub table: Alias,
    /// The SQL command this policy restricts (SELECT, INSERT, etc.).
    pub command: RlsCommand,
    /// The role this policy applies to.
    pub role: RlsRole,
    /// The SQL expression for the `USING` clause (e.g., "auth.uid() = user_id").
    /// This determines which rows are visible or modifiable.
    pub using: String,
    /// Optional SQL expression for the `WITH CHECK` clause.
    /// This validates rows being inserted or updated.
    pub check: Option<String>,
    /// The schema the table resides in (usually "public").
    pub schema: Option<String>, // Defaults to "public" if None
}

impl RlsPolicy {
    /// Generates the `CREATE POLICY` SQL statement.
    pub fn create_policy_sql(&self) -> String {
        let schema_prefix = self.schema.as_deref().unwrap_or("public");
        let check_clause = self.check.as_ref().map_or(String::new(), |c| format!(" WITH CHECK ({})", c));
        // Ensure policy names and table names are properly quoted if they contain special characters or are keywords.
        // Alias::new() doesn't automatically quote for SQL generation in this context.
        // A more robust solution might involve a dedicated SQL identifier quoting function.
        format!(
            "CREATE POLICY "{policy_name}" ON "{schema}"."{table_name}" FOR {command} TO {role} USING ({using}){check_clause};",
            policy_name = self.name, // Policy names are identifiers, quote them
            schema = schema_prefix,
            table_name = self.table.to_string(), // Alias::to_string() gives the name
            command = self.command,
            role = self.role,
            using = self.using,
            check_clause = check_clause
        )
    }

    /// Generates the `DROP POLICY` SQL statement.
    pub fn drop_policy_sql(&self) -> String {
        let schema_prefix = self.schema.as_deref().unwrap_or("public");
        format!(
            "DROP POLICY IF EXISTS "{policy_name}" ON "{schema}"."{table_name}";",
            policy_name = self.name,
            schema = schema_prefix,
            table_name = self.table.to_string()
        )
    }
}

/// Generates the SQL statement to enable RLS on a table.
pub fn enable_rls_sql(table: &Alias, schema: Option<&str>) -> String {
    let schema_prefix = schema.unwrap_or("public");
    format!(
        "ALTER TABLE "{schema}"."{table_name}" ENABLE ROW LEVEL SECURITY;",
        schema = schema_prefix,
        table_name = table.to_string()
    )
}

/// Generates the SQL statement to disable RLS on a table.
pub fn disable_rls_sql(table: &Alias, schema: Option<&str>) -> String {
    let schema_prefix = schema.unwrap_or("public");
    format!(
        "ALTER TABLE "{schema}"."{table_name}" DISABLE ROW LEVEL SECURITY;",
        schema = schema_prefix,
        table_name = table.to_string()
    )
}
"" 