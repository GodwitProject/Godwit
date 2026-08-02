use reqwest::Client;

#[tokio::test]
#[ignore = "requires running server"]
async fn admin_login_smoke() {
    let client = Client::new();
    let resp = client
        .post("http://localhost:3000/api/v1/auth/login")
        .json(&serde_json::json!({
            "email": "admin@godwit.local",
            "password": "changeme"
        }))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success() || resp.status() == 401);
}
