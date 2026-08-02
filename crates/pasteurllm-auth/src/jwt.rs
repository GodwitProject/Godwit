use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use pasteurllm_core::PasteurError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
}

impl Claims {
    pub fn new(user_id: Uuid, organization_id: Uuid, role: &str) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id.to_string(),
            user_id,
            organization_id,
            role: role.to_string(),
            iat: now.timestamp(),
            exp: (now + Duration::minutes(15)).timestamp(),
        }
    }
}

pub fn issue(secret: &str, claims: Claims, ttl: Duration) -> Result<String, PasteurError> {
    let mut claims = claims;
    let now = Utc::now();
    claims.iat = now.timestamp();
    claims.exp = (now + ttl).timestamp();
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| PasteurError::Auth(e.to_string()))
}

pub fn verify(secret: &str, token: &str) -> Result<Claims, PasteurError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map(|data| data.claims)
    .map_err(|e| PasteurError::Auth(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_and_verify_token() {
        let claims = Claims::new(Uuid::new_v4(), Uuid::new_v4(), "org_admin");
        let token = issue("secret", claims.clone(), Duration::minutes(15)).unwrap();
        let verified = verify("secret", &token).unwrap();
        assert_eq!(verified.user_id, claims.user_id);
    }
}
