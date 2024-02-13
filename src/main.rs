use std::net::TcpListener;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:3456").expect("Failed to bind random port");
    println!("listening at :3456");
    zero2prod::run(listener)?.await
}
