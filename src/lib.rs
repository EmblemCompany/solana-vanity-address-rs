//! Library for generating Solana vanity addresses.

use solana_sdk::{signature::{Keypair, Signer}, pubkey::Pubkey};

#[cfg(feature = "api")]
pub mod api;
use rayon::prelude::*;

/// Result of a successful vanity address search.
pub struct VanityResult {
    pub keypair: Keypair,
    pub elapsed: std::time::Duration,
    pub attempts: u64,
}

/// Searches for a Solana keypair whose public key starts with the given prefix.
/// Uses the specified number of threads.
pub fn find_vanity_address(prefix: &str, num_threads: usize) -> VanityResult {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    let found = AtomicBool::new(false);
    let attempts = AtomicU64::new(0);
    let start_time = Instant::now();
    let result = Arc::new(Mutex::new(None::<Keypair>));

    rayon::ThreadPoolBuilder::new().num_threads(num_threads).build_global().ok();

    while !found.load(Ordering::SeqCst) {
        let result_clone = Arc::clone(&result);
        (0..100_000).into_par_iter().for_each(|_| {
            if found.load(Ordering::SeqCst) {
                return;
            }
            let keypair = Keypair::new();
            let pubkey_str = keypair.pubkey().to_string();
            attempts.fetch_add(1, Ordering::Relaxed);
            if pubkey_str.starts_with(prefix) {
                found.store(true, Ordering::SeqCst);
                // Now thread-safe with Mutex
                let mut result_guard = result_clone.lock().unwrap();
                *result_guard = Some(keypair);
            }
        });
    }

    VanityResult {
        keypair: result.lock().unwrap().take().expect("Keypair should be found"),
        elapsed: start_time.elapsed(),
        attempts: attempts.load(Ordering::Relaxed),
    }
}

/// Searches for a Solana keypair whose public key ends with the given suffix.
/// Uses the specified number of threads.
pub fn find_vanity_address_with_suffix(suffix: &str, num_threads: usize) -> VanityResult {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    let found = AtomicBool::new(false);
    let attempts = AtomicU64::new(0);
    let start_time = Instant::now();
    let result = Arc::new(Mutex::new(None::<Keypair>));

    rayon::ThreadPoolBuilder::new().num_threads(num_threads).build_global().ok();

    while !found.load(Ordering::SeqCst) {
        let result_clone = Arc::clone(&result);
        (0..100_000).into_par_iter().for_each(|_| {
            if found.load(Ordering::SeqCst) {
                return;
            }
            let keypair = Keypair::new();
            let pubkey_str = keypair.pubkey().to_string();
            attempts.fetch_add(1, Ordering::Relaxed);
            if pubkey_str.ends_with(suffix) {
                found.store(true, Ordering::SeqCst);
                let mut result_guard = result_clone.lock().unwrap();
                *result_guard = Some(keypair);
            }
        });
    }

    VanityResult {
        keypair: result.lock().unwrap().take().expect("Keypair should be found"),
        elapsed: start_time.elapsed(),
        attempts: attempts.load(Ordering::Relaxed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_keypair_entropy() {
        // Generate multiple keypairs and ensure they are all unique
        let mut keypairs = HashSet::new();
        let mut pubkeys = HashSet::new();

        for _ in 0..1000 {
            let keypair = Keypair::new();
            let pubkey = keypair.pubkey().to_string();
            let secret = keypair.to_bytes();

            // Check that we have not seen this keypair before
            assert!(keypairs.insert(secret.to_vec()), "Duplicate keypair generated!");
            assert!(pubkeys.insert(pubkey), "Duplicate public key generated!");
        }

        println!("✓ Generated 1000 unique keypairs - entropy is working correctly");
    }

    #[test]
    fn test_keypair_randomness_distribution() {
        // Check that the first byte of public keys has good distribution
        let mut byte_counts = vec![0u32; 256];
        let iterations = 10000;

        for _ in 0..iterations {
            let keypair = Keypair::new();
            let pubkey_bytes = keypair.pubkey().to_bytes();
            byte_counts[pubkey_bytes[0] as usize] += 1;
        }

        // Calculate chi-square statistic
        let expected = iterations as f64 / 256.0;
        let chi_square: f64 = byte_counts.iter()
            .map(|&count| {
                let diff = count as f64 - expected;
                (diff * diff) / expected
            })
            .sum();

        // For 255 degrees of freedom, chi-square should be around 255
        // We will accept values between 200 and 320 (p-value roughly 0.01 to 0.99)
        assert!(chi_square > 200.0 && chi_square < 320.0,
                "Chi-square value {} indicates poor randomness", chi_square);

        println!("✓ Randomness distribution test passed (chi-square: {:.2})", chi_square);
    }

    #[test]
    fn test_concurrent_keypair_generation() {
        // Test that concurrent generation produces unique keys
        use std::sync::{Arc, Mutex};
        use rayon::prelude::*;

        let keypairs = Arc::new(Mutex::new(HashSet::new()));

        (0..1000).into_par_iter().for_each(|_| {
            let keypair = Keypair::new();
            let mut set = keypairs.lock().unwrap();
            assert!(set.insert(keypair.to_bytes().to_vec()),
                    "Duplicate keypair in concurrent generation!");
        });

        println!("✓ Concurrent generation produces unique keypairs");
    }
}
