//! Library for generating Solana vanity addresses.

use solana_sdk::signature::{Keypair, Signer};

#[cfg(feature = "api")]
pub mod api;
use rayon::prelude::*;

/// Utilities for deterministic EVM CREATE2 vanity-address mining.
///
/// Unlike the Solana functions above, this module never creates a wallet or
/// handles private keys. It searches random salts for a deployment whose
/// predicted CREATE2 address matches the requested hexadecimal pattern.
pub mod evm {
    use rand::RngCore;
    use rayon::prelude::*;
    use sha3::{Digest, Keccak256};
    use std::{
        fmt,
        sync::atomic::{AtomicU64, Ordering},
        time::Instant,
    };

    pub const MAX_THREADS: usize = 256;
    pub const MAX_ATTEMPTS: u64 = 100_000_000;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum AddressTarget {
        Token,
        Curve,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum MatchType {
        Prefix,
        Suffix,
    }

    /// The fully-resolved pieces of a Pons V2 launch deployment.
    ///
    /// `token_init_code_prefix` and `token_init_code_suffix` are the token's
    /// complete init code split around the single, ABI-encoded 32-byte curve
    /// address word. That lets the miner derive the token init-code hash for
    /// every candidate curve entirely offline.
    #[derive(Clone, Debug)]
    pub struct PonsV2Create2Search {
        pub launch_deployer: [u8; 20],
        pub original_deployer: [u8; 20],
        pub curve_init_code_hash: [u8; 32],
        pub token_init_code_prefix: Vec<u8>,
        pub token_init_code_suffix: Vec<u8>,
        pub target: AddressTarget,
        pub match_type: MatchType,
        pub pattern: String,
        pub threads: usize,
        pub max_attempts: u64,
    }

    #[derive(Clone, Debug)]
    pub struct PonsV2Create2Result {
        pub token_address: [u8; 20],
        pub curve_address: [u8; 20],
        pub salt: [u8; 32],
        pub derived_salt: [u8; 32],
        pub attempts: u64,
        pub elapsed: std::time::Duration,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Create2MiningError {
        EmptyPattern,
        InvalidPattern,
        InvalidThreadCount,
        InvalidAttemptLimit,
        MissingTokenTemplate,
        Exhausted,
    }

    impl fmt::Display for Create2MiningError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            let message = match self {
                Self::EmptyPattern => "Pattern cannot be empty",
                Self::InvalidPattern => "Pattern must contain only hexadecimal characters",
                Self::InvalidThreadCount => "Thread count must be between 1 and 256",
                Self::InvalidAttemptLimit => "maxAttempts must be between 1 and 100000000",
                Self::MissingTokenTemplate => "Token init-code template cannot be empty",
                Self::Exhausted => "No matching address found before maxAttempts was reached",
            };
            formatter.write_str(message)
        }
    }

    impl std::error::Error for Create2MiningError {}

    fn keccak(bytes: impl AsRef<[u8]>) -> [u8; 32] {
        let hash = Keccak256::digest(bytes.as_ref());
        let mut output = [0_u8; 32];
        output.copy_from_slice(&hash);
        output
    }

    /// Reproduces the salt namespace used by `PonsV2LaunchDeployer`:
    /// `keccak256(abi.encode(originalDeployer, salt))`.
    pub fn pons_v2_launch_salt(original_deployer: [u8; 20], salt: [u8; 32]) -> [u8; 32] {
        let mut encoded = [0_u8; 64];
        encoded[12..32].copy_from_slice(&original_deployer);
        encoded[32..].copy_from_slice(&salt);
        keccak(encoded)
    }

    /// Calculates an EIP-1014 CREATE2 address from a deployer, salt, and init
    /// code hash. No chain access is involved.
    pub fn create2_address(
        deployer: [u8; 20],
        salt: [u8; 32],
        init_code_hash: [u8; 32],
    ) -> [u8; 20] {
        let mut preimage = [0_u8; 85];
        preimage[0] = 0xff;
        preimage[1..21].copy_from_slice(&deployer);
        preimage[21..53].copy_from_slice(&salt);
        preimage[53..].copy_from_slice(&init_code_hash);
        let hash = keccak(preimage);
        let mut address = [0_u8; 20];
        address.copy_from_slice(&hash[12..]);
        address
    }

    fn token_init_code_hash(prefix: &[u8], curve_address: [u8; 20], suffix: &[u8]) -> [u8; 32] {
        let mut hasher = Keccak256::new();
        hasher.update(prefix);
        // Solidity ABI encodes an address as a left-padded 32-byte word.
        hasher.update([0_u8; 12]);
        hasher.update(curve_address);
        hasher.update(suffix);
        let hash = hasher.finalize();
        let mut output = [0_u8; 32];
        output.copy_from_slice(&hash);
        output
    }

    fn matches(address: [u8; 20], pattern: &str, match_type: MatchType) -> bool {
        let address = hex::encode(address);
        match match_type {
            MatchType::Prefix => address.starts_with(pattern),
            MatchType::Suffix => address.ends_with(pattern),
        }
    }

