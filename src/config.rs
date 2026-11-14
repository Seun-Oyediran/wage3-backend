use std::env;

use alloy::primitives::Address;

#[derive(Debug)]
pub struct Config {
    pub port: u16,
    pub database_url: String,

    pub jwt_secret: String,
    pub jwt_refresh_secret: String,

    pub access_token_ttl_minutes: i64,
    pub refresh_token_ttl_days: i64,

    pub hmac_secret: String,

    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_redirect_uri: String,
    pub frontend_url: String,

    pub resend_api_key: String,
    pub email_from: String,

    pub rpc_url: String,
    pub chain_id: i64,
    pub token_address: Address,
    pub token_symbol: String,
    pub token_decimals: i32,
    pub batcher_address: Address,
    pub batcher_start_block: u64,
    pub indexer_max_range: u64,
}

impl Config {
    pub fn new() -> Result<Self, String> {
        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .map_err(|e| format!("PORT must be a valid port number: {}", e))?;

        let database_url =
            env::var("DATABASE_URL").map_err(|_| "Database url is compulsory".to_string())?;

        let jwt_secret =
            env::var("JWT_SECRET").map_err(|_| "Jwt secret is compulsory".to_string())?;
        let jwt_refresh_secret = env::var("JWT_REFRESH_SECRET")
            .map_err(|_| "Jwt refresh secret is compulsory".to_string())?;

        let access_token_ttl_minutes = env::var("ACCESS_TOKEN_TTL_MINUTES")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<i64>()
            .map_err(|e| format!("ACCESS_TOKEN_TTL_MINUTES must be a valid number: {}", e))?;

        if access_token_ttl_minutes < 1 {
            return Err("ACCESS_TOKEN_TTL_MINUTES must be greater than 0".to_string());
        }

        let refresh_token_ttl_days = env::var("REFRESH_TOKEN_TTL_DAYS")
            .unwrap_or_else(|_| "14".to_string())
            .parse::<i64>()
            .map_err(|e| format!("REFRESH_TOKEN_TTL_DAYS must be a valid number: {}", e))?;

        if refresh_token_ttl_days < 1 {
            return Err("REFRESH_TOKEN_TTL_DAYS must be greater than 0".to_string());
        }

        let hmac_secret =
            env::var("HMAC_SECRET").map_err(|_| "Hmac secret is compulsory".to_string())?;

        let google_client_id = env::var("GOOGLE_CLIENT_ID")
            .map_err(|_| "Google client id is compulsory".to_string())?;
        let google_client_secret = env::var("GOOGLE_CLIENT_SECRET")
            .map_err(|_| "Google client secret is compulsory".to_string())?;
        let google_redirect_uri = env::var("GOOGLE_REDIRECT_URI")
            .map_err(|_| "Google redirect uri is compulsory".to_string())?;
        let frontend_url =
            env::var("FRONTEND_URL").map_err(|_| "Frontend url is compulsory".to_string())?;

        let resend_api_key = env::var("RESEND_API_KEY")
            .map_err(|_| "Resend api key is compulsory".to_string())?;
        let email_from =
            env::var("EMAIL_FROM").map_err(|_| "Email from is compulsory".to_string())?;

        let rpc_url = env::var("RPC_URL").map_err(|_| "Rpc url is compulsory".to_string())?;
        let chain_id = env::var("CHAIN_ID")
            .unwrap_or_else(|_| "84532".to_string())
            .parse::<i64>()
            .map_err(|e| format!("Chain id must be a valid number: {}", e))?;

        let token_address = env::var("TOKEN_ADDRESS")
            .map_err(|_| "Token address is compulsory".to_string())?
            .parse::<Address>()
            .map_err(|e| format!("TOKEN_ADDRESS must be a valid address: {}", e))?;
        let token_symbol = env::var("TOKEN_SYMBOL").unwrap_or_else(|_| "USDC".to_string());
        let token_decimals = env::var("TOKEN_DECIMALS")
            .unwrap_or_else(|_| "6".to_string())
            .parse::<i32>()
            .map_err(|e| format!("TOKEN_DECIMALS must be a valid number: {}", e))?;

        let batcher_start_block = env::var("BATCHER_START_BLOCK")
            .map_err(|_| "Start block is compulsory".to_string())?
            .parse::<u64>()
            .map_err(|e| format!("Start block must be a valid number: {}", e))?;

        let indexer_max_range = env::var("INDEXER_MAX_RANGE")
            .unwrap_or_else(|_| "200".to_string())
            .parse::<u64>()
            .map_err(|e| format!("Indexer max range must be a valid number: {}", e))?;

        if indexer_max_range < 1 {
            return Err("Indexer max range  must be greater than 0".to_string());
        }

        let batcher_address = env::var("BATCHER_ADDRESS")
            .map_err(|_| "Batcher address is compulsory".to_string())?
            .parse::<Address>()
            .map_err(|e| format!("BATCHER_ADDRESS must be a valid address: {}", e))?;

        Ok(Self {
            port,
            jwt_secret,
            jwt_refresh_secret,
            access_token_ttl_minutes,
            refresh_token_ttl_days,
            database_url,
            hmac_secret,
            google_client_id,
            google_client_secret,
            google_redirect_uri,
            frontend_url,
            resend_api_key,
            email_from,
            rpc_url,
            chain_id,
            batcher_start_block,
            indexer_max_range,
            batcher_address,
            token_address,
            token_symbol,
            token_decimals,
        })
    }
}
