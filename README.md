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

// 複雑な結合クエリ
let joined_data = supabase
    .from("posts")
    .select("id, title, content")
    .include("comments", "post_id", Some("id, text, user_id"))
    .inner_join("users", "user_id", "id")
    .execute()
    .await?;

// 全文検索
let search_results = supabase
    .from("articles")
    .select("id, title, content")
    .text_search("content", "search terms", Some("english"))
    .execute()
    .await?;

// CSVエクスポート
let csv_data = supabase
    .from("large_table")
    .select("*")
    .limit(1000)
    .export_csv()
    .await?;

// ファイルとして保存
std::fs::write("export.csv", csv_data)?;

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

## トランザクション

```rust
// トランザクションを開始
let transaction = supabase
    .from("users")
    .begin_transaction(
        Some(IsolationLevel::ReadCommitted),  // 分離レベル
        Some(TransactionMode::ReadWrite),     // 読み書きモード
        Some(30)                              // タイムアウト（秒）
    )
    .await?;

// トランザクション内で複数の操作を実行
// 1. データの挿入
let insert_result = transaction
    .from("users")
    .insert(serde_json::json!({
        "name": "トランザクションユーザー",
        "email": "transaction@example.com"
    }))
    .execute()
    .await?;

let user_id = insert_result[0]["id"].as_i64().unwrap();

// 2. 関連データの挿入
let profile_result = transaction
    .from("profiles")
    .insert(serde_json::json!({
        "user_id": user_id,
        "bio": "トランザクションで作成されたプロフィール"
    }))
    .execute()
    .await?;

// 3. セーブポイントを作成
transaction.savepoint("user_created").await?;

// 4. データの更新
transaction
    .from("users")
    .update(serde_json::json!({ "status": "active" }))
    .eq("id", &user_id.to_string())
    .execute()
    .await?;

// 5. トランザクションをコミット
transaction.commit().await?;

// エラー処理を含む例
let transaction = supabase
    .from("items")
    .begin_transaction(None, None, None)
    .await?;

transaction
    .from("items")
    .insert(serde_json::json!({ "name": "アイテム1" }))
    .execute()
    .await?;

// セーブポイントを作成
transaction.savepoint("item1_inserted").await?;

// 何らかの条件でロールバックが必要になった場合
if some_condition {
    // セーブポイントにロールバック
    transaction.rollback_to_savepoint("item1_inserted").await?;
} else if another_condition {
    // トランザクション全体をロールバック
    transaction.rollback().await?;
    return Err("トランザクションがロールバックされました".into());
} else {
    // すべての操作が成功した場合はコミット
    transaction.commit().await?;
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

// メール確認機能
// メール確認リクエストの送信
let options = EmailConfirmOptions {
    redirect_to: Some("https://your-app.com/confirm-success".to_string()),
};

supabase
    .auth()
    .send_confirm_email_request("user@example.com", Some(options))
    .await?;

// メール確認トークンの検証（確認リンクからのトークン）
let session = supabase
    .auth()
    .verify_email("confirmation-token-from-email")
    .await?;

println!("Email confirmed for user: {}", session.user.email.unwrap_or_default());

// パスワードリセットトークンの検証と新パスワード設定
let session = supabase
    .auth()
    .verify_password_reset("reset-token-from-email", "new-secure-password")
    .await?;

println!("Password reset for user: {}", session.user.email.unwrap_or_default());
```

## OAuth認証

```rust
// OAuth認証URLの生成
let auth_url = supabase
    .auth()
    .get_oauth_sign_in_url(
        OAuthProvider::Google,
        Some(OAuthSignInOptions {
            redirect_to: Some("https://your-app.com/callback".to_string()),
            scopes: Some("email profile".to_string()),
            ..Default::default()
        })
    );

println!("Sign in URL: {}", auth_url);

// コールバックからのコードを使用してセッションを取得
let session = supabase
    .auth()
    .exchange_code_for_session("received_code_from_oauth_callback")
    .await?;

println!("Authenticated user: {:?}", session.user);
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

// 画像変換
let transform_options = ImageTransformOptions::new()
    .with_width(300)
    .with_height(200)
    .with_resize("cover")
    .with_format("webp")
    .with_quality(90);

// 変換された画像を取得
let transformed_image = supabase
    .storage()
    .from("bucket-name")
    .transform_image("folder/image.png", transform_options.clone())
    .await?;

// 変換された画像の公開URLを取得
let public_transform_url = supabase
    .storage()
    .from("bucket-name")
    .get_public_transform_url("folder/image.png", transform_options.clone());

// 変換された画像の署名付きURLを取得
let signed_transform_url = supabase
    .storage()
    .from("bucket-name")
    .create_signed_transform_url("folder/image.png", transform_options, 60)
    .await?;

// ファイルの削除
supabase
    .storage()
    .from("bucket-name")
    .remove(vec!["folder/file.txt", "folder/another-file.txt"])
    .await?;
```

