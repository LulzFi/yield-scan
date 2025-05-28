use serde_json::json;

pub mod router_api;
pub mod server;

pub struct APIState {}

pub struct DBRefresh;

pub trait HttpResponseExt {
    fn response_text(value: &str) -> Self;
    fn response_data<T: serde::Serialize>(value: T) -> Self;
    fn response_error(error: &str) -> Self;
    fn response_error_notfound() -> Self;
}

impl HttpResponseExt for actix_web::HttpResponse {
    fn response_text(value: &str) -> Self {
        Self::Ok().body(value.to_string())
    }

    fn response_data<T: serde::Serialize>(value: T) -> Self {
        Self::Ok().json(json!({ "error_code": 0, "data": value }))
    }

    fn response_error(error: &str) -> Self {
        Self::Ok().json(json!({ "error_code": 1, "error": error }))
    }

    fn response_error_notfound() -> Self {
        Self::Ok().json(json!({ "error_code": 1, "error": "not found" }))
    }
}
