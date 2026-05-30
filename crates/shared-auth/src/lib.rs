use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub merchant_id: Option<String>,
    pub exp: usize,
}

#[derive(Debug, Clone)]
pub enum Actor {
    Merchant { actor_id: Uuid, merchant_id: Uuid },
    Administrator { actor_id: Uuid },
}

impl Actor {
    pub fn actor_id(&self) -> Uuid {
        match self {
            Actor::Merchant { actor_id, .. } => *actor_id,
            Actor::Administrator { actor_id } => *actor_id,
        }
    }

    pub fn is_merchant(&self) -> bool {
        matches!(self, Actor::Merchant { .. })
    }

    pub fn is_administrator(&self) -> bool {
        matches!(self, Actor::Administrator { .. })
    }

    pub fn merchant_id(&self) -> Option<Uuid> {
        match self {
            Actor::Merchant { merchant_id, .. } => Some(*merchant_id),
            Actor::Administrator { .. } => None,
        }
    }
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid token: {0}")]
    InvalidToken(String),
    #[error("Missing required claim: {0}")]
    MissingClaim(String),
}

pub fn parse_claims(token: &str, secret: &str) -> Result<Actor, AuthError> {
    let token = token.trim_start_matches("Bearer ");

    let mut validation = Validation::new(Algorithm::HS256);
    validation.required_spec_claims.clear();
    validation.validate_exp = true;

    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

    let claims = data.claims;

    let sub_uuid = Uuid::parse_str(&claims.sub)
        .map_err(|_| AuthError::InvalidToken("sub is not a valid UUID".into()))?;

    match claims.role.as_str() {
        "merchant" => {
            let merchant_id = claims.merchant_id.ok_or_else(|| {
                AuthError::MissingClaim("merchant_id required for merchant role".into())
            })?;
            let merchant_uuid = Uuid::parse_str(&merchant_id)
                .map_err(|_| AuthError::InvalidToken("merchant_id is not a valid UUID".into()))?;
            Ok(Actor::Merchant {
                actor_id: sub_uuid,
                merchant_id: merchant_uuid,
            })
        }
        "administrator" => Ok(Actor::Administrator { actor_id: sub_uuid }),
        other => Err(AuthError::InvalidToken(format!("Unknown role: {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};

    fn make_token(claims: Claims, secret: &str) -> String {
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap()
    }

    #[test]
    fn valid_merchant_token() {
        let claims = Claims {
            sub: "00000000-0000-0000-0000-000000000001".into(),
            role: "merchant".into(),
            merchant_id: Some("00000000-0000-0000-0000-000000000001".into()),
            exp: 9999999999,
        };
        let token = make_token(claims, "secret");
        let actor = parse_claims(&token, "secret").unwrap();
        assert!(actor.is_merchant());
        assert!(!actor.is_administrator());
        assert!(actor.merchant_id().is_some());
    }

    #[test]
    fn valid_admin_token() {
        let claims = Claims {
            sub: "00000000-0000-0000-0000-000000000002".into(),
            role: "administrator".into(),
            merchant_id: None,
            exp: 9999999999,
        };
        let token = make_token(claims, "secret");
        let actor = parse_claims(&token, "secret").unwrap();
        assert!(actor.is_administrator());
        assert!(!actor.is_merchant());
        assert!(actor.merchant_id().is_none());
    }

    #[test]
    fn merchant_missing_merchant_id_rejected() {
        let claims = Claims {
            sub: "00000000-0000-0000-0000-000000000001".into(),
            role: "merchant".into(),
            merchant_id: None,
            exp: 9999999999,
        };
        let token = make_token(claims, "secret");
        let result = parse_claims(&token, "secret");
        assert!(result.is_err());
    }

    #[test]
    fn invalid_signature_rejected() {
        let claims = Claims {
            sub: "00000000-0000-0000-0000-000000000001".into(),
            role: "merchant".into(),
            merchant_id: Some("00000000-0000-0000-0000-000000000001".into()),
            exp: 9999999999,
        };
        let token = make_token(claims, "other-secret");
        let result = parse_claims(&token, "real-secret");
        assert!(result.is_err());
    }

    #[test]
    fn expired_token_rejected() {
        let claims = Claims {
            sub: "00000000-0000-0000-0000-000000000001".into(),
            role: "merchant".into(),
            merchant_id: Some("00000000-0000-0000-0000-000000000001".into()),
            exp: 1000000000,
        };
        let token = make_token(claims, "secret");
        let result = parse_claims(&token, "secret");
        assert!(result.is_err());
    }
}