    fn validate(search: &PonsV2Create2Search) -> Result<(), Create2MiningError> {
        if search.pattern.is_empty() {
            return Err(Create2MiningError::EmptyPattern);
        }
        if search.pattern.len() > 40 || !search.pattern.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(Create2MiningError::InvalidPattern);
        }
        if !(1..=MAX_THREADS).contains(&search.threads) {
            return Err(Create2MiningError::InvalidThreadCount);
        }
        if !(1..=MAX_ATTEMPTS).contains(&search.max_attempts) {
            return Err(Create2MiningError::InvalidAttemptLimit);
        }
        if search.token_init_code_prefix.is_empty() && search.token_init_code_suffix.is_empty() {
            return Err(Create2MiningError::MissingTokenTemplate);
        }
        Ok(())
    }

    /// Mines a random Pons V2 launch salt with a local CPU thread pool.
    ///
    /// The only inputs are deployment terms already resolved by the caller;
    /// this function performs neither JSON-RPC calls nor transaction signing.
    pub fn mine_pons_v2_create2(
        search: &PonsV2Create2Search,
    ) -> Result<PonsV2Create2Result, Create2MiningError> {
        validate(search)?;
        let start = Instant::now();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(search.threads)
            .build()
            .map_err(|_| Create2MiningError::InvalidThreadCount)?;
        let attempts = AtomicU64::new(0);

        let result = pool.install(|| {
            (0..search.max_attempts).into_par_iter().find_map_any(|_| {
                attempts.fetch_add(1, Ordering::Relaxed);
                let mut salt = [0_u8; 32];
                rand::thread_rng().fill_bytes(&mut salt);
                let derived_salt = pons_v2_launch_salt(search.original_deployer, salt);
                let curve_address = create2_address(
                    search.launch_deployer,
                    derived_salt,
                    search.curve_init_code_hash,
                );
                let token_address = create2_address(
                    search.launch_deployer,
                    derived_salt,
                    token_init_code_hash(
                        &search.token_init_code_prefix,
                        curve_address,
                        &search.token_init_code_suffix,
                    ),
                );
                let matched_address = match search.target {
                    AddressTarget::Token => token_address,
                    AddressTarget::Curve => curve_address,
                };

                matches(
                    matched_address,
                    &search.pattern.to_ascii_lowercase(),
                    search.match_type,
                )
                .then_some((salt, derived_salt, curve_address, token_address))
            })
        });

        let Some((salt, derived_salt, curve_address, token_address)) = result else {
            return Err(Create2MiningError::Exhausted);
        };

        Ok(PonsV2Create2Result {
            token_address,
            curve_address,
            salt,
            derived_salt,
            attempts: attempts.load(Ordering::Relaxed),
            elapsed: start.elapsed(),
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn matches_the_eip_1014_create2_reference_vector() {
            let address = create2_address([0_u8; 20], [0_u8; 32], keccak([0_u8]));
            assert_eq!(
                hex::encode(address),
                "4d1a2e2bb4f88f0250f26ffff098b0b30b26bf38"
            );
        }

        #[test]
        fn pons_salt_is_bound_to_the_original_deployer() {
            let salt = [7_u8; 32];
            assert_ne!(
                pons_v2_launch_salt([1_u8; 20], salt),
                pons_v2_launch_salt([2_u8; 20], salt)
            );
        }

        #[test]
        fn mines_a_token_prefix_without_rpc_or_keys() {
            let result = mine_pons_v2_create2(&PonsV2Create2Search {
                launch_deployer: [3_u8; 20],
                original_deployer: [4_u8; 20],
                curve_init_code_hash: [5_u8; 32],
                token_init_code_prefix: vec![6_u8; 12],
                token_init_code_suffix: vec![7_u8; 12],
                target: AddressTarget::Token,
                match_type: MatchType::Prefix,
                pattern: "0".to_string(),
                threads: 2,
                max_attempts: 10_000,
            })
            .expect("one hexadecimal leading digit should be found quickly");

            assert!(hex::encode(result.token_address).starts_with('0'));
        }
    }
}

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

    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .ok();

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
        keypair: result
            .lock()
            .unwrap()
            .take()
            .expect("Keypair should be found"),
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

    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .ok();

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
        keypair: result
            .lock()
            .unwrap()
            .take()
            .expect("Keypair should be found"),
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
            assert!(
                keypairs.insert(secret.to_vec()),
                "Duplicate keypair generated!"
            );
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
        let chi_square: f64 = byte_counts
            .iter()
            .map(|&count| {
                let diff = count as f64 - expected;
                (diff * diff) / expected
            })
            .sum();

        // For 255 degrees of freedom, chi-square should be around 255
        // We will accept values between 200 and 320 (p-value roughly 0.01 to 0.99)
        assert!(
            chi_square > 200.0 && chi_square < 320.0,
            "Chi-square value {} indicates poor randomness",
            chi_square
        );

        println!(
            "✓ Randomness distribution test passed (chi-square: {:.2})",
            chi_square
        );
    }

    #[test]
    fn test_concurrent_keypair_generation() {
        // Test that concurrent generation produces unique keys
        use rayon::prelude::*;
        use std::sync::{Arc, Mutex};

        let keypairs = Arc::new(Mutex::new(HashSet::new()));

        (0..1000).into_par_iter().for_each(|_| {
            let keypair = Keypair::new();
            let mut set = keypairs.lock().unwrap();
            assert!(
                set.insert(keypair.to_bytes().to_vec()),
                "Duplicate keypair in concurrent generation!"
            );
        });

        println!("✓ Concurrent generation produces unique keypairs");
    }
}
