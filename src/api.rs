use crate::evm::{
    AddressTarget, Create2MiningError, MAX_ATTEMPTS, MAX_THREADS, MatchType, PonsV2Create2Search,
    mine_pons_v2_create2,
};
use crate::{find_vanity_address, find_vanity_address_with_suffix};
use actix_web::{
    App, HttpRequest, HttpResponse, HttpServer, Result, http::header, middleware, web,
};
use serde::{Deserialize, Serialize};
use solana_sdk::signature::Signer;
use std::env;

#[derive(Deserialize)]
pub struct GenerateParams {
    #[serde(rename = "type")]
    search_type: Option<String>, // "prefix" or "suffix", defaults to "suffix"
    pattern: String,
    threads: Option<usize>,
}

#[derive(Serialize)]
pub struct GenerateResponse {
    address: String,
    private_key: String,
    pattern: String,
    search_type: String,
    attempts: u64,
    time_ms: u128,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    error: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvmCreate2TargetParams {
    /// `token` or `curve`.
    address: String,
    /// `prefix` or `suffix`.
    #[serde(rename = "type")]
    match_type: String,
    pattern: String,
}

/// A Pons V2 launch resolved by a trusted caller before it reaches the miner.
///
/// The init-code pieces intentionally come from the caller. The worker cannot
/// derive them from a factory address alone without either an RPC call or a
/// copy of the full Pons deployment bytecode.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvmCreate2MineParams {
    launch_deployer: String,
    original_deployer: String,
    curve_init_code_hash: String,
    token_init_code_prefix: String,
    token_init_code_suffix: String,
    target: EvmCreate2TargetParams,
    threads: Option<usize>,
    max_attempts: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvmCreate2MineResponse {
    token_address: String,
    curve_address: String,
    salt: String,
    derived_salt: String,
    target: String,
    pattern: String,
    match_type: String,
    attempts: u64,
    time_ms: u128,
    rpc_calls: u8,
}

fn parse_fixed_hex<const LENGTH: usize>(
    value: &str,
    field: &str,
) -> std::result::Result<[u8; LENGTH], String> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    if value.len() != LENGTH * 2 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "{field} must be a 0x-prefixed {LENGTH}-byte hexadecimal value"
        ));
    }
    let decoded = hex::decode(value).map_err(|_| format!("{field} must be hexadecimal"))?;
    let mut output = [0_u8; LENGTH];
    output.copy_from_slice(&decoded);
    Ok(output)
}

fn parse_hex(value: &str, field: &str) -> std::result::Result<Vec<u8>, String> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    if value.len() % 2 != 0 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("{field} must be an even-length hexadecimal value"));
    }
    hex::decode(value).map_err(|_| format!("{field} must be hexadecimal"))
}

fn as_hex(bytes: impl AsRef<[u8]>) -> String {
    format!("0x{}", hex::encode(bytes))
}

fn api_key_is_valid(request: &HttpRequest) -> bool {
    let Ok(expected_key) = env::var("EVM_VANITY_API_KEY") else {
        // Authentication is opt-in to preserve uncomplicated local development.
        return true;
    };
    if expected_key.is_empty() {
        return true;
    }

    request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|provided_key| provided_key == expected_key)
}

fn parse_evm_search(
    params: EvmCreate2MineParams,
) -> std::result::Result<PonsV2Create2Search, String> {
    let target = match params.target.address.as_str() {
        "token" => AddressTarget::Token,
        "curve" => AddressTarget::Curve,
        _ => return Err("target.address must be 'token' or 'curve'".to_string()),
    };
    let match_type = match params.target.match_type.as_str() {
        "prefix" => MatchType::Prefix,
        "suffix" => MatchType::Suffix,
        _ => return Err("target.type must be 'prefix' or 'suffix'".to_string()),
    };
    let threads = params
        .threads
        .unwrap_or_else(num_cpus::get)
        .min(MAX_THREADS);
    let max_attempts = params.max_attempts.unwrap_or(10_000_000).min(MAX_ATTEMPTS);
    let prefix = parse_hex(&params.token_init_code_prefix, "tokenInitCodePrefix")?;
    let suffix = parse_hex(&params.token_init_code_suffix, "tokenInitCodeSuffix")?;
    if prefix.len() + suffix.len() > 262_144 {
        return Err("token init-code template must be at most 256 KiB".to_string());
    }

    Ok(PonsV2Create2Search {
        launch_deployer: parse_fixed_hex(&params.launch_deployer, "launchDeployer")?,
        original_deployer: parse_fixed_hex(&params.original_deployer, "originalDeployer")?,
        curve_init_code_hash: parse_fixed_hex(&params.curve_init_code_hash, "curveInitCodeHash")?,
        token_init_code_prefix: prefix,
        token_init_code_suffix: suffix,
        target,
        match_type,
        pattern: params.target.pattern.to_ascii_lowercase(),
        threads,
        max_attempts,
    })
}

