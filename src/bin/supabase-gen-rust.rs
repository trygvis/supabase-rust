#![cfg(feature = "schema-convert")]

use clap::{App, Arg};
use std::path::PathBuf;
use supabase_rust_postgrest::schema::generate_rust_from_typescript_cli;

fn main() {
    let matches = App::new("supabase-gen-rust")
        .version("0.1.0")
        .author("Author")
        .about("Generate Rust types from TypeScript definitions")
        .arg(
            Arg::with_name("typescript_file")
                .short("t")
                .long("typescript")
                .value_name("FILE")
                .help("TypeScript definition file")
                .required(true),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("FILE")
                .help("Output Rust file")
                .required(true),
        )
        .get_matches();

    let typescript_file = matches.value_of("typescript_file").unwrap();
    let output_file = matches.value_of("output").unwrap();

    generate_rust_from_typescript_cli(typescript_file, output_file);
} 