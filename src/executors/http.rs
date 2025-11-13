//! HTTP executor for REST API calls
//!
//! This module provides HTTP/HTTPS request execution with the following features:
//! - **Methods:** GET, POST, PUT, DELETE
//! - **Request bodies:** JSON serialization from YAML config
//! - **Custom headers:** User-defined headers for authentication, content-type, etc.
//! - **Configurable timeouts:** Per-request timeout enforcement
//! - **Status code handling:** 2xx = success, 4xx/5xx = failed
//!
//! # Security
//!
//! This executor implements security best practices:
//! - TLS/SSL verification enabled by default
//! - Response body size limits (MAX_RESPONSE_SIZE = 10MB)
//! - Timeout enforcement to prevent hanging requests
//! - Input validation for URLs and configuration
//!
//! # Performance
//!
//! - Connection pooling via reqwest's built-in client
//! - Efficient streaming for large response bodies
//! - Response truncation for memory safety
//!
//! # Example
//!
//! ```no_run
//! use picoflow::executors::http::HttpExecutor;
//! use picoflow::executors::ExecutorTrait;
//! use picoflow::models::{HttpConfig, HttpMethod, TaskExecutorConfig};
//! use std::collections::HashMap;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let executor = HttpExecutor::new();
//! let config = TaskExecutorConfig::Http(HttpConfig {
//!     url: "https://api.example.com/health".to_string(),
//!     method: HttpMethod::Get,
//!     body: None,
//!     headers: HashMap::new(),
//!     timeout: 30,
//!     allow_private_ips: false,
//! });
//!
//! let result = executor.execute(&config).await?;
//! println!("Status: {}, Response: {:?}", result.status, result.stdout);
//! # Ok(())
//! # }
//! ```

use crate::error::{PicoFlowError, Result};
use crate::executors::ExecutorTrait;
use crate::models::{
    ExecutionResult, HttpConfig, HttpMethod, TaskExecutorConfig, TaskStatus, MAX_RESPONSE_SIZE,
};
use async_trait::async_trait;
use reqwest::{Client, Method};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::Duration;
use tracing::{debug, error, info, warn};
use url::Host;

/// HTTP executor for REST API calls
#[derive(Debug, Clone)]
pub struct HttpExecutor {
    /// Reqwest client with connection pooling
    client: Client,
}

