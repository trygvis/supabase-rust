# Supabase Rust

Rust クライアントライブラリ for [Supabase](https://supabase.com) - JavaScript版 [supabase-js](https://github.com/supabase/supabase-js) と互換性を持つRust実装です。

[![Crate](https://img.shields.io/crates/v/supabase-rust.svg)](https://crates.io/crates/supabase-rust)
[![Docs](https://docs.rs/supabase-rust/badge.svg)](https://docs.rs/supabase-rust)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## 機能

- 💾 **Database**: PostgreSQLデータベースへの接続とデータの操作（QueryBuilder, RPC）
- 🔐 **Auth**: ユーザーの認証と管理（サインアップ、サインイン、セッション管理）
- 📁 **Storage**: 大容量ファイルの保存と管理（アップロード、ダウンロード、一覧取得）
- 🔄 **Realtime**: リアルタイムデータ変更の購読
- 🔥 **Edge Functions**: サーバーレス関数の実行
- 🔍 **PostgREST**: 高度なフィルタリングと関係性のクエリ

## インストール

```toml
[dependencies]
supabase-rust = "0.1.0"
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

## 基本的な使い方

### クライアント初期化

```rust
use supabase_rust::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Supabase クライアントの初期化
    let supabase = Supabase::new("https://your-project-url.supabase.co", "your-anon-key");
    
    Ok(())
}
```

### データベース操作

```rust
// データの取得
let data = supabase
    .from("your-table")
    .select("*")
    .execute()
    .await?;

println!("Data: {:?}", data);

// フィルタリング
let filtered_data = supabase
    .from("your-table")
    .select("id, name, created_at")
    .eq("status", "active")
    .order("created_at", Some(Direction::Descending))
    .limit(10)
    .execute()
    .await?;

// データの挿入
let new_record = serde_json::json!({
    "name": "New Item",
    "description": "Description"
});

let insert_result = supabase
    .from("your-table")
    .insert(new_record)
    .execute()
    .await?;

// データの更新
let update_result = supabase
    .from("your-table")
    .update(serde_json::json!({"status": "inactive"}))
    .eq("id", 123)
    .execute()
    .await?;

// データの削除
let delete_result = supabase
    .from("your-table")
    .delete()
    .eq("id", 123)
    .execute()
    .await?;

// RPC関数の呼び出し
let rpc_result = supabase
    .rpc("calculate_total", serde_json::json!({"user_id": 123}))
    .execute()
    .await?;
```

## 認証

```rust
// ユーザー登録
let auth_response = supabase
    .auth()
    .sign_up("user@example.com", "password123")
    .await?;

// ログイン
let auth_response = supabase
    .auth()
    .sign_in_with_password("user@example.com", "password123")
    .await?;

// 現在のユーザー情報の取得
let user = supabase.auth().get_user().await?;

// セッションの更新
let session = supabase.auth().refresh_session().await?;

// ログアウト
supabase.auth().sign_out().await?;

// パスワードリセット
supabase
    .auth()
    .reset_password_for_email("user@example.com")
    .await?;
```

## Storage

```rust
// ファイルのアップロード
let upload_result = supabase
    .storage()
    .from("bucket-name")
    .upload("folder/file.txt", file_data, Some(FileOptions::new()))
    .await?;

// ファイルダウンロード
let file_data = supabase
    .storage()
    .from("bucket-name")
    .download("folder/file.txt")
    .await?;

// ファイル一覧の取得
let files = supabase
    .storage()
    .from("bucket-name")
    .list("folder", Some(ListOptions::new().limit(100)))
    .await?;

// 公開URLの生成
let public_url = supabase
    .storage()
    .from("bucket-name")
    .get_public_url("folder/file.txt");

// 署名付きURLの生成
let signed_url = supabase
    .storage()
    .from("bucket-name")
    .create_signed_url("folder/file.txt", 60)
    .await?;

// ファイルの削除
supabase
    .storage()
    .from("bucket-name")
    .remove(vec!["folder/file.txt", "folder/another-file.txt"])
    .await?;
```

## Realtime

```rust
// リアルタイム購読
let _subscription = supabase
    .channel("table-changes")
    .on(
        DatabaseChanges::new("your-table")
            .event(ChannelEvent::Insert)
            .event(ChannelEvent::Update)
            .event(ChannelEvent::Delete),
        |payload| {
            println!("Change received: {:?}", payload);
        },
    )
    .subscribe()
    .await?;

// カスタムチャネルの購読
let _broadcast_subscription = supabase
    .channel("custom-channel")
    .on(
        BroadcastChanges::new("custom-event"),
        |payload| {
            println!("Broadcast received: {:?}", payload);
        },
    )
    .subscribe()
    .await?;

// 購読解除
// subscriptionが破棄されると自動的に購読解除されます
```

## Edge Functions

```rust
// Edge Functionの呼び出し
let function_result = supabase
    .functions()
    .invoke::<serde_json::Value>("function-name", Some(serde_json::json!({"param": "value"})))
    .await?;
```

## エラーハンドリング

```rust
match supabase.from("your-table").select("*").execute().await {
    Ok(data) => {
        println!("Success: {:?}", data);
    }
    Err(err) => match err {
        Error::ApiError(api_error) => {
            println!("API Error: {} ({})", api_error.message, api_error.code);
        }
        Error::AuthError(auth_error) => {
            println!("Auth Error: {}", auth_error);
        }
        Error::StorageError(storage_error) => {
            println!("Storage Error: {}", storage_error);
        }
        _ => {
            println!("Other Error: {}", err);
        }
    },
}
```

## 実行環境

- サポートRust バージョン: 1.65以上
- `tokio` ランタイム上での非同期操作

## 互換性

このライブラリは [supabase-js](https://github.com/supabase/supabase-js) と互換性のあるAPIを提供することを目指していますが、完全な機能パリティはまだ実現されていません。APIの互換性についての詳細は[互換性ドキュメント](docs/COMPATIBILITY.md)を参照してください。

## ライセンス

[MIT License](LICENSE)

## 貢献

貢献は歓迎します！詳細は [CONTRIBUTING.md](CONTRIBUTING.md) をご覧ください。

## セキュリティ

セキュリティ上の脆弱性を発見した場合は、[SECURITY.md](SECURITY.md)に記載されている連絡先に報告してください。