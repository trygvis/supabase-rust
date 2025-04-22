# Supabase PostgREST Client for Rust

PostgreSQLデータベースへのREST APIアクセスを提供するSupabaseクライアントライブラリです。

## 機能

- 基本的なCRUD操作（`select`, `insert`, `update`, `delete`）
- フィルタリング（`eq`, `gt`, `lt` など）
- 順序付けとページネーション
- トランザクション
- RPC関数呼び出し
- CSV出力
- TypeScript型定義からRust型への変換（`schema-convert`フィーチャ使用時）
- 型安全なデータベース操作

## インストール

Cargo.tomlに依存関係を追加してください：

```toml
[dependencies]
supabase-rust-postgrest = "0.1.1"
```

TypeScript型からRust型への変換機能を使用する場合：

```toml
[dependencies]
supabase-rust-postgrest = { version = "0.1.1", features = ["schema-convert"] }
```

## 基本的な使い方

```rust
use supabase_rust_postgrest::PostgrestClient;
use reqwest::Client;

async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let http_client = Client::new();
    let db = PostgrestClient::new(
        "https://your-project.supabase.co", 
        "your-anon-key", 
        "your_table", 
        http_client
    );
    
    // データの取得
    let response: Vec<serde_json::Value> = db
        .select("*")
        .eq("column", "value")
        .execute()
        .await?;
    
    // データの挿入
    let data = serde_json::json!({
        "name": "John Doe",
        "email": "john@example.com"
    });
    
    let inserted = db
        .insert(&data)
        .await?;
        
    // データの更新
    let update_data = serde_json::json!({
        "name": "Jane Doe"
    });
    
    let updated = db
        .eq("id", "1")
        .update(&update_data)
        .await?;
        
    // データの削除
    let deleted = db
        .eq("id", "1")
        .delete()
        .await?;
        
    Ok(())
}
```

## TypeScript型定義からRust型への変換

このクレートは、Supabaseの`supabase gen types typescript`コマンドで生成されたTypeScript型定義をRustの型に変換する機能を提供します。この機能を使用するには、`schema-convert`フィーチャを有効にする必要があります。

### コマンドラインから変換する

```bash
# リポジトリのルートディレクトリで実行
supabase gen types typescript > types.ts

# TypeScript型定義からRust型を生成
cargo run --features schema-convert --bin supabase-gen-rust -- \
    --input-file ./types.ts \
    --output-dir ./src/generated \
    --module-name schema
```

### プログラムから変換する

```rust
use std::path::Path;
use supabase_rust_postgrest::{
    convert_typescript_to_rust,
    SchemaConvertOptions,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input_file = Path::new("./types.ts");
    let options = SchemaConvertOptions::default();
    
    let output_path = convert_typescript_to_rust(input_file, options)?;
    println!("Generated Rust types at: {:?}", output_path);
    
    Ok(())
}
```

## 型安全なデータベース操作

変換されたRust型を使用して、型安全なデータベース操作を行うことができます。

```rust
use serde::{Deserialize, Serialize};
use supabase_rust_postgrest::{
    PostgrestClient, Table, PostgrestClientTypeExtension
};

// 自動生成されたモデルを使用する場合は、
// mod schema; 
// use schema::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: Option<i32>,
    name: String,
    email: String,
}

impl Table for User {
    fn table_name() -> &'static str {
        "users"
    }
}

async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let http_client = reqwest::Client::new();
    let client = PostgrestClient::new(
        "https://your-project.supabase.co", 
        "your-anon-key", 
        "", // テーブル名は自動的に設定される
        http_client
    );
    
    // 型安全なクエリ
    let users: Vec<User> = client
        .query_typed::<User>()
        .eq("name", "John")
        .execute()
        .await?;
        
    // 型安全な挿入
    let new_user = User {
        id: None,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };
    
    let inserted: User = client
        .insert_typed(&new_user)?
        .execute()
        .await?;
        
    // 型安全な更新
    let mut user_to_update = users[0].clone();
    user_to_update.name = "Bob".to_string();
    
    let updated: User = client
        .update_typed(&user_to_update)?
        .eq("id", &user_to_update.id.unwrap().to_string())
        .execute()
        .await?;
        
    // 型安全な削除
    client
        .delete_typed::<User>()
        .eq("id", &users[0].id.unwrap().to_string())
        .execute()
        .await?;
        
    Ok(())
}
```

## ライセンス

MIT 