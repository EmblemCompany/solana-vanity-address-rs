use actix_web::{web, App, HttpResponse, HttpServer, Result, middleware};
use serde::{Deserialize, Serialize};
use std::env;
use crate::{find_vanity_address, find_vanity_address_with_suffix};
use solana_sdk::signature::Signer;

#[derive(Deserialize)]
pub struct GenerateParams {
    #[serde(rename = "type")]
    search_type: Option<String>,  // "prefix" or "suffix", defaults to "suffix"
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
    if !params.pattern.chars().all(|c| c.is_ascii_alphanumeric() && c != '0' && c != 'O' && c != 'I' && c != 'l') {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: "Pattern must contain only Base58 characters (no 0, O, I, or l)".to_string(),
        }));
    }

    let search_type = params.search_type.as_deref().unwrap_or("suffix");
    let threads = params.threads.unwrap_or(64).min(64); // Default to 64 threads

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
                    .header("Access-Control-Allow-Origin", "*")
                    .header("Access-Control-Allow-Methods", "GET, OPTIONS")
                    .header("Access-Control-Allow-Headers", "Content-Type")
            )
            .route("/", web::get().to(health))
            .route("/health", web::get().to(health))
            .route("/generate", web::get().to(generate_address))
    })
    .bind(&bind_addr)?
    .run()
    .await
}