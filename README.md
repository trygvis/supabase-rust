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
|PostgresT|⚠️|27/30 (90%)|Core functions (CRUD, filtering, RPC, transactions) implemented. **NOTE:** Type-safe operations (`schema-convert` feature) are experimental/disabled. **Requires local patch** for INSERT/UPDATE/DELETE when used with PostgREST versions returning empty success responses (e.g., local v12.2.11).|
|Storage|⚠️|19/20 (95%)|Image transformation and extensions beyond JS version. Low test coverage noted. Example requires bucket setup & auth fix.|
|Realtime|⚠️|11/14 (~80%)|Core PubSub, DB changes, Presence implemented with filters & auto-reconnect. Needs more tests, docs & refactoring. Example fails compilation.|
|Functions|⚠️|5/6 (85%)|Basic and streaming functionality implemented, enhancing binary support. Missing automated tests. Examples require Edge Function setup.|
|Migration|❓|N/A|Utility crate present (`crates/migration`), purpose/status needs documentation.|

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

**API Compatibility**: 27/30 (90%)
**Status:** Core library tests passing. The type-safe operations feature (`schema-convert`) is **experimental and currently disabled** in `crates/postgrest/src/lib.rs` (commented out) due to potential issues or incompleteness. **NOTE:** Local PostgREST instances (like v12.2.11) might return empty responses for successful INSERT/UPDATE/DELETE, requiring a local patch to `crates/postgrest/src/lib.rs` to handle these cases (see recent debugging history).

- ✅ Basic CRUD operations for tables/views
- ✅ Complex filtering (conditional operators, JSON operations, full-text search)
- ✅ Result control via ORDER BY, LIMIT, OFFSET, RANGE
- ✅ Transaction support (savepoints, rollbacks)
- ✅ RPC (Remote Procedure Calls)
- ✅ Count options for results
- ✅ Response format control (CSV output support)
- ✅ Single/multiple row processing optimization
- ⚠️ Relationship auto-expansion - Basic implementation complete, nested relationships in development
- ⚠️ Type-safe operations (`insert_typed`, etc.) - API exists in `crates/postgrest/src/schema.rs` but is **experimental and disabled** via feature gate/comments in `lib.rs`. Requires generated types from `schema-convert` feature.
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

**API Compatibility**: 11/14 (~80%) | **Tests:** ⚠️ (Low Coverage) | **Docs:** ⚠️ (Needs Examples) | **Code Structure:** ⚠️ (Large `lib.rs`)

- ✅ Channel creation and management
- ✅ Broadcast messaging
- ✅ Postgres changes monitoring (INSERT/UPDATE/DELETE/ALL)
- ✅ Event filtering (including various operators)
- ✅ Automatic reconnection (configurable options)
- ✅ Explicit error handling (`RealtimeError`)
- ✅ Async primitives (`Arc`, `RwLock`, `mpsc`) used for concurrency.
- ⚠️ Presence feature - Basic implementation complete, state synchronization likely needs more testing/refinement.
- ⚠️ Test Coverage - Requires significant expansion (unit/integration tests for various scenarios).
- ⚠️ Documentation - Needs more practical usage examples.
- ⚠️ Code Structure - `lib.rs` (1000+ lines) could be split into smaller modules.
- ❌ Channel Status Notifications - In development
- ❌ Complex JOIN table monitoring - Planned

#### Functions (`@supabase/functions-js`)

**API Compatibility**: ~5/6 (~85%) | **Tests:** ❌ (Missing)

- ✅ Edge function invocation
- ✅ Function execution with parameters (JSON/serializable body)
- ✅ Authentication integration (via headers)
- ✅ Error handling (Network, Timeout, Non-Success Status, Error Details Parsing)
- ✅ Streaming responses (Raw Bytes, Line-based JSON/SSE)
- ✅ Binary data responses (`invoke_binary` returns `Bytes`)
- ⚠️ Lack of automated tests.
- ⚠️ Potential for code simplification (reduce duplication in request setup).

### Project Status Summary

