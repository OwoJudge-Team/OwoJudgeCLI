use crate::config::AppConfig;
use anyhow::{anyhow, Result};
use reqwest::{Client, Method, Response};
use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Clone)]
pub struct OwoClient {
    client: Client,
    config: AppConfig,
}

impl OwoClient {
    pub fn new(config: AppConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    fn base_url(&self) -> Result<String> {
        let url = self.config
            .base_url
            .clone()
            .ok_or_else(|| anyhow!("Judge URL not configured. Run `config set-url <URL>` first."))?;

        if !url.starts_with("http://") && !url.starts_with("https://") {
            Ok(format!("http://{}", url))
        } else {
            Ok(url)
        }
    }

    fn update_cookies(&mut self, response: &Response) -> Result<()> {
        let cookies: Vec<String> = response
            .headers()
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .map(str::to_owned)
            .collect();

        if !cookies.is_empty() {
            for raw_cookie in cookies {
                let parts: Vec<&str> = raw_cookie.split(';').collect();
                if let Some(kv) = parts.first() {
                    let kv_parts: Vec<&str> = kv.splitn(2, '=').collect();
                    if kv_parts.len() == 2 {
                        self.config
                            .cookies
                            .insert(kv_parts[0].trim().to_string(), kv_parts[1].trim().to_string());
                    }
                }
            }
            self.config.save()?;
        }
        Ok(())
    }

    pub async fn request<T: Serialize + ?Sized, R: DeserializeOwned>(
        &mut self,
        method: Method,
        path: &str,
        body: Option<&T>,
    ) -> Result<R> {
        let res = self.request_raw(method, path, body).await?;
        let data = res.json::<R>().await?;
        Ok(data)
    }

    pub async fn request_raw<T: Serialize + ?Sized>(
        &mut self,
        method: Method,
        path: &str,
        body: Option<&T>,
    ) -> Result<Response> {
        let url = format!("{}{}", self.base_url()?, path);
        let mut req = self.client.request(method, &url);

        let cookie_header = self.config.get_cookie_header();
        if !cookie_header.is_empty() {
            req = req.header(reqwest::header::COOKIE, cookie_header);
        }

        if let Some(b) = body {
            req = req.json(b);
        }

        let res = req.send().await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(anyhow!("Request failed: {} - {}", status, text));
        }

        self.update_cookies(&res)?;
        Ok(res)
    }

    pub async fn get<R: DeserializeOwned>(&mut self, path: &str) -> Result<R> {
        self.request::<(), R>(Method::GET, path, None).await
    }

    pub fn resolve_url(&self, url: &str) -> Result<String> {
        if url.starts_with("http://") || url.starts_with("https://") {
            Ok(url.to_string())
        } else {
            let base = self.base_url()?;
            let path = if url.starts_with('/') { url } else { &format!("/{}", url) };
            Ok(format!("{}{}", base.trim_end_matches('/'), path))
        }
    }

    pub async fn download_url(&self, url: &str) -> Result<Vec<u8>> {
        let mut req = self.client.get(url);
        // Send OwoJudge cookies only for same-origin requests
        if let Ok(base) = self.base_url() {
            if url.starts_with(&base) {
                let cookie_header = self.config.get_cookie_header();
                if !cookie_header.is_empty() {
                    req = req.header(reqwest::header::COOKIE, cookie_header);
                }
            }
        }
        let res = req.send().await?;
        if !res.status().is_success() {
            return Err(anyhow!("Download failed: {}", res.status()));
        }
        Ok(res.bytes().await?.to_vec())
    }

    pub async fn post<T: Serialize + ?Sized, R: DeserializeOwned>(
        &mut self,
        path: &str,
        body: &T,
    ) -> Result<R> {
        self.request(Method::POST, path, Some(body)).await
    }


    pub async fn login(&mut self, username: &str, password: &str) -> Result<()> {
        let body = serde_json::json!({
            "username": username,
            "password": password
        });

        self.request_raw(Method::POST, "/api/auth", Some(&body)).await?;
        Ok(())
    }

    pub async fn logout(&mut self) -> Result<()> {
        self.request_raw(Method::POST, "/api/auth/logout", None::<&()>).await?;
        self.config.cookies.clear();
        self.config.save()?;
        Ok(())
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
        let client = OwoClient::new(config);
        assert_eq!(client.base_url().unwrap(), "http://localhost:8080");

        let config_https = AppConfig {
            base_url: Some("https://example.com".to_string()),
            cookies: Default::default(),
        };
        let client_https = OwoClient::new(config_https);
        assert_eq!(client_https.base_url().unwrap(), "https://example.com");

        let config_http = AppConfig {
            base_url: Some("http://example.com".to_string()),
            cookies: Default::default(),
        };
        let client_http = OwoClient::new(config_http);
        assert_eq!(client_http.base_url().unwrap(), "http://example.com");
    }
}
