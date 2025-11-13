//! Integration tests for HTTP executor with mock HTTP server

use picoflow::executors::http::HttpExecutor;
use picoflow::executors::ExecutorTrait;
use picoflow::models::{HttpConfig, HttpMethod, TaskExecutorConfig, TaskStatus};
use std::collections::HashMap;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_http_get_success() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Setup mock response
    Mock::given(method("GET"))
        .and(path("/api/health"))
        .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
        .mount(&mock_server)
        .await;

    // Create executor and config
    let executor = HttpExecutor::new();
    let config = TaskExecutorConfig::Http(HttpConfig {
        url: format!("{}/api/health", mock_server.uri()),
        method: HttpMethod::Get,
        body: None,
        headers: HashMap::new(),
        timeout: 5,
        allow_private_ips: true, // Allow localhost for testing
    });

    // Execute request
    let result = executor.execute(&config).await.unwrap();

    // Verify result
    assert_eq!(result.status, TaskStatus::Success);
    assert_eq!(result.exit_code, Some(200));
    assert_eq!(result.stdout, Some("OK".to_string()));
    assert_eq!(result.stderr, None);
    assert!(!result.output_truncated);
}

#[tokio::test]
async fn test_http_post_with_json_body() {
    let mock_server = MockServer::start().await;

    // Setup mock to expect JSON body
    Mock::given(method("POST"))
        .and(path("/api/users"))
        .and(body_json(serde_json::json!({
            "name": "test_user",
            "email": "test@example.com"
        })))
        .respond_with(ResponseTemplate::new(201).set_body_string(r#"{"id": 123}"#))
        .mount(&mock_server)
        .await;

    let executor = HttpExecutor::new();

    // Create YAML body value
    let body_yaml: serde_yaml::Value = serde_yaml::from_str(
        r#"
name: test_user
email: test@example.com
"#,
    )
    .unwrap();

    let config = TaskExecutorConfig::Http(HttpConfig {
        url: format!("{}/api/users", mock_server.uri()),
        method: HttpMethod::Post,
        body: Some(body_yaml),
        headers: HashMap::new(),
        timeout: 5,
        allow_private_ips: true, // Allow localhost for testing
    });

    let result = executor.execute(&config).await.unwrap();

    assert_eq!(result.status, TaskStatus::Success);
    assert_eq!(result.exit_code, Some(201));
    assert!(result.stdout.unwrap().contains("123"));
}

#[tokio::test]
async fn test_http_put_request() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/api/users/123"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Updated"))
        .mount(&mock_server)
        .await;

    let executor = HttpExecutor::new();

    let body_yaml: serde_yaml::Value = serde_yaml::from_str(
        r#"
name: updated_name
"#,
    )
    .unwrap();

    let config = TaskExecutorConfig::Http(HttpConfig {
        url: format!("{}/api/users/123", mock_server.uri()),
        method: HttpMethod::Put,
        body: Some(body_yaml),
        headers: HashMap::new(),
        timeout: 5,
        allow_private_ips: true, // Allow localhost for testing
    });

    let result = executor.execute(&config).await.unwrap();

    assert_eq!(result.status, TaskStatus::Success);
    assert_eq!(result.exit_code, Some(200));
}

#[tokio::test]
async fn test_http_delete_request() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/api/users/123"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let executor = HttpExecutor::new();
    let config = TaskExecutorConfig::Http(HttpConfig {
        url: format!("{}/api/users/123", mock_server.uri()),
        method: HttpMethod::Delete,
        body: None,
        headers: HashMap::new(),
        timeout: 5,
        allow_private_ips: true, // Allow localhost for testing
    });

    let result = executor.execute(&config).await.unwrap();

    assert_eq!(result.status, TaskStatus::Success);
    assert_eq!(result.exit_code, Some(204));
}

