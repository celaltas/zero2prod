use actix_web::{
    dev::Server, get, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use std::net::TcpListener;

#[derive(serde::Deserialize)]
struct FormData {
    email: String,
    name: String,
}



async fn health_check(_request: HttpRequest) -> impl Responder {
    HttpResponse::Ok().finish()
}
async fn subscribe(_form:web::Form<FormData>) -> impl Responder {
    HttpResponse::Ok().finish()
}

pub fn run(listener: TcpListener) -> Result<Server, std::io::Error> {
    let server = HttpServer::new(|| {
        App::new()
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
    })
    .listen(listener)?
    .run();
    Ok(server)
}
