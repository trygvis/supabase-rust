//! TypeScript型定義からRustの型に変換するモジュール
//!
//! supabase gen types typescript で生成された型定義をRustの型に変換する機能を提供します。
//! schema-convert featureが有効な場合にのみ一部機能が使用可能です。
//!
//! また、型安全なデータベース操作のためのトレイトも提供します。

// 基本機能: 型安全なデータベース操作
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;
use serde_json::Value;
use crate::PostgrestError;

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
                    code: None,
                    message: Some("No records found".to_string()),
                    details: None,
                    hint: None,
                },
                // Placeholder: Determine appropriate status code
                status: reqwest::StatusCode::NOT_FOUND, // Example
            });
        }

        if results.len() > 1 {
            return Err(crate::PostgrestError::ApiError {
                details: crate::PostgrestApiErrorDetails {
                    code: None,
                    message: Some("More than one record found".to_string()),
                    details: None,
                    hint: None,
                },
                // Placeholder: Determine appropriate status code
                status: reqwest::StatusCode::INTERNAL_SERVER_ERROR, // Example
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
        let response: Vec<T> = serde_json::from_value(response_value)
            .map_err(|e| PostgrestError::DeserializationError(e.to_string()))?;

        if response.is_empty() {
            return Err(crate::PostgrestError::ApiError {
                details: crate::PostgrestApiErrorDetails {
                    code: None,
                    message: Some("No record was inserted".to_string()),
                    details: None,
                    hint: None,
                },
                // Placeholder: Determine appropriate status code
                status: reqwest::StatusCode::INTERNAL_SERVER_ERROR, // Example
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
        let response: Vec<T> = serde_json::from_value(response_value)
            .map_err(|e| PostgrestError::DeserializationError(e.to_string()))?;

        if response.is_empty() {
            return Err(crate::PostgrestError::ApiError {
                details: crate::PostgrestApiErrorDetails {
                    code: None,
                    message: Some("No record was updated".to_string()),
                    details: None,
                    hint: None,
                },
                // Placeholder: Determine appropriate status code
                status: reqwest::StatusCode::INTERNAL_SERVER_ERROR, // Example
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
            module_name: "schema".to_string(),
            derives: vec![
                "Debug".to_string(),
                "Clone".to_string(),
                "PartialEq".to_string(),
                "serde::Serialize".to_string(),
                "serde::Deserialize".to_string(),
            ],
            type_mapping: Default::default(),
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
    let base_type = match ts_type {
        TypeExpr::Ident(TsIdent { name, .. }) => {
            // Check custom mapping first
            if let Some(rust_type_str) = options.type_mapping.get(name.as_str()) {
                path(rust_type_str).into_token_stream()
            } else {
                // Default mapping
                match name.as_str() {
                    "string" => quote! { String },
                    "number" => quote! { f64 }, // Or i64? Needs context or config. Defaulting to f64.
                    "boolean" => quote! { bool },
                    "Date" => quote! { chrono::DateTime<chrono::Utc> }, // Assuming chrono
                    "Json" => quote! { serde_json::Value },
                    "any" | "unknown" => quote! { serde_json::Value }, // Map any/unknown to Value
                    // Add other basic types or known Supabase types (like Uuid, Timestamptz etc. if needed)
                    _ => {
                         // Assume it's a generated enum or struct (PascalCase)
                        let type_ident = ident(name);
                         quote! { #type_ident }
                    }
                }
            }
        }
        TypeExpr::Array(inner_type) => {
            let inner_rust_type = map_ts_type_to_rust(inner_type, options, false)?; // Inner type of vec is never optional itself
            quote! { Vec<#inner_rust_type> }
        }
        TypeExpr::TypeOperator(TypeOperator { operator, type_expr }) => {
            // Handle cases like `string | null`
            if let (ts_type::Operator::Union, TypeExpr::Tuple(elements)) = (operator, &**type_expr) {
                 let non_null_types: Vec<_> = elements.iter().filter(|t| !matches!(t, TypeExpr::Literal(TsLiteral::Null))).collect();
                 if non_null_types.len() == 1 && elements.len() > non_null_types.len() {
                     // Exactly one non-null type + null -> Option<T>
                     let inner_rust_type = map_ts_type_to_rust(non_null_types[0], options, false)?;
                     quote! { Option<#inner_rust_type> }
                 } else {
                     // More complex union, map to Value for now
                     eprintln!("Warning: Complex union type {:?} mapped to serde_json::Value", ts_type);
                     quote! { serde_json::Value }
                 }
            } else {
                 eprintln!("Warning: Unsupported TypeOperator {:?} mapped to serde_json::Value", ts_type);
                 quote! { serde_json::Value }
            }
        }
        TypeExpr::Literal(TsLiteral::Null) => {
             // This case should ideally be handled by the Union logic above for Option<T>
             eprintln!("Warning: Standalone 'null' type encountered, mapping to Option<()> (likely incorrect context)");
             quote! { Option<()> }
        }
        _ => {
            eprintln!("Warning: Unsupported TypeScript type {:?} mapped to serde_json::Value", ts_type);
            quote! { serde_json::Value } // Fallback for unsupported types
        }
    };

    // Wrap in Option if the field itself was optional OR if the type includes null (handled above)
    if is_optional && !base_type.to_string().starts_with("Option <") {
        Ok(quote! { Option<#base_type> })
    } else {
        Ok(base_type)
    }
}

// TypeScript型定義ファイルをRustの型定義に変換する関数
#[cfg(feature = "schema-convert")]
pub fn convert_typescript_to_rust(
    typescript_file: &Path,
    options: SchemaConvertOptions,
) -> Result<PathBuf, crate::PostgrestError> {
    let mut file = File::open(typescript_file)
        .map_err(|e| PostgrestError::ApiError(format!("Failed to open TypeScript file: {}", e)))?;
    let mut typescript_content = String::new();
    file.read_to_string(&mut typescript_content)
        .map_err(|e| PostgrestError::ApiError(format!("Failed to read TypeScript file: {}", e)))?;

    let definitions = typescript_type_def::parse_str(&typescript_content)
        .map_err(|e| PostgrestError::ApiError(format!("Failed to parse TypeScript: {:?}", e)))?;

    // Find the main 'Database' interface
    let database_interface = definitions.interfaces.iter().find(|i| i.name == "Database")
        .ok_or_else(|| PostgrestError::ApiError("Could not find 'Database' interface in TypeScript definitions".to_string()))?;

    // Find the target schema (e.g., 'public') within 'Database'
    let schema_interface_type = database_interface.members.iter().find_map(|m| {
        if let Member::Property(prop) = m {
            if prop.key == options.schema_name {
                 if let Some(type_ann) = &prop.type_ann {
                     if let TypeExpr::Ident(id) = &*type_ann.type_expr {
                         // Find the interface definition referenced by this identifier
                         definitions.interfaces.iter().find(|i| i.name == id.name)
                     } else { None }
                 } else { None }
            } else { None }
        } else { None }
    }).ok_or_else(|| PostgrestError::ApiError(format!("Could not find schema '{}' in Database interface", options.schema_name)))?;


    let mut generated_items: Vec<TokenStream> = Vec::new();

    // Process Tables, Views, Enums within the schema
    for member in &schema_interface_type.members {
         if let Member::Property(prop) = member {
             let category_name = &prop.key; // Should be "Tables", "Views", or "Enums"
             if let Some(type_ann) = &prop.type_ann {
                 if let TypeExpr::Ident(id) = &*type_ann.type_expr {
                      // Find the definition for this category (e.g., the interface containing table definitions)
                     let category_def = definitions.interfaces.iter().find(|i| i.name == id.name)
                          .ok_or_else(|| PostgrestError::ApiError(format!("Could not find definition for category '{}'", category_name)))?;

                     match category_name.as_str() {
                         "Tables" | "Views" => {
                             // Process each table/view within the category
                             for table_member in &category_def.members {
                                 if let Member::Property(table_prop) = table_member {
                                     let table_ts_name = &table_prop.key; // Original TS table/view name (usually snake_case)
                                     let struct_name_str = pascal_case(table_ts_name); // Convert to PascalCase for Rust struct
                                     let struct_ident = ident(&struct_name_str);

                                     if let Some(table_type_ann) = &table_prop.type_ann {
                                         if let TypeExpr::Ident(table_id) = &*table_type_ann.type_expr {
                                             // Find the actual interface definition for this table/view
                                             let table_interface = definitions.interfaces.iter().find(|i| i.name == table_id.name)
                                                 .ok_or_else(|| PostgrestError::ApiError(format!("Could not find interface definition for table/view '{}'", table_ts_name)))?;

                                             let mut struct_fields = Vec::new();
                                             for field_member in &table_interface.members {
                                                 if let Member::Property(field_prop) = field_member {
                                                     let field_ts_name = &field_prop.key;
                                                     let field_rust_name_str = snake_case(field_ts_name);
                                                     let field_ident = ident(&field_rust_name_str);
                                                     let is_optional = field_prop.optional;

                                                     if let Some(field_type_ann) = &field_prop.type_ann {
                                                          let field_rust_type = map_ts_type_to_rust(&field_type_ann.type_expr, &options, is_optional)?;
                                                          let serde_rename = if field_ts_name != &field_rust_name_str {
                                                               quote! { #[serde(rename = #field_ts_name)] }
                                                          } else { quote! {} };

                                                         struct_fields.push(quote! {
                                                             #serde_rename
                                                             pub #field_ident: #field_rust_type,
                                                         });
                                                     } else {
                                                          eprintln!("Warning: Field '{}' in table '{}' has no type annotation, skipping.", field_ts_name, table_ts_name);
                                                     }
                                                 }
                                             }

                                             let derives: Vec<syn::Path> = options.derives.iter().map(|s| path(s)).collect();
                                             let table_name_literal = table_ts_name.as_str(); // Use original name for table_name()

                                             generated_items.push(quote! {
                                                 #[derive(#(#derives),*)]
                                                 pub struct #struct_ident {
                                                     #(#struct_fields)*
                                                 }

                                                 impl crate::schema::Table for #struct_ident {
                                                     fn table_name() -> &'static str {
                                                         #table_name_literal
                                                     }
                                                 }
                                             });
                                         }
                                     }
                                 }
                             }
                         }
                         "Enums" => {
                             // Process each enum within the category
                             for enum_member in &category_def.members {
                                 if let Member::Property(enum_prop) = enum_member {
                                     let enum_ts_name = &enum_prop.key; // Original TS enum name
                                     let enum_name_str = pascal_case(enum_ts_name);
                                     let enum_ident = ident(&enum_name_str);

                                     if let Some(enum_type_ann) = &enum_prop.type_ann {
                                         if let TypeExpr::Ident(enum_id) = &*enum_type_ann.type_expr {
                                              // Find the actual type alias definition for this enum
                                             let enum_alias = definitions.type_aliases.iter().find(|a| a.name == enum_id.name)
                                                 .ok_or_else(|| PostgrestError::ApiError(format!("Could not find type alias definition for enum '{}'", enum_ts_name)))?;

                                             if let TypeExpr::TypeOperator(TypeOperator { operator: ts_type::Operator::Union, type_expr }) = &enum_alias.type_expr {
                                                  if let TypeExpr::Tuple(variants) = &**type_expr {
                                                     let mut enum_variants = Vec::new();
                                                     for variant in variants {
                                                         if let TypeExpr::Literal(TsLiteral::String(variant_name)) = variant {
                                                             let variant_ident = ident(&pascal_case(variant_name)); // PascalCase for Rust enum variant
                                                             let serde_rename = if variant_name != &pascal_case(variant_name) {
                                                                 quote! { #[serde(rename = #variant_name)] }
                                                             } else { quote! {} };
                                                             enum_variants.push(quote! {
                                                                 #serde_rename
                                                                 #variant_ident,
                                                             });
                                                         } else {
                                                              eprintln!("Warning: Non-string literal found in enum '{}', skipping variant: {:?}", enum_ts_name, variant);
                                                         }
                                                     }

                                                     // Add common derives for enums
                                                     let mut enum_derives = options.derives.clone();
                                                     for d in ["Eq", "serde::Serialize", "serde::Deserialize"] { // Ensure basic derives
                                                          if !enum_derives.contains(&d.to_string()) {
                                                              enum_derives.push(d.to_string());
                                                          }
                                                     }
                                                     let enum_derives_path: Vec<syn::Path> = enum_derives.iter().map(|s| path(s)).collect();


                                                     generated_items.push(quote! {
                                                         #[derive(#(#enum_derives_path),*)]
                                                         // Add rename_all if needed based on TS enum value casing
                                                         // #[serde(rename_all = "camelCase")]
                                                         pub enum #enum_ident {
                                                             #(#enum_variants)*
                                                         }
                                                     });

                                                 } else {
                                                      eprintln!("Warning: Enum type alias '{}' is not a simple union of string literals.", enum_ts_name);
                                                 }
                                             } else {
                                                 eprintln!("Warning: Enum type alias '{}' is not a TypeOperator::Union.", enum_ts_name);
                                             }
                                         }
                                     }
                                 }
                             }
                         }
                         _ => {
                              eprintln!("Warning: Unknown category '{}' found in schema definition.", category_name);
                         }
                     }
                 }
             }
         }
    }


    // Combine generated items into a single TokenStream
    let final_code = quote! {
        #![allow(unused_imports, non_camel_case_types, non_snake_case)] // Suppress warnings for generated code
        use super::{Table}; // Assuming Table trait is in the parent module (schema.rs)
        use serde::{Serialize, Deserialize}; // Ensure serde is in scope
        // Add chrono if Date was mapped
        // use chrono::{DateTime, Utc};

        #(#generated_items)*
    };

    // Format the generated code using syn::prettyplease or rustfmt crate if added as dependency
    // let formatted_code = prettyplease::unparse(&syn::parse2(final_code).unwrap());
    let code_string = final_code.to_string(); // Using basic to_string for now


    // Ensure output directory exists
    fs::create_dir_all(&options.output_dir).map_err(|e| {
        PostgrestError::ApiError(format!("Failed to create output directory: {}", e))
    })?;

    // Construct output file path (e.g., ./src/generated/schema.rs)
    let output_file_name = format!("{}.rs", options.module_name);
    let output_path = options.output_dir.join(output_file_name);

    // Write the generated code to the file
    let mut output_file = File::create(&output_path)
        .map_err(|e| PostgrestError::ApiError(format!("Failed to create output file: {}", e)))?;
    output_file.write_all(code_string.as_bytes())
        .map_err(|e| PostgrestError::ApiError(format!("Failed to write Rust code: {}", e)))?;

    println!("Successfully generated Rust types at: {:?}", output_path);
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
    Err(crate::PostgrestError::ApiError {
        details: crate::PostgrestApiErrorDetails {
            code: None,
            message: Some(
                "Schema conversion feature is not enabled. Enable with the 'schema-convert' feature."
                    .to_string(),
            ),
            details: None,
            hint: None,
        },
        // Placeholder: Determine appropriate status code
        status: reqwest::StatusCode::NOT_IMPLEMENTED, // Example
    })
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
