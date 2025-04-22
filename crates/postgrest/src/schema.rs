//! TypeScript型定義からRustの型に変換するモジュール
//!
//! supabase gen types typescript で生成された型定義をRustの型に変換する機能を提供します。
//! schema-convert featureが有効な場合にのみ使用可能です。
//!
//! また、型安全なデータベース操作のためのトレイトも提供します。

#[cfg(feature = "schema-convert")]
use {
    convert_case::{Case, Casing},
    proc_macro2::{Span, TokenStream},
    quote::{quote, ToTokens},
    std::fs::{self, File},
    std::io::{self, Read, Write},
    std::path::{Path, PathBuf},
    typescript_type_def::{
        fields::{OptionalField, RequiredField},
        types::{Interface, Type, Union},
        TypeDefinitions,
    },
};

#[cfg(feature = "schema-convert")]
use crate::PostgrestError;

use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;

/// データベースのテーブルを表すトレイト
pub trait Table {
    /// このテーブルの名前
    fn table_name() -> &'static str;
}

/// データベース操作を型安全に行うための拡張トレイト
pub trait PostgrestClientTypeExtension {
    /// 型安全なクエリメソッド
    ///
    /// # 例
    ///
    /// ```
    /// use supabase_rust_postgrest::{PostgrestClient, Table};
    ///
    /// #[derive(serde::Serialize, serde::Deserialize, Debug)]
    /// struct User {
    ///     id: i32,
    ///     name: String,
    ///     email: String,
    /// }
    ///
    /// impl Table for User {
    ///     fn table_name() -> &'static str {
    ///         "users"
    ///     }
    /// }
    ///
    /// async fn fetch_users(client: &PostgrestClient) -> Result<Vec<User>, Box<dyn std::error::Error>> {
    ///     // 型安全なクエリ
    ///     let users = client.query_typed::<User>().execute().await?;
    ///     Ok(users)
    /// }
    /// ```
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
    client: crate::PostgrestClient,
    _phantom: PhantomData<T>,
}

/// 型安全な挿入操作ビルダー
pub struct TypedInsertBuilder<T>
where
    T: Table + Serialize,
{
    client: crate::PostgrestClient,
    values: T,
}

/// 型安全な更新操作ビルダー
pub struct TypedUpdateBuilder<T>
where
    T: Table + Serialize,
{
    client: crate::PostgrestClient,
    values: T,
}

/// 型安全な削除操作ビルダー
pub struct TypedDeleteBuilder<T>
where
    T: Table,
{
    client: crate::PostgrestClient,
    _phantom: PhantomData<T>,
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

    /// より大きい条件フィルタ
    pub fn gt(self, column: &str, value: &str) -> Self {
        Self {
            client: self.client.gt(column, value),
            _phantom: PhantomData,
        }
    }

    /// 以上条件フィルタ
    pub fn gte(self, column: &str, value: &str) -> Self {
        Self {
            client: self.client.gte(column, value),
            _phantom: PhantomData,
        }
    }

    /// より小さい条件フィルタ
    pub fn lt(self, column: &str, value: &str) -> Self {
        Self {
            client: self.client.lt(column, value),
            _phantom: PhantomData,
        }
    }

    /// 以下条件フィルタ
    pub fn lte(self, column: &str, value: &str) -> Self {
        Self {
            client: self.client.lte(column, value),
            _phantom: PhantomData,
        }
    }

    /// LIKE条件フィルタ
    pub fn like(self, column: &str, pattern: &str) -> Self {
        Self {
            client: self.client.like(column, pattern),
            _phantom: PhantomData,
        }
    }

    /// ILIKE条件フィルタ (大文字小文字を区別しない)
    pub fn ilike(self, column: &str, pattern: &str) -> Self {
        Self {
            client: self.client.ilike(column, pattern),
            _phantom: PhantomData,
        }
    }

    /// IN条件フィルタ
    pub fn in_list(self, column: &str, values: &[&str]) -> Self {
        Self {
            client: self.client.in_list(column, values),
            _phantom: PhantomData,
        }
    }

    /// NOT条件フィルタ
    pub fn not(self, column: &str, operator_with_value: &str) -> Self {
        Self {
            client: self.client.not(column, operator_with_value),
            _phantom: PhantomData,
        }
    }

    /// 並び順指定
    pub fn order(self, column: &str, direction: crate::SortOrder) -> Self {
        Self {
            client: self.client.order(column, direction),
            _phantom: PhantomData,
        }
    }

    /// 取得件数制限
    pub fn limit(self, count: i32) -> Self {
        Self {
            client: self.client.limit(count),
            _phantom: PhantomData,
        }
    }

    /// オフセット指定
    pub fn offset(self, count: i32) -> Self {
        Self {
            client: self.client.offset(count),
            _phantom: PhantomData,
        }
    }

    /// GROUP BY句
    pub fn group_by(self, columns: &str) -> Self {
        Self {
            client: self.client.group_by(columns),
            _phantom: PhantomData,
        }
    }

    /// レコード数カウント
    pub fn count(self, exact: bool) -> Self {
        Self {
            client: self.client.count(exact),
            _phantom: PhantomData,
        }
    }
}

