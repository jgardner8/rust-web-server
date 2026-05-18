use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

use reqwest::blocking::Client;

fn find_free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    listener.local_addr().unwrap().port()
}

fn wait_for_server_ready(port: u16) { 
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{}", port);
    for _ in 0..50 {
        if client.get(&base_url).send().is_ok() {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn start_server(port: u16) -> Child {
    let cargo_bin = env!("CARGO_BIN_EXE_http_server");
    let child = Command::new(cargo_bin)
        .env("PORT", port.to_string())
        .spawn()
        .expect("Failed to start server");

    wait_for_server_ready(port);

    child
}

fn spawn_server() -> (Client, String, Child) {
    let port = find_free_port();
    let base_url = format!("http://127.0.0.1:{}", port);
    let child = start_server(port);
    let client = Client::new();
    (client, base_url, child)
}

#[test]
fn test_get_root_serves_index() {
    let (client, base_url, mut server) = spawn_server();
    let response = client.get(&base_url).send().expect("Request failed");

    assert_eq!(response.status().as_u16(), 200);
    let body = response.text().expect("Failed to read body");
    assert!(body.contains("<title>Rust Web Server - Index</title>"));
    assert!(body.contains("This page is displayed at / and /index.html"));

    server.kill().ok();
}

#[test]
fn test_get_index_html() {
    let (client, base_url, mut server) = spawn_server();
    let response = client
        .get(format!("{}/index.html", base_url))
        .send()
        .expect("Request failed");

    assert_eq!(response.status().as_u16(), 200);
    let body = response.text().expect("Failed to read body");
    assert!(body.contains("<title>Rust Web Server - Index</title>"));

    server.kill().ok();
}

#[test]
fn test_get_greeting_form() {
    let (client, base_url, mut server) = spawn_server();
    let response = client
        .get(format!("{}/greeting_form", base_url))
        .send()
        .expect("Request failed");

    assert_eq!(response.status().as_u16(), 200);
    let body = response.text().expect("Failed to read body");
    assert!(body.contains("What greeting do you want to say?"));

    server.kill().ok();
}

#[test]
fn test_post_greeting_form_with_valid_data() {
    let (client, base_url, mut server) = spawn_server();
    let response = client
        .post(format!("{}/greeting_form", base_url))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body("say=Hello&to=World&times=1")
        .send()
        .expect("Request failed");

    assert_eq!(response.status().as_u16(), 200);
    let body = response.text().expect("Failed to read body");
    assert!(body.contains("I will say <strong>Hello</strong> to <strong>World</strong>"));

    server.kill().ok();
}

#[test]
fn test_post_greeting_form_with_too_many_times_returns_bad_request() {
    let (client, base_url, mut server) = spawn_server();
    let response = client
        .post(format!("{}/greeting_form", base_url))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body("say=Hello&to=World&times=5")
        .send()
        .expect("Request failed");

    assert_eq!(response.status().as_u16(), 400);

    server.kill().ok();
}

#[test]
fn test_get_greeting_query_params_with_valid_data() {
    let (client, base_url, mut server) = spawn_server();
    let response = client
        .get(format!(
            "{}/greeting_query_params?say=Hi&to=Mom&times=2",
            base_url
        ))
        .send()
        .expect("Request failed");

    assert_eq!(response.status().as_u16(), 200);
    let body = response.text().expect("Failed to read body");
    assert!(body.contains("I will say <strong>Hi</strong> to <strong>Mom</strong>"));

    server.kill().ok();
}

#[test]
fn test_get_greeting_query_params_with_too_many_times_returns_bad_request() {
    let (client, base_url, mut server) = spawn_server();
    let response = client
        .get(format!(
            "{}/greeting_query_params?say=Hi&to=Mom&times=10",
            base_url
        ))
        .send()
        .expect("Request failed");

    assert_eq!(response.status().as_u16(), 400);

    server.kill().ok();
}

#[test]
fn test_get_query_params_with_no_params() {
    let (client, base_url, mut server) = spawn_server();
    let response = client
        .get(format!("{}/query_params", base_url))
        .send()
        .expect("Request failed");

    assert_eq!(response.status().as_u16(), 200);
    let body = response.text().expect("Failed to read body");
    assert!(body.contains("Dynamic page - call me with some query parameters!"));

    server.kill().ok();
}

#[test]
fn test_get_query_params_with_params() {
    let (client, base_url, mut server) = spawn_server();
    let response = client
        .get(format!("{}/query_params?foo=bar&baz=qux", base_url))
        .send()
        .expect("Request failed");

    assert_eq!(response.status().as_u16(), 200);
    let body = response.text().expect("Failed to read body");
    assert!(body.contains("Called /query_params with query parameters"));
    assert!(body.contains("foo"));
    assert!(body.contains("bar"));

    server.kill().ok();
}

#[test]
fn test_get_user_me_with_valid_cookie() {
    let (client, base_url, mut server) = spawn_server();
    let response = client
        .get(format!("{}/user/me", base_url))
        .header("user-cookie", "test")
        .send()
        .expect("Request failed");

    assert_eq!(response.status().as_u16(), 200);
    let body = response.text().expect("Failed to read body");
    assert!(body.contains("Welcome user!"));

    server.kill().ok();
}

#[test]
fn test_get_user_me_with_invalid_cookie_returns_forbidden() {
    let (client, base_url, mut server) = spawn_server();
    let response = client
        .get(format!("{}/user/me", base_url))
        .header("user-cookie", "wrong")
        .send()
        .expect("Request failed");

    assert_eq!(response.status().as_u16(), 403);

    server.kill().ok();
}

#[test]
fn test_get_user_me_with_no_cookie_returns_unauthorized() {
    let (client, base_url, mut server) = spawn_server();
    let response = client
        .get(format!("{}/user/me", base_url))
        .send()
        .expect("Request failed");

    assert_eq!(response.status().as_u16(), 401);

    server.kill().ok();
}

#[test]
fn test_get_user_by_id() {
    let (client, base_url, mut server) = spawn_server();
    let response = client
        .get(format!("{}/user/42", base_url))
        .send()
        .expect("Request failed");

    assert_eq!(response.status().as_u16(), 200);
    let body = response.text().expect("Failed to read body");
    assert!(body.contains("User 42"));

    server.kill().ok();
}

#[test]
fn test_post_user_with_json() {
    let (client, base_url, mut server) = spawn_server();
    let response = client
        .post(format!("{}/user", base_url))
        .header("Content-Type", "application/json")
        .body(r#"{"id": 1, "name": "Alice", "preferences": {"dark_mode": true, "trash": [1, "two", null]}}"#)
        .send()
        .expect("Request failed");

    assert_eq!(response.status().as_u16(), 201);
    let body = response.text().expect("Failed to read body");
    assert!(body.contains("User model:"));
    assert!(body.contains("id = 1"));
    assert!(body.contains("name = Alice"));
    assert!(body.contains("dark_mode = true"));

    server.kill().ok();
}

#[test]
fn test_post_user_with_invalid_json_returns_bad_request() {
    let (client, base_url, mut server) = spawn_server();
    let response = client
        .post(format!("{}/user", base_url))
        .header("Content-Type", "application/json")
        .body("not valid json")
        .send()
        .expect("Request failed");

    assert_eq!(response.status().as_u16(), 400);

    server.kill().ok();
}

#[test]
fn test_post_user_with_json_that_does_not_fit_model_returns_bad_request() {
    let (client, base_url, mut server) = spawn_server();
    let response = client
        .post(format!("{}/user", base_url))
        .header("Content-Type", "application/json")
        .body(r#"{"id": "1", "name": "Alice", "preferences": {"dark_mode": true, "trash": [1, "two", null]}}"#)
        .send()
        .expect("Request failed");

    assert_eq!(response.status().as_u16(), 400);

    server.kill().ok();
}
#[test]
fn test_unknown_route_returns_not_found() {
    let (client, base_url, mut server) = spawn_server();
    let response = client
        .get(format!("{}/nonexistent", base_url))
        .send()
        .expect("Request failed");

    assert_eq!(response.status().as_u16(), 404);
    let body = response.text().expect("Failed to read body");
    assert!(body.contains("404") || body.contains("Not Found"));

    server.kill().ok();
}
