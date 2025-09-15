# Solana Vanity Address API

## Overview
HTTP API endpoint for generating Solana vanity addresses on demand. Returns a single keypair matching the specified pattern.

## Deployment

### Heroku Setup
1. Create a new Heroku app
2. Add Rust buildpack:
   ```bash
   heroku buildpacks:add emk/rust
   ```
3. Deploy:
   ```bash
   git push heroku main
   ```

### Local Development
```bash
# Build with API features
cargo build --release --features api --bin solana-vanity-api

# Run locally
PORT=8080 ./target/release/solana-vanity-api
```

## API Endpoints

### Health Check
```
GET /health
```

Response:
```json
{
  "status": "healthy",
  "service": "solana-vanity-api"
}
```

### Generate Vanity Address
```
GET /generate?pattern=<PATTERN>&type=<TYPE>&threads=<THREADS>
```

Parameters:
- `pattern` (required): The pattern to search for (max 5 characters)
- `type` (optional): `prefix` or `suffix` (default: `suffix`)
- `threads` (optional): Number of threads to use (default: 64, max: 64)

Response:
```json
{
  "address": "sjfsmt8Wp3rqLgLm9rrrwK4YzRSrGLoPVmpJNmzbonk",
  "private_key": "4nBTGC37SgQ7Ewcn6n6BAfvPP9taiNY91kbfuuDEYCEUhLeMJuUE2G8AztuMHKQdTKGXv2ut6sV9hU1JxAPp4k5S",
  "pattern": "bonk",
  "search_type": "suffix",
  "attempts": 2068427,
  "time_ms": 6066
}
```

### Example Requests

Generate address ending with "bonk":
```bash
curl "https://your-app.herokuapp.com/generate?pattern=bonk&type=suffix"
```

Generate address starting with "Sol":
```bash
curl "https://your-app.herokuapp.com/generate?pattern=Sol&type=prefix"
```

Generate with custom thread count:
```bash
curl "https://your-app.herokuapp.com/generate?pattern=xy&threads=32"
```

## Performance Notes

- Patterns with 1-2 characters: < 1 second
- Patterns with 3 characters: 1-5 seconds
- Patterns with 4 characters: 5-30 seconds
- Patterns with 5 characters: 30 seconds - several minutes

The API limits patterns to 5 characters maximum to ensure reasonable response times.

## Security

- Each generated keypair uses cryptographically secure random number generation
- Private keys are returned in Base58 format
- Store private keys securely - they provide full control of the address
- Consider using HTTPS in production to protect private keys in transit