Overall project completion: ~89% // Adjusted slightly based on Realtime review
**Root Client Notes:** The main `Supabase` client includes convenience filter methods (`.eq()`, `.gt()`, etc.). Ensure these align with Postgrest capabilities and are fully tested.

Current development focus:
- **Investigating and enabling the experimental type-safe PostgREST operations (`schema-convert` feature).**
- Realtime Enhancements
  - ✅ Filter capabilities implemented
  - ⚠️ Expand Test Coverage (Unit & Integration)
  - ⚠️ Add Usage Examples to Documentation
  - ⚠️ Refactor `lib.rs` into smaller modules
  - ⚠️ Improve Presence state sync robustness
  - ❌ Channel status notifications
  - ❌ Complex JOIN table monitoring
- Async Processing Optimization
  - ✅ Throughput improvements

### Future Development

1.  **Priority Implementation Items** (Targeting Q3/Q4 2024):
    *   **PostgresT:** Investigate, fix, and enable the experimental type-safe methods (`schema-convert` feature, commented out code in `lib.rs`).
    *   **Realtime:** Improve Test Coverage & Documentation, Refactor large `lib.rs`.
    *   **Functions:** Implement Automated Tests, Simplify request setup code.
    *   **Storage:** Improve Test Coverage.
    *   **Core:** Validate/Refactor Root Client Filter Methods.
    *   **Migration:** Define scope, implement, and document `crates/migration`.
    *   **API Enhancements:** Continue work on Admin API extensions, Advanced RLS, Realtime features (Channel status, JOIN monitoring), Async optimization.

2.  **Module-Specific Roadmap**:

    **Auth** (95% → 100%, by Q3 2024):
    *   Advanced multi-factor authentication (MFA) features
        *   WebAuthn/passkey support improvements
        *   Backup code management
    *   Advanced JWT verification
        *   Custom claim validation
        *   JWKS support enhancements
    *   Admin API extensions
        *   Organization management
        *   Risk management and auditing

    **PostgresT** (90% → 100%, by Q3 2024):
    *   **Critical:** Resolve type-safe method import/compilation issues blocking examples.
    *   Relationship auto-expansion with nested relationships
        *   Efficient retrieval of multi-level relationships
        *   Circular reference handling
    *   Advanced RLS support
        *   Complex policy condition application
        *   RLS verification tools

    **Storage** (95% → 100%, by Q3 2024):
    *   **Testing:** Improve test coverage significantly using mocking frameworks.
    *   Recursive folder operations
        *   Efficient handling of deep directory structures
        *   Batch operation optimization
    *   Detailed access control
        *   Custom policy definitions
        *   Time-limited access

    **Realtime** (80% → 100%, by Q4 2024): // Aligned with overview table
    *   **Testing & Docs:** Expand Unit/Integration tests and add practical usage examples.
    *   **Refactoring:** Split large `lib.rs` into smaller, more manageable modules.
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

    **Functions** (85% → 100%, by Q3 2024):
    *   **Testing:** Implement comprehensive automated tests.
    *   **Refactoring:** Simplify request setup code to reduce duplication.
    *   Complete binary data support
        *   Efficient binary streams
        *   File upload/download via functions
    *   Advanced error handling
        *   Detailed error information
        *   Retry strategies

    **Migration** (New Target: Basic implementation by Q4 2024):
    *   Define scope and implement core migration utilities (apply, revert, status).
    *   Document usage and best practices within the Supabase context.

    **Client Core** (Ongoing):
    *   Validate and potentially refactor root client filter methods (`.eq()`, etc.).
    *   Ensure consistency and avoid redundancy with `PostgrestClient` filtering.

### Recent Updates

- v0.2.0 (Latest)
  - Refactored workspace dependencies to v0.2.0
  - Added initial schema generation tools (Experimental: Note feature gate in root Cargo.toml)
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

**Current Status (as of recent testing):**

