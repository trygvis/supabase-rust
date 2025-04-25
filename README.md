# Supabase Rust

Rust client library for [Supabase](https://supabase.com) - A Rust implementation compatible with JavaScript's [supabase-js](https://github.com/supabase/supabase-js).

[![Crate](https://img.shields.io/crates/v/supabase-rust.svg)](https://crates.io/crates/supabase-rust)
[![Docs](https://docs.rs/supabase-rust/badge.svg)](https://docs.rs/supabase-rust)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Compatibility with Supabase JS and Implementation Status

This section explains the current implementation status and compatibility with the JavaScript version of Supabase (v2.x).

### Overview

|Module|Status|API Compatibility|Notes|
|------|------|----------------|-----|
|Auth|✅|38/40 (95%)|Authentication features: Email/password auth, OAuth, Phone auth, MFA, Password reset, Admin API implemented|
|PostgresT|⚠️|~88% (27/30 - Type Safety Removed)|Core functions (CRUD, filtering, RPC, transactions) implemented. **NOTE:** Type-safe operations feature removed. **Requires local patch** for INSERT/UPDATE/DELETE when used with certain PostgREST versions returning empty success responses (e.g., local v12.2.11).|
|Storage|⚠️|19/20 (95%)|Image transformation and extensions beyond JS version. Low test coverage noted. Example may require bucket setup & auth fix (WIP).|
|Realtime|⚠️|11/14 (~80%)|Core PubSub, DB changes, Presence implemented with filters & auto-reconnect. Needs significant tests, docs & refactoring (large `lib.rs`). Example compilation fix (WIP).|
|Functions|⚠️|5/6 (85%)|Basic and streaming functionality implemented, enhancing binary support. Missing automated tests. Examples require Edge Function setup.|
|Migration|🚧|N/A|Utility crate for database migrations using `sea-orm-migration`/`refinery`. Status: In Development/Experimental (Not Implemented).|

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
**Status:** Core library tests passing for standard operations. **NOTE:** Local PostgREST instances (like v12.2.11) might return empty responses for successful INSERT/UPDATE/DELETE, requiring a local patch to `crates/postgrest/src/lib.rs` to handle these cases.

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

**API Compatibility**: 19/20 (95%)

- ✅ Bucket management (create, get, update, delete)
- ✅ File operations (upload, download, list, delete)
- ✅ File moving and copying
- ✅ Signed URL generation
- ✅ Public URL generation
- ✅ Multipart uploads (large file support)
- ✅ Image transformation (resize, format conversion, quality control)
- ⚠️ Folder operations - Basic implementation complete, recursive operations in development
- ⚠️ Access control - Basic implementation complete, detailed policy support in development
- ⚠️ Low test coverage - Requires significant improvement using mocking frameworks.

#### Realtime (`@supabase/realtime-js`)

**API Compatibility**: 11/14 (~80%) | **Tests:** ⚠️ (Very Low Coverage - Needs Significant Expansion) | **Docs:** ⚠️ (Needs More Examples) | **Code Structure:** ⚠️ (Large `lib.rs` - Needs Refactoring)

- ✅ Channel creation and management
- ✅ Broadcast messaging
- ✅ Postgres changes monitoring (INSERT/UPDATE/DELETE/ALL)
- ✅ Event filtering (including various operators)
- ✅ Automatic reconnection (configurable options)
- ✅ Explicit error handling (`RealtimeError`)
- ✅ Async primitives (`Arc`, `RwLock`, `mpsc`) used for concurrency.
- ⚠️ Presence feature - Basic implementation complete, state synchronization requires more testing/refinement.
- ⚠️ Test Coverage - Requires significant expansion (unit/integration tests for various scenarios).
- ⚠️ Documentation - Needs more practical usage examples.
- ⚠️ Code Structure - `lib.rs` (1000+ lines) needs splitting into smaller modules.
- ❌ Channel Status Notifications - In development
- ❌ Complex JOIN table monitoring - Planned

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

### Project Status Summary

Overall project completion: ~85% (Adjusted: PostgREST type safety removed, focus shifted)
**Root Client Notes:** The main `Supabase` client includes convenience filter methods (`.eq()`, `.gt()`, etc.). Ensure these align with Postgrest capabilities and are fully tested.
**Production Readiness Concerns:** 
- **Critically Low Test Coverage:** Storage, Realtime, and Functions modules require significant expansion of unit, integration, and potentially BDD tests (as per project rules).
- **Realtime Module Maturity:** Requires refactoring of the large `lib.rs` file into smaller, more manageable modules, along with improved documentation and examples.
- **Unimplemented Migration Crate:** The `crates/migration` utility is currently unimplemented/experimental.
- **Model-Driven Development:** Adherence to the `.ssot`-based model-driven development approach needs verification and potentially implementation.

**Current Development Focus:**
- **Increase Test Coverage:** Prioritize comprehensive testing across Storage, Realtime, and especially Functions.
- **Realtime Enhancements:** Refactor `lib.rs`, add examples, improve tests, and enhance documentation.
- **Stabilize Examples:** Fix remaining issues in `storage`, `postgrest`, and `realtime` examples.
- **Implement Migration Crate:** Define scope and implement basic functionality, potentially using `sea-orm-migration` or `refinery`.
- **Verify Model-Driven Approach:** Ensure `.ssot` files and state machines are defined and drive development as per project rules.

### Future Development

1.  **Priority Implementation Items** (Revised Priorities):
    *   **Testing:** Drastically improve test coverage for `Storage`, `Realtime`, `Functions`, incorporating various testing strategies (unit, integration, BDD).
    *   **Realtime:** Refactor large `lib.rs`, Improve Test Coverage & Documentation, implement missing features.
    *   **Functions:** Implement Automated Tests, Simplify request setup code.
    *   **Model-Driven Development:** Establish/verify `.ssot` state machine definitions for core modules.
    *   **Migration:** Define scope, implement basic functionality using `sea-orm-migration`/`refinery`, and document `crates/migration`.
    *   **PostgresT:** Complete Relationship auto-expansion (nested) and Advanced RLS support.
    *   **Storage:** Complete Folder operations and Access control features.
    *   **Core:** Validate/Refactor Root Client Filter Methods.
    *   **Auth:** Complete Advanced JWT verification and Admin API extensions.

2.  **Module-Specific Roadmap**:

    **Auth** (95% → 100%, Target Q3 2024):
    *   Advanced multi-factor authentication (MFA) features
        *   WebAuthn/passkey support improvements
        *   Backup code management
    *   Advanced JWT verification
        *   Custom claim validation
        *   JWKS support enhancements
    *   Admin API extensions
        *   Organization management
        *   Risk management and auditing

    **PostgresT** (~88% → 95%, Target Q4 2024):
    *   Relationship auto-expansion with nested relationships
        *   Efficient retrieval of multi-level relationships
        *   Circular reference handling
    *   Advanced RLS support
        *   Complex policy condition application
        *   RLS verification tools

    **Storage** (95% → 100%, Target Q4 2024):
    *   **Testing:** Improve test coverage significantly using mocking frameworks (High Priority).
    *   Recursive folder operations
        *   Efficient handling of deep directory structures
        *   Batch operation optimization
    *   Detailed access control
        *   Custom policy definitions
        *   Time-limited access

    **Realtime** (80% → 95%, Target Q1 2025):
    *   **Testing & Docs:** Expand Unit/Integration tests and add practical usage examples (High Priority).
    *   **Refactoring:** Split large `lib.rs` into smaller, more manageable modules (High Priority).
    *   **Presence:** Improve state synchronization robustness and testing.
    *   Complete Presence feature implementation
        *   State synchronization
        *   Presence conflict resolution
    *   Channel Status Notifications
        *   Connection state monitoring
        *   Reconnection strategies
    *   Complex JOIN table monitoring
        *   Related table change tracking
        *   Efficient notification filtering

    **Functions** (85% → 100%, Target Q4 2024):
    *   **Testing:** Implement comprehensive automated tests (High Priority).
    *   **Refactoring:** Simplify request setup code to reduce duplication.
    *   Complete binary data support
        *   Efficient binary streams
        *   File upload/download via functions
    *   Advanced error handling
        *   Detailed error information
        *   Retry strategies

    **Migration** (0% → 50% - Basic Implementation, Target Q1 2025):
    *   Define scope and implement core migration utilities (apply, revert, status).
    *   Document usage and best practices within the Supabase context.

### Recent Updates

- v0.2.0 (Latest)
  - Refactored workspace dependencies to v0.2.0
  - Removed experimental schema generation / type-safe PostgREST features.
  - Improved error handling across all modules
  - Enhanced binary support in Functions module
  - Fixed compatibility issues with latest Supabase JS

### System Requirements

- Rust 1.86.0 or higher
- Compatible with Supabase projects using v2.x API

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