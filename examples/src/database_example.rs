use supabase_rust::prelude::*;
use dotenv::dotenv;
use std::env;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
struct Task {
    id: Option<Uuid>,
    title: String,
    description: Option<String>,
    status: String,
    priority: i32,
    due_date: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    user_id: String,
}

impl Default for Task {
    fn default() -> Self {
        Self {
            id: None,
            title: String::new(),
            description: None,
            status: "pending".to_string(),
            priority: 1,
            due_date: None,
            created_at: None,
            updated_at: None,
            user_id: String::new(),
        }
    }
}

mod advanced_examples {
    use supabase_rust::prelude::*;
    use serde::{Deserialize, Serialize};
    use std::fs;
    use std::env;
    
    // 高度なPostgreSQL機能の例
    pub async fn run_advanced_examples() -> Result<(), Box<dyn std::error::Error>> {
        // Supabaseのクレデンシャルを環境変数から取得
        let supabase_url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
        let supabase_key = env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set");
        
        // Supabaseクライアントの初期化
        let supabase = Supabase::new(&supabase_url, &supabase_key);
        
        println!("\n=== 高度なPostgreSQL機能の例 ===\n");
        
        // 結合クエリの例
        run_join_queries(&supabase).await?;
        
        // 全文検索の例
        run_text_search(&supabase).await?;
        
        // CSVエクスポートの例
        run_csv_export(&supabase).await?;
        
        Ok(())
    }
    
    #[derive(Debug, Serialize, Deserialize)]
    struct Post {
        id: i32,
        title: String,
        content: String,
        #[serde(rename = "user_id")]
        user_id: i32,
        #[serde(rename = "created_at")]
        created_at: String,
        user: Option<User>,
        comments: Option<Vec<Comment>>,
    }
    
    #[derive(Debug, Serialize, Deserialize)]
    struct User {
        id: i32,
        name: String,
        email: String,
    }
    
    #[derive(Debug, Serialize, Deserialize)]
    struct Comment {
        id: i32,
        content: String,
        #[serde(rename = "post_id")]
        post_id: i32,
        #[serde(rename = "user_id")]
        user_id: i32,
        #[serde(rename = "created_at")]
        created_at: String,
    }
    
    async fn run_join_queries(supabase: &Supabase) -> Result<(), Box<dyn std::error::Error>> {
        println!("結合クエリの例:");
        
        // 投稿とそれに関連するユーザー情報を取得
        let posts_with_users = supabase
            .from("posts")
            .select("*")
            .inner_join("users", "user_id", "id")
            .limit(5)
            .execute::<Post>()
            .await?;
        
        println!("Posts with users (inner join):");
        for post in &posts_with_users {
            println!("Post: {}, User: {:?}", post.title, post.user);
        }
        
        // 投稿とそれに関連するコメントを取得
        let posts_with_comments = supabase
            .from("posts")
            .select("*")
            .include("comments", "post_id", Some("*"))
            .limit(3)
            .execute::<Post>()
            .await?;
        
        println!("\nPosts with comments (include):");
        for post in &posts_with_comments {
            println!("Post: {}, Comments count: {}", 
                post.title, 
                post.comments.as_ref().map(|c| c.len()).unwrap_or(0)
            );
        }
        
        Ok(())
    }
    
    #[derive(Debug, Serialize, Deserialize)]
    struct Article {
        id: i32,
        title: String,
        content: String,
        #[serde(rename = "created_at")]
        created_at: String,
    }
    
    async fn run_text_search(supabase: &Supabase) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n全文検索の例:");
        
        // 記事の内容を全文検索
        let search_term = "database";
        let articles = supabase
            .from("articles")
            .select("*")
            .text_search("content", search_term, Some("english"))
            .execute::<Article>()
            .await?;
        
        println!("Articles containing '{}' (text search):", search_term);
        for article in &articles {
            println!("Title: {}", article.title);
            println!("Content preview: {}", &article.content[..std::cmp::min(50, article.content.len())]);
            println!("---");
        }
        
        Ok(())
    }
    
    async fn run_csv_export(supabase: &Supabase) -> Result<(), Box<dyn std::error::Error>> {
        println!("\nCSVエクスポートの例:");
        
        // ユーザーテーブルをCSVとしてエクスポート
        let csv_data = supabase
            .from("users")
            .select("id,name,email,created_at")
            .limit(100)
            .export_csv()
            .await?;
        
        // CSVデータを出力
        let preview_lines: Vec<&str> = csv_data.lines().take(5).collect();
        println!("CSV export preview (first {} lines of {} total):", 
            preview_lines.len(), 
            csv_data.lines().count()
        );
        
        for line in preview_lines {
            println!("{}", line);
        }
        
        // CSVファイルとして保存
        let file_path = "users_export.csv";
        fs::write(file_path, csv_data)?;
        println!("\nCSV data saved to: {}", file_path);
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenv().ok();
    
    println!("=== Supabase Database Examples ===");
    
    // 基本的なCRUD操作の例を実行
    basic_examples::run_basic_examples().await?;
    
    // RPC呼び出しの例を実行
    rpc_examples::run_rpc_examples().await?;
    
    // 高度なフィルタリングの例を実行
    filter_examples::run_filter_examples().await?;
    
    // 高度なPostgreSQL機能の例を実行
    println!("\n高度なPostgreSQL機能の例を実行しますか？(y/n)");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    
    if input.trim().to_lowercase() == "y" {
        advanced_examples::run_advanced_examples().await?;
    }
    
    Ok(())
}