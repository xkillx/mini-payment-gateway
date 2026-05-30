use axum::http::header::AUTHORIZATION;
use axum::{
    extract::{FromRequestParts, Request},
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::Response,
};
use shared_auth::{parse_claims, Actor};
use shared_config::AppConfig;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AuthenticatedActor(pub Actor);

impl<S: Send + Sync> FromRequestParts<S> for AuthenticatedActor {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let config = parts
            .extensions
            .get::<AppConfig>()
            .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Config not available"))?;

        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, "Missing Authorization header"))?;

        match parse_claims(header, &config.jwt_secret) {
            Ok(actor) => Ok(AuthenticatedActor(actor)),
            Err(e) => {
                tracing::warn!("Auth failed: {e}");
                Err((
                    StatusCode::UNAUTHORIZED,
                    "Invalid or missing authentication",
                ))
            }
        }
    }
}

pub async fn auth_middleware(mut req: Request, next: Next) -> Response {
    let config = req.extensions().get::<AppConfig>().cloned();
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_owned());

    if let (Some(config), Some(header)) = (config, auth_header) {
        match parse_claims(&header, &config.jwt_secret) {
            Ok(actor) => {
                req.extensions_mut().insert(actor);
            }
            Err(e) => {
                tracing::warn!("Auth middleware rejected: {e}");
            }
        }
    }

    next.run(req).await
}
