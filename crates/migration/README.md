# Running Migrator CLI

## Traditional Step-by-Step Migrations

- Generate a new migration file
    ```sh
    cargo run -- generate MIGRATION_NAME
    ```
- Apply all pending migrations
    ```sh
    cargo run
    ```
    ```sh
    cargo run -- up
    ```
- Apply first 10 pending migrations
    ```sh
    cargo run -- up -n 10
    ```
- Rollback last applied migrations
    ```sh
    cargo run -- down
    ```
- Rollback last 10 applied migrations
    ```sh
    cargo run -- down -n 10
    ```
- Drop all tables from the database, then reapply all migrations
    ```sh
    cargo run -- fresh
    ```
- Rollback all applied migrations, then reapply all migrations
    ```sh
    cargo run -- refresh
    ```
- Rollback all applied migrations
    ```sh
    cargo run -- reset
    ```
- Check the status of all migrations
    ```sh
    cargo run -- status
    ```

## Schema-Based Migrations (Using Refinery)

This project now supports schema-based migrations using the Refinery crate. Schema-based migrations allow you to define your entire database schema in SQL files and apply them in one go.

### Adding New Schema Migrations

1. Create a new SQL file in the `src/schema_migrations/sql` directory following the naming convention `V{VERSION}__{NAME}.sql` (e.g., `V1__initial_schema.sql`, `V2__add_posts_table.sql`).

2. Write your SQL schema definition in the file. Include everything from table creation to indexes and constraints.

### Running Schema Migrations

- Apply all schema migrations
    ```sh
    cargo run -- up --schema
    ```

- Apply schema migrations from a custom directory
    ```sh
    cargo run -- up --schema --schema-dir /path/to/migrations
    ```

- Check status (includes both traditional and schema migrations)
    ```sh
    cargo run -- status --schema
    ```

- Refresh database with schema migrations
    ```sh
    cargo run -- fresh --schema
    ```

### Advantages of Schema-Based Migrations

- **Complete schema definition**: Each migration file contains a complete definition of tables or features.
- **Better for version control**: Changes to the schema are more visible in diffs.
- **Simplicity**: Schema migrations are easier to understand since they represent the desired state.

### Notes on Schema Migrations

- Schema migrations don't support `down` directly. To revert changes, create a new migration that makes the necessary modifications.
- Schema migrations are tracked in the `refinery_schema_history` table.
- Schema migration files are embedded in the binary at compile time.
