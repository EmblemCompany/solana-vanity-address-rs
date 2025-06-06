//! Library for generating Solana vanity addresses.

use solana_sdk::{signature::{Keypair, Signer}, pubkey::Pubkey};
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
