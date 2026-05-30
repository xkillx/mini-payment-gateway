use axum::http::HeaderMap;
use axum::http::HeaderName;
use axum::http::Request;
use tower_http::request_id::{
    MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer,
};
use uuid::Uuid;

pub const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

#[derive(Clone, Default)]
pub struct RequestIdGenerator;

impl MakeRequestId for RequestIdGenerator {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        let id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext))
            .to_string()
            .parse()
            .ok()?;
        Some(RequestId::new(id))
    }
}

pub fn request_id_layer() -> SetRequestIdLayer<RequestIdGenerator> {
    SetRequestIdLayer::new(X_REQUEST_ID.clone(), RequestIdGenerator)
}

pub fn propagate_request_id_layer() -> PropagateRequestIdLayer {
    PropagateRequestIdLayer::new(X_REQUEST_ID.clone())
}

pub fn get_request_id(headers: &HeaderMap) -> String {
    headers
        .get(&X_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_owned())
        .unwrap_or_else(|| Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string())
}