## 大容量ファイルのチャンクアップロード

```rust
// 大きなファイルをチャンクでアップロードする
let file_path = std::path::Path::new("/path/to/large-file.mp4");
let result = supabase
    .storage()
    .from("videos")
    .upload_large_file(
        "videos/large-file.mp4",
        file_path,
        5 * 1024 * 1024, // 5MBチャンクサイズ
        Some(FileOptions::new().with_content_type("video/mp4"))
    )
    .await?;

println!("Uploaded file: {:?}", result);

// 手動でマルチパートアップロードを制御する場合
// 1. マルチパートアップロードを初期化
let init_result = supabase
    .storage()
    .from("videos")
    .initiate_multipart_upload(
        "videos/large-file.mp4",
        Some(FileOptions::new().with_content_type("video/mp4"))
    )
    .await?;

// 2. チャンクを個別にアップロード
let chunk_data = bytes::Bytes::from(vec![0u8; 1024]); // 実際のデータ
let part_result = supabase
    .storage()
    .from("videos")
    .upload_part(&init_result.upload_id, 1, chunk_data)
    .await?;

// 3. マルチパートアップロードを完了
let complete_result = supabase
    .storage()
    .from("videos")
    .complete_multipart_upload(
        &init_result.upload_id,
        "videos/large-file.mp4",
        vec![part_result]
    )
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

## リアルタイム接続の管理

```rust
// カスタム接続オプションでリアルタイムクライアントを初期化
let options = RealtimeClientOptions {
    auto_reconnect: true,
    max_reconnect_attempts: Some(10),
    reconnect_interval: 2000, // 2秒
    ..Default::default()
};

// 接続状態の変更を監視
let realtime = supabase.realtime();
let mut state_receiver = realtime.on_state_change();

// 別スレッドで状態変更を監視
tokio::spawn(async move {
    while let Ok(state) = state_receiver.recv().await {
        println!("Connection state changed: {:?}", state);
        
        match state {
            ConnectionState::Connected => {
                println!("接続成功!");
            }
            ConnectionState::Reconnecting => {
                println!("再接続中...");
            }
            ConnectionState::Disconnected => {
                println!("切断されました");
            }
            _ => {}
        }
    }
});

// テーブル変更の購読
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

// 手動で接続を終了
supabase.realtime().disconnect().await?;
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
| **データベース (PostgreSQL)** | ✅ 完全実装 | ✅ 完全実装 | 90% |
| **認証 (Auth)** | ✅ 完全実装 | ✅ 基本実装済み | 90% |
| **ストレージ (Storage)** | ✅ 完全実装 | ✅ 基本実装済み | 95% |
| **リアルタイム (Realtime)** | ✅ 完全実装 | ✅ 基本実装済み | 95% |
| **Edge Functions** | ✅ 完全実装 | ✅ 基本実装済み | 85% |
| **TypeScript/型安全** | ✅ 完全実装 | ✅ Rustの型システム | 90% |

### 詳細状況

#### データベース機能 (90%)
- ✅ 基本的なSELECT, INSERT, UPDATE, DELETEオペレーション
- ✅ 基本的なフィルタリング
- ✅ RPC関数呼び出し
- ✅ 基本的なリレーションシップクエリ
- ✅ 複雑な結合クエリ（内部結合、外部結合、子テーブル含める）
- ✅ 高度なPostgREST機能（全文検索、地理空間データ検索、グループ化）
- ✅ CSVエクスポート機能
- ✅ 行レベルセキュリティ（RLS）対応
- ✅ トランザクション処理

#### 認証 (90%)
- ✅ メール・パスワード認証
- ✅ 基本的なセッション管理
- ✅ ユーザー情報取得
- ✅ パスワードリセット
- ✅ OAuth認証
- ✅ 多要素認証（MFA）
- ✅ 匿名認証
- ✅ 電話番号認証
- ✅ メール確認機能

#### ストレージ (95%)
- ✅ ファイルアップロード/ダウンロード
- ✅ バケット管理
- ✅ ファイル一覧取得
- ✅ 公開URL生成
- ✅ 基本的な署名付きURL
- ✅ 大容量ファイルのチャンクアップロード
- ✅ 画像変換機能（リサイズ、フォーマット変換、画質調整）
- ✅ S3互換APIのサポート

