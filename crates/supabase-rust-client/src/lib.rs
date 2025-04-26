// src/lib.rs

pub mod client;
pub mod error;
pub mod models;

// Re-export key components
pub use client::SupabaseClientWrapper; // Example, adjust as needed
pub use error::SupabaseError;
pub use models::Item; // Example, adjust as needed // Example, adjust as needed

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
