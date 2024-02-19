use crate::helpers::spawn_app;
use reqwest::Client;

#[actix_rt::test]
async fn test_health_check() {
    let app = spawn_app().await;
    let client = Client::new();
    let response = client
        .get(&format!("{}/health_check", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
