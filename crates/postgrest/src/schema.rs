//! TypeScript型定義からRustの型に変換するモジュール
//!
//! supabase gen types typescript で生成された型定義をRustの型に変換する機能を提供します。
//! schema-convert featureが有効な場合にのみ一部機能が使用可能です。
//!
//! また、型安全なデータベース操作のためのトレイトも提供します。

// 基本機能: 型安全なデータベース操作
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;
use std::path::PathBuf;

// TypeScript変換関連の機能
#[cfg(feature = "schema-convert")]
use {
    convert_case::{Case, Casing},
    proc_macro2::{Span, TokenStream},
    quote::{quote, ToTokens},
    std::fs::{self, File},
    std::io::{self, Read, Write},
    std::path::Path,
    typescript_type_def::{
        fields::{OptionalField, RequiredField},
        types::{Interface, Type, Union},
        TypeDefinitions,
    },
};

#[cfg(feature = "schema-convert")]
use crate::PostgrestError;

/// データベースのテーブルを表すトレイト
pub trait Table {
    /// このテーブルの名前
    fn table_name() -> &'static str;
}

/// データベース操作を型安全に行うための拡張トレイト
pub trait PostgrestClientTypeExtension {
    /// 型安全なクエリメソッド
    fn query_typed<T: Table + DeserializeOwned>(&self) -> TypedPostgrestClient<T>;

    /// 挿入操作のための型安全なメソッド
    fn insert_typed<T: Table + Serialize + DeserializeOwned>(
        &self,
        values: &T,
    ) -> Result<TypedInsertBuilder<T>, crate::PostgrestError>;

    /// 更新操作のための型安全なメソッド
    fn update_typed<T: Table + Serialize + DeserializeOwned>(
        &self,
        values: &T,
    ) -> Result<TypedUpdateBuilder<T>, crate::PostgrestError>;

    /// 削除操作のための型安全なメソッド
    fn delete_typed<T: Table>(&self) -> TypedDeleteBuilder<T>;
}

/// 型安全なPostgREST APIクライアント
pub struct TypedPostgrestClient<T>
where
    T: Table,
{
    pub(crate) client: crate::PostgrestClient,
    pub(crate) _phantom: PhantomData<T>,
}

/// 型安全な挿入操作ビルダー
pub struct TypedInsertBuilder<T>
where
    T: Table + Serialize,
{
    pub(crate) client: crate::PostgrestClient,
    pub(crate) values: T,
}

/// 型安全な更新操作ビルダー
pub struct TypedUpdateBuilder<T>
where
    T: Table + Serialize,
{
    pub(crate) client: crate::PostgrestClient,
    pub(crate) values: T,
}

/// 型安全な削除操作ビルダー
pub struct TypedDeleteBuilder<T>
where
    T: Table,
{
    pub(crate) client: crate::PostgrestClient,
    pub(crate) _phantom: PhantomData<T>,
}

impl<T> TypedPostgrestClient<T>
where
    T: Table + DeserializeOwned,
{
    /// 型安全なクエリ実行
    pub async fn execute(&self) -> Result<Vec<T>, crate::PostgrestError> {
        self.client.execute().await
    }

    /// 型安全な単一レコード取得
    pub async fn single(&self) -> Result<T, crate::PostgrestError> {
        let mut results: Vec<T> = self.client.execute().await?;

        if results.is_empty() {
            return Err(crate::PostgrestError::ApiError(
                "No records found".to_string(),
            ));
        }

        if results.len() > 1 {
            return Err(crate::PostgrestError::ApiError(
                "More than one record found".to_string(),
            ));
        }

        Ok(results.remove(0))
    }

    /// 選択するカラムを指定
    pub fn select(self, columns: &str) -> Self {
        Self {
            client: self.client.select(columns),
            _phantom: PhantomData,
        }
    }

    /// 等価条件フィルタ
    pub fn eq(self, column: &str, value: &str) -> Self {
        Self {
            client: self.client.eq(column, value),
            _phantom: PhantomData,
        }
    }
}

impl<T> TypedInsertBuilder<T>
where
    T: Table + Serialize,
{
    /// 型安全な挿入実行
    pub async fn execute(&self) -> Result<T, crate::PostgrestError>
    where
        T: DeserializeOwned,
    {
        let response: Vec<T> = self.client.insert(&self.values).await?;
        if response.is_empty() {
            return Err(crate::PostgrestError::ApiError(
                "No record was inserted".to_string(),
            ));
        }
        Ok(response[0].clone())
    }
}

impl<T> TypedUpdateBuilder<T>
where
    T: Table + Serialize,
{
    /// 型安全な更新実行
    pub async fn execute(&self) -> Result<T, crate::PostgrestError>
    where
        T: DeserializeOwned,
    {
        let response: Vec<T> = self.client.update(&self.values).await?;
        if response.is_empty() {
            return Err(crate::PostgrestError::ApiError(
                "No record was updated".to_string(),
            ));
        }
        Ok(response[0].clone())
    }

    /// 等価条件フィルタ
    pub fn eq(mut self, column: &str, value: &str) -> Self {
        self.client = self.client.eq(column, value);
        self
    }
}

