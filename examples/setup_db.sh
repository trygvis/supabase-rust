#!/bin/bash

# This script helps set up the database tables needed for the examples
# It assumes you have the Supabase CLI installed and have a local Supabase instance running

echo "Setting up database tables for Supabase Rust examples..."

# Check if Supabase CLI is installed
if ! command -v supabase &> /dev/null; then
    echo "Supabase CLI not found. Please install it first."
    echo "Visit https://supabase.com/docs/guides/cli for installation instructions."
    exit 1
fi

# Get current directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

# Path to the SQL file
SQL_FILE="$SCRIPT_DIR/schema/create_tables.sql"

# Execute the SQL file against the local Supabase instance
echo "Running SQL script to create tables..."
cat "$SQL_FILE" | supabase db execute

if [ $? -eq 0 ]; then
    echo "Database setup completed successfully!"
    echo "You can now run the examples with 'cargo run --bin example_name'"
else
    echo "Error: Failed to execute SQL script."
    echo "Make sure your local Supabase instance is running."
    echo "You can start it with 'supabase start'"
fi 