impl<T> TypedInsertBuilder<T>
where
    T: Table + Serialize,
{
    /// 挿入操作の実行
    pub async fn execute(&self) -> Result<T, crate::PostgrestError>
    where
        T: DeserializeOwned,
    {
        let result = self.client.insert(&self.values).await?;
        let inserted: T = serde_json::from_value(result).map_err(|e| {
            crate::PostgrestError::DeserializationError(format!(
                "Failed to deserialize result: {}",
                e
            ))
        })?;
        Ok(inserted)
    }
}

impl<T> TypedUpdateBuilder<T>
where
    T: Table + Serialize,
{
    /// 更新操作の実行
    pub async fn execute(&self) -> Result<T, crate::PostgrestError>
    where
        T: DeserializeOwned,
    {
        let result = self.client.update(&self.values).await?;
        let updated: T = serde_json::from_value(result).map_err(|e| {
            crate::PostgrestError::DeserializationError(format!(
                "Failed to deserialize result: {}",
                e
            ))
        })?;
        Ok(updated)
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
    /// 削除操作の実行
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
        // 元のクライアントから新しいクライアントを作成し、テーブル名を設定
        let client = crate::PostgrestClient::new(
            &self.base_url,
            &self.api_key,
            T::table_name(),
            self.http_client.clone(),
        );

        TypedPostgrestClient {
            client,
            _phantom: PhantomData,
        }
    }

    fn insert_typed<T: Table + Serialize + DeserializeOwned>(
        &self,
        values: &T,
    ) -> Result<TypedInsertBuilder<T>, crate::PostgrestError> {
        // 元のクライアントから新しいクライアントを作成し、テーブル名を設定
        let client = crate::PostgrestClient::new(
            &self.base_url,
            &self.api_key,
            T::table_name(),
            self.http_client.clone(),
        );

        Ok(TypedInsertBuilder {
            client,
            values: serde_json::from_value(serde_json::to_value(values)?)?,
        })
    }

    fn update_typed<T: Table + Serialize + DeserializeOwned>(
        &self,
        values: &T,
    ) -> Result<TypedUpdateBuilder<T>, crate::PostgrestError> {
        // 元のクライアントから新しいクライアントを作成し、テーブル名を設定
        let client = crate::PostgrestClient::new(
            &self.base_url,
            &self.api_key,
            T::table_name(),
            self.http_client.clone(),
        );

        Ok(TypedUpdateBuilder {
            client,
            values: serde_json::from_value(serde_json::to_value(values)?)?,
        })
    }

    fn delete_typed<T: Table>(&self) -> TypedDeleteBuilder<T> {
        // 元のクライアントから新しいクライアントを作成し、テーブル名を設定
        let client = crate::PostgrestClient::new(
            &self.base_url,
            &self.api_key,
            T::table_name(),
            self.http_client.clone(),
        );

        TypedDeleteBuilder {
            client,
            _phantom: PhantomData,
        }
    }
}

/// TypeScript型定義ファイルをRustの型定義に変換するオプション
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

