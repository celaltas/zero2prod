use std::net::TcpListener;

use reqwest::Client;

#[actix_rt::test]
async fn test_health_check() {
    let addrs = spawn_app();
    let client = Client::new();
    let response = client
        .get(&format!("http://{}/health_check", &addrs))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let server = zero2prod::run(listener).expect("Failed to bind address");
    tokio::spawn(server);
    format!("127.0.0.1:{}", port)
}
