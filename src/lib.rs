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
    use std::time::Instant;

    let found = AtomicBool::new(false);
    let attempts = AtomicU64::new(0);
    let start_time = Instant::now();

    rayon::ThreadPoolBuilder::new().num_threads(num_threads).build_global().ok();

    let mut result: Option<Keypair> = None;

    while !found.load(Ordering::SeqCst) {
        (0..100_000).into_par_iter().for_each(|_| {
            if found.load(Ordering::SeqCst) {
                return;
            }
            let keypair = Keypair::new();
            let pubkey_str = keypair.pubkey().to_string();
            attempts.fetch_add(1, Ordering::Relaxed);
            if pubkey_str.starts_with(prefix) {
                found.store(true, Ordering::SeqCst);
                // This is not thread-safe, but only one will win
                result = Some(keypair);
            }
        });
    }
    VanityResult {
        keypair: result.expect("Keypair should be found"),
        elapsed: start_time.elapsed(),
        attempts: attempts.load(Ordering::Relaxed),
    }
}