/// TypeScript型定義ファイルをRustの型定義に変換する
#[cfg(feature = "schema-convert")]
pub fn convert_typescript_to_rust(
    typescript_file: &Path,
    options: SchemaConvertOptions,
) -> Result<PathBuf, PostgrestError> {
    // TypeScript型定義ファイルを読み取る
    let mut file = File::open(typescript_file).map_err(|e| {
        PostgrestError::InvalidParameters(format!("Failed to open TypeScript file: {}", e))
    })?;

    let mut content = String::new();
    file.read_to_string(&mut content).map_err(|e| {
        PostgrestError::InvalidParameters(format!("Failed to read TypeScript file: {}", e))
    })?;

    // TypeScript型定義をパースする
    let type_defs = TypeDefinitions::parse(&content).map_err(|e| {
        PostgrestError::InvalidParameters(format!("Failed to parse TypeScript: {}", e))
    })?;

    // Rust型定義に変換する
    let rust_code = convert_type_definitions_to_rust(&type_defs, &options)?;

    // 出力ディレクトリを作成する
    fs::create_dir_all(&options.output_dir).map_err(|e| {
        PostgrestError::InvalidParameters(format!("Failed to create output directory: {}", e))
    })?;

    // Rust型定義を保存する
    let output_file = options
        .output_dir
        .join(format!("{}.rs", options.module_name));
    let mut file = File::create(&output_file).map_err(|e| {
        PostgrestError::InvalidParameters(format!("Failed to create output file: {}", e))
    })?;

    file.write_all(rust_code.as_bytes()).map_err(|e| {
        PostgrestError::InvalidParameters(format!("Failed to write output file: {}", e))
    })?;

    Ok(output_file)
}

/// TypeScript型定義構造体をRustコードに変換する
#[cfg(feature = "schema-convert")]
fn convert_type_definitions_to_rust(
    type_defs: &TypeDefinitions,
    options: &SchemaConvertOptions,
) -> Result<String, PostgrestError> {
    let mut rust_code = String::new();

    // ヘッダーとドキュメンテーション
    rust_code.push_str("//! Generated database schema types\n");
    rust_code.push_str("//! This file is automatically generated from Supabase schema.\n");
    rust_code.push_str("//! Do not edit this file directly.\n\n");

    // インポート
    rust_code.push_str("use serde::{Deserialize, Serialize};\n");
    rust_code.push_str("use std::collections::HashMap;\n\n");

    // 型定義
    for interface in &type_defs.interfaces {
        rust_code.push_str(&convert_interface_to_rust(interface, options)?);
        rust_code.push_str("\n\n");
    }

    // 型エイリアスの処理
    for type_alias in &type_defs.type_aliases {
        let name = pascal_case(&type_alias.name);
        let rust_type = get_rust_type_for_typescript_type(&type_alias.value, options)?;

        rust_code.push_str(&format!(
            "/// TypeScript: type {} = {}\n",
            type_alias.name, type_alias.value
        ));

        // 複雑な型エイリアスの場合は処理が必要
        if let Type::Union(_) = &type_alias.value {
            let mut inner_rust_code = String::new();
            add_union_to_rust(
                &type_alias.value,
                &mut inner_rust_code,
                &name,
                &options.derives,
            )?;
            rust_code.push_str(&inner_rust_code);
        } else {
            // 単純な型エイリアスの場合
            let derives = options.derives.join(", ");
            rust_code.push_str(&format!("#[derive({})]\n", derives));
            rust_code.push_str(&format!("pub type {} = {};\n", name, rust_type));
        }

        rust_code.push_str("\n");
    }

    Ok(rust_code)
}

/// TypeScriptのインターフェースをRustの構造体に変換する
#[cfg(feature = "schema-convert")]
fn convert_interface_to_rust(
    interface: &Interface,
    options: &SchemaConvertOptions,
) -> Result<String, PostgrestError> {
    let struct_name = pascal_case(&interface.name);
    let derives = options.derives.join(", ");

    let mut rust_code = String::new();

    // ドキュメント
    rust_code.push_str(&format!("/// TypeScript Interface: {}\n", interface.name));

    // デリバティブ
    rust_code.push_str(&format!("#[derive({})]\n", derives));

    // 構造体定義
    rust_code.push_str(&format!("pub struct {} {{\n", struct_name));

    // 必須フィールド
    for field in &interface.required_fields {
        let field_name = snake_case(&field.name);
        let rust_type = get_rust_type_for_typescript_type(&field.value, options)?;

        rust_code.push_str(&format!(
            "    /// TypeScript: {}: {}\n",
            field.name, field.value
        ));
        rust_code.push_str(&format!("    #[serde(rename = \"{}\")]\n", field.name));
        rust_code.push_str(&format!("    pub {}: {},\n\n", field_name, rust_type));
    }

    // オプショナルフィールド
    for field in &interface.optional_fields {
        let field_name = snake_case(&field.name);
        let rust_type = get_rust_type_for_typescript_type(&field.value, options)?;

        rust_code.push_str(&format!(
            "    /// TypeScript: {}?: {}\n",
            field.name, field.value
        ));
        rust_code.push_str(&format!("    #[serde(rename = \"{}\")]\n", field.name));

        if field.name.contains("-") || field.name.contains(".") {
            rust_code.push_str(&format!("    #[serde(rename = \"{}\")]\n", field.name));
        }

        rust_code.push_str(&format!(
            "    pub {}: Option<{}>,\n\n",
            field_name, rust_type
        ));
    }

    // 構造体の終わり
    rust_code.push_str("}\n");

    Ok(rust_code)
}

