# Supabase Rust

Rust クライアントライブラリ for [Supabase](https://supabase.io) - JavaScript版 supabase-js と互換性を持つRust実装です。

## 機能

- 💾 **Database**: PostgreSQLデータベースへの接続とデータの操作
- 🔐 **Auth**: ユーザーの認証と管理
- 📁 **Storage**: 大容量ファイルの保存と管理
- 🔄 **Realtime**: リアルタイムデータ変更の購読
- 🔥 **Edge Functions**: サーバーレス関数の実行

## インストール

```toml
[dependencies]
supabase-rust = "0.1.0"
```

## 基本的な使い方

```rust
use supabase_rust::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Supabase クライアントの初期化
    let supabase = Supabase::new("https://your-project-url.supabase.co", "your-anon-key");
    
    // データの取得
    let data = supabase
        .from("your-table")
        .select("*")
        .execute()
        .await?;
    
    println!("Data: {:?}", data);
    
    Ok(())
}
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
    .sign_in("user@example.com", "password123")
    .await?;

// 現在のユーザー情報の取得
let user = supabase.auth().get_user().await?;
```

## Storage

```rust
// ファイルのアップロード
let upload_result = supabase
    .storage()
    .from("bucket-name")
    .upload("folder/file.txt", file_data)
    .await?;

// ファイルダウンロード
let file_data = supabase
    .storage()
    .from("bucket-name")
    .download("folder/file.txt")
    .await?;
```

## 実行環境

- サポートRust バージョン: 1.65以上

## ライセンス

MIT License

## 貢献

貢献は歓迎します！詳細は [CONTRIBUTING.md](CONTRIBUTING.md) をご覧ください。