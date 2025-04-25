# Supabase Rust

Rust client library for [Supabase](https://supabase.com) - A Rust implementation compatible with JavaScript's [supabase-js](https://github.com/supabase/supabase-js).

[![Crate](https://img.shields.io/crates/v/supabase-rust.svg)](https://crates.io/crates/supabase-rust)
[![Docs](https://docs.rs/supabase-rust/badge.svg)](https://docs.rs/supabase-rust)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Compatibility with Supabase JS and Implementation Status

This section explains the current implementation status and compatibility with the JavaScript version of Supabase (v2.x).

### Overview

| Module      | Status | API Compatibility   | Notes                                                                                                                                                                                             |
| :---------- | :----- | :------------------ | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Auth**    | ✅     | 38/40 (95%)         | Email/password auth, OAuth, Phone auth, MFA, Password reset, Admin API implemented.                                                                                                               |
| **PostgresT** | ⚠️     | ~88% (27/30 - Type Safety Removed) | Core functions (CRUD, filtering, RPC, transactions) implemented. Test coverage improved for basic CRUD, filters, and modifiers. `Prefer: return=representation` header correctly added for mutations. **NOTE:** Type-safe ops removed. **Requires local patch** for empty INSERT/UPDATE/DELETE responses with certain PostgREST versions. |
| **Storage** | ✅     | **20/20 (100%)**    | **File moving (`move_object`) added.** Image transformation and extensions beyond JS version. Low test coverage noted.                                                                              |
| **Realtime**  | ❌     | 11/14 (~80%)        | Core PubSub, DB changes, Presence implemented. **Tests have critical timeout issues and very low coverage.** Needs significant tests, docs & refactoring (large `lib.rs`).                   |
| **Functions** | ⚠️     | 5/6 (85%)           | Basic and streaming functionality implemented, enhancing binary support. Missing automated tests. Examples require Edge Function setup.                                                           |
| **Migration** | 🚧     | N/A                 | Utility crate for database migrations using `sea-orm-migration`/`refinery`. Status: In Development/Experimental (Not Implemented).                                                              |

_Status Icons: ✅ Implemented & Tested (Basic), ⚠️ Implemented (Low Test/Docs/Needs Refactor), ❌ Critical Issues (Tests Failing/Timeout), 🚧 In Development/Not Implemented_

### Detailed Compatibility Report

#### Auth (`@supabase/auth-js`)

**API Compatibility**: 38/40 (95%)

- ✅ Email/password signup and signin
- ✅ Session management (get, refresh, destroy)
- ✅ Password reset
- ✅ OAuth provider authentication (All 12 providers supported: Google, GitHub, Facebook, etc.)
- ✅ One-time password (OTP) authentication
- ✅ User information retrieval and updates
- ✅ Email confirmation flow
- ✅ Anonymous authentication
- ✅ Phone number authentication
- ✅ Multi-factor authentication (MFA) - Basic and advanced features implemented
- ⚠️ JWT verification - Basic implementation complete, advanced verification in development
- ⚠️ Admin methods - User management, listing, updates implemented; organization management in development

#### PostgresT (`@supabase/postgrest-js`)

**API Compatibility**: ~88% (27/30 - Type Safety Feature Removed)
**Status:** Core library tests improved, covering basic CRUD, filters, modifiers, and error handling. `Prefer: return=representation` header is now correctly added for `insert`/`update`/`delete`. **NOTE:** Local PostgREST instances (like v12.2.11) might return empty responses for successful INSERT/UPDATE/DELETE, requiring a local patch to `crates/postgrest/src/lib.rs` to handle these cases. Type-safe operations removed.

- ✅ Basic CRUD operations for tables/views
- ✅ Complex filtering (conditional operators, JSON operations, full-text search)
- ✅ Result control via ORDER BY, LIMIT, OFFSET, RANGE
- ✅ Transaction support (savepoints, rollbacks)
- ✅ RPC (Remote Procedure Calls)
- ✅ Count options for results
- ✅ Response format control (CSV output support)
- ✅ Single/multiple row processing optimization
- ⚠️ Relationship auto-expansion - Basic implementation complete, nested relationships in development
- ❌ Type-safe operations (`insert_typed`, etc.) - **Removed.**
- ❌ Advanced Row Level Security (RLS) policy support - In development

### PostgresT RLS (Row Level Security)

Row Level Security (RLS) is a powerful PostgreSQL security feature that enables row-level access control for database tables. The Supabase Rust client allows you to work with RLS as follows:

#### Bypassing RLS (Admin Privileges)

Using admin role to bypass RLS policies:

```rust
// Note: Using this method requires a service role key
let service_client = supabase.with_service_key("your-service-role-key");

// Access all user data by bypassing RLS
let all_users = service_client
    .from("users")
    .select("*")
    .ignore_rls()  // Bypass RLS policies
    .execute()
    .await?;
```

#### RLS Policy Usage Example

Typical RLS setup to allow users to access only their own data:

1. First, set up policies in PostgreSQL:

```sql
-- Enable RLS on table
ALTER TABLE profiles ENABLE ROW LEVEL SECURITY;

-- Set policy that allows viewing only owned profiles
CREATE POLICY "Profiles are viewable by owners only" 
ON profiles 
FOR SELECT 
USING (auth.uid() = user_id);

-- Set policy that allows updating only owned profiles
CREATE POLICY "Profiles are updatable by owners only" 
ON profiles 
FOR UPDATE 
USING (auth.uid() = user_id);
```

2. In the Rust client, access with JWT:

```rust
// Authenticate user
let session = supabase
    .auth()
    .sign_in_with_password(SignInWithPasswordCredentials {
        email: "user@example.com".to_string(),
        password: "password123".to_string(),
        ..Default::default()
    })
    .await?;

// Access data with session JWT token
// RLS policies are automatically applied
let my_profile = supabase
    .from("profiles")
    .select("*")
    .with_auth(&session.access_token)  // Set JWT token
    .execute()
    .await?;

// Results only include profiles owned by the current user
```

#### Dynamic RLS Filtering

For more complex use cases, dynamic filtering with functions and JSONB data is possible:

```sql
-- Access control based on user roles
CREATE POLICY "Admins can access all data"
ON documents
FOR ALL
USING (
  auth.jwt() ->> 'role' = 'admin' 
  OR auth.uid() = owner_id
);
```

```rust
// Users with admin role can see all documents
// Regular users can only see their own documents
let documents = supabase
    .from("documents")
    .select("*")
    .with_auth(&session.access_token)  // Set JWT token
    .execute()
    .await?;
```

#### Security Best Practices

1. **Always Enable RLS**: Enable RLS on all important tables and set default deny policies
2. **Principle of Least Privilege**: Grant users only the minimum access permissions needed
3. **Limited Service Role Usage**: Use `ignore_rls()` only on backend servers, never expose to clients
4. **JWT Verification**: Always verify JWT signatures to prevent forged JWTs

```rust
// Always use verified tokens
if let Some(verified_token) = supabase.auth().verify_token(&input_token).await? {
    // Access data with verified token
    let data = supabase
        .from("secured_table")
        .select("*")
        .with_auth(&verified_token)
        .execute()
        .await?;
}
```

#### Storage (`@supabase/storage-js`)

**API Compatibility**: **20/20 (100%)**

- ✅ Bucket management (create, get, update, delete)
- ✅ File operations (upload, download, list, delete)
- ✅ **File moving and copying (`move_object`)**
- ✅ Signed URL generation
- ✅ Public URL generation
- ✅ Multipart uploads (large file support)
- ✅ Image transformation (resize, format conversion, quality control)
- ⚠️ Folder operations - Basic implementation complete, recursive operations in development
- ⚠️ Access control - Basic implementation complete, detailed policy support in development
- ⚠️ Low test coverage - Requires significant improvement using mocking frameworks.

#### Realtime (`@supabase/realtime-js`)

**API Compatibility**: 11/14 (~80%) | **Tests:** ❌ (Critical Timeout Issues & Very Low Coverage) | **Docs:** ⚠️ (Needs More Examples) | **Code Structure:** ⚠️ (Large `lib.rs` - Needs Refactoring)

- ✅ Channel creation and management
- ✅ Broadcast messaging
- ✅ Postgres changes monitoring (INSERT/UPDATE/DELETE/ALL)
- ✅ Event filtering (including various operators)
- ✅ Automatic reconnection (configurable options)
- ✅ Explicit error handling (`RealtimeError`)
- ✅ Async primitives (`Arc`, `RwLock`, `mpsc`) used for concurrency.
- ❌ **Critical Issue:** Integration tests (`test_connect_disconnect`) are timing out, indicating potential connection or disconnection logic problems. Test coverage is extremely low.

#### Functions (`@supabase/functions-js`)

**API Compatibility**: ~5/6 (~85%) | **Tests:** ❌ (Missing - High Priority)

- ✅ Edge function invocation
- ✅ Function execution with parameters (JSON/serializable body)
- ✅ Authentication integration (via headers)
- ✅ Error handling (Network, Timeout, Non-Success Status, Error Details Parsing)
- ✅ Streaming responses (Raw Bytes, Line-based JSON/SSE)
- ✅ Binary data responses (`invoke_binary` returns `Bytes`)
- ⚠️ Lack of automated tests - Critical for production readiness.
- ⚠️ Potential for code simplification (reduce duplication in request setup).

### Crate Publishing Order

Due to inter-crate dependencies within the workspace, the crates must be published to crates.io in a specific order:

1.  **Core Libraries (any order):**
    *   `supabase-rust-auth`
    *   `supabase-rust-functions`
    *   `supabase-rust-postgrest`
    *   `supabase-rust-realtime`
    *   `supabase-rust-storage`
2.  **Main Library:**
    *   `supabase-rust-gftd` (depends on core libraries)
3.  **Examples:**
    *   `supabase-rust-examples` (depends on `supabase-rust-gftd`)

You can use a tool like `cargo-workspaces` (`cargo install cargo-workspaces`) to manage publishing the entire workspace automatically, which respects these dependencies:

```bash
cargo workspaces publish --from-git
```

### Project Completion Assessment (as of YYYY-MM-DD)

-   **Overall:** The project provides functional Rust clients for major Supabase services (Auth, PostgREST, Storage, Functions, basic Realtime). Compatibility with `supabase-js` is generally high.
-   **Strengths:** Covers essential Supabase features, including recent additions like Storage `move_object`.
-   **Areas for Improvement:**
    -   **Realtime Crate:** Requires immediate attention. Test timeouts must be fixed, and test coverage significantly increased. Code refactoring is also recommended. **Publishing is blocked until Realtime issues are resolved.**
    -   **Testing:** Overall test coverage, especially for integration scenarios and error handling across all crates, needs improvement. The new `storage::move_object` requires dedicated tests.
    -   **Examples:** `storage_example.rs` contains dead code warnings that need to be addressed. Examples should be verified against the latest crate versions.
    -   **Documentation:** Inline code documentation (doc comments) should be expanded for better usability.
-   **Conclusion:** The project has a solid foundation but requires significant work on testing, stabilization (especially Realtime), and documentation before being reliably publishable.

## Roadmap

-   **Immediate Priority (Blocking Publish):**
    -   [ ] **Fix Realtime Test Timeout:** Investigate and resolve the timeout issue in `crates/realtime/tests/integration_test.rs` (`test_connect_disconnect`).
    -   [ ] **Increase Realtime Test Coverage:** Add comprehensive tests for Realtime channel joining, message broadcasting, DB changes, presence, filtering, and error handling.
-   **High Priority:**
    -   [ ] **Test `storage::move_object`:** Add specific integration tests for the newly added file move functionality.
    -   [ ] **Address `storage_example.rs` Warnings:** Remove or utilize the dead code identified by the compiler warnings.
    -   [ ] **Increase Overall Test Coverage:** Add more integration tests for Auth, PostgREST, Storage, and Functions, covering edge cases and error conditions.
-   **Medium Priority:**
    -   [ ] **Refactor Realtime Crate:** Break down the large `lib.rs` into smaller, more manageable modules.
    -   [ ] **Improve Documentation:** Add detailed doc comments to public functions, structs, and enums across all crates.
    -   [ ] **Review Examples:** Ensure all examples compile and run correctly with the latest code and provide clear setup instructions (e.g., required Edge Functions, bucket policies).
-   **Low Priority / Future:**
    -   [ ] **Implement Missing Features:** Review `supabase-js` for any potentially valuable features not yet implemented (e.g., advanced RLS support, recursive Storage folder operations).
    -   [ ] **Implement Migration Crate:** Develop the `crates/migration` utility.
    -   [ ] **Publish Crates:** Once tests pass reliably and critical issues are resolved, publish the updated versions to crates.io.

## Getting Started

### Installation

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
supabase-rust = "0.2.0"
```

### Basic Usage

```rust
use supabase_rust::{Supabase, PostgrestError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Profile {
    id: String,
    username: String,
    avatar_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client
    let supabase = Supabase::new("https://your-project.supabase.co", "your-anon-key");
    
    // Authenticate
    let session = supabase
        .auth()
        .sign_in_with_password("user@example.com", "password123")
        .await?;
    
    // Query data with authentication
    let profiles: Vec<Profile> = supabase
        .from("profiles")
        .select("*")
        .with_auth(&session.access_token)
        .execute()
        .await?
        .data()?;
        
    println!("Retrieved profiles: {:?}", profiles);
    
    Ok(())
}
```

### Advanced Features

#### Transactions

```rust
// Start a transaction
let transaction = supabase.from("profiles").transaction();

// Perform operations within transaction
let update_result = transaction
    .from("profiles")
    .update(json!({ "status": "active" }))
    .eq("id", "123")
    .execute()
    .await?;
    
let insert_result = transaction
    .from("logs")
    .insert(json!({ "user_id": "123", "action": "status_update" }))
    .execute()
    .await?;
    
// Commit the transaction (or rollback on error)
transaction.commit().await?;
```

#### Storage with Image Transformations

```rust
// Upload an image
let path = "avatars/profile.jpg";
supabase
    .storage()
    .from("public-bucket")
    .upload(path, file_bytes)
    .await?;

// Generate image transformation URL (resize to 100x100, format as webp)
let transform_url = supabase
    .storage()
    .from("public-bucket")
    .get_public_transform_url(path, |transform| {
        transform
            .width(100)
            .height(100)
            .format("webp")
            .quality(80)
    });
```

#### Realtime Subscriptions

```rust
// Subscribe to database changes
let subscription = supabase
    .realtime()
    .channel("schema-db-changes")
    .on_postgres_changes("profiles", |changes| {
        changes
            .event("INSERT")
            .event("UPDATE")
            .event("DELETE")
    })
    .subscribe(move |payload| {
        println!("Change detected: {:?}", payload);
    })
    .await?;
    
// Later, unsubscribe when no longer needed
subscription.unsubscribe().await?;
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Project Structure and Priorities

The project is organized as a Cargo workspace to manage different Supabase features as separate crates:

-   **`crates/`**: Contains the core implementation for each Supabase service:
    -   `auth`: Authentication (High Priority - Core functionality)
    -   `postgrest`: Database interactions (High Priority - Core functionality)
    -   `storage`: File storage (Medium Priority - Needs significant test coverage)
    -   `realtime`: Realtime subscriptions (Medium Priority - Needs significant tests, docs, refactoring)
    -   `functions`: Edge function invocation (Medium Priority - Needs significant tests)
    -   `migration`: Database migration utilities (Low Priority - Not Implemented)
-   **`src/`**: The main `supabase-rust` library crate that ties the modules together.
-   **`examples/`**: Usage examples for each module (High Priority - Needs fixing and maintenance).
-   **`tests/`**: Integration tests (Medium Priority - Coverage needs expansion).
-   **`supabase/`**: Supabase-specific configuration or migration files (related to the `migration` crate).
-   **`docs/`**: Additional documentation (Low Priority - Needs population).
-   **`Cargo.toml`**: Workspace and root crate definition.
-   **`Makefile`**: Development and build tasks.

Current priorities are **increasing test coverage** across all modules, stabilizing the examples, refactoring the Realtime crate, and starting the implementation of the Migration crate.

## Examples

The `/examples` directory contains various usage examples for the different crates.

**Prerequisites:**

*   A running Supabase instance (local or cloud).
*   Environment variables set in `examples/.env`:
    *   `SUPABASE_URL`: Your Supabase project URL.
    *   `SUPABASE_KEY`: Your Supabase project `anon` key.
    *   `SUPABASE_SERVICE_ROLE_KEY`: (Required for `auth_admin_example`) Your Supabase project `service_role` key.
*   For `functions_example` and `functions_binary_example`: Deployed Edge Functions (`hello-world`, `generate-image`, etc.) in your Supabase project.
*   For `storage_example`: A Storage bucket configured in your Supabase project.

**Running Examples:**

1.  Navigate to the examples directory: `cd examples`
2.  Ensure your Supabase instance is running.
3.  Run the database setup script (first time): `./setup_db.sh` (or ensure the `tasks` table exists with RLS policies from `schema/create_tables.sql`).
4.  Run a specific example: `cargo run --bin <example_name>` (e.g., `cargo run --bin auth_example`).

**Current Status (Post-Fixes - Needs Verification):**

| Example                    | Status                     | Notes                                                                                                                            |
| -------------------------- | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `auth_example`             | ✅ Runs                     |                                                                                                                                  |
| `database_example`         | ✅ Runs (with patch)       | Requires local patch to `crates/postgrest/src/lib.rs` to handle empty INSERT/UPDATE/DELETE responses from local PostgREST v12.2.11. |
| `storage_example`          | ✅ Runs (Needs Verification) | Attempted fix for auth header. Still requires bucket setup in Supabase project.                                                |
| `realtime_example`         | ✅ Runs (Needs Verification) | Attempted fix for type error.                                                                                                   |
| `postgrest_example`        | ✅ Runs (Needs Verification) | Attempted fix for COUNT filter error.                                                                                            |
| `functions_example`        | ❌ Fails (Setup Required)  | Fails due to missing `hello-world` Edge Function. Requires Supabase project setup.                                             |
| `auth_admin_example`       | ❌ Fails (Setup Required)  | Panics due to missing `SUPABASE_SERVICE_ROLE_KEY` environment variable. Requires setup.                                          |
| `functions_binary_example` | ❌ Fails (Setup Required)  | Fails due to missing Edge Functions (e.g., `generate-image`). Requires Supabase project setup.                                     |

## Production Readiness Assessment (Updated)

While core functionality exists, this library is **not yet recommended for production use** due to the following:

1.  **Insufficient Testing:** Critical modules (`Storage`, `Realtime`, `Functions`) lack adequate test coverage, increasing the risk of undiscovered bugs.
2.  **Realtime Maturity:** The `Realtime` crate requires significant refactoring, documentation improvement, and testing before being considered stable.
3.  **Missing Migration Tool:** Database schema management is crucial for production, and the `Migration` crate is not yet implemented.
4.  **Example Stability:** While fixes have been attempted, thorough verification of all examples across different environments is needed.

Focus areas for achieving production readiness are comprehensive testing, Realtime module stabilization, and Migration crate implementation.