pub async fn generate_address(params: web::Query<GenerateParams>) -> Result<HttpResponse> {
    // Validate pattern length
    if params.pattern.is_empty() {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: "Pattern cannot be empty".to_string(),
        }));
    }

    if params.pattern.len() > 5 {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: "Pattern too long (max 5 characters for reasonable response time)".to_string(),
        }));
    }

    // Validate pattern characters (Base58)
    if !params
        .pattern
        .chars()
        .all(|c| c.is_ascii_alphanumeric() && c != '0' && c != 'O' && c != 'I' && c != 'l')
    {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: "Pattern must contain only Base58 characters (no 0, O, I, or l)".to_string(),
        }));
    }

    let search_type = params.search_type.as_deref().unwrap_or("suffix");
    let threads = params.threads.unwrap_or(64).min(256); // Allow up to 256 threads

    let result = match search_type {
        "prefix" => find_vanity_address(&params.pattern, threads),
        "suffix" => find_vanity_address_with_suffix(&params.pattern, threads),
        _ => {
            return Ok(HttpResponse::BadRequest().json(ErrorResponse {
                error: "Invalid search type. Use 'prefix' or 'suffix'".to_string(),
            }));
        }
    };

    let response = GenerateResponse {
        address: result.keypair.pubkey().to_string(),
        private_key: bs58::encode(result.keypair.to_bytes()).into_string(),
        pattern: params.pattern.clone(),
        search_type: search_type.to_string(),
        attempts: result.attempts,
        time_ms: result.elapsed.as_millis(),
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Mines a Pons V2 CREATE2 token or curve address entirely in-process.
///
/// The handler offloads the CPU-bound Rayon pool from Actix's request worker.
/// It deliberately accepts no signer, transaction, RPC URL, or private key.
pub async fn mine_evm_create2(
    request: HttpRequest,
    params: web::Json<EvmCreate2MineParams>,
) -> Result<HttpResponse> {
    if !api_key_is_valid(&request) {
        return Ok(HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }));
    }

    let search = match parse_evm_search(params.into_inner()) {
        Ok(search) => search,
        Err(error) => return Ok(HttpResponse::BadRequest().json(ErrorResponse { error })),
    };
    let target = match search.target {
        AddressTarget::Token => "token",
        AddressTarget::Curve => "curve",
    };
    let match_type = match search.match_type {
        MatchType::Prefix => "prefix",
        MatchType::Suffix => "suffix",
    };
    let pattern = search.pattern.clone();

    match web::block(move || mine_pons_v2_create2(&search)).await {
        Ok(Ok(result)) => Ok(HttpResponse::Ok().json(EvmCreate2MineResponse {
            token_address: as_hex(result.token_address),
            curve_address: as_hex(result.curve_address),
            salt: as_hex(result.salt),
            derived_salt: as_hex(result.derived_salt),
            target: target.to_string(),
            pattern,
            match_type: match_type.to_string(),
            attempts: result.attempts,
            time_ms: result.elapsed.as_millis(),
            rpc_calls: 0,
        })),
        Ok(Err(Create2MiningError::Exhausted)) => {
            Ok(HttpResponse::UnprocessableEntity().json(ErrorResponse {
                error: "No matching address found before maxAttempts was reached".to_string(),
            }))
        }
        Ok(Err(error)) => Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: error.to_string(),
        })),
        Err(_) => Ok(HttpResponse::InternalServerError().json(ErrorResponse {
            error: "Vanity mining task failed".to_string(),
        })),
    }
}

pub async fn health() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "solana-vanity-api"
    })))
}

pub async fn run_server() -> std::io::Result<()> {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_addr = format!("0.0.0.0:{}", port);

    println!("Starting server on {}", bind_addr);

    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(
                middleware::DefaultHeaders::new()
                    .add(("Access-Control-Allow-Origin", "*"))
                    .add(("Access-Control-Allow-Methods", "GET, POST, OPTIONS"))
                    .add((
                        "Access-Control-Allow-Headers",
                        "Content-Type, Authorization",
                    )),
            )
            .route("/", web::get().to(health))
            .route("/health", web::get().to(health))
            .route("/generate", web::get().to(generate_address))
            .route("/v1/evm/create2/mine", web::post().to(mine_evm_create2))
    })
    .bind(&bind_addr)?
    .run()
    .await
}
