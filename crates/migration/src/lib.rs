pub use sea_orm_migration::prelude::*;

// 例のマイグレーションモジュールを宣言
mod examples;

// init コマンドで生成された新しいマイグレーション
mod m20220101_000001_create_table;

// RLS ポリシー用のユーティリティ
pub mod utils;

// スキーマベースのマイグレーション
pub mod schema_migrations;
// Re-export schema migration components
pub use schema_migrations::{SchemaMigrator, DirectoryMigrator};

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        let mut migrations: Vec<Box<dyn MigrationTrait>> = vec![];

        // examples ディレクトリからのマイグレーション
        migrations.push(Box::new(
            examples::m20240101_000000_initial_setup::Migration,
        ));
        // RLS マイグレーションを有効化
        migrations.push(Box::new(
            examples::m20240101_000001_create_test_rls_table::Migration,
        ));

        // init コマンドで生成された新しいマイグレーション (開発テスト用)
        migrations.push(Box::new(m20220101_000001_create_table::Migration));

        // 実際のプロジェクトマイグレーションはここに追加
        // migrations.push(Box::new(m20240510_123000_create_users_table::Migration));

        migrations
    }
}
