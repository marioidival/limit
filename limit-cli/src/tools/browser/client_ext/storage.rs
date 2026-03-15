//! Storage, cookies, network, and device operations

use crate::tools::browser::executor::BrowserError;
use crate::tools::browser::types::{Cookie, Request};
use serde_json::Value as JsonValue;

/// Storage, cookies, network, and device operations for browser client
pub trait StorageExt {
    /// Get all cookies
    fn cookies(&self) -> impl std::future::Future<Output = Result<Vec<Cookie>, BrowserError>> + Send;

    /// Set a cookie
    fn cookies_set(
        &self,
        name: &str,
        value: &str,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Get storage value (local or session)
    fn storage_get(
        &self,
        storage_type: &str,
        key: Option<&str>,
    ) -> impl std::future::Future<Output = Result<JsonValue, BrowserError>> + Send;

    /// Set storage value (local or session)
    fn storage_set(
        &self,
        storage_type: &str,
        key: &str,
        value: &str,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Get network requests
    fn network_requests(
        &self,
        filter: Option<&str>,
    ) -> impl std::future::Future<Output = Result<Vec<Request>, BrowserError>> + Send;

    /// Set viewport size
    fn set_viewport(
        &self,
        width: u32,
        height: u32,
        scale: Option<f32>,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Set device emulation
    fn set_device(
        &self,
        name: &str,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;

    /// Set geolocation
    fn set_geo(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> impl std::future::Future<Output = Result<(), BrowserError>> + Send;
}

impl StorageExt for super::super::BrowserClient {
    async fn cookies(&self) -> Result<Vec<Cookie>, BrowserError> {
        let output = self.executor().execute(&["cookies"]).await?;

        if output.success {
            let cookies = output
                .stdout
                .lines()
                .filter(|line| !line.is_empty())
                .filter_map(|line| {
                    let parts: Vec<&str> = line.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        Some(Cookie {
                            name: parts[0].to_string(),
                            value: parts[1].to_string(),
                        })
                    } else {
                        None
                    }
                })
                .collect();
            Ok(cookies)
        } else {
            Err(BrowserError::Other(format!(
                "Failed to get cookies: {}",
                output.stderr
            )))
        }
    }

    async fn cookies_set(&self, name: &str, value: &str) -> Result<(), BrowserError> {
        if name.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Cookie name cannot be empty".to_string(),
            ));
        }

        let output = self
            .executor()
            .execute(&["cookies", "set", name, value])
            .await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to set cookie: {}",
                output.stderr
            )))
        }
    }

    async fn storage_get(
        &self,
        storage_type: &str,
        key: Option<&str>,
    ) -> Result<JsonValue, BrowserError> {
        let valid_types = ["local", "session"];
        if !valid_types.contains(&storage_type) {
            return Err(BrowserError::InvalidArguments(format!(
                "Invalid storage type '{}'. Valid types: {}",
                storage_type,
                valid_types.join(", ")
            )));
        }

        let output = if let Some(k) = key {
            self.executor()
                .execute(&["storage", storage_type, "get", k])
                .await?
        } else {
            self.executor()
                .execute(&["storage", storage_type, "get"])
                .await?
        };

        if output.success {
            let trimmed = output.stdout.trim();
            if trimmed.is_empty() {
                Ok(JsonValue::Null)
            } else {
                Ok(serde_json::from_str(trimmed)
                    .unwrap_or_else(|_| JsonValue::String(trimmed.to_string())))
            }
        } else {
            Err(BrowserError::Other(format!(
                "Failed to get storage: {}",
                output.stderr
            )))
        }
    }

    async fn storage_set(
        &self,
        storage_type: &str,
        key: &str,
        value: &str,
    ) -> Result<(), BrowserError> {
        let valid_types = ["local", "session"];
        if !valid_types.contains(&storage_type) {
            return Err(BrowserError::InvalidArguments(format!(
                "Invalid storage type '{}'. Valid types: {}",
                storage_type,
                valid_types.join(", ")
            )));
        }

        if key.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Key cannot be empty".to_string(),
            ));
        }

        let output = self
            .executor()
            .execute(&["storage", storage_type, "set", key, value])
            .await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to set storage: {}",
                output.stderr
            )))
        }
    }

    async fn network_requests(&self, filter: Option<&str>) -> Result<Vec<Request>, BrowserError> {
        let output = if let Some(f) = filter {
            self.executor().execute(&["network", "requests", f]).await?
        } else {
            self.executor().execute(&["network", "requests"]).await?
        };

        if output.success {
            let requests = output
                .stdout
                .lines()
                .filter(|line| !line.is_empty())
                .filter_map(|line| {
                    let parts: Vec<&str> = line.splitn(2, ' ').collect();
                    if parts.len() == 2 {
                        Some(Request {
                            method: parts[0].to_string(),
                            url: parts[1].to_string(),
                        })
                    } else {
                        None
                    }
                })
                .collect();
            Ok(requests)
        } else {
            Err(BrowserError::Other(format!(
                "Failed to get network requests: {}",
                output.stderr
            )))
        }
    }

    async fn set_viewport(
        &self,
        width: u32,
        height: u32,
        scale: Option<f32>,
    ) -> Result<(), BrowserError> {
        let w = width.to_string();
        let h = height.to_string();

        let output = if let Some(s) = scale {
            let s_str = s.to_string();
            self.executor()
                .execute(&["set", "viewport", &w, &h, &s_str])
                .await?
        } else {
            self.executor().execute(&["set", "viewport", &w, &h]).await?
        };

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to set viewport: {}",
                output.stderr
            )))
        }
    }

    async fn set_device(&self, name: &str) -> Result<(), BrowserError> {
        if name.is_empty() {
            return Err(BrowserError::InvalidArguments(
                "Device name cannot be empty".to_string(),
            ));
        }

        let output = self.executor().execute(&["set", "device", name]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to set device: {}",
                output.stderr
            )))
        }
    }

    async fn set_geo(&self, latitude: f64, longitude: f64) -> Result<(), BrowserError> {
        let lat = latitude.to_string();
        let lng = longitude.to_string();

        let output = self.executor().execute(&["set", "geo", &lat, &lng]).await?;

        if output.success {
            Ok(())
        } else {
            Err(BrowserError::Other(format!(
                "Failed to set geolocation: {}",
                output.stderr
            )))
        }
    }
}
