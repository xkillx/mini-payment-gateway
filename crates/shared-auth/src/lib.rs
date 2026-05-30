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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Role {
    Merchant,
    Administrator,
}

#[derive(Debug, Clone)]
pub struct ParsedClaims {
    pub actor_id: Uuid,
    pub role: Role,
    pub merchant_id: Option<Uuid>,
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

    pub fn role(&self) -> Role {
        match self {
            Actor::Merchant { .. } => Role::Merchant,
            Actor::Administrator { .. } => Role::Administrator,
        }
    }
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Missing token")]
    MissingToken,
    #[error("Malformed token")]
    MalformedToken,
    #[error("Expired token")]
    ExpiredToken,
    #[error("Invalid token: {0}")]
    InvalidToken(String),
    #[error("Invalid claims: {0}")]
    InvalidClaims(String),
}

pub fn parse_authorization_header(auth_header: &str) -> Result<&str, AuthError> {
    let rest = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AuthError::MalformedToken)?;
    if rest.is_empty() {
        return Err(AuthError::MalformedToken);
    }
    Ok(rest)
}

pub fn parse_claims(token: &str, secret: &str) -> Result<ParsedClaims, AuthError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.required_spec_claims = HashSet::from(["exp".to_owned()]);
    validation.validate_exp = true;

    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("ExpiredSignature") {
            AuthError::ExpiredToken
        } else {
            AuthError::InvalidToken(msg)
        }
    })?;

    let claims = data.claims;

    let actor_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AuthError::InvalidClaims("sub is not a valid UUID".into()))?;

    match claims.role.as_str() {
        "merchant" => {
            let merchant_id_str = claims.merchant_id.ok_or_else(|| {
                AuthError::InvalidClaims("merchant_id required for merchant role".into())
            })?;
            let merchant_id = Uuid::parse_str(&merchant_id_str)
                .map_err(|_| AuthError::InvalidClaims("merchant_id is not a valid UUID".into()))?;
            Ok(ParsedClaims {
                actor_id,
                role: Role::Merchant,
                merchant_id: Some(merchant_id),
            })
        }
        "administrator" => {
            if claims.merchant_id.is_some() {
                return Err(AuthError::InvalidClaims(
                    "administrator token must not include merchant_id".into(),
                ));
            }
            Ok(ParsedClaims {
                actor_id,
                role: Role::Administrator,
                merchant_id: None,
            })
        }
        other => Err(AuthError::InvalidClaims(format!("Unknown role: {other}"))),
    }
}

use std::collections::HashSet;

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

    fn merchant_claims() -> Claims {
        Claims {
            sub: "00000000-0000-0000-0000-000000000001".into(),
            role: "merchant".into(),
            merchant_id: Some("00000000-0000-0000-0000-000000000001".into()),
            exp: 9999999999,
        }
    }

    fn admin_claims() -> Claims {
        Claims {
            sub: "00000000-0000-0000-0000-000000000002".into(),
            role: "administrator".into(),
            merchant_id: None,
            exp: 9999999999,
        }
    }

    #[test]
    fn valid_merchant_token() {
        let token = make_token(merchant_claims(), "secret");
        let claims = parse_claims(&token, "secret").unwrap();
        assert_eq!(claims.role, Role::Merchant);
        assert_eq!(
            claims.actor_id,
            Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
        );
        assert!(claims.merchant_id.is_some());
    }

    #[test]
    fn valid_admin_token() {
        let token = make_token(admin_claims(), "secret");
        let claims = parse_claims(&token, "secret").unwrap();
        assert_eq!(claims.role, Role::Administrator);
        assert!(claims.merchant_id.is_none());
    }

    #[test]
    fn missing_authorization_header_rejected() {
        let result = parse_authorization_header("");
        assert!(matches!(result, Err(AuthError::MalformedToken)));
    }

    #[test]
    fn raw_token_without_bearer_rejected() {
        let token = make_token(merchant_claims(), "secret");
        let result = parse_authorization_header(&token);
        assert!(matches!(result, Err(AuthError::MalformedToken)));
    }

    #[test]
    fn non_bearer_scheme_rejected() {
        let token = make_token(merchant_claims(), "secret");
        let header = format!("Token {token}");
        let result = parse_authorization_header(&header);
        assert!(matches!(result, Err(AuthError::MalformedToken)));
    }

    #[test]
    fn empty_bearer_token_rejected() {
        let result = parse_authorization_header("Bearer ");
        assert!(matches!(result, Err(AuthError::MalformedToken)));
    }

    #[test]
    fn expired_token_rejected() {
        let claims = Claims {
            exp: 1000000000,
            ..merchant_claims()
        };
        let token = make_token(claims, "secret");
        let result = parse_claims(&token, "secret");
        assert!(matches!(result, Err(AuthError::ExpiredToken)));
    }

    #[test]
    fn invalid_signature_rejected() {
        let token = make_token(merchant_claims(), "other-secret");
        let result = parse_claims(&token, "real-secret");
        assert!(matches!(result, Err(AuthError::InvalidToken(_))));
    }

    #[test]
    fn merchant_missing_merchant_id_rejected() {
        let claims = Claims {
            merchant_id: None,
            ..merchant_claims()
        };
        let token = make_token(claims, "secret");
        let result = parse_claims(&token, "secret");
        assert!(matches!(result, Err(AuthError::InvalidClaims(_))));
    }

    #[test]
    fn admin_with_merchant_id_rejected() {
        let claims = Claims {
            merchant_id: Some("00000000-0000-0000-0000-000000000001".into()),
            ..admin_claims()
        };
        let token = make_token(claims, "secret");
        let result = parse_claims(&token, "secret");
        assert!(matches!(result, Err(AuthError::InvalidClaims(_))));
    }

    #[test]
    fn invalid_uuid_in_sub_rejected() {
        let claims = Claims {
            sub: "not-a-uuid".into(),
            ..merchant_claims()
        };
        let token = make_token(claims, "secret");
        let result = parse_claims(&token, "secret");
        assert!(matches!(result, Err(AuthError::InvalidClaims(_))));
    }

    #[test]
    fn invalid_uuid_in_merchant_id_rejected() {
        let claims = Claims {
            merchant_id: Some("not-a-uuid".into()),
            ..merchant_claims()
        };
        let token = make_token(claims, "secret");
        let result = parse_claims(&token, "secret");
        assert!(matches!(result, Err(AuthError::InvalidClaims(_))));
    }
}
