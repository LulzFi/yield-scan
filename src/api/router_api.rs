use crate::api::HttpResponseExt;
use actix_web::{
    HttpResponse, Responder, get,
    web::{self},
};

pub fn register(config: &mut web::ServiceConfig) {
    config.service(status);
}

#[get("/status")]
async fn status() -> impl Responder {
    HttpResponse::response_data("OK")
}