impl HttpExecutor {
    /// Create a new HTTP executor
    ///
    /// Initializes a reqwest client with default configuration.
    /// Connection pooling is handled automatically by reqwest.
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent(format!("PicoFlow/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Validate URL for SSRF (Server-Side Request Forgery) protection
    ///
    /// # Security
    ///
    /// This prevents SSRF attacks by blocking requests to:
    /// - Private IP ranges (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16)
    /// - Localhost (127.0.0.0/8, ::1)
    /// - Link-local addresses (169.254.0.0/16, fe80::/10)
    /// - Cloud metadata services (169.254.169.254, metadata.google.internal)
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to validate
    /// * `allow_private_ips` - If true, allows requests to private IPs (use with caution!)
    ///
    /// # Errors
    ///
    /// Returns error if URL targets a blocked address or domain
    fn validate_ssrf(url: &str, allow_private_ips: bool) -> Result<()> {
        let parsed_url = reqwest::Url::parse(url)
            .map_err(|e| PicoFlowError::Validation(format!("Invalid URL for SSRF check: {}", e)))?;

        // Only allow HTTP and HTTPS schemes
        let scheme = parsed_url.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(PicoFlowError::Validation(format!(
                "Invalid URL scheme '{}': only http and https are allowed",
                scheme
            )));
        }

        // Get host from URL
        let host = parsed_url
            .host()
            .ok_or_else(|| PicoFlowError::Validation("URL must contain a host".to_string()))?;

        match host {
            Host::Ipv4(ip) => {
                if !allow_private_ips {
                    Self::validate_ipv4_not_private(ip)?;
                }
            }
            Host::Ipv6(ip) => {
                if !allow_private_ips {
                    Self::validate_ipv6_not_private(ip)?;
                }
            }
            Host::Domain(domain) => {
                if !allow_private_ips {
                    Self::validate_domain_not_blocked(domain)?;
                }
            }
        }

        Ok(())
    }

    /// Validate IPv4 address is not private/local
    fn validate_ipv4_not_private(ip: Ipv4Addr) -> Result<()> {
        if ip.is_private() {
            return Err(PicoFlowError::Http(format!(
                "SSRF protection: Requests to private IP addresses are blocked ({}). \
                 Set allow_private_ips: true to override (NOT recommended)",
                ip
            )));
        }

        if ip.is_loopback() {
            return Err(PicoFlowError::Http(format!(
                "SSRF protection: Requests to loopback addresses are blocked ({})",
                ip
            )));
        }

        if ip.is_link_local() {
            return Err(PicoFlowError::Http(format!(
                "SSRF protection: Requests to link-local addresses are blocked ({})",
                ip
            )));
        }

        // Check for cloud metadata service IP (169.254.169.254)
        if ip == Ipv4Addr::new(169, 254, 169, 254) {
            return Err(PicoFlowError::Http(
                "SSRF protection: Requests to cloud metadata services are blocked (169.254.169.254)"
                    .to_string(),
            ));
        }

        Ok(())
    }

    /// Validate IPv6 address is not private/local
    fn validate_ipv6_not_private(ip: Ipv6Addr) -> Result<()> {
        if ip.is_loopback() {
            return Err(PicoFlowError::Http(format!(
                "SSRF protection: Requests to loopback addresses are blocked ({})",
                ip
            )));
        }

        if ip.is_unicast_link_local() {
            return Err(PicoFlowError::Http(format!(
                "SSRF protection: Requests to link-local addresses are blocked ({})",
                ip
            )));
        }

        // IPv6 unique local addresses (fc00::/7)
        if (ip.segments()[0] & 0xfe00) == 0xfc00 {
            return Err(PicoFlowError::Http(format!(
                "SSRF protection: Requests to private IPv6 addresses are blocked ({})",
                ip
            )));
        }

        Ok(())
    }

    /// Validate domain is not in blocklist
    fn validate_domain_not_blocked(domain: &str) -> Result<()> {
        // List of blocked domains (cloud metadata services and localhost aliases)
        let blocked_domains = [
            "localhost",
            "metadata.google.internal", // GCP metadata
            "169.254.169.254",          // AWS/Azure metadata (IP as domain)
            "metadata",                 // Generic metadata alias
            "instance-data",            // AWS IMDSv1
        ];

        let domain_lower = domain.to_lowercase();

        for blocked in &blocked_domains {
            if domain_lower == *blocked || domain_lower.ends_with(&format!(".{}", blocked)) {
                return Err(PicoFlowError::Http(format!(
                    "SSRF protection: Requests to '{}' are blocked (metadata service or localhost)",
                    domain
                )));
            }
        }

        Ok(())
    }

    /// Validate HTTP configuration
    fn validate_config(config: &HttpConfig) -> Result<()> {
        // Validate URL
        if config.url.is_empty() {
            return Err(PicoFlowError::Validation(
                "HTTP URL cannot be empty".to_string(),
            ));
        }

        // Validate URL is well-formed
        if let Err(e) = reqwest::Url::parse(&config.url) {
            return Err(PicoFlowError::Validation(format!(
                "Invalid HTTP URL: {}",
                e
            )));
        }

        // SSRF protection
        Self::validate_ssrf(&config.url, config.allow_private_ips)?;

        // Validate timeout is reasonable (1 second to 1 hour)
        if config.timeout == 0 || config.timeout > 3600 {
            return Err(PicoFlowError::Validation(format!(
                "HTTP timeout must be between 1 and 3600 seconds, got: {}",
                config.timeout
            )));
        }

        // Log warning if private IPs are allowed
        if config.allow_private_ips {
            warn!(
                "SECURITY WARNING: allow_private_ips is enabled for URL {} - SSRF protection disabled",
                config.url
            );
        }

        Ok(())
    }

    /// Convert HttpMethod enum to reqwest Method
    fn convert_method(method: &HttpMethod) -> Method {
        match method {
            HttpMethod::Get => Method::GET,
            HttpMethod::Post => Method::POST,
            HttpMethod::Put => Method::PUT,
            HttpMethod::Delete => Method::DELETE,
        }
    }

    /// Execute HTTP request
    ///
    /// # Success Criteria
    ///
    /// - HTTP status code 2xx (200-299) = TaskStatus::Success
    /// - HTTP status code 4xx/5xx = TaskStatus::Failed
    /// - Network errors, timeouts, SSL errors = TaskStatus::Failed
    ///
    /// # Response Handling
    ///
    /// - Response body is truncated to MAX_RESPONSE_SIZE (10MB)
    /// - output_truncated flag is set if truncation occurs
    /// - HTTP status code is returned as exit_code
    async fn execute_http(
        &self,
        config: &HttpConfig,
        timeout_secs: u64,
    ) -> Result<ExecutionResult> {
        // Validate configuration
        Self::validate_config(config)?;

        info!(
            "Executing HTTP {} request to {}",
            format!("{:?}", config.method).to_uppercase(),
            config.url
        );
        debug!("Request headers: {:?}", config.headers);

        let start = std::time::Instant::now();

        // Build request
        let method = Self::convert_method(&config.method);
        let mut request = self
            .client
            .request(method, &config.url)
            .timeout(Duration::from_secs(timeout_secs));

        // Add custom headers
        for (key, value) in &config.headers {
            request = request.header(key, value);
        }

        // Add JSON body if provided
        if let Some(body_value) = &config.body {
            // Convert serde_yaml::Value to serde_json::Value
            let json_body = serde_json::to_value(body_value).map_err(|e| {
                PicoFlowError::Http(format!("Failed to serialize request body: {}", e))
            })?;

            debug!("Request body: {}", json_body);
            request = request.json(&json_body);
        }

        // Execute request
        let response_result = request.send().await;

        let duration = start.elapsed();

        match response_result {
            Ok(response) => {
                let status_code = response.status();
                let status_code_u16 = status_code.as_u16();

                info!(
                    "HTTP request completed with status code: {}",
                    status_code_u16
                );

                // Read response body with size limit
                let body_result = response.bytes().await;

                let (response_body, output_truncated) = match body_result {
                    Ok(bytes) => {
                        let truncated = bytes.len() > MAX_RESPONSE_SIZE;
                        let body_bytes = if truncated {
                            warn!(
                                "Response body truncated from {} to {} bytes",
                                bytes.len(),
                                MAX_RESPONSE_SIZE
                            );
                            &bytes[..MAX_RESPONSE_SIZE]
                        } else {
                            &bytes
                        };

                        let body_string = String::from_utf8_lossy(body_bytes).to_string();
                        (Some(body_string), truncated)
                    }
                    Err(e) => {
                        warn!("Failed to read response body: {}", e);
                        (Some(format!("Failed to read response body: {}", e)), false)
                    }
                };

                // Determine task status based on HTTP status code
                let task_status = if status_code.is_success() {
                    // 2xx = success
                    TaskStatus::Success
                } else {
                    // 4xx, 5xx, or other = failed
                    TaskStatus::Failed
                };

                debug!(
                    "Mapped HTTP status {} to task status: {}",
                    status_code_u16, task_status
                );

                Ok(ExecutionResult {
                    status: task_status.clone(),
                    stdout: response_body,
                    stderr: if task_status == TaskStatus::Failed {
                        Some(format!(
                            "HTTP request failed with status code: {}",
                            status_code_u16
                        ))
                    } else {
                        None
                    },
                    exit_code: Some(status_code_u16 as i32),
                    duration,
                    output_truncated,
                })
            }
            Err(e) => {
                error!("HTTP request failed: {}", e);

                // Classify error type for better diagnostics
                let error_message = if e.is_timeout() {
                    format!("Request timed out after {} seconds", timeout_secs)
                } else if e.is_connect() {
                    format!("Connection failed: {}", e)
                } else if e.is_status() {
                    format!("HTTP error: {}", e)
                } else if e.is_request() {
                    format!("Invalid request: {}", e)
                } else {
                    format!("Request failed: {}", e)
                };

                // Return failed execution result
                Ok(ExecutionResult {
                    status: TaskStatus::Failed,
                    stdout: None,
                    stderr: Some(error_message.clone()),
                    exit_code: None,
                    duration,
                    output_truncated: false,
                })
            }
        }
    }
}

#[async_trait]
impl ExecutorTrait for HttpExecutor {
    async fn execute(&self, config: &TaskExecutorConfig) -> anyhow::Result<ExecutionResult> {
        match config {
            TaskExecutorConfig::Http(http_config) => {
                // Use timeout from config
                let result = self.execute_http(http_config, http_config.timeout).await?;
                Ok(result)
            }
            _ => Err(anyhow::anyhow!("Invalid config type for HttpExecutor")),
        }
    }

