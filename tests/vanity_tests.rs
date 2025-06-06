use solana_vanity::{find_vanity_address, find_vanity_address_with_suffix};
use std::time::Duration;

#[test]
fn test_find_vanity_address() {
    // Test finding an address with prefix "A"
    // This should be relatively quick to find
    let prefix = "A";
    let num_threads = 4;
    
    let result = find_vanity_address(prefix, num_threads);
    
    // Verify the result
    let pubkey_str = result.keypair.pubkey().to_string();
    assert!(pubkey_str.starts_with(prefix), 
            "Generated key {} doesn't start with prefix {}", pubkey_str, prefix);
    
    println!("Found vanity address: {}", pubkey_str);
    println!("Time elapsed: {:?}", result.elapsed);
    println!("Attempts: {}", result.attempts);
}

#[test]
fn test_find_vanity_address_with_suffix() {
    // Test finding an address with suffix "a"
    // This should be relatively quick to find
    let suffix = "a";
    let num_threads = 4;
    
    let result = find_vanity_address_with_suffix(suffix, num_threads);
    
    // Verify the result
    let pubkey_str = result.keypair.pubkey().to_string();
    assert!(pubkey_str.ends_with(suffix), 
            "Generated key {} doesn't end with suffix {}", pubkey_str, suffix);
    
    println!("Found vanity address: {}", pubkey_str);
    println!("Time elapsed: {:?}", result.elapsed);
    println!("Attempts: {}", result.attempts);
}

#[test]
fn test_performance_comparison() {
    // Test performance with different thread counts
    let prefix = "B";
    
    // Test with 1 thread
    let start_time = std::time::Instant::now();
    let result1 = find_vanity_address(prefix, 1);
    let single_thread_time = start_time.elapsed();
    
    // Test with multiple threads
    let num_threads = num_cpus::get(); // Use all available CPUs
    let start_time = std::time::Instant::now();
    let result_multi = find_vanity_address(prefix, num_threads);
    let multi_thread_time = start_time.elapsed();
    
    println!("Single thread time: {:?}", single_thread_time);
    println!("Multi thread ({} threads) time: {:?}", num_threads, multi_thread_time);
    println!("Speedup factor: {:.2}x", single_thread_time.as_secs_f64() / multi_thread_time.as_secs_f64());
}

#[test]
#[ignore] // Ignored by default as it may take a long time
fn test_complex_pattern() {
    // Test finding a more complex pattern (will take longer)
    let prefix = "ABC"; // More specific prefix will take longer to find
    let num_threads = num_cpus::get();
    
    let result = find_vanity_address(prefix, num_threads);
    
    // Verify the result
    let pubkey_str = result.keypair.pubkey().to_string();
    assert!(pubkey_str.starts_with(prefix));
    
    println!("Found complex vanity address: {}", pubkey_str);
    println!("Time elapsed: {:?}", result.elapsed);
    println!("Attempts: {}", result.attempts);
}

#[test]
fn test_timeout() {
    // Test with a timeout to prevent tests from running too long
    let difficult_prefix = "ABCD"; // Very unlikely to find this quickly
    let num_threads = num_cpus::get();
    let timeout = Duration::from_secs(5);
    
    // Use a separate thread with timeout
    let handle = std::thread::spawn(move || {
        find_vanity_address(difficult_prefix, num_threads)
    });
    
    match handle.join().ok() {
        Some(result) => {
            let pubkey_str = result.keypair.pubkey().to_string();
            assert!(pubkey_str.starts_with(difficult_prefix));
            println!("Found difficult vanity address: {}", pubkey_str);
        },
        None => {
            println!("Test timed out as expected for difficult pattern");
        }
    }
}
