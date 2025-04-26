# Supabase Rust

Rust client library for [Supabase](https://supabase.com) - A Rust implementation compatible with JavaScript's [supabase-js](https://github.com/supabase/supabase-js).

[![Crate](https://img.shields.io/crates/v/supabase-rust-client.svg)](https://crates.io/crates/supabase-rust-client)
[![Docs](https://docs.rs/supabase-rust-client/0.4.0/badge.svg)](https://docs.rs/supabase-rust-client/0.4.0)
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
    *   `supabase-rust-client` (depends on core libraries)
    *   `supabase-rust-gftd` (deprecated)
3.  **Examples:**
    *   `supabase-rust-examples` (depends on `supabase-rust-client`)

You can use a tool like `cargo-workspaces` (`cargo install cargo-workspaces`) to manage publishing the entire workspace automatically, which respects these dependencies:

```bash
cargo workspaces publish --from-git
```

### Project Completion Assessment (as of 2025-04-26)

-   **Overall:** The project provides functional Rust clients for major Supabase services (Auth, PostgREST, Storage, Functions, basic Realtime). Compatibility with `supabase-js` is generally high.
-   **Strengths:** Covers essential Supabase features, including recent additions like Storage `move_object`.
-   **Areas for Improvement:**
    -   **Realtime Crate:** **CRITICAL:** Requires immediate attention. Test timeouts **must be fixed** before this crate can be considered stable or publishable. Test coverage is extremely low, and code refactoring is needed.
    -   **Testing:** Overall test coverage, especially for integration scenarios and error handling across all crates, needs significant improvement. The new `storage::move_object` requires dedicated tests.
    -   **PostgREST:** The removal of type-safe operations is a significant change. The requirement for a local patch for certain PostgREST versions needs a more robust solution.
    -   **Examples:** `storage_example.rs` contains dead code warnings that need to be addressed. Examples should be verified against the latest crate versions.
    -   **Documentation:** Inline code documentation (doc comments) should be expanded for better usability.
-   **Conclusion:** The project has a solid foundation but requires significant work on testing, stabilization (especially Realtime), documentation, and resolving the PostgREST patch requirement before being reliably publishable and production-ready.

## Roadmap

**Priority: High** ⭐️⭐️⭐️⭐️⭐️

*   **Fix Realtime Crate:**
    *   Resolve critical test timeout issues (`test_connect_disconnect`).
    *   Significantly increase test coverage (integration, error handling, edge cases).
    *   Refactor `lib.rs` for better maintainability and readability.
    *   Improve documentation with clear examples.
    *   **Goal:** Stabilize Realtime for reliable use and enable publishing.
*   **Improve Overall Test Coverage:**
    *   Increase integration test coverage across all crates (Auth, PostgREST, Storage, Functions).
    *   Add specific tests for error handling scenarios.
    *   Add tests for newly added features like `storage::move_object`.
    *   **Goal:** Ensure library robustness and reliability.
*   **Address PostgREST Empty Response Issue:**
    *   Investigate the root cause of empty responses in specific PostgREST versions.
    *   Determine if a client-side workaround is the best long-term solution or if coordination with PostgREST is needed.
    *   Remove the need for local patching if possible.
    *   **Goal:** Provide a seamless experience without requiring manual patches.
*   **Enhance Documentation:**
    *   Expand inline doc comments (`///`) for all public functions, structs, and enums.
    *   Add more usage examples, especially for complex scenarios (e.g., advanced Realtime filters, PostgREST RPC).
    *   Review and update existing examples (`examples/` directory) for clarity and correctness.
    *   Address dead code warnings in examples (`storage_example.rs`).
    *   **Goal:** Improve developer experience and reduce learning curve.

**Priority: Medium** ⭐️⭐️⭐️

*   **Functions Crate Tests:**
    *   Implement comprehensive automated tests for the Functions crate.
    *   **Goal:** Ensure Functions client reliability.
*   **PostgREST Type Safety (Revisit):**
    *   Evaluate options for re-introducing type safety in PostgREST operations, potentially through macros or alternative approaches, considering the maintenance overhead.
    *   **Goal:** Improve developer ergonomics for PostgREST interactions if feasible.
*   **Migration Crate Development:**
    *   Define the scope and features for the `Migration` utility crate.
    *   Begin implementation using `sea-orm-migration` or `refinery`.
    *   **Goal:** Provide basic database migration capabilities.

**Priority: Low** ⭐️⭐️

*   **Advanced Auth Features:**
    *   Complete implementation of advanced JWT verification and admin organization management.
*   **Advanced PostgREST Features:**
    *   Implement nested relationship auto-expansion.
    *   Implement advanced RLS policy support exploration.
*   **Advanced Storage Features:**
    *   Implement recursive folder operations.
    *   Implement detailed storage access control policy support.

**Completed Recently:** ✅

*   Added `storage::move_object` functionality.
*   Improved basic test coverage for PostgREST CRUD, filters, and modifiers.
*   Added `Prefer: return=representation` header for PostgREST mutations.
*   Implemented basic MFA support in Auth.
*   Implemented binary data support in Functions.

## Getting Started

### Installation

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
supabase-rust = "0.3.0"
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
-   **`tests/`


DATABASE_URL=postgres://postgres:postgres@localhost:54322/postgres
SUPABASE_URL=http://127.0.0.1:54321
SUPABASE_KEY=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6ImFub24iLCJleHAiOjE5ODM4MTI5OTZ9.CRXP1A7WOeoJeXxjNni43kdQwgnWNReilDMblYTn_I0
SUPABASE_ROLE_KEY=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9f