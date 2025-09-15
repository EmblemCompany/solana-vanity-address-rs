# Railway Deployment

## Quick Deploy

[![Deploy on Railway](https://railway.app/button.svg)](https://railway.app/template/deploy?referralCode=emblem)

## Manual Deployment

1. Install Railway CLI:
```bash
npm install -g @railway/cli
```

2. Login to Railway:
```bash
railway login
```

3. Initialize new project:
```bash
railway link
```

4. Deploy:
```bash
railway up
```

## Configuration

Railway will automatically:
- Detect Rust project
- Build with `--features api --bin solana-vanity-api`
- Set PORT environment variable
- Start the API server

## Environment Variables

No environment variables required. The app will use:
- PORT (provided by Railway)
- Default 256 threads for generation

## Performance

Railway provides:
- **No request timeout limits** (unlike Heroku's 30s)
- Better CPU performance for compute-intensive tasks
- Automatic HTTPS
- Global CDN

## API Endpoints

Once deployed, your API will be available at:
`https://your-app.railway.app`

- `GET /health` - Health check
- `GET /generate?pattern=<pattern>&type=<suffix|prefix>&threads=<1-256>` - Generate vanity address

## Example Usage

```bash
# Generate address ending with "bonk"
curl "https://your-app.railway.app/generate?pattern=bonk&type=suffix&threads=256"

# Generate address starting with "Sol"
curl "https://your-app.railway.app/generate?pattern=Sol&type=prefix&threads=256"
```

## Monitoring

View logs:
```bash
railway logs
```

## Costs

Railway offers:
- $5 free credit monthly
- Pay-as-you-go pricing
- ~$0.000463/GB RAM/hour
- ~$0.000463/vCPU/hour

For this API, expect ~$5-10/month with moderate usage.