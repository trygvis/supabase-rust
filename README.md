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

Supabase Rustは、JavaScript版 [supabase-js](https://github.com/supabase/supabase-js) と互換性を持つように設計されています。APIの設計は類似していますが、Rustの言語仕様に合わせた実装になっています。

現在の実装では、supabase-jsの主要機能を提供していますが、一部の高度な機能はまだ実装中です。詳細は「開発状況」セクションをご覧ください。

## 開発状況

### 機能カバレッジ（supabase-jsとの比較）

| 機能 | supabase-js (TypeScript) | supabase-rust | カバレッジ | 
|------|------------------------|--------------|---------|
| **データベース (PostgreSQL)** | ✅ 完全実装 | ✅ 基本実装済み | 70% |
| **認証 (Auth)** | ✅ 完全実装 | ✅ 基本実装済み | 60% |
| **ストレージ (Storage)** | ✅ 完全実装 | ✅ 基本実装済み | 60% |
| **リアルタイム (Realtime)** | ✅ 完全実装 | ✅ 基本実装済み | 50% |
| **Edge Functions** | ✅ 完全実装 | ✅ 基本実装済み | 40% |
| **TypeScript/型安全** | ✅ 完全実装 | ✅ Rustの型システム | 90% |

### 詳細状況

#### データベース機能 (70%)
- ✅ 基本的なSELECT, INSERT, UPDATE, DELETEオペレーション
- ✅ 基本的なフィルタリング
- ✅ RPC関数呼び出し
- ✅ 基本的なリレーションシップクエリ
- 🔄 複雑な結合クエリ（実装中）
- 🔄 高度なPostgREST機能（実装中）
- ❌ CSVエクスポート機能（未実装）

#### 認証 (60%)
- ✅ メール・パスワード認証
- ✅ 基本的なセッション管理
- ✅ ユーザー情報取得
- ✅ パスワードリセット
- 🔄 OAuth認証（実装中）
- ❌ 多要素認証（未実装）
- ❌ 匿名認証（未実装）
- ❌ 電話番号認証（未実装）

#### ストレージ (60%)
- ✅ ファイルアップロード/ダウンロード
- ✅ バケット管理
- ✅ ファイル一覧取得
- ✅ 公開URL生成
- ✅ 基本的な署名付きURL
- 🔄 大容量ファイルのチャンクアップロード（実装中）
- ❌ 画像変換機能（未実装）

#### リアルタイム (50%)
- ✅ データベース変更監視
- ✅ カスタムチャンネル購読
- 🔄 高度なリアルタイムフィルタリング（実装中）
- ❌ Presenceサポート（未実装）
- 🔄 切断・再接続のロバスト性（改善中）

#### Edge Functions (40%)
- ✅ 基本的な関数呼び出し
- 🔄 高度なパラメータサポート（実装中）
- 🔄 詳細なエラーハンドリング（改善中）

### 今後の開発予定

1. **機能の拡充**: OAuth、MFA、Presenceなどの高度な機能を追加
2. **テストカバレッジの向上**: より包括的なテストスイートの開発
3. **ドキュメントの充実**: より詳細なAPIドキュメントの提供
4. **パフォーマンス最適化**: Rustの特性を活かしたパフォーマンス向上
5. **エコシステムの拡大**: ORMとの統合やフレームワーク特化型のヘルパーの開発

## コントリビューション

バグ報告、機能リクエスト、プルリクエストなど、あらゆる形でのコントリビューションを歓迎します。詳細は[コントリビューションガイド](CONTRIBUTING.md)をご覧ください。

## ライセンス

[MIT License](LICENSE)

## 貢献

貢献は歓迎します！詳細は [CONTRIBUTING.md](CONTRIBUTING.md) をご覧ください。

## セキュリティ

セキュリティ上の脆弱性を発見した場合は、[SECURITY.md](SECURITY.md)に記載されている連絡先に報告してください。