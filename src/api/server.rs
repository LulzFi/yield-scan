use super::APIState;
use crate::libs::config::{HTTP_BIND, HTTP_PORT};
use actix_cors::Cors;
use actix_web::{App, HttpResponse, HttpServer, Responder, get, middleware, web};
use log::info;
use std::sync::Arc;

impl APIState {
    pub fn new() -> Self {
        APIState {}
    }
}

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("rsquant RESTful API Server")
}

async fn not_found() -> impl Responder {
    HttpResponse::Ok().body("404 Not Found")
}

pub async fn run(wait_forever: bool) {
    let state = web::Data::new(Arc::new(APIState::new()));
    let server = HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(middleware::Logger::default())
            .wrap(Cors::default().allow_any_origin().allow_any_method().allow_any_header())
            .default_service(web::route().to(not_found))
            .service(index)
            .configure(super::router_api::register)
    })
    .bind((HTTP_BIND.as_str(), *HTTP_PORT))
    .unwrap()
    .run();

    info!("RESTful API server started at http://{}:{}", *HTTP_BIND, *HTTP_PORT);

    if wait_forever {
        info!("Running on API_ONLY mode");
        server.await.unwrap();
    } else {
        tokio::spawn(server);
    }
}
