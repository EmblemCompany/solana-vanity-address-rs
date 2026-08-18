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

Existing Solana endpoints are unchanged. The EVM endpoint below is additive and
does not create a keypair, sign a transaction, or make an RPC request.

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

---

## Robinhood Chain / Pons V2 offline CREATE2 mining

### `POST /v1/evm/create2/mine`

Mines a salt for a **fully resolved Pons V2 launch**. The worker only performs
local Keccak-256 and CREATE2 calculations. It never receives a wallet private
key, an RPC URL, or transaction calldata to broadcast.

This endpoint preserves Pons V2's deployment rules:

1. `derivedSalt = keccak256(abi.encode(originalDeployer, salt))`
2. `curve = CREATE2(launchDeployer, derivedSalt, curveInitCodeHash)`
3. `token = CREATE2(launchDeployer, derivedSalt, keccak256(prefix ++ abi.encode(curve) ++ suffix))`

`prefix` and `suffix` are the exact final token init code split around the
single ABI-encoded curve-address word. They must be prepared from the same
launch terms that will be sent to Pons; changing the name, symbol, metadata,
socials, factory dependencies, launch configuration, creator fee recipient, or
economics invalidates a mined result.

Pons V2 currently uses the following Robinhood Chain launch deployer:

```
0x3711ceA4feaDE896C913C68F01Eda97Cb06D1A42
```

#### Request body

```json
{
  "launchDeployer": "0x3711ceA4feaDE896C913C68F01Eda97Cb06D1A42",
  "originalDeployer": "0x1234567890abcdef1234567890abcdef12345678",
  "curveInitCodeHash": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "tokenInitCodePrefix": "0x60806040...",
  "tokenInitCodeSuffix": "0x...",
  "target": {
    "address": "token",
    "type": "suffix",
    "pattern": "eb1"
  },
  "threads": 32,
  "maxAttempts": 10000000
}
```

| Field | Required | Description |
| --- | --- | --- |
| `launchDeployer` | yes | CREATE2 deployer address (20-byte hex). For Pons V2, use the deployed launch deployer above. |
| `originalDeployer` | yes | Account that will call `launchToken`; it namespaces the salt. |
| `curveInitCodeHash` | yes | `keccak256` of the final `PonsV2BondingCurve` init code. |
| `tokenInitCodePrefix` | yes | Token init code before the 32-byte ABI curve-address word. |
| `tokenInitCodeSuffix` | yes | Token init code after that word. |
| `target.address` | yes | `token` or `curve`. |
| `target.type` | yes | `prefix` or `suffix`. |
| `target.pattern` | yes | 1–40 hexadecimal characters; matching is case-insensitive. |
| `threads` | no | CPU threads, default host-core count, maximum 256. |
| `maxAttempts` | no | Candidate salts to try, default 10,000,000, maximum 100,000,000. |

#### Successful response

```json
{
  "tokenAddress": "0x0d4cb264d7d2b3b3b5466e65ae3e8bc2603f0eb1",
  "curveAddress": "0x5bf8b9b6d05f1cfe26e6332c8a16a3495f53ce5c",
  "salt": "0xb47d...",
  "derivedSalt": "0x173a...",
  "target": "token",
  "pattern": "eb1",
  "matchType": "suffix",
  "attempts": 1843267,
  "timeMs": 426,
  "rpcCalls": 0
}
```

The returned `salt` (not `derivedSalt`) is the value to put in Pons'
`TokenParams.salt`. Before submitting, call the verified
`PonsV2LaunchDeployer.predictLaunchAddresses` once with the final deployment
struct and require that it returns the same two addresses. This one final read
is a guard against stale terms; it is not part of the mining loop.

#### Failure responses

- `400`: malformed deployment input or unsupported target/match type.
- `401`: `EVM_VANITY_API_KEY` is set and the `Authorization: Bearer <key>`
  header is missing or invalid.
- `422`: no candidate matched before `maxAttempts`.
- `500`: worker execution failed.

#### Secure deployment

Set `EVM_VANITY_API_KEY` in production and call the service only from a
first-party backend/API route. Never expose it directly to browsers. The
service accepts no signing key by design; the creator signs the final Pons
factory `launchToken` transaction locally with their wallet.
