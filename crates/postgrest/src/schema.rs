//! TypeScript型定義からRustの型に変換するモジュール
//!
//! supabase gen types typescript で生成された型定義をRustの型に変換する機能を提供します。
//! schema-convert featureが有効な場合にのみ一部機能が使用可能です。
//!
//! また、型安全なデータベース操作のためのトレイトも提供します。

// 基本機能: 型安全なデータベース操作
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use crate::PostgrestError;
use std::marker::PhantomData;

// TypeScript変換関連の機能
/*
#[cfg(feature = "schema-convert")]
use {
    convert_case::{Case, Casing},
    proc_macro2::{Ident, Span, TokenStream},
    quote::{quote, ToTokens},
    syn::{self, Member},
    typescript_type_def::{
        type_expr::{self, Ident as TsIdent, Literal as TsLiteral, TypeExpr, TypeOperator},
        Declaration, DefinitionFileOptions, TypeDef,
    },
};
*/

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
    fn insert_typed<T: Table + Serialize + DeserializeOwned + Clone>(
        &self,
        values: &T,
    ) -> Result<TypedInsertBuilder<T>, crate::PostgrestError>;

    /// 更新操作のための型安全なメソッド
    fn update_typed<T: Table + Serialize + DeserializeOwned + Clone>(
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
            return Err(crate::PostgrestError::ApiError {
                details: crate::PostgrestApiErrorDetails {
                    code: Some("PGRST116".to_string()),
                    message: Some("No rows found".to_string()),
                    details: Some(format!(
                        "Query for table '{}' returned no results.",
                        T::table_name()
                    )),
                    hint: Some(
                        "Check your query filters or ensure the table is not empty.".to_string(),
                    ),
                },
                status: reqwest::StatusCode::NOT_FOUND,
            });
        }

        if results.len() > 1 {
            return Err(crate::PostgrestError::ApiError {
                details: crate::PostgrestApiErrorDetails {
                    code: Some("PGRST116".to_string()),
                    message: Some("More than one row found".to_string()),
                    details: Some(format!("Query for table '{}' expected a single row but found {}.", T::table_name(), results.len())),
                    hint: Some("Use additional filters to ensure a unique result or use .execute() to retrieve multiple rows.".to_string()),
                },
                status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            });
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
        T: DeserializeOwned + Clone,
    {
        let response_value: Value = self.client.insert(&self.values).await?;
        let response: Vec<T> = serde_json::from_value(response_value.clone()).map_err(|e| {
            PostgrestError::DeserializationError(format!(
                "Failed to deserialize insert response: {}, value: {}",
                e, response_value
            ))
        })?;

        if response.is_empty() {
            return Err(crate::PostgrestError::ApiError {
                details: crate::PostgrestApiErrorDetails {
                    code: None,
                    message: Some("Insert operation returned no data".to_string()),
                    details: Some(format!("Insert into table '{}' did not return the expected row. Check RLS policies or insertion logic.", T::table_name())),
                    hint: None,
                },
                status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            });
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
        T: DeserializeOwned + Clone,
    {
        let response_value: Value = self.client.update(&self.values).await?;
        let response: Vec<T> = serde_json::from_value(response_value.clone()).map_err(|e| {
            PostgrestError::DeserializationError(format!(
                "Failed to deserialize update response: {}, value: {}",
                e, response_value
            ))
        })?;

        if response.is_empty() {
            return Err(crate::PostgrestError::ApiError {
                details: crate::PostgrestApiErrorDetails {
                    code: None,
                    message: Some("Update operation returned no data".to_string()),
                    details: Some(format!("Update on table '{}' did not affect any rows or return data. Check filters or RLS policies.", T::table_name())),
                    hint: None,
                },
                status: reqwest::StatusCode::NOT_FOUND,
            });
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
            client: crate::PostgrestClient::new(
                &self.base_url,
                &self.api_key,
                T::table_name(),
                self.http_client.clone(),
            ),
            _phantom: PhantomData,
        }
    }

    fn insert_typed<T: Table + Serialize + DeserializeOwned + Clone>(
        &self,
        values: &T,
    ) -> Result<TypedInsertBuilder<T>, crate::PostgrestError> {
        Ok(TypedInsertBuilder {
            client: crate::PostgrestClient::new(
                &self.base_url,
                &self.api_key,
                T::table_name(),
                self.http_client.clone(),
            ),
            values: values.clone(),
        })
    }

    fn update_typed<T: Table + Serialize + DeserializeOwned + Clone>(
        &self,
        values: &T,
    ) -> Result<TypedUpdateBuilder<T>, crate::PostgrestError> {
        Ok(TypedUpdateBuilder {
            client: crate::PostgrestClient::new(
                &self.base_url,
                &self.api_key,
                T::table_name(),
                self.http_client.clone(),
            ),
            values: values.clone(),
        })
    }

    fn delete_typed<T: Table>(&self) -> TypedDeleteBuilder<T> {
        TypedDeleteBuilder {
            client: crate::PostgrestClient::new(
                &self.base_url,
                &self.api_key,
                T::table_name(),
                self.http_client.clone(),
            ),
            _phantom: PhantomData,
        }
    }
}

// TypeScript型定義ファイルをRustの型定義に変換するオプション
/*
#[cfg(feature = "schema-convert")]
pub struct SchemaConvertOptions {
    /// 出力ディレクトリ
    pub output_dir: PathBuf,
    /// モジュール名
    pub module_name: String,
    /// Rustのデリバティブ（e.g. Debug, Clone, Serialize, Deserialize）
    pub derives: Vec<String>,
    /// カスタム型マッピング (TypeScript type name -> Rust type path)
    pub type_mapping: HashMap<String, String>,
    /// スキーマ名 (e.g., "public")
    pub schema_name: String,
}

#[cfg(feature = "schema-convert")]
impl Default for SchemaConvertOptions {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("./src/generated"),
            module_name: "types".to_string(),
            derives: vec![
                "Debug".to_string(),
                "Clone".to_string(),
                "PartialEq".to_string(),
                "Serialize".to_string(),
                "Deserialize".to_string(),
                "TypeDef".to_string(),
            ],
            type_mapping: HashMap::new(),
            schema_name: "public".to_string(),
        }
    }
}

// Helper to create syn::Ident from string
#[cfg(feature = "schema-convert")]
fn ident(s: &str) -> Ident {
    Ident::new(s, Span::call_site())
}

// Helper to create syn::Path from string
#[cfg(feature = "schema-convert")]
fn path(s: &str) -> syn::Path {
    syn::parse_str(s).expect("Failed to parse path string")
}

// Helper function to map TypeScript type expressions to Rust TokenStream
#[cfg(feature = "schema-convert")]
fn map_ts_type_to_rust(
    ts_type: &TypeExpr,
    options: &SchemaConvertOptions,
    is_optional: bool,
) -> Result<TokenStream, PostgrestError> {
    let rust_type = match ts_type {
        TypeExpr::Ident(TsIdent { name, .. }) => {
            match name.as_str() {
                "string" => quote! { String },
                "number" => quote! { f64 },
                "boolean" => quote! { bool },
                "any" | "unknown" => quote! { serde_json::Value },
                "null" => quote! { () },
                "undefined" => quote! { () },
                "Date" => quote! { chrono::DateTime<chrono::Utc> },
                custom => {
                    if let Some(mapped_type) = options.type_mapping.get(custom) {
                        let mapped_path = path(mapped_type);
                         quote! { #mapped_path }
                    } else {
                        let custom_ident = ident(&pascal_case(custom));
                        quote! { types::#custom_ident }
                    }
                }
            }
        }
        TypeExpr::Array(inner_type) => {
            let inner_rust_type = map_ts_type_to_rust(inner_type, options, false)?;
            quote! { Vec<#inner_rust_type> }
        }
        TypeExpr::TypeOperator(TypeOperator { operator, type_expr }) => {
            if let (type_expr::Operator::Union, TypeExpr::Tuple(elements)) = (operator, &**type_expr) {
                let non_null_elements: Vec<_> = elements
                    .iter()
                    .filter(|t| !matches!(t, TypeExpr::Literal(TsLiteral::Null) | TypeExpr::Ident(TsIdent{name,..}) if name == "undefined"))
                    .collect();

                if non_null_elements.len() == 1 {
                    let inner_type = map_ts_type_to_rust(non_null_elements[0], options, false)?;
                    return Ok(quote! { Option<#inner_type> });
                } else if non_null_elements.is_empty() {
                    return Ok(quote! { Option<()> });
                }
            }
            return Err(PostgrestError::InvalidParameters(format!(
                "Unsupported TypeScript type operator: {:?}",
                operator
            )));
        }
        TypeExpr::Literal(TsLiteral::Null) => {
            quote! { () }
        }
        _ => {
            return Err(PostgrestError::InvalidParameters(format!(
                "Unsupported TypeScript type expression: {:?}",
                ts_type
            )))
        }
    };

    if is_optional {
        Ok(quote! { Option<#rust_type> })
    } else {
        Ok(rust_type)
    }
}

// TypeScript型定義ファイルをRustの型定義に変換する関数
#[cfg(feature = "schema-convert")]
pub fn convert_typescript_to_rust(
    typescript_file: &Path,
    options: SchemaConvertOptions,
) -> Result<PathBuf, crate::PostgrestError> {
    let mut file = File::open(typescript_file).map_err(|e| {
        PostgrestError::ApiError {
            details: crate::PostgrestApiErrorDetails {
                code: None, message: Some("Failed to open TypeScript file".to_string()), details: Some(e.to_string()), hint: None
            },
            status: reqwest::StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;
    let mut typescript_content = String::new();
    file.read_to_string(&mut typescript_content).map_err(|e| {
        PostgrestError::ApiError {
            details: crate::PostgrestApiErrorDetails {
                code: None, message: Some("Failed to read TypeScript file".to_string()), details: Some(e.to_string()), hint: None
            },
            status: reqwest::StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    let definitions: Vec<Declaration> = vec![];

    let database_interface = definitions
        .iter()
        .find_map(|def| match def {
            Declaration::Interface(interface) if interface.name == "Database" => Some(interface),
            _ => None,
        })
        .ok_or_else(|| {
            PostgrestError::ApiError {
                 details: crate::PostgrestApiErrorDetails {
                     code: None, message: Some("Schema structure error".to_string()), details: Some("Could not find 'Database' interface in TypeScript definitions".to_string()), hint: Some("Ensure 'supabase gen types typescript' was run correctly.".to_string())
                 },
                 status: reqwest::StatusCode::BAD_REQUEST
             }
        })?;

    let schema = database_interface
        .members
        .iter()
        .find_map(|m| {
             if let Member::Property(prop) = m {
                 if let Some(key) = &prop.key {
                     let key_str = key.to_token_stream().to_string().replace('\"', "");
                     if key_str == options.schema_name {
                         if let Some(type_ann) = &prop.type_ann {
                             if let TypeExpr::Ident(id) = &*type_ann.type_expr {
                                 return Some(id.name.clone());
                             }
                         }
                     }
                 }
             }
            None
        })
         .ok_or_else(|| {
              PostgrestError::ApiError {
                 details: crate::PostgrestApiErrorDetails {
                     code: None, message: Some("Schema structure error".to_string()), details: Some(format!("Could not find schema '{}' in Database interface", options.schema_name)), hint: None
                 },
                 status: reqwest::StatusCode::BAD_REQUEST
             }
         })?;


    let mut rust_structs = TokenStream::new();
    let mut rust_enums = TokenStream::new();

    let schema_definition = definitions
         .iter()
         .find_map(|def| match def {
             Declaration::Interface(interface) if interface.name == schema => Some(interface),
             _ => None,
         })
         .ok_or_else(|| {
              PostgrestError::ApiError {
                  details: crate::PostgrestApiErrorDetails {
                      code: None, message: Some("Schema structure error".to_string()), details: Some(format!("Could not find definition for schema '{}'", schema)), hint: None
                  },
                  status: reqwest::StatusCode::BAD_REQUEST
              }
         })?;

    for category_member in &schema_definition.members {
         if let Member::Property(category_prop) = category_member {
             if let Some(category_key) = &category_prop.key {
                 let category_name = category_key.to_token_stream().to_string().replace('\"', "");
                 if category_name == "Tables" || category_name == "Views" {
                      if let Some(category_type_ann) = &category_prop.type_ann {
                          if let TypeExpr::Ident(category_id) = &*category_type_ann.type_expr {
                              let category_ts_name = &category_id.name;
                              let category_definition = definitions
                                  .iter()
                                  .find_map(|def| match def {
                                      Declaration::Interface(interface) if interface.name == *category_ts_name => Some(interface),
                                      _ => None,
                                  })
                                  .ok_or_else(|| {
                                       PostgrestError::ApiError {
                                           details: crate::PostgrestApiErrorDetails {
                                               code: None, message: Some("Schema structure error".to_string()), details: Some(format!("Could not find definition for category '{}'", category_ts_name)), hint: None
                                           },
                                           status: reqwest::StatusCode::BAD_REQUEST
                                       }
                                  })?;

                              for table_member in &category_definition.members {
                                   if let Member::Property(table_prop) = table_member {
                                       if let Some(table_key) = &table_prop.key {
                                           let table_name = table_key.to_token_stream().to_string().replace('\"', "");
                                           if let Some(table_type_ann) = &table_prop.type_ann {
                                                if let TypeExpr::Ident(table_id) = &*table_type_ann.type_expr {
                                                    let table_ts_name = &table_id.name;
                                                    let table_definition = definitions
                                                         .iter()
                                                         .find_map(|def| match def {
                                                             Declaration::Interface(interface) if interface.name == *table_ts_name => Some(interface),
                                                             _ => None,
                                                         })
                                                         .ok_or_else(|| {
                                                              PostgrestError::ApiError {
                                                                 details: crate::PostgrestApiErrorDetails {
                                                                     code: None, message: Some("Schema structure error".to_string()), details: Some(format!("Could not find interface definition for table/view '{}'", table_ts_name)), hint: None
                                                                 },
                                                                 status: reqwest::StatusCode::BAD_REQUEST
                                                             }
                                                         })?;

                                                    let rust_struct_name = ident(&pascal_case(&table_name));
                                                    let mut fields = TokenStream::new();
                                                     for field_member in &table_definition.members {
                                                         if let Member::Property(field_prop) = field_member {
                                                             if let Some(field_key) = &field_prop.key {
                                                                 let field_name_str = field_key.to_token_stream().to_string().replace('\"', "");
                                                                 let field_name = ident(&snake_case(&field_name_str));
                                                                 if let Some(field_type_ann) = &field_prop.type_ann {
                                                                      let field_type = map_ts_type_to_rust(
                                                                         &*field_type_ann.type_expr,
                                                                         &options,
                                                                         field_prop.optional.unwrap_or(false),
                                                                     )?;
                                                                     let rename_attr = if field_name_str != field_name.to_string().replace('_', "") {
                                                                         quote! { #[serde(rename = #field_name_str)] }
                                                                     } else {
                                                                         quote! {}
                                                                     };
                                                                     fields.extend(quote! {
                                                                         #rename_attr
                                                                         pub #field_name: #field_type,
                                                                     });
                                                                 }
                                                             }
                                                         }
                                                     }

                                                    let derives = options.derives.iter().map(|d| path(d));
                                                    rust_structs.extend(quote! {
                                                        #[derive(#(#derives),*)]
                                                        pub struct #rust_struct_name {
                                                            #fields
                                                        }

                                                        impl Table for #rust_struct_name {
                                                            fn table_name() -> &'static str {
                                                                #table_name
                                                            }
                                                        }
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if category_name == "Enums" {
                     if let Some(category_type_ann) = &category_prop.type_ann {
                         if let TypeExpr::Ident(category_id) = &*category_type_ann.type_expr {
                             let category_ts_name = &category_id.name;
                             let category_definition = definitions
                                 .iter()
                                 .find_map(|def| match def {
                                     Declaration::Interface(interface) if interface.name == *category_ts_name => Some(interface),
                                     _ => None,
                                 })
                                  .ok_or_else(|| {
                                       PostgrestError::ApiError {
                                           details: crate::PostgrestApiErrorDetails {
                                               code: None, message: Some("Schema structure error".to_string()), details: Some(format!("Could not find definition for category '{}'", category_ts_name)), hint: None
                                           },
                                           status: reqwest::StatusCode::BAD_REQUEST
                                       }
                                  })?;

                             for enum_member in &category_definition.members {
                                  if let Member::Property(enum_prop) = enum_member {
                                      if let Some(enum_key) = &enum_prop.key {
                                          let enum_name = enum_key.to_token_stream().to_string().replace('\"', "");
                                           if let Some(enum_type_ann) = &enum_prop.type_ann {
                                               if let TypeExpr::Ident(enum_id) = &*enum_type_ann.type_expr {
                                                   let enum_ts_name = &enum_id.name;
                                                    let enum_definition = definitions
                                                        .iter()
                                                        .find_map(|def| match def {
                                                             Declaration::TypeAlias(alias) if alias.name == *enum_ts_name => Some(alias),
                                                            _ => None,
                                                        })
                                                         .ok_or_else(|| {
                                                              PostgrestError::ApiError {
                                                                 details: crate::PostgrestApiErrorDetails {
                                                                     code: None, message: Some("Schema structure error".to_string()), details: Some(format!("Could not find type alias definition for enum '{}'", enum_ts_name)), hint: None
                                                                 },
                                                                  status: reqwest::StatusCode::BAD_REQUEST
                                                             }
                                                         })?;

                                                   let rust_enum_name = ident(&pascal_case(&enum_name));
                                                   let mut variants = TokenStream::new();
                                                    let variant_type_exprs = match &enum_definition.type_expr {
                                                        TypeExpr::TypeOperator(TypeOperator { operator: type_expr::Operator::Union, type_expr }) => {
                                                            if let TypeExpr::Tuple(exprs) = &**type_expr {
                                                                Some(exprs)
                                                            } else { None }
                                                        },
                                                        TypeExpr::Union(union_expr) => Some(&union_expr.types),
                                                        _ => None,
                                                    };

                                                    if let Some(variant_exprs) = variant_type_exprs {
                                                        for variant in variant_exprs {
                                                             if let TypeExpr::Literal(TsLiteral::String(variant_name_val)) = variant {
                                                                 let variant_name = ident(&pascal_case(&variant_name_val.value));
                                                                 let original_name = &variant_name_val.value;
                                                                 variants.extend(quote! {
                                                                     #[serde(rename = #original_name)]
                                                                     #variant_name,
                                                                 });
                                                             }
                                                         }
                                                    }

                                                   let derives = options.derives.iter().map(|d| path(d));
                                                   rust_enums.extend(quote! {
                                                       #[derive(#(#derives),*)]
                                                       pub enum #rust_enum_name {
                                                           #variants
                                                       }
                                                   });
                                               }
                                           }
                                       }
                                   }
                               }
                            }
                        }
                    }
                }
            }
        }
    }


    let output_module_name = ident(&options.module_name);
    let generated_code = quote! {
        pub mod #output_module_name {
            use super::*;
            use serde::{Serialize, Deserialize};
             use crate::Table;
             use typescript_type_def::TypeDef;

            #rust_enums
            #rust_structs
        }
    };

    let output_path = options.output_dir.join(format!("{}.rs", options.module_name));

    if let Some(parent_dir) = output_path.parent() {
        fs::create_dir_all(parent_dir).map_err(|e| {
             PostgrestError::ApiError {
                 details: crate::PostgrestApiErrorDetails {
                     code: None, message: Some("Filesystem error".to_string()), details: Some(format!("Failed to create output directory: {}", e)), hint: None
                 },
                 status: reqwest::StatusCode::INTERNAL_SERVER_ERROR
             }
        })?;
    }

    let mut output_file = File::create(&output_path).map_err(|e| {
         PostgrestError::ApiError {
            details: crate::PostgrestApiErrorDetails {
                code: None, message: Some("Filesystem error".to_string()), details: Some(format!("Failed to create output file: {}", e)), hint: None
            },
            status: reqwest::StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;
    output_file.write_all(generated_code.to_string().as_bytes()).map_err(|e| {
         PostgrestError::ApiError {
            details: crate::PostgrestApiErrorDetails {
                code: None, message: Some("Filesystem error".to_string()), details: Some(format!("Failed to write Rust code: {}", e)), hint: None
            },
            status: reqwest::StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    Ok(output_path)
}

#[cfg(feature = "schema-convert")]
pub fn generate_rust_from_typescript_cli(
    input_file: &str,
    output_file: &str,
) -> Result<(), crate::PostgrestError> {
     println!(
         "Warning: Schema conversion from TypeScript file ({}) is likely broken due to library changes.",
         input_file
     );
     println!("Consider generating types directly from Rust structs implementing TypeDef.");
    let options = SchemaConvertOptions {
         output_dir: PathBuf::from(output_file).parent().unwrap_or_else(|| Path::new(".")).to_path_buf(),
        module_name: Path::new(output_file)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        ..Default::default()
    };

    convert_typescript_to_rust(&PathBuf::from(input_file), options)?;
    Ok(())
}

#[cfg(feature = "schema-convert")]
fn pascal_case(s: &str) -> String {
    s.to_case(Case::Pascal)
}

#[cfg(feature = "schema-convert")]
fn snake_case(s: &str) -> String {
    s.to_case(Case::Snake)
}
*/
