# supabase-rust と supabase-js の互換性

このドキュメントでは、supabase-rust と supabase-js の互換性の状況を記載しています。

## バージョン互換性

| supabase-rust | supabase-js 互換バージョン |
|---------------|---------------------------|
| 0.1.0         | 2.x                       |

## 機能互換性の現状

| 機能 | supabase-js (v2.x) | supabase-rust (0.1.0) | 互換性状況 |
|------|-------------------|----------------------|------------|
| **Database** | ✅ | 🟡 | 基本的なクエリビルダー、フィルタリング機能を実装済み。高度なフィルター、関係クエリは一部制限あり |
| **Auth** | ✅ | 🟡 | 基本認証（サインアップ、サインイン）実装済み。OAuth、MFA等は未実装 |
| **Storage** | ✅ | 🟡 | 基本的なアップロード、ダウンロード、一覧取得機能を実装済み |
| **Realtime** | ✅ | 🟡 | 基本的なチャネル購読機能を実装済み。高度な機能は一部制限あり |
| **Edge Functions** | ✅ | 🟡 | 基本的な呼び出し機能を実装済み |
| **RLS** | ✅ | 🟡 | サポート済み（サーバー側機能に依存） |
| **pgvector** | ✅ | ❌ | 未実装 |

✅ = 完全実装, 🟡 = 部分的実装, ❌ = 未実装

## API互換性の詳細

### Database

```rust
// supabase-rust
let data = supabase
    .from("table")
    .select("*")
    .eq("column", "value")
    .order("created_at", Some(Direction::Descending))
    .limit(10)
    .execute()
    .await?;
```

```javascript
// supabase-js
const data = await supabase
    .from('table')
    .select('*')
    .eq('column', 'value')
    .order('created_at', { ascending: false })
    .limit(10)
```

### Auth

```rust
// supabase-rust
let auth_response = supabase
    .auth()
    .sign_in_with_password("user@example.com", "password123")
    .await?;
```

```javascript
// supabase-js
const { data, error } = await supabase.auth.signInWithPassword({
  email: 'user@example.com',
  password: 'password123',
})
```

### Storage

```rust
// supabase-rust
let upload_result = supabase
    .storage()
    .from("bucket-name")
    .upload("file.txt", file_data, Some(FileOptions::new()))
    .await?;
```

```javascript
// supabase-js
const { data, error } = await supabase
    .storage
    .from('bucket-name')
    .upload('file.txt', fileData, {
      cacheControl: '3600',
      upsert: false
    })
```

## 今後の実装予定

以下の機能は今後のリリースで追加予定です：

1. OAuth認証サポート
2. MFA（多要素認証）サポート
3. pgvectorサポート
4. 高度なRealtimeフィルタリング
5. ストレージポリシー管理

## 既知の制限事項

1. TypeScriptの型推論に相当する機能はまだ限定的です
2. supabase-jsの全てのヘルパー関数が実装されているわけではありません
3. エラーメッセージの形式がsupabase-jsと完全に一致するとは限りません

## フィードバック

互換性に関する問題を発見した場合は、[Issues](https://github.com/your-username/supabase-rust/issues)に報告してください。