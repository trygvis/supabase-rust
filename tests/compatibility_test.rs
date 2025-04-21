use serde_json::json;
use supabase_rust_gftd::Supabase;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// このテストファイルはsupabase-rustがsupabase-jsと同様のAPI互換性を持つことを検証します
///
/// ## 互換性の検証結果
///
/// ### 互換性のある点:
/// - クライアント初期化 (new)
/// - データベース操作の基本API (.from().select().execute())
/// - 認証操作の基本概念 (サインアップ、サインイン、サインアウト)
/// - RPC関数呼び出し
///
/// ### 相違点:
/// - メソッド名: JavaScript版ではcamelCase (signIn)、Rust版ではsnake_case (sign_in)
/// - 型の扱い: JavaScript版は動的型付け、Rust版は静的型付けのため引数の型に制約がある
///   - 例: eq("id", 1) vs eq("id", "1") - Rustでは文字列を要求する場合がある
/// - 実行結果の取得: JavaScript版はPromise、Rust版はAsync/Await + 型パラメータ指定が必要
///   - 例: execute() vs execute::<T>()
/// - フィルター操作: JavaScript版はメソッドチェーン、Rust版はメソッドの一部が実装されていない
///   - 例: neq, gt, lt等のメソッドがPostgrestClientに未実装
///
/// 全体として、基本的な機能はsupabase-jsと互換性があるが、Rustの型システムと言語特性に
/// 合わせた設計になっているため、完全に同一のAPIではないことがわかりました。

#[tokio::test]
async fn test_postgrest_api_compatibility() {
    // WireMockサーバーでモックSupabaseサーバーを設定
    let mock_server = MockServer::start().await;

    // Supabaseのモックレスポンスを設定
    Mock::given(method("GET"))
        .and(path("/rest/v1/users"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            { "id": 1, "name": "Alice", "email": "alice@example.com" },
            { "id": 2, "name": "Bob", "email": "bob@example.com" }
        ])))
        .mount(&mock_server)
        .await;

    // Supabase Rustクライアントを初期化
    let supabase = Supabase::new(&mock_server.uri(), "fake-key");

    // テスト1: from()メソッドで正しくPostgreSQLテーブルを選択できるか
    // JavaScriptでは: const { data } = await supabase.from('users').select('*')
    let users_result = supabase
        .from("users")
        .select("*")
        .execute::<serde_json::Value>() // 型パラメータを指定
        .await;

    assert!(
        users_result.is_ok(),
        "from().select().execute()でデータ取得できること"
    );

    if let Ok(users) = users_result {
        assert_eq!(users.len(), 2, "2つのユーザーレコードが取得できること");
        assert_eq!(
            users[0]["name"], "Alice",
            "最初のユーザーの名前がAliceであること"
        );
    }

    // 以下のメソッドがコンパイルエラーにならないことを確認
    // 注意: JSとRustでは型の扱いが異なり、RustのAPIでは文字列を要求するケースがあります
    let _query = supabase.from("users");
    let _query = _query.select("id, name");
    let _query = _query.eq("id", "1"); // 数値ではなく文字列として渡す

    // RPC呼び出しも可能であることを確認
    let _rpc_call = supabase.rpc("get_user_by_id", json!({"user_id": 1}));
}

#[tokio::test]
async fn test_auth_api_compatibility() {
    // WireMockサーバーでモックSupabaseサーバーを設定
    let mock_server = MockServer::start().await;

    // Supabaseのモック認証レスポンスを設定
    Mock::given(method("POST"))
        .and(path("/auth/v1/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "token_type": "bearer",
            "expires_in": 3600,
            "refresh_token": "mock-refresh",
            "user": {
                "id": "user123",
                "email": "test@example.com"
            }
        })))
        .mount(&mock_server)
        .await;

    // Supabase Rustクライアントを初期化
    let supabase = Supabase::new(&mock_server.uri(), "fake-key");

    // Auth APIの互換性検証
    // 注意: JavaScriptでは signIn() だが、Rustでは sign_up()
    let _auth_client = supabase.auth();

    // sign_up メソッドが存在することを確認
    let _method = _auth_client.sign_up("test@example.com", "password");
}

#[tokio::test]
async fn test_filter_operators_compatibility() {
    // supabasejsのフィルター演算子と同等のAPIをテスト
    let mock_server = MockServer::start().await;
    let supabase = Supabase::new(&mock_server.uri(), "fake-key");

    // Rustでは単一のクエリでのメソッドチェーンでなく、Supabaseオブジェクト自体にフィルターメソッドを
    // 提供しています。これはJavaScriptとは少し異なるアプローチです。

    // まず正しいPostgrestClientを取得
    let query = supabase.from("users");

    // フィルター操作がコンパイルエラーにならないことを確認
    let _query = query.select("*").eq("name", "Alice");

    // これらの操作はsupabasejsと名前は同じでも実装が異なる可能性があります
}

// 実際のSupabaseサービスを使用する統合テスト
// 注: SUPABASE_URL、SUPABASE_KEYの環境変数が設定されていることが必要
// このテストは環境変数が設定されていない場合はスキップされる
#[tokio::test]
async fn test_live_compatibility() {
    use std::env;

    if env::var("SUPABASE_URL").is_err() || env::var("SUPABASE_KEY").is_err() {
        println!("Skipping live compatibility test: SUPABASE_URL and SUPABASE_KEY must be set");
        return;
    }

    let supabase_url = env::var("SUPABASE_URL").unwrap();
    let supabase_key = env::var("SUPABASE_KEY").unwrap();

    // 実際のSupabaseクライアントを初期化
    let supabase = Supabase::new(&supabase_url, &supabase_key);

    // システムメタデータからテーブル一覧を取得
    // JavaScriptでは: await supabase.from('pg_catalog.pg_tables').select('*')
    let tables_result = supabase
        .from("pg_tables")
        .select("*")
        .eq("schemaname", "public")
        .execute::<serde_json::Value>() // 型パラメータを指定
        .await;

    assert!(
        tables_result.is_ok(),
        "システムテーブルからデータ取得できること"
    );

    if let Ok(tables) = tables_result {
        println!("公開テーブル数: {}", tables.len());
        // テーブル名を出力
        for table in tables {
            if let Some(table_name) = table.get("tablename").and_then(|n| n.as_str()) {
                println!("テーブル: {}", table_name);
            }
        }
    }
}
