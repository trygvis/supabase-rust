# Supabase Rust Migration

このクレートは、Supabase プロジェクト用の SeaORM マイグレーションを管理します。

## 設定

1. `.env` ファイルに `DATABASE_URL` を設定してください：

```
DATABASE_URL=postgres://postgres:postgres@localhost:54322/postgres
```

## マイグレーションコマンド

以下のコマンドを使用してマイグレーションを実行・検証できます：

### マイグレーションステータスの確認

```
cargo run -- status
```

すべてのマイグレーションの適用状態を表示します。

### マイグレーションの適用（up）

```
cargo run -- up
```

適用されていないすべてのマイグレーションを適用します。

### マイグレーションの巻き戻し（down）

```
cargo run -- down
```

適用済みのすべてのマイグレーションを巻き戻します。

### マイグレーションの再適用（fresh）

```
cargo run -- fresh
```

すべてのマイグレーションを巻き戻してから再適用します（リフレッシュ）。

### 通常の CLI モード

```
cargo run
```

または

```
cargo run -- cli
```

SeaORM の CLI インターフェースを使用して対話的にマイグレーションを管理します。

## 新しいマイグレーションの作成

1. `src` ディレクトリに新しいマイグレーションファイルを作成します：
   - 命名規則: `m{YYYYMMDD}_{HHMMSS}_{description}.rs`
   - 例: `m20240510_123000_create_users_table.rs`

2. `lib.rs` の `Migrator::migrations()` 関数に新しいマイグレーションを追加します。

## 開発用マイグレーション

サンプルマイグレーションは `examples` ディレクトリに配置されています。これらは開発やテスト用のリファレンスとして使用してください。 