/// TypeScriptのユニオン型をRustのenumに変換する
#[cfg(feature = "schema-convert")]
fn add_union_to_rust(
    union_type: &Type,
    rust_code: &mut String,
    enum_name: &str,
    derives: &[String],
) -> Result<(), PostgrestError> {
    if let Type::Union(union) = union_type {
        let derives_str = derives.join(", ");

        // ドキュメントとデリバティブ
        rust_code.push_str(&format!("/// TypeScript Union Type\n"));
        rust_code.push_str(&format!("#[derive({})]\n", derives_str));

        // セルダ付きenumを作成する必要がある場合
        if union.union_types.iter().any(|t| match t {
            Type::String(_) | Type::Number(_) | Type::Boolean(_) | Type::Null => false,
            _ => true,
        }) {
            // serdeタグを付加
            rust_code.push_str("#[serde(tag = \"type\")]\n");

            // enum定義開始
            rust_code.push_str(&format!("pub enum {} {{\n", enum_name));

            // バリアント
            for (i, variant_type) in union.union_types.iter().enumerate() {
                let variant_name = match variant_type {
                    Type::String(s) => pascal_case(s),
                    Type::Number(n) => format!("Num{}", n),
                    Type::Boolean(b) => {
                        if *b {
                            "True".to_string()
                        } else {
                            "False".to_string()
                        }
                    }
                    Type::Null => "Null".to_string(),
                    Type::Literal(lit) => pascal_case(&format!("{}", lit)),
                    Type::Identifier(id) => pascal_case(id),
                    Type::Reference(r) => pascal_case(&r),
                    _ => format!("Variant{}", i),
                };

                rust_code.push_str(&format!("    {},\n", variant_name));
            }

            // enum定義終了
            rust_code.push_str("}\n");
        } else {
            // 単純な型のユニオンはnewtype patternを使用
            rust_code.push_str(&format!("pub enum {} {{\n", enum_name));

            for (i, variant_type) in union.union_types.iter().enumerate() {
                match variant_type {
                    Type::String(s) => {
                        rust_code.push_str(&format!("    /// {}\n", s));
                        rust_code.push_str(&format!("    String(String),\n"));
                    }
                    Type::Number(_) => {
                        rust_code.push_str("    Number(f64),\n");
                    }
                    Type::Boolean(_) => {
                        rust_code.push_str("    Boolean(bool),\n");
                    }
                    Type::Null => {
                        rust_code.push_str("    Null,\n");
                    }
                    _ => {
                        rust_code.push_str(&format!("    Variant{}(String),\n", i));
                    }
                }
            }

            rust_code.push_str("}\n");
        }
    } else {
        return Err(PostgrestError::InvalidParameters(
            "Expected union type".to_string(),
        ));
    }

    Ok(())
}

