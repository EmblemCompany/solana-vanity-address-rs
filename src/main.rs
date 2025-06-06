use clap::Parser;
use solana_sdk::{ signature::{ Keypair, Signer }, pubkey::Pubkey };
use std::{ sync::atomic::{ AtomicBool, Ordering }, time::Instant };
use rayon::prelude::*;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// The desired prefix for the vanity address (case-sensitive)
    #[arg(short, long)]
    prefix: String,

    /// Number of threads to use (defaults to available CPU cores)
    #[arg(short, long, default_value_t = num_cpus::get())]
    threads: usize,
}

fn main() {
    let args = Args::parse();
    let prefix = args.prefix;
    let num_threads = args.threads;

    println!("Searching for Solana vanity address starting with: \"{}\"", prefix);
    println!("Using {} threads...", num_threads);

    let found = AtomicBool::new(false);
    let start_time = Instant::now();
    let attempts = std::sync::atomic::AtomicU64::new(0);

    rayon::ThreadPoolBuilder::new().num_threads(num_threads).build_global().unwrap();

    // Loop indefinitely until a match is found
    loop {
        if found.load(Ordering::SeqCst) {
            break; // Stop if another thread found a match
        }

        // Generate keypairs in parallel
        (0..100000).into_par_iter().for_each(|_| {
            // Process in batches for efficiency
            if found.load(Ordering::SeqCst) {
                return;
            }

            let keypair = Keypair::new();
            let pubkey_str = keypair.pubkey().to_string(); // Base58 encoded address

            attempts.fetch_add(1, Ordering::Relaxed); // Atomically increment the attempt counter

            if pubkey_str.starts_with(&prefix) {
                found.store(true, Ordering::SeqCst);
                let elapsed = start_time.elapsed();
                println!("\n🎉 Found a vanity address!");
                println!("Address: {}", pubkey_str);
                println!(
                    "Private Key (Base58): {}",
                    bs58::encode(keypair.secret().as_ref()).into_string()
                );
                println!("Private Key (bytes): {:?}", keypair.secret().as_ref());
                println!("Time elapsed: {:?}", elapsed);
                // Optionally save to a file here
            }
        });
    }
}
