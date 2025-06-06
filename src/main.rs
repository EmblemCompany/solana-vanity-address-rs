use clap::Parser;
use solana_vanity::{find_vanity_address};
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

    let result = find_vanity_address(&prefix, num_threads);
    let pubkey_str = result.keypair.pubkey().to_string();
    println!("\n🎉 Found a vanity address!");
    println!("Address: {}", pubkey_str);
    println!(
        "Private Key (Base58): {}",
        bs58::encode(result.keypair.secret().as_ref()).into_string()
    );
    println!("Private Key (bytes): {:?}", result.keypair.secret().as_ref());
    println!("Time elapsed: {:?}", result.elapsed);
}