impl<T> TypedDeleteBuilder<T>
where
    T: Table,
{
    /// 型安全な削除実行
    pub async fn execute(&self) -> Result<(), crate::PostgrestError> {
        self.client.delete().await?;
        Ok(())
    }

    /// 等価条件フィルタ
    pub fn eq(mut self, column: &str, value: &str) -> Self {
        self.client = self.client.eq(column, value);
        self
    }
}

impl PostgrestClientTypeExtension for crate::PostgrestClient {
    fn query_typed<T: Table + DeserializeOwned>(&self) -> TypedPostgrestClient<T> {
        TypedPostgrestClient {
            client: self.from(&T::table_name()),
            _phantom: PhantomData,
        }
    }

    fn insert_typed<T: Table + Serialize + DeserializeOwned>(
        &self,
        values: &T,
    ) -> Result<TypedInsertBuilder<T>, crate::PostgrestError> {
        Ok(TypedInsertBuilder {
            client: self.from(&T::table_name()),
            values: values.clone(),
        })
    }

    fn update_typed<T: Table + Serialize + DeserializeOwned>(
        &self,
        values: &T,
    ) -> Result<TypedUpdateBuilder<T>, crate::PostgrestError> {
        Ok(TypedUpdateBuilder {
            client: self.from(&T::table_name()),
            values: values.clone(),
        })
    }

    fn delete_typed<T: Table>(&self) -> TypedDeleteBuilder<T> {
        TypedDeleteBuilder {
            client: self.from(&T::table_name()),
            _phantom: PhantomData,
        }
    }
}

// TypeScript型定義ファイルをRustの型定義に変換するオプション
#[cfg(feature = "schema-convert")]
pub struct SchemaConvertOptions {
    /// 出力ディレクトリ
    pub output_dir: PathBuf,
    /// モジュール名
    pub module_name: String,
    /// Rustのデリバティブ（e.g. Debug, Clone, Serialize, Deserialize）
    pub derives: Vec<String>,
    /// カスタム型マッピング
    pub type_mapping: Option<std::collections::HashMap<String, String>>,
}

#[cfg(feature = "schema-convert")]
impl Default for SchemaConvertOptions {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("./src/models"),
            module_name: "models".to_string(),
            derives: vec![
                "Debug".to_string(),
                "Clone".to_string(),
                "Serialize".to_string(),
                "Deserialize".to_string(),
            ],
            type_mapping: None,
        }
    }
}

// TypeScript型定義ファイルをRustの型定義に変換する関数
#[cfg(feature = "schema-convert")]
pub fn convert_typescript_to_rust(
    typescript_file: &Path,
    options: SchemaConvertOptions,
) -> Result<PathBuf, crate::PostgrestError> {
    let typescript_content = std::fs::read_to_string(typescript_file).map_err(|e| {
        crate::PostgrestError::ApiError(format!("Failed to read TypeScript file: {}", e))
    })?;

    // TypeScript→Rust変換ロジックはここに実装（プロジェクトの仕様に応じて）
    let rust_content = format!(
        "// Generated from {} by supabase-rust\n\n",
        typescript_file.display()
    );

    // 出力ディレクトリの作成
    fs::create_dir_all(&options.output_dir).map_err(|e| {
        crate::PostgrestError::ApiError(format!("Failed to create output directory: {}", e))
    })?;

    // 出力ファイルパスの決定
    let filename = typescript_file
        .file_name()
        .ok_or_else(|| {
            crate::PostgrestError::ApiError("Invalid TypeScript file path".to_string())
        })?
        .to_string_lossy()
        .replace(".ts", ".rs");

    let output_path = options.output_dir.join(filename);

    // ファイルに書き込み
    fs::write(&output_path, rust_content).map_err(|e| {
        crate::PostgrestError::ApiError(format!("Failed to write Rust file: {}", e))
    })?;

    Ok(output_path)
}

// CLI用エントリーポイント
#[cfg(feature = "schema-convert")]
pub fn generate_rust_from_typescript_cli(
    input_file: &str,
    output_file: &str,
) -> Result<(), crate::PostgrestError> {
    println!("Converting TypeScript to Rust: {} -> {}", input_file, output_file);
    
    let options = SchemaConvertOptions {
        output_dir: PathBuf::from(output_file).parent().unwrap_or_else(|| Path::new(".")).to_path_buf(),
        ..Default::default()
    };
    
    let _output_path = convert_typescript_to_rust(Path::new(input_file), options)?;
    println!("Conversion complete: {}", output_file);
    
    Ok(())
}

// feature無効時のダミー関数
#[cfg(not(feature = "schema-convert"))]
pub fn generate_rust_from_typescript_cli(
    _input_file: &str,
    _output_file: &str,
) -> Result<(), crate::PostgrestError> {
    Err(crate::PostgrestError::ApiError(
        "Schema conversion feature is not enabled. Enable with the 'schema-convert' feature.".to_string(),
    ))
}

// 補助関数: Pascal Case変換
#[cfg(feature = "schema-convert")]
fn pascal_case(s: &str) -> String {
    s.to_case(Case::Pascal)
}

// 補助関数: Snake Case変換
#[cfg(feature = "schema-convert")]
fn snake_case(s: &str) -> String {
    s.to_case(Case::Snake)
}
