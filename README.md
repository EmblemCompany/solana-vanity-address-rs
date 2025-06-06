# Solana Vanity Address Generator

This project is a fast CLI tool to generate Solana wallet addresses (public keys) with a custom prefix (vanity address), leveraging multithreading for speed.

## Features
- Generates Solana addresses with a user-specified prefix
- Multithreaded for high performance (using all CPU cores by default)
- Outputs both the public address and private key

## Installation

1. **Install Rust** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
   Follow the on-screen instructions to complete the installation.

2. **Clone this repository:**
   ```bash
   git clone <repo-url>
   cd solana-vanity-address-rs
   ```

3. **Build the project:**
   ```bash
   cargo build --release
   ```

## Usage

Run the program with your desired prefix:

```bash
cargo run --release -- --prefix <PREFIX>
```

- Replace `<PREFIX>` with the string you want your Solana address to start with (case-sensitive).
- Optionally, specify the number of threads (defaults to all CPU cores):

```bash
cargo run --release -- --prefix <PREFIX> --threads 8
```

### Example

```bash
cargo run --release -- --prefix Sol
```

This will search for a Solana address starting with `Sol` using all available CPU cores.

## Output
- The program prints the matching address, the private key (in Base58 and bytes), and the time taken.

## Library Usage

You can use this crate as a library in your own Rust project:

Add to your `Cargo.toml`:

```toml
solana-vanity = "0.1.0"
```

Example usage:

```rust
use solana_vanity::find_vanity_address;

let result = find_vanity_address("Sol", 8);
println!("Address: {}", result.keypair.pubkey());
```

## License
MIT