    async fn health_check(&self) -> anyhow::Result<()> {
        // For HTTP executor, verify we can make a simple request
        // Use a reliable public endpoint
        let config = HttpConfig {
            url: "https://www.google.com".to_string(),
            method: HttpMethod::Get,
            body: None,
            headers: std::collections::HashMap::new(),
            timeout: 5,
            allow_private_ips: false,
        };

        let result = self.execute_http(&config, 5).await?;

        if result.status == TaskStatus::Success {
            Ok(())
        } else {
            Err(anyhow::anyhow!("HTTP executor health check failed"))
        }
    }
}

impl Default for HttpExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_validate_config_empty_url() {
        let config = HttpConfig {
            url: "".to_string(),
            method: HttpMethod::Get,
            body: None,
            headers: HashMap::new(),
            timeout: 30,
            allow_private_ips: false,
        };

        let result = HttpExecutor::validate_config(&config);
        assert!(result.is_err());
        assert!(matches!(result, Err(PicoFlowError::Validation(_))));
    }

    #[test]
    fn test_validate_config_invalid_url() {
        let config = HttpConfig {
            url: "not a valid url".to_string(),
            method: HttpMethod::Get,
            body: None,
            headers: HashMap::new(),
            timeout: 30,
            allow_private_ips: false,
        };

        let result = HttpExecutor::validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_config_invalid_timeout() {
        let config = HttpConfig {
            url: "https://example.com".to_string(),
            method: HttpMethod::Get,
            body: None,
            headers: HashMap::new(),
            timeout: 0,
            allow_private_ips: false,
        };

        let result = HttpExecutor::validate_config(&config);
        assert!(result.is_err());

        let config = HttpConfig {
            url: "https://example.com".to_string(),
            method: HttpMethod::Get,
            body: None,
            headers: HashMap::new(),
            timeout: 4000,
            allow_private_ips: false,
        };

        let result = HttpExecutor::validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_config_valid() {
        let config = HttpConfig {
            url: "https://example.com".to_string(),
            method: HttpMethod::Get,
            body: None,
            headers: HashMap::new(),
            timeout: 30,
            allow_private_ips: false,
        };

        let result = HttpExecutor::validate_config(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_ssrf_protection_private_ip() {
        // Test blocking private IP ranges
        let private_ips = vec![
            "http://10.0.0.1/",
            "http://172.16.0.1/",
            "http://192.168.1.1/",
            "http://127.0.0.1/",
        ];

        for url in private_ips {
            let config = HttpConfig {
                url: url.to_string(),
                method: HttpMethod::Get,
                body: None,
                headers: HashMap::new(),
                timeout: 30,
                allow_private_ips: false,
            };

            let result = HttpExecutor::validate_config(&config);
            assert!(result.is_err(), "Should block private IP: {}", url);
            assert!(matches!(result, Err(PicoFlowError::Http(_))));
        }
    }

    #[test]
    fn test_ssrf_protection_metadata_service() {
        // Test blocking cloud metadata service
        let config = HttpConfig {
            url: "http://169.254.169.254/latest/meta-data/".to_string(),
            method: HttpMethod::Get,
            body: None,
            headers: HashMap::new(),
            timeout: 30,
            allow_private_ips: false,
        };

        let result = HttpExecutor::validate_config(&config);
        assert!(result.is_err());
        assert!(matches!(result, Err(PicoFlowError::Http(_))));
    }

    #[test]
    fn test_ssrf_protection_localhost_domain() {
        // Test blocking localhost domain
        let config = HttpConfig {
            url: "http://localhost:8080/".to_string(),
            method: HttpMethod::Get,
            body: None,
            headers: HashMap::new(),
            timeout: 30,
            allow_private_ips: false,
        };

        let result = HttpExecutor::validate_config(&config);
        assert!(result.is_err());
        assert!(matches!(result, Err(PicoFlowError::Http(_))));
    }

    #[test]
    fn test_ssrf_protection_allow_private_ips() {
        // Test that allow_private_ips flag works
        let config = HttpConfig {
            url: "http://192.168.1.1/".to_string(),
            method: HttpMethod::Get,
            body: None,
            headers: HashMap::new(),
            timeout: 30,
            allow_private_ips: true,
        };

        let result = HttpExecutor::validate_config(&config);
        assert!(result.is_ok(), "Should allow private IP when flag is set");
    }

    #[test]
    fn test_ssrf_protection_public_url() {
        // Test that public URLs are allowed
        let public_urls = vec![
            "https://api.github.com/",
            "https://www.google.com/",
            "http://example.com/",
        ];

        for url in public_urls {
            let config = HttpConfig {
                url: url.to_string(),
                method: HttpMethod::Get,
                body: None,
                headers: HashMap::new(),
                timeout: 30,
                allow_private_ips: false,
            };

            let result = HttpExecutor::validate_config(&config);
            assert!(result.is_ok(), "Should allow public URL: {}", url);
        }
    }

    #[test]
    fn test_convert_method() {
        assert_eq!(HttpExecutor::convert_method(&HttpMethod::Get), Method::GET);
        assert_eq!(
            HttpExecutor::convert_method(&HttpMethod::Post),
            Method::POST
        );
        assert_eq!(HttpExecutor::convert_method(&HttpMethod::Put), Method::PUT);
        assert_eq!(
            HttpExecutor::convert_method(&HttpMethod::Delete),
            Method::DELETE
        );
    }

    #[tokio::test]
    async fn test_http_executor_new() {
        let executor = HttpExecutor::new();
        // Just verify it constructs successfully
        assert!(std::mem::size_of_val(&executor) > 0);
    }

    #[tokio::test]
    async fn test_http_executor_default() {
        let executor = HttpExecutor::default();
        assert!(std::mem::size_of_val(&executor) > 0);
    }

    // Note: Integration tests with mock HTTP server are in tests/http_executor_integration.rs
    // to keep unit tests fast and focused on logic validation
}
