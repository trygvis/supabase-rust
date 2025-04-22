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
|PostgresT|90%|27/30|Transaction support, advanced filtering implemented|
|Storage|95%|19/20|Image transformation and extensions beyond JS version|
|Realtime|75%|11/14|Basic PubSub, Postgres changes monitoring implemented|
|Functions|85%|5/6|Basic and streaming functionality implemented, enhancing binary support|

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

- ✅ Basic CRUD operations for tables/views
- ✅ Complex filtering (conditional operators, JSON operations, full-text search)
- ✅ Result control via ORDER BY, LIMIT, OFFSET, RANGE
- ✅ Transaction support (savepoints, rollbacks)
- ✅ RPC (Remote Procedure Calls)
- ✅ Count options for results
- ✅ Response format control (CSV output support)
- ✅ Single/multiple row processing optimization
- ⚠️ Relationship auto-expansion - Basic implementation complete, nested relationships in development
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

#### Realtime (`@supabase/realtime-js`)

**API Compatibility**: 11/14 (75%)

- ✅ Channel creation and management
- ✅ Broadcast messaging
- ✅ Postgres changes monitoring (INSERT/UPDATE/DELETE)
- ✅ Event filtering
- ✅ Automatic reconnection
- ⚠️ Presence feature - Basic implementation complete, state synchronization being improved
- ❌ Channel Status Notifications - In development
- ❌ Complex JOIN table monitoring - Planned

#### Functions (`@supabase/functions-js`)

**API Compatibility**: 5/6 (85%)

- ✅ Edge function invocation
- ✅ Function execution with parameters
- ✅ Authentication integration
- ✅ Error handling
- ✅ Streaming responses - Text/JSON streaming supported, binary streaming implemented (v0.1.2)
- ❌ Binary data support - Basic implementation complete, advanced features in development

### Future Development

1. **Priority Implementation Items** (Through Q2 2024):
   - Admin API Extensions
     - Organization/team management
     - Detailed permission settings
   - Advanced Row Level Security (RLS) Features
     - Support for complex policy conditions
     - Policy application verification
   - Async Processing Optimization
     - Throughput improvements
     - Error handling enhancements

2. **Module-Specific Roadmap**:

   **Auth** (95% → 100%, by Q2 2024):
   - Advanced multi-factor authentication (MFA) features
     - WebAuthn/passkey support improvements
     - Backup code management
   - Advanced JWT verification
     - Custom claim validation
     - JWKS support enhancements
   - Admin API extensions
     - Organization management
     - Risk management and auditing

   **PostgresT** (90% → 100%, by Q2 2024):
   - Relationship auto-expansion with nested relationships
     - Efficient retrieval of multi-level relationships
     - Circular reference handling
   - Advanced RLS support
     - Complex policy condition application
     - RLS verification tools

   **Storage** (95% → 100%, by Q2 2024):
   - Recursive folder operations
     - Efficient handling of deep directory structures
     - Batch operation optimization
   - Detailed access control
     - Custom policy definitions
     - Time-limited access

## Getting Started

### Installation

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
supabase-rust = "0.1.2"
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