/// TypeScriptの型をRustの型に変換する
#[cfg(feature = "schema-convert")]
fn get_rust_type_for_typescript_type(
    ts_type: &Type,
    options: &SchemaConvertOptions,
) -> Result<String, PostgrestError> {
    match ts_type {
        Type::String(_) => Ok("String".to_string()),
        Type::Number(_) => Ok("f64".to_string()),
        Type::Boolean(_) => Ok("bool".to_string()),
        Type::Null => Ok("()".to_string()),
        Type::Any => Ok("serde_json::Value".to_string()),
        Type::Unknown => Ok("serde_json::Value".to_string()),
        Type::Undefined => Ok("Option<()>".to_string()),
        Type::Void => Ok("()".to_string()),
        Type::Never => Ok("!".to_string()),
        Type::Array(array_type) => {
            let inner_type = get_rust_type_for_typescript_type(array_type, options)?;
            Ok(format!("Vec<{}>", inner_type))
        }
        Type::Tuple(tuple_types) => {
            let mut tuple_elements = Vec::new();
            for t in tuple_types {
                let rust_type = get_rust_type_for_typescript_type(t, options)?;
                tuple_elements.push(rust_type);
            }
            Ok(format!("({})", tuple_elements.join(", ")))
        }
        Type::Literal(lit) => {
            Ok("String".to_string()) // リテラル型はStringにマッピング
        }
        Type::Object(obj) => Ok("HashMap<String, serde_json::Value>".to_string()),
        Type::Identifier(id) => {
            // カスタムの型マッピングを適用
            if let Some(type_mapping) = &options.type_mapping {
                if let Some(mapped_type) = type_mapping.get(id) {
                    return Ok(mapped_type.clone());
                }
            }

            // 一般的な型マッピング
            match id.as_str() {
                "string" => Ok("String".to_string()),
                "number" => Ok("f64".to_string()),
                "boolean" => Ok("bool".to_string()),
                "object" => Ok("HashMap<String, serde_json::Value>".to_string()),
                "any" => Ok("serde_json::Value".to_string()),
                "Date" => Ok("chrono::DateTime<chrono::Utc>".to_string()),
                "JSON" => Ok("serde_json::Value".to_string()),
                "Record" => Ok("HashMap<String, serde_json::Value>".to_string()),
                _ => Ok(pascal_case(id)), // 他のすべての型は同じ名前でPascalCaseに変換
            }
        }
        Type::Reference(ref_name) => {
            Ok(pascal_case(ref_name)) // 型参照もPascalCaseに変換
        }
        Type::Union(union) => {
            // 単純なオプション型の場合
            if union.union_types.len() == 2
                && union.union_types.iter().any(|t| matches!(t, Type::Null))
            {
                let non_null_type = union
                    .union_types
                    .iter()
                    .find(|t| !matches!(t, Type::Null))
                    .unwrap_or(&Type::Any);
                let inner_type = get_rust_type_for_typescript_type(non_null_type, options)?;
                Ok(format!("Option<{}>", inner_type))
            } else {
                // 複合ユニオン型はenumにマッピングする必要があるが、ここでは型の名前を返す
                // 実際のenum定義は別途生成する
                Ok("serde_json::Value".to_string()) // 複雑なユニオン型の場合は汎用型を使用
            }
        }
        Type::Intersection(_) => {
            // 交差型はRustでは直接表現できないため、汎用型を使用
            Ok("serde_json::Value".to_string())
        }
        Type::Function(_) => {
            // 関数型はRustでは通常シリアライズしないため、汎用型を使用
            Ok("serde_json::Value".to_string())
        }
        _ => Err(PostgrestError::InvalidParameters(format!(
            "Unsupported TypeScript type: {:?}",
            ts_type
        ))),
    }
}

/// PascalCase変換
#[cfg(feature = "schema-convert")]
fn pascal_case(s: &str) -> String {
    s.to_case(Case::Pascal)
}

/// snake_case変換
#[cfg(feature = "schema-convert")]
fn snake_case(s: &str) -> String {
    s.to_case(Case::Snake)
}

/// CLIからの型生成コマンドを処理する
#[cfg(feature = "schema-convert")]
pub fn generate_rust_from_typescript_cli(
    input_file: &str,
    output_dir: Option<&str>,
    module_name: Option<&str>,
) -> Result<(), PostgrestError> {
    let options = SchemaConvertOptions {
        output_dir: output_dir
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("src/generated")),
        module_name: module_name
            .map(String::from)
            .unwrap_or_else(|| "schema".to_string()),
        ..Default::default()
    };

    let input_path = PathBuf::from(input_file);
    let output_path = convert_typescript_to_rust(&input_path, options)?;

    println!("✅ Successfully generated Rust types at: {:?}", output_path);
    Ok(())
}

/// スキーマの生成用モジュールが無効な場合のスタブ
#[cfg(not(feature = "schema-convert"))]
pub fn generate_rust_from_typescript_cli(
    _input_file: &str,
    _output_dir: Option<&str>,
    _module_name: Option<&str>,
) -> Result<(), crate::PostgrestError> {
    Err(crate::PostgrestError::InvalidParameters(
        "schema-convert feature is not enabled. Enable it with --features schema-convert"
            .to_string(),
    ))
}
