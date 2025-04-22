.PHONY: all build test clean gen-types gen-types-rust watch-rs

# Default directory paths - can be overridden from command line
TYPES_OUTPUT_DIR ?= src/generated
MODULE_NAME ?= schema
SUPABASE_DIR ?= supabase
TYPES_TMP_FILE ?= $(SUPABASE_DIR)/types.ts

all: build

build:
	cargo build

test:
	cargo test

clean:
	cargo clean
	rm -f $(TYPES_TMP_FILE)
	@echo "Cleaned build artifacts and temporary type files"

# Generate TypeScript types from Supabase
gen-types:
	@echo "Generating TypeScript types from Supabase schema..."
	@mkdir -p $(SUPABASE_DIR)
	@supabase gen types typescript > $(TYPES_TMP_FILE)
	@echo "✅ TypeScript types generated at $(TYPES_TMP_FILE)"

# Generate Rust types from TypeScript definitions
gen-types-rust: gen-types
	@echo "Converting TypeScript types to Rust..."
	@mkdir -p $(TYPES_OUTPUT_DIR)
	@cargo run --features schema-convert --bin supabase-gen-rust -- \
		--input-file $(TYPES_TMP_FILE) \
		--output-dir $(TYPES_OUTPUT_DIR) \
		--module-name $(MODULE_NAME)
	@echo "✅ Rust types generated in $(TYPES_OUTPUT_DIR)/$(MODULE_NAME).rs"
	@echo ""
	@echo "You can now import the generated types with:"
	@echo "  mod $(MODULE_NAME);"
	@echo "  use $(MODULE_NAME)::*;"

# Watch for changes and rebuild (requires cargo-watch)
watch-rs:
	cargo watch -c -x "check -p supabase-rust-postgrest" -x "test -p supabase-rust-postgrest" -x "fmt"

# Help command
help:
	@echo "Available commands:"
	@echo "  make build              - Build the project"
	@echo "  make test               - Run tests"
	@echo "  make clean              - Clean build artifacts and temporary files"
	@echo "  make gen-types          - Generate TypeScript types from Supabase schema"
	@echo "  make gen-types-rust     - Generate Rust types from Supabase schema"
	@echo "  make watch-rs           - Watch for changes and run check, test, and format"
	@echo ""
	@echo "Customization:"
	@echo "  make gen-types-rust TYPES_OUTPUT_DIR=my/custom/dir MODULE_NAME=database"
	@echo "  make gen-types-rust SUPABASE_DIR=custom/supabase/path" 