# Supabase Rust サンプル

このディレクトリには、Supabase Rust クライアントライブラリを使用するためのサンプルコードが含まれています。

## 前提条件

- Rust (1.56.0以降)
- Cargo
- [Supabaseアカウント](https://app.supabase.com)とプロジェクト

## 設定

### 環境変数の設定

1. `.env`ファイルを作成し、以下の内容を追加します：

```
SUPABASE_URL=https://your-project-id.supabase.co
SUPABASE_KEY=your-anon-key
```

2. `your-project-id`と`your-anon-key`を、あなたのSupabaseプロジェクトの値に置き換えてください。これらの値は、Supabaseプロジェクトのダッシュボードの「Settings」→「API」で確認できます。

3. データベーススキーマを設定します。`schema`ディレクトリ内のSQLファイルを使用してSupabaseのSQLエディタでテーブルを作成してください。詳細は`schema/README.md`を参照してください。

### 環境変数の直接設定（.envファイルの代わり）

ターミナルで直接環境変数を設定する場合は以下のコマンドを使用します：

**Linuxまたは macOS:**
```bash
export SUPABASE_URL=https://your-project-id.supabase.co
export SUPABASE_KEY=your-anon-key
cargo run --bin database_example
```

**Windows (CMD):**
```cmd
set SUPABASE_URL=https://your-project-id.supabase.co
set SUPABASE_KEY=your-anon-key
cargo run --bin database_example
```

**Windows (PowerShell):**
```powershell
$env:SUPABASE_URL="https://your-project-id.supabase.co"
$env:SUPABASE_KEY="your-anon-key"
cargo run --bin database_example
```

## サンプルの実行

各サンプルは以下のコマンドで実行できます：

```bash
# 認証サンプル
cargo run --bin auth_example

# データベースサンプル
cargo run --bin database_example

# ストレージサンプル
cargo run --bin storage_example

# リアルタイムサンプル
cargo run --bin realtime_example

# PostgreRESTサンプル
cargo run --bin postgrest_example

# Edge Functions サンプル
cargo run --bin functions_example
```

## サンプルの説明

### auth_example

ユーザー認証の基本機能をデモします：

- ユーザー登録（サインアップ）
- ログイン（サインイン）
- ユーザー情報の取得
- メタデータの更新
- ログアウト（サインアウト）

### database_example

Supabaseデータベースの基本操作をデモします：

- データの作成（INSERT）
- データの読み取り（SELECT）
- データの更新（UPDATE）
- データの削除（DELETE）

### storage_example

Supabaseストレージの機能をデモします：

- バケットの作成と管理
- ファイルのアップロード
- ファイルのダウンロード
- 画像変換機能

### realtime_example

Supabaseリアルタイム機能をデモします：

- リアルタイムサブスクリプション
- データベース変更の購読
- フィルタリングされたサブスクリプション

### postgrest_example

PostgreRESTの高度な機能をデモします：

- 複雑なクエリ
- フィルタリングとソート
- トランザクション
- 関連データの取得

### functions_example

Edge Functionsの使用をデモします：

- 関数の呼び出し
- 認証付き関数の呼び出し
- カスタムヘッダーの使用

## トラブルシューティング

- **APIキーのエラー**: 環境変数が正しく設定されていることを確認してください。
- **テーブルがないエラー**: `schema/create_tables.sql`を実行してデータベーススキーマを作成してください。
- **Edge Function エラー**: Supabaseダッシュボードで必要な関数を作成してください。

## 注意事項

- これらのサンプルはテスト目的で使用されます。本番環境では適切なセキュリティ対策を講じてください。
- テストユーザーが作成されますが、これは一時的なものです。実際のユーザーデータには影響しません。 