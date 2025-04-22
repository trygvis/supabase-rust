//! Supabase TypeScript型定義をRustの型定義に変換するCLIツール
//!
//! 使用例:
//! ```
//! cargo run --features schema-convert --bin supabase-gen-rust -- \
//!     --input-file ./supabase/types.ts \
//!     --output-dir ./src/generated \
//!     --module-name schema
//! ```

use std::path::PathBuf;
use supabase_rust_postgrest::schema::generate_rust_from_typescript_cli;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    let mut input_file = None;
    let mut output_dir = None;
    let mut module_name = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_usage();
                return;
            }
            "--input-file" | "-i" => {
                if i + 1 < args.len() {
                    input_file = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: Missing value for --input-file");
                    print_usage();
                    return;
                }
            }
            "--output-dir" | "-o" => {
                if i + 1 < args.len() {
                    output_dir = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: Missing value for --output-dir");
                    print_usage();
                    return;
                }
            }
            "--module-name" | "-m" => {
                if i + 1 < args.len() {
                    module_name = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: Missing value for --module-name");
                    print_usage();
                    return;
                }
            }
            _ => {
                eprintln!("Error: Unknown argument {}", args[i]);
                print_usage();
                return;
            }
        }
    }

    let input_file = match input_file {
        Some(path) => path,
        None => {
            eprintln!("Error: --input-file is required");
            print_usage();
            return;
        }
    };

    match generate_rust_from_typescript_cli(
        &input_file,
        output_dir.as_deref(),
        module_name.as_deref(),
    ) {
        Ok(_) => {
            println!("Successfully generated Rust types from TypeScript types.");
        }
        Err(e) => {
            eprintln!("Error generating Rust types: {}", e);
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    println!("Usage: supabase-gen-rust [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --input-file, -i <FILE>     TypeScript型定義ファイル（必須）");
    println!("  --output-dir, -o <DIR>      出力ディレクトリ（デフォルト: src/generated）");
    println!("  --module-name, -m <NAME>    モジュール名（デフォルト: schema）");
    println!("  --help, -h                  ヘルプを表示");
    println!();
    println!("Example:");
    println!("  supabase-gen-rust --input-file ./supabase/types.ts --output-dir ./src/generated --module-name schema");
}
