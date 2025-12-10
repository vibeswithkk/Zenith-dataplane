/// HTTP Client Module for WASM Plugins
/// Provides HTTP request capabilities with sandboxing

use std::sync::atomic::{AtomicU64, Ordering};

static HTTP_CALL_COUNT: AtomicU64 = AtomicU64::new(0);

/// HTTP method
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum HttpMethod {
    Get = 0,
    Post = 1,
    Put = 2,
    Delete = 3,
}

impl From<u32> for HttpMethod {
    fn from(val: u32) -> Self {
        match val {
            1 => HttpMethod::Post,
            2 => HttpMethod::Put,
            3 => HttpMethod::Delete,
            _ => HttpMethod::Get,
        }
    }
}

/// HTTP response
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub body: Vec<u8>,
    pub headers: Vec<(String, String)>,
}

/// HTTP API
pub struct HttpAPI;

impl HttpAPI {
    /// Make an HTTP request (synchronous for MVP)
    pub fn request(
        method: HttpMethod,
        url: &str,
        body: Option<&[u8]>,
        timeout_ms: u64,
    ) -> Result<HttpResponse, String> {
        HTTP_CALL_COUNT.fetch_add(1, Ordering::Relaxed);
        
        // Security: Validate URL (allow-list in production)
        if !Self::is_url_allowed(url) {
            return Err("URL not in allow-list".to_string());
        }
        
        // For MVP, return mock response
        // In production, use reqwest or similar
        tracing::info!("HTTP {:?} request to {}", method, url);
        
        Ok(HttpResponse {
            status_code: 200,
            body: b"{\"success\": true}".to_vec(),
            headers: vec![
                ("content-type".to_string(), "application/json".to_string()),
            ],
        })
    }
    
    /// Simple GET request
    pub fn get(url: &str) -> Result<HttpResponse, String> {
        Self::request(HttpMethod::Get, url, None, 5000)
    }
    
    /// Simple POST request
    pub fn post(url: &str, body: &[u8]) -> Result<HttpResponse, String> {
        Self::request(HttpMethod::Post, url, Some(body), 5000)
    }
    
    /// Check if URL is allowed (security sandbox)
    fn is_url_allowed(url: &str) -> bool {
        // In production, maintain an allow-list
        // For MVP, allow localhost and example domains
        url.starts_with("http://localhost") ||
        url.starts_with("https://api.example.com") ||
        url.starts_with("https://httpbin.org")
    }
    
    /// Get HTTP call count
    pub fn get_call_count() -> u64 {
        HTTP_CALL_COUNT.load(Ordering::Relaxed)
    }
}

// C ABI exports
#[no_mangle]
pub unsafe extern "C" fn zenith_http_get(
    url_ptr: *const u8,
    url_len: usize,
    out_ptr: *mut u8,
    out_len: usize,
) -> i32 {
    if url_ptr.is_null() || out_ptr.is_null() {
        return -1;
    }
    
    let url_slice = std::slice::from_raw_parts(url_ptr, url_len);
    let url = match std::str::from_utf8(url_slice) {
        Ok(s) => s,
        Err(_) => return -2,
    };
    
    match HttpAPI::get(url) {
        Ok(response) => {
            let copy_len = response.body.len().min(out_len);
            let out_slice = std::slice::from_raw_parts_mut(out_ptr, copy_len);
            out_slice.copy_from_slice(&response.body[..copy_len]);
            response.status_code as i32
        }
        Err(_) => -3,
    }
}

#[no_mangle]
pub unsafe extern "C" fn zenith_http_post(
    url_ptr: *const u8,
    url_len: usize,
    body_ptr: *const u8,
    body_len: usize,
    out_ptr: *mut u8,
    out_len: usize,
) -> i32 {
    if url_ptr.is_null() || body_ptr.is_null() || out_ptr.is_null() {
        return -1;
    }
    
    let url_slice = std::slice::from_raw_parts(url_ptr, url_len);
    let url = match std::str::from_utf8(url_slice) {
        Ok(s) => s,
        Err(_) => return -2,
    };
    
    let body = std::slice::from_raw_parts(body_ptr, body_len);
    
    match HttpAPI::post(url, body) {
        Ok(response) => {
            let copy_len = response.body.len().min(out_len);
            let out_slice = std::slice::from_raw_parts_mut(out_ptr, copy_len);
            out_slice.copy_from_slice(&response.body[..copy_len]);
            response.status_code as i32
        }
        Err(_) => -3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_validation() {
        assert!(HttpAPI::is_url_allowed("http://localhost:8080/api"));
        assert!(HttpAPI::is_url_allowed("https://api.example.com/data"));
        assert!(!HttpAPI::is_url_allowed("https://malicious.com"));
    }

    #[test]
    fn test_http_get() {
        let response = HttpAPI::get("http://localhost/test").unwrap();
        assert_eq!(response.status_code, 200);
    }

    #[test]
    fn test_http_post() {
        let response = HttpAPI::post("http://localhost/api", b"{\"test\": 1}").unwrap();
        assert_eq!(response.status_code, 200);
    }

    #[test]
    fn test_blocked_url() {
        let result = HttpAPI::get("https://evil.com");
        assert!(result.is_err());
    }
}
