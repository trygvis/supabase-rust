// src/lib.rs

pub mod models;
pub mod client;
pub mod error;

// Re-export key components
pub use client::SupabaseClientWrapper; // Example, adjust as needed
pub use models::Item; // Example, adjust as needed
pub use error::SupabaseError; // Example, adjust as needed


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
} 