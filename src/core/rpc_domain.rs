//! X1NS Domain Service Integration
//! 
//! This module provides integration with the X1NS domain name service.
//! API Documentation: https://api.x1ns.xyz

use serde::Deserialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

/// X1NS API base URL
const X1NS_API_BASE: &str = "https://api.x1ns.xyz";

/// Response from the X1NS primary domain API
#[derive(Debug, Clone, Deserialize)]
pub struct PrimaryDomainResponse {
    /// The wallet address queried
    #[allow(dead_code)]
    pub address: String,
    /// Whether the wallet has a primary domain set
    #[serde(rename = "hasPrimary")]
    pub has_primary: bool,
    /// The primary domain name (only present if has_primary is true)
    /// Note: API returns "domain" field, not "primaryDomain"
    pub domain: Option<String>,
    /// The domain's address (optional)
    #[serde(rename = "domainAddress")]
    #[allow(dead_code)]
    pub domain_address: Option<String>,
    /// Timestamp (optional)
    #[allow(dead_code)]
    pub timestamp: Option<String>,
}

/// Error type for domain service operations
#[derive(Debug)]
pub enum DomainError {
    /// Network or HTTP error
    NetworkError(String),
    /// Failed to parse response
    ParseError(String),
    /// API returned an error
    ApiError(String),
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomainError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            DomainError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            DomainError::ApiError(msg) => write!(f, "API error: {}", msg),
        }
    }
}

/// Get the primary domain for a wallet address
/// 
/// # Arguments
/// * `address` - The wallet address (X1 public key)
/// 
/// # Returns
/// * `Ok(Some(domain))` - If the wallet has a primary domain set
/// * `Ok(None)` - If the wallet does not have a primary domain
/// * `Err(DomainError)` - If there was an error querying the API
/// 
/// # Example
/// ```
/// let domain = get_primary_domain("DEQWNRhQmNg7T6UQxV8d2oJAanFHBu9YkNyXDb7GvzvA").await;
/// match domain {
///     Ok(Some(name)) => println!("Primary domain: {}", name),
///     Ok(None) => println!("No primary domain set"),
///     Err(e) => println!("Error: {}", e),
/// }
/// ```
pub async fn get_primary_domain(address: &str) -> Result<Option<String>, DomainError> {
    let url = format!("{}/api/primary/{}", X1NS_API_BASE, address);
    
    log::debug!("Querying X1NS primary domain for address: {}", address);
    
    // Create request options
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);
    
    // Create request
    let request = Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| DomainError::NetworkError(format!("Failed to create request: {:?}", e)))?;
    
    // Execute fetch
    let window = web_sys::window()
        .ok_or_else(|| DomainError::NetworkError("No window object available".to_string()))?;
    
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| DomainError::NetworkError(format!("Fetch failed: {:?}", e)))?;
    
    let resp: Response = resp_value.dyn_into()
        .map_err(|e| DomainError::NetworkError(format!("Failed to convert response: {:?}", e)))?;
    
    // Check HTTP status
    if !resp.ok() {
        // 404 might mean no domain found, treat as no primary domain
        if resp.status() == 404 {
            log::debug!("X1NS returned 404 for address {}, treating as no primary domain", address);
            return Ok(None);
        }
        return Err(DomainError::ApiError(format!("HTTP {} {}", resp.status(), resp.status_text())));
    }
    
    // Parse JSON response
    let json = JsFuture::from(
        resp.json().map_err(|e| DomainError::ParseError(format!("Failed to get JSON: {:?}", e)))?
    )
    .await
    .map_err(|e| DomainError::ParseError(format!("Failed to parse JSON: {:?}", e)))?;
    
    // Deserialize response
    let response: PrimaryDomainResponse = serde_wasm_bindgen::from_value(json)
        .map_err(|e| DomainError::ParseError(format!("Failed to deserialize response: {:?}", e)))?;
    
    log::debug!("X1NS response for {}: has_primary={}, domain={:?}", 
        address, response.has_primary, response.domain);
    
    if response.has_primary {
        Ok(response.domain)
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Note: These tests require a browser environment with fetch API
    // They are primarily for documentation purposes
    
    #[test]
    fn test_primary_domain_response_deserialize() {
        let json = r#"{
            "address": "3NvVAGuTQr9DFQhNGjMyLFAAC22L1k2AEL3V1LE25XfP",
            "hasPrimary": true,
            "domain": "xen_artist.x1",
            "domainAddress": "4NwU8rHu9kDVKprNTg9DT7aU858UCGZy8yHAdGE8stYN",
            "timestamp": "1765370933"
        }"#;
        
        let response: PrimaryDomainResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.address, "3NvVAGuTQr9DFQhNGjMyLFAAC22L1k2AEL3V1LE25XfP");
        assert!(response.has_primary);
        assert_eq!(response.domain, Some("xen_artist.x1".to_string()));
    }
    
    #[test]
    fn test_no_primary_domain_response_deserialize() {
        let json = r#"{
            "address": "SomeOtherAddress123",
            "hasPrimary": false,
            "domain": null
        }"#;
        
        let response: PrimaryDomainResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.address, "SomeOtherAddress123");
        assert!(!response.has_primary);
        assert_eq!(response.domain, None);
    }
}

