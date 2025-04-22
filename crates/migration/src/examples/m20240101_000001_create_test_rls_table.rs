use sea_orm_migration::prelude::*;
// Use helpers from the library root (lib.rs)
use supabase_rust_migration_lib::{RlsCommand, RlsPolicy, RlsRole, enable_rls_sql, disable_rls_sql};

// Define identifiers for the table and columns
#[derive(DeriveIden)]
enum TestRlsTable {
    Table, // Represents the table name "test_rls_table"
    Id,
    OwnerId, // Assuming this will store the user's auth.uid()
    Data,
    CreatedAt,
}

// Derive the migration name struct
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        println!("Applying migration: m20240101_000001_create_test_rls_table");

        // --- 1. Create Table ---
        manager
            .create_table(
                Table::create()
                    .table(TestRlsTable::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TestRlsTable::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TestRlsTable::OwnerId).uuid().not_null()) // Store Supabase auth user ID
                    .col(ColumnDef::new(TestRlsTable::Data).string())
                    .col(
                        ColumnDef::new(TestRlsTable::CreatedAt)
                            .timestamp_with_time_zone()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;
        println!(" -> Table 'test_rls_table' created.");

        // --- 2. Enable RLS ---
        let enable_sql = enable_rls_sql(&TestRlsTable::Table.into_alias(), Some("public"));
        let enable_stmt = Statement::from_string(manager.get_database_backend(), enable_sql);
        manager.exec_stmt(enable_stmt).await?;
        println!(" -> RLS enabled for 'test_rls_table'.");

        // --- 3. Create RLS Policy ---
        // Example: Allow authenticated users to select their own data
        let select_policy = RlsPolicy {
            name: "Allow auth user select own data".to_string(),
            table: TestRlsTable::Table.into_alias(),
            command: RlsCommand::Select,
            role: RlsRole::Authenticated,
            using: "auth.uid() = "owner_id"".to_string(), // Note: Ensure column name is quoted if needed
            check: None,
            schema: Some("public".to_string()),
        };

        let create_policy_sql = select_policy.create_policy_sql();
        let create_policy_stmt = Statement::from_string(manager.get_database_backend(), create_policy_sql);
        manager.exec_stmt(create_policy_stmt).await?;
        println!(" -> RLS policy '{}' created.", select_policy.name);

        // Add more policies here (INSERT, UPDATE, DELETE) as needed using the same pattern

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        println!("Reverting migration: m20240101_000001_create_test_rls_table");

        // --- 1. Drop RLS Policy ---
        // Construct a temporary policy just to get the drop SQL
        let policy_to_drop = RlsPolicy {
             name: "Allow auth user select own data".to_string(),
             table: TestRlsTable::Table.into_alias(),
             // Other fields are not needed for drop_policy_sql but need defaults
             command: RlsCommand::All, role: RlsRole::Public, using: String::new(), check: None, schema: Some("public".to_string()),
        };
        let drop_policy_sql = policy_to_drop.drop_policy_sql();
        let drop_policy_stmt = Statement::from_string(manager.get_database_backend(), drop_policy_sql);
        manager.exec_stmt(drop_policy_stmt).await?;
        println!(" -> RLS policy '{}' dropped.", policy_to_drop.name);

        // --- 2. Disable RLS (Optional) ---
        // Uncomment if you want RLS disabled after rollback
        // let disable_sql = disable_rls_sql(&TestRlsTable::Table.into_alias(), Some("public"));
        // let disable_stmt = Statement::from_string(manager.get_database_backend(), disable_sql);
        // manager.exec_stmt(disable_stmt).await?;
        // println!(" -> RLS disabled for 'test_rls_table'.");


        // --- 3. Drop Table ---
        manager
            .drop_table(Table::drop().table(TestRlsTable::Table).to_owned())
            .await?;
        println!(" -> Table 'test_rls_table' dropped.");

        Ok(())
    }
} 