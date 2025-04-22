use sea_orm_migration::prelude::*;

// RLS ポリシーのコマンドタイプを表す列挙型
#[derive(Debug, Clone)]
pub enum RlsCommand {
    Select,
    Insert,
    Update,
    Delete,
    All,
}

// RLS ポリシーのロールを表す列挙型
#[derive(Debug, Clone)]
pub enum RlsRole {
    Public,             // すべてのユーザー (anonymous も含む)
    Authenticated,      // 認証済みユーザーのみ
    Anon,               // 匿名ユーザーのみ
    CustomRole(String), // カスタムロール名
}

// RLS ポリシーの設定を表す構造体
#[derive(Debug, Clone)]
pub struct RlsPolicy {
    pub name: String,
    pub table: String,
    pub command: RlsCommand,
    pub role: RlsRole,
    pub using: String,
    pub check: Option<String>,
    pub schema: Option<String>,
}

impl RlsPolicy {
    // ポリシー作成 SQL を生成
    pub fn create_policy_sql(&self) -> String {
        let command = match self.command {
            RlsCommand::Select => "SELECT",
            RlsCommand::Insert => "INSERT",
            RlsCommand::Update => "UPDATE",
            RlsCommand::Delete => "DELETE",
            RlsCommand::All => "ALL",
        };

        let role = match &self.role {
            RlsRole::Public => "PUBLIC",
            RlsRole::Authenticated => "AUTHENTICATED",
            RlsRole::Anon => "ANON",
            RlsRole::CustomRole(role_name) => role_name,
        };

        let schema_prefix = match &self.schema {
            Some(schema) => format!("{}.", schema),
            None => "".to_string(),
        };

        let check_clause = match &self.check {
            Some(check_expr) => format!(" WITH CHECK ({})", check_expr),
            None => "".to_string(),
        };

        format!(
            "CREATE POLICY \"{}\" ON {}{} FOR {} TO {} USING ({}){};",
            self.name, schema_prefix, self.table, command, role, self.using, check_clause
        )
    }

    // ポリシー削除 SQL を生成
    pub fn drop_policy_sql(&self) -> String {
        let schema_prefix = match &self.schema {
            Some(schema) => format!("{}.", schema),
            None => "".to_string(),
        };

        format!(
            "DROP POLICY IF EXISTS \"{}\" ON {}{};",
            self.name, schema_prefix, self.table
        )
    }
}

// テーブルの RLS を有効化する SQL を生成
pub fn enable_rls_sql(table: &str, schema: Option<&str>) -> String {
    let schema_prefix = match schema {
        Some(schema) => format!("{}.", schema),
        None => "".to_string(),
    };

    format!(
        "ALTER TABLE {}{} ENABLE ROW LEVEL SECURITY;",
        schema_prefix, table
    )
}

// テーブルの RLS を無効化する SQL を生成
pub fn disable_rls_sql(table: &str, schema: Option<&str>) -> String {
    let schema_prefix = match schema {
        Some(schema) => format!("{}.", schema),
        None => "".to_string(),
    };

    format!(
        "ALTER TABLE {}{} DISABLE ROW LEVEL SECURITY;",
        schema_prefix, table
    )
}

// RLS 強制を設定する SQL を生成 (所有者にも適用するかどうか)
pub fn force_rls_sql(table: &str, schema: Option<&str>, force: bool) -> String {
    let schema_prefix = match schema {
        Some(schema) => format!("{}.", schema),
        None => "".to_string(),
    };

    let force_str = if force { "ENABLE" } else { "DISABLE" };

    format!(
        "ALTER TABLE {}{} FORCE ROW LEVEL SECURITY {};",
        schema_prefix, table, force_str
    )
}