#[tokio::test]
async fn test_http_custom_headers() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/protected"))
        .and(header("Authorization", "Bearer test_token"))
        .and(header("X-Custom-Header", "custom_value"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Authorized"))
        .mount(&mock_server)
        .await;

    let executor = HttpExecutor::new();
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer test_token".to_string());
    headers.insert("X-Custom-Header".to_string(), "custom_value".to_string());

    let config = TaskExecutorConfig::Http(HttpConfig {
        url: format!("{}/api/protected", mock_server.uri()),
        method: HttpMethod::Get,
        body: None,
        headers,
        timeout: 5,
        allow_private_ips: true, // Allow localhost for testing
    });

    let result = executor.execute(&config).await.unwrap();

    assert_eq!(result.status, TaskStatus::Success);
    assert_eq!(result.exit_code, Some(200));
    assert_eq!(result.stdout, Some("Authorized".to_string()));
}

#[tokio::test]
async fn test_http_4xx_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/not-found"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .mount(&mock_server)
        .await;

    let executor = HttpExecutor::new();
    let config = TaskExecutorConfig::Http(HttpConfig {
        url: format!("{}/api/not-found", mock_server.uri()),
        method: HttpMethod::Get,
        body: None,
        headers: HashMap::new(),
        timeout: 5,
        allow_private_ips: true, // Allow localhost for testing
    });

    let result = executor.execute(&config).await.unwrap();

    assert_eq!(result.status, TaskStatus::Failed);
    assert_eq!(result.exit_code, Some(404));
    assert!(result.stderr.unwrap().contains("404"));
}

#[tokio::test]
async fn test_http_5xx_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/error"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let executor = HttpExecutor::new();
    let config = TaskExecutorConfig::Http(HttpConfig {
        url: format!("{}/api/error", mock_server.uri()),
        method: HttpMethod::Get,
        body: None,
        headers: HashMap::new(),
        timeout: 5,
        allow_private_ips: true, // Allow localhost for testing
    });

    let result = executor.execute(&config).await.unwrap();

    assert_eq!(result.status, TaskStatus::Failed);
    assert_eq!(result.exit_code, Some(500));
    assert!(result.stderr.unwrap().contains("500"));
}

#[tokio::test]
async fn test_http_timeout() {
    let mock_server = MockServer::start().await;

    // Setup mock to delay response longer than timeout
    Mock::given(method("GET"))
        .and(path("/api/slow"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(10)))
        .mount(&mock_server)
        .await;

    let executor = HttpExecutor::new();
    let config = TaskExecutorConfig::Http(HttpConfig {
        url: format!("{}/api/slow", mock_server.uri()),
        method: HttpMethod::Get,
        body: None,
        headers: HashMap::new(),
        timeout: 1, // 1 second timeout
        allow_private_ips: true, // Allow localhost for testing
    });

    let result = executor.execute(&config).await.unwrap();

    // Timeout should result in failed status
    assert_eq!(result.status, TaskStatus::Failed);
    assert!(result.stderr.unwrap().contains("timed out"));
}

#[tokio::test]
async fn test_http_large_response_truncation() {
    let mock_server = MockServer::start().await;

    // Create response larger than MAX_RESPONSE_SIZE (10MB)
    // Use 11MB to ensure truncation
    let large_body = "x".repeat(11 * 1024 * 1024);

    Mock::given(method("GET"))
        .and(path("/api/large"))
        .respond_with(ResponseTemplate::new(200).set_body_string(large_body))
        .mount(&mock_server)
        .await;

    let executor = HttpExecutor::new();
    let config = TaskExecutorConfig::Http(HttpConfig {
        url: format!("{}/api/large", mock_server.uri()),
        method: HttpMethod::Get,
        body: None,
        headers: HashMap::new(),
        timeout: 30,
        allow_private_ips: true, // Allow localhost for testing
    });

    let result = executor.execute(&config).await.unwrap();

    assert_eq!(result.status, TaskStatus::Success);
    assert_eq!(result.exit_code, Some(200));
    assert!(result.output_truncated); // Should be truncated
    assert_eq!(
        result.stdout.unwrap().len(),
        10 * 1024 * 1024 // MAX_RESPONSE_SIZE
    );
}

#[tokio::test]
async fn test_http_connection_error() {
    let executor = HttpExecutor::new();

    // Use invalid host that will fail to connect
    let config = TaskExecutorConfig::Http(HttpConfig {
        url: "http://invalid-host-that-does-not-exist-12345.com".to_string(),
        method: HttpMethod::Get,
        body: None,
        headers: HashMap::new(),
        timeout: 5,
        allow_private_ips: true, // Allow localhost for testing
    });

    let result = executor.execute(&config).await.unwrap();

    // Connection failure should result in failed status
    assert_eq!(result.status, TaskStatus::Failed);
    assert!(result.stderr.is_some());
}

#[tokio::test]
async fn test_invalid_config_type() {
    use picoflow::models::ShellConfig;

    let executor = HttpExecutor::new();

    // Create wrong config type
    let config = TaskExecutorConfig::Shell(ShellConfig {
        command: "/bin/echo".to_string(),
        args: vec![],
        workdir: None,
        env: None,
    });

    let result = executor.execute(&config).await;

    // Should return error for wrong config type
    assert!(result.is_err());
}
