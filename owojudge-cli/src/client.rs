use crate::config::AppConfig;
use anyhow::{anyhow, Result};
use reqwest::{Client, Method, Response};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct OwoClient {
    client: Client,
    pub config: Arc<Mutex<AppConfig>>,
}

impl OwoClient {
    pub fn new(config: AppConfig) -> Result<Self> {
        let client = Client::builder().build()?;

        Ok(Self {
            client,
            config: Arc::new(Mutex::new(config)),
        })
    }

    fn base_url(&self) -> Result<String> {
        let config = self.config.lock().unwrap();
        let url = config
            .base_url
            .clone()
            .ok_or_else(|| anyhow!("Judge URL not configured. Run `config set-url <URL>` first."))?;

        if !url.starts_with("http://") && !url.starts_with("https://") {
            Ok(format!("http://{}", url))
        } else {
            Ok(url)
        }
    }

    fn update_cookies(&self, response: &Response) -> Result<()> {
        let cookies: Vec<String> = response
            .headers()
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
            .collect();

        if !cookies.is_empty() {
            let mut config = self.config.lock().unwrap();
            for raw_cookie in cookies {
                let parts: Vec<&str> = raw_cookie.split(';').collect();
                if let Some(kv) = parts.first() {
                    let kv_parts: Vec<&str> = kv.splitn(2, '=').collect();
                    if kv_parts.len() == 2 {
                        config
                            .cookies
                            .insert(kv_parts[0].to_string(), kv_parts[1].to_string());
                    }
                }
            }
            config.save()?;
        }
        Ok(())
    }

    pub async fn request<T: Serialize + ?Sized, R: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&T>,
    ) -> Result<R> {
        let url = format!("{}{}", self.base_url()?, path);
        let mut req = self.client.request(method, &url);

        {
            let config = self.config.lock().unwrap();
            let cookie_header = config.get_cookie_header();
            if !cookie_header.is_empty() {
                req = req.header(reqwest::header::COOKIE, cookie_header);
            }
        }

        if let Some(b) = body {
            req = req.json(b);
        }

        let res = req.send().await?;
        self.update_cookies(&res)?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(anyhow!("Request failed: {} - {}", status, text));
        }

        let data = res.json::<R>().await?;
        Ok(data)
    }

    pub async fn get<R: DeserializeOwned>(&self, path: &str) -> Result<R> {
        self.request::<(), R>(Method::GET, path, None).await
    }

    pub async fn post<T: Serialize + ?Sized, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<R> {
        self.request(Method::POST, path, Some(body)).await
    }

    // For login, we don't necessarily expect a JSON response body or we might just want to check status.
    // The API doc says 201 Created for login.
    pub async fn login(&self, username: &str, password: &str) -> Result<()> {
        let url = format!("{}/api/auth", self.base_url()?);
        let body = serde_json::json!({
            "username": username,
            "password": password
        });

        let mut req = self.client.post(&url).json(&body);

        // Include existing cookies just in case (though probably not needed for login)
        {
            let config = self.config.lock().unwrap();
            let cookie_header = config.get_cookie_header();
            if !cookie_header.is_empty() {
                req = req.header(reqwest::header::COOKIE, cookie_header);
            }
        }

        let res = req.send().await?;

        if res.status().is_success() {
            self.update_cookies(&res)?;
            Ok(())
        } else {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            Err(anyhow!("Login failed: {} - {}", status, text))
        }
    }

    pub async fn logout(&self) -> Result<()> {
        let url = format!("{}/api/auth/logout", self.base_url()?);
        let mut req = self.client.post(&url);

        {
            let config = self.config.lock().unwrap();
            let cookie_header = config.get_cookie_header();
            if !cookie_header.is_empty() {
                req = req.header(reqwest::header::COOKIE, cookie_header);
            }
        }

        let res = req.send().await?;

        // Even if it fails, we should probably clear local cookies
        // But let's check status first.
        if res.status().is_success() {
            let mut config = self.config.lock().unwrap();
            config.cookies.clear();
            config.save()?;
            Ok(())
        } else {
            Err(anyhow!("Logout failed: {}", res.status()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_url_scheme() {
        let config = AppConfig {
            base_url: Some("localhost:8080".to_string()),
            cookies: Default::default(),
        };
        let client = OwoClient::new(config).unwrap();
        assert_eq!(client.base_url().unwrap(), "http://localhost:8080");

        let config_https = AppConfig {
            base_url: Some("https://example.com".to_string()),
            cookies: Default::default(),
        };
        let client_https = OwoClient::new(config_https).unwrap();
        assert_eq!(client_https.base_url().unwrap(), "https://example.com");

        let config_http = AppConfig {
            base_url: Some("http://example.com".to_string()),
            cookies: Default::default(),
        };
        let client_http = OwoClient::new(config_http).unwrap();
        assert_eq!(client_http.base_url().unwrap(), "http://example.com");
    }
}