#### リアルタイム (95%)
- ✅ データベース変更監視
- ✅ カスタムチャネル購読
- ✅ 切断・再接続のロバスト性
- ✅ Presenceサポート
- ✅ 高度なリアルタイムフィルタリング

#### Edge Functions (85%)
- ✅ 基本的な関数呼び出し
- ✅ 高度なパラメータサポート
- ✅ 詳細なエラーハンドリング
- ✅ 異なるレスポンス形式（JSON, テキスト, バイナリ）のサポート
- ✅ ストリーミングレスポンスのサポート
- 🔄 ストリームの自動変換機能の拡張（実装中）

### 今後の開発予定

1. **データベース機能の強化**:
   - 複雑な結合クエリの最適化
   - データベースプールの管理と効率化

2. **認証の拡張**:
   - WebAuthn/パスキーサポートの追加
   - 組織機能のサポート
   - 詳細な権限管理の実装

3. **ストレージの拡張**:
   - S3互換API機能の拡張
   - 大容量ファイル処理の最適化
   - バケット権限管理の詳細制御

4. **リアルタイム機能の強化**:
   - バッチ購読処理の最適化
   - オフライン同期サポート

5. **Edge Functions拡張**:
   - Deno/Rustランタイムサポート
   - ウェブフック統合
   - ローカル開発環境との連携

6. **パフォーマンスとセキュリティ**:
   - メモリ使用量の最適化
   - スレッド安全性の強化
   - 暗号化機能の拡張

## 匿名認証

```rust
// 匿名認証でサインイン
let anonymous_session = supabase
    .auth()
    .sign_in_anonymously()
    .await?;

println!("Anonymous user ID: {}", anonymous_session.user.id);
```

## 電話番号認証

```rust
// 電話番号認証 - ステップ1: 認証コード送信
let verification = supabase
    .auth()
    .send_verification_code("+81901234567")
    .await?;

println!("Verification ID: {}", verification.verification_id);
println!("Code sent to: {}", verification.phone);
println!("Expires at: {}", verification.expires_at);

// 電話番号認証 - ステップ2: コード検証とサインイン
// ユーザーがSMSで受け取ったコード
let sms_code = "123456"; // 実際の例ではユーザー入力から取得

let session = supabase
    .auth()
    .verify_phone_code(
        "+81901234567",
        &verification.verification_id,
        sms_code
    )
    .await?;

println!("Logged in with phone: {:?}", session.user.phone);
```

## ストリーミングレスポンス (Edge Functions)

```rust
// ストリーミングレスポンスの取得
let stream = supabase
    .functions()
    .invoke_stream::<serde_json::Value>(
        "stream-data",
        Some(serde_json::json!({"count": 100})),
        None
    )
    .await?;

// バイトストリームから行ストリームに変換
let line_stream = supabase.functions().stream_to_lines(stream);

// ストリームを処理
tokio::pin!(line_stream);
while let Some(line_result) = line_stream.next().await {
    match line_result {
        Ok(line) => {
            println!("Received line: {}", line);
            // 行を必要に応じてJSONとしてパース
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                println!("Parsed JSON: {:?}", json);
            }
        },
        Err(e) => {
            eprintln!("Error reading stream: {}", e);
            break;
        }
    }
}

// JSONストリームを直接取得
let json_stream = supabase
    .functions()
    .invoke_json_stream::<serde_json::Value>(
        "stream-events",
        Some(serde_json::json!({"eventType": "user-activity"})),
        None
    )
    .await?;

// JSONイベントを処理
tokio::pin!(json_stream);
while let Some(json_result) = json_stream.next().await {
    match json_result {
        Ok(json) => {
            println!("Received JSON event: {:?}", json);
        },
        Err(e) => {
            eprintln!("Error in JSON stream: {}", e);
            break;
        }
    }
}
```

## コントリビューション

バグ報告、機能リクエスト、プルリクエストなど、あらゆる形でのコントリビューションを歓迎します。詳細は[コントリビューションガイド](CONTRIBUTING.md)をご覧ください。

## ライセンス

[MIT License](LICENSE)

## 貢献

貢献は歓迎します！詳細は [CONTRIBUTING.md](CONTRIBUTING.md) をご覧ください。

## セキュリティ

セキュリティ上の脆弱性を発見した場合は、[SECURITY.md](SECURITY.md)に記載されている連絡先に報告してください。

