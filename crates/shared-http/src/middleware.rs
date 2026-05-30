use axum::http::HeaderName;
use axum::http::Request;
use tower_http::request_id::{
    MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer,
};
use uuid::Uuid;

const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

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
    SetRequestIdLayer::new(X_REQUEST_ID, RequestIdGenerator)
}

pub fn propagate_request_id_layer() -> PropagateRequestIdLayer {
    PropagateRequestIdLayer::new(X_REQUEST_ID)
}
