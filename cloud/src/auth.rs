// Authentication module for Zeus Cloud

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // User ID
    pub exp: usize,   // Expiration time
    pub iat: usize,   // Issued at
}

impl Claims {
    pub fn new(user_id: String) -> Self {
        let now = Utc::now();
        let exp = now + Duration::hours(24);
        
        Claims {
            sub: user_id,
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
        }
    }
}

/// Generate JWT token
pub fn generate_token(user_id: String, secret: &str) -> anyhow::Result<String> {
    let claims = Claims::new(user_id);
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;
    Ok(token)
}

/// Verify JWT token
pub fn verify_token(token: &str, secret: &str) -> anyhow::Result<Claims> {
    let validation = Validation::default();
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;
    Ok(token_data.claims)
}