## 多要素認証（MFA）

```rust
// MFAを使用したサインイン - 第一ステップ
let result = supabase
    .auth()
    .sign_in_with_password_mfa("user@example.com", "password123")
    .await?;

// 結果の処理
match result {
    Ok(session) => {
        // MFAが必要ない場合 - ログイン成功
        println!("Logged in successfully: {:?}", session.user.email);
    },
    Err(challenge) => {
        // MFA認証が必要 - 第二ステップへ
        println!("MFA required with challenge ID: {}", challenge.id);
        
        // ユーザーからTOTPコード（例: Authenticatorアプリのコード）を取得
        let totp_code = "123456"; // 実際のコードをユーザーから取得する
        
        // MFAチャレンジを検証
        let session = supabase
            .auth()
            .verify_mfa_challenge(&challenge.id, totp_code)
            .await?;
            
        println!("MFA verification successful: {:?}", session.user.email);
    }
}

// MFA TOTPファクターの登録
let setup_info = supabase
    .auth()
    .enroll_totp()
    .await?;

println!("TOTP secret: {}", setup_info.secret);
println!("QR code: {}", setup_info.qr_code);

// TOTPの検証と有効化
let factor = supabase
    .auth()
    .verify_totp("factor-id-from-setup", "123456")
    .await?;

println!("MFA factor enabled: {:?}", factor.status);

// ユーザーのMFAファクター一覧を取得
let factors = supabase
    .auth()
    .list_factors()
    .await?;

for factor in factors {
    println!("Factor: {} ({})", factor.id, factor.factor_type);
}

// MFAファクターの削除
supabase
    .auth()
    .unenroll_factor("factor-id")
    .await?;
```

## Presenceサポート

```rust
// Presenceを使用してユーザーのオンライン状態を追跡
let channel = supabase
    .channel("room:123");

// Presenceの変更を監視
let _subscription = channel
    .on_presence(|presence_diff| {
        // 新規参加ユーザーの処理
        for (user_id, user_data) in &presence_diff.joins {
            println!("User joined: {}, data: {:?}", user_id, user_data);
        }
        
        // 退室ユーザーの処理
        for (user_id, _) in &presence_diff.leaves {
            println!("User left: {}", user_id);
        }
    })
    .subscribe()
    .await?;

// ユーザー状態を追跡
let user_id = "user-123";
let user_data = serde_json::json!({
    "name": "John Doe",
    "status": "online",
    "last_seen_at": "2023-07-01T12:00:00Z"
});

// Presenceの状態を設定
channel
    .track_presence(user_id, user_data)
    .await?;

// Presenceの状態を同期
let mut presence_state = PresenceState::new();

// 状態更新時に同期
presence_state.sync(&presence_diff);

// 現在オンラインのユーザー一覧を取得
let online_users = presence_state.list();
println!("Online users: {:?}", online_users);
```

## 拡張されたEdge Functions

```rust
// 様々なレスポンスタイプに対応
// JSON応答を取得
let json_result = supabase
    .functions()
    .invoke_json::<serde_json::Value, _>(
        "get-user-data",
        Some(serde_json::json!({"user_id": 123}))
    )
    .await?;

println!("User data: {:?}", json_result);

// テキスト応答を取得
let text_result = supabase
    .functions()
    .invoke_text::<serde_json::Value>(
        "generate-text",
        Some(serde_json::json!({"prompt": "Hello world"}))
    )
    .await?;

println!("Generated text: {}", text_result);

// タイムアウトを設定
let options = FunctionOptions {
    timeout_seconds: Some(30),
    ..Default::default()
};

// 詳細な応答情報を取得
let response = supabase
    .functions()
    .invoke::<UserData, _>(
        "get-complete-user-data",
        Some(serde_json::json!({"user_id": 123})),
        Some(options)
    )
    .await?;

println!("Status code: {}", response.status);
println!("Headers: {:?}", response.headers);
println!("User data: {:?}", response.data);

// エラーハンドリング
match supabase.functions().invoke_json::<serde_json::Value, _>("function-name", Some(payload)).await {
    Ok(data) => {
        println!("Success: {:?}", data);
    },
    Err(err) => match err {
        FunctionsError::TimeoutError => {
            println!("Function timed out");
        },
        FunctionsError::FunctionError { message, status, details } => {
            println!("Function error: {} (status: {})", message, status);
            if let Some(details) = details {
                println!("Error details: {:?}", details);
            }
        },
        _ => {
            println!("Other error: {}", err);
        }
    }
}
```