| Example                    | Status                     | Notes                                                                                                                            |
| -------------------------- | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `auth_example`             | ✅ Runs                     |                                                                                                                                  |
| `database_example`         | ✅ Runs (with patch)       | Requires local patch to `crates/postgrest/src/lib.rs` to handle empty INSERT/UPDATE/DELETE responses from local PostgREST v12.2.11. |
| `storage_example`          | ❌ Fails                   | Errors due to missing authorization header and/or missing bucket. Requires setup/auth fix.                                       |
| `realtime_example`         | ❌ Fails Compilation       | Type error (`E0308`) in `realtime_example.rs:448`. Needs code fix (`&user_id`).                                                    |
| `postgrest_example`        | ❌ Fails Execution         | Fails in Example 6 (COUNT) with filter parse error (`PGRST100` for `(exact)`). Needs code/filter fix.                              |
| `functions_example`        | ❌ Fails                   | Fails due to missing `hello-world` Edge Function. Requires Supabase setup.                                                        |
| `auth_admin_example`       | ❌ Fails                   | Panics due to missing `SUPABASE_SERVICE_ROLE_KEY` environment variable. Requires setup.                                          |
| `functions_binary_example` | ❌ Fails                   | Fails due to missing Edge Functions (e.g., `generate-image`). Requires Supabase setup.                                             |

## Roadmap

1.  **Priority Implementation Items** (Targeting Q3/Q4 2024):
    *   **PostgresT:** Investigate, fix, and enable the experimental type-safe methods (`schema-convert` feature, commented out code in `lib.rs`).
    *   **Realtime:** Improve Test Coverage & Documentation, Refactor large `lib.rs`.
    *   **Functions:** Implement Automated Tests, Simplify request setup code.
    *   **Storage:** Improve Test Coverage.
    *   **Core:** Validate/Refactor Root Client Filter Methods.
    *   **Migration:** Define scope, implement, and document `crates/migration`.
    *   **API Enhancements:** Continue work on Admin API extensions, Advanced RLS, Realtime features (Channel status, JOIN monitoring), Async optimization.

2.  **Module-Specific Roadmap**:

    **Auth** (95% → 100%, by Q3 2024):
    *   Advanced multi-factor authentication (MFA) features
        *   WebAuthn/passkey support improvements
        *   Backup code management
    *   Advanced JWT verification
        *   Custom claim validation
        *   JWKS support enhancements
    *   Admin API extensions
        *   Organization management
        *   Risk management and auditing

    **PostgresT** (90% → 100%, by Q3 2024):
    *   **Critical:** Resolve type-safe method import/compilation issues blocking examples.
    *   Relationship auto-expansion with nested relationships
        *   Efficient retrieval of multi-level relationships
        *   Circular reference handling
    *   Advanced RLS support
        *   Complex policy condition application
        *   RLS verification tools

    **Storage** (95% → 100%, by Q3 2024):
    *   **Testing:** Improve test coverage significantly using mocking frameworks.
    *   Recursive folder operations
        *   Efficient handling of deep directory structures
        *   Batch operation optimization
    *   Detailed access control
        *   Custom policy definitions
        *   Time-limited access

    **Realtime** (80% → 100%, by Q4 2024): // Aligned with overview table
    *   **Testing & Docs:** Expand Unit/Integration tests and add practical usage examples.
    *   **Refactoring:** Split large `lib.rs` into smaller, more manageable modules.
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

    **Functions** (85% → 100%, by Q3 2024):
    *   **Testing:** Implement comprehensive automated tests.
    *   **Refactoring:** Simplify request setup code to reduce duplication.
    *   Complete binary data support
        *   Efficient binary streams
        *   File upload/download via functions
    *   Advanced error handling
        *   Detailed error information
        *   Retry strategies

    **Migration** (New Target: Basic implementation by Q4 2024):
    *   Define scope and implement core migration utilities (apply, revert, status).
    *   Document usage and best practices within the Supabase context.

    **Client Core** (Ongoing):
    *   Validate and potentially refactor root client filter methods (`.eq()`, etc.).
    *   Ensure consistency and avoid redundancy with `PostgrestClient` filtering.