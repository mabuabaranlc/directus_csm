use gloo_net::http::{Request, RequestBuilder};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

/// HTTP API client for communicating with the Nexus backend
#[derive(Clone, Debug)]
pub struct ApiClient {
    base_url: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ApiListResponse<T> {
    pub data: Vec<T>,
    #[serde(default)]
    pub meta: Option<Value>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ApiError {
    pub errors: Vec<ApiErrorDetail>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ApiErrorDetail {
    pub message: String,
    #[serde(default)]
    pub extensions: Option<Value>,
}

impl ApiClient {
    pub fn new() -> Self {
        Self::from_current_origin()
    }

    pub fn with_base_url(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub fn from_current_origin() -> Self {
        let base = web_sys::window()
            .and_then(|w| w.location().origin().ok())
            .unwrap_or_default();
        Self::with_base_url(&base)
    }

    fn get_token() -> Option<String> {
        gloo_storage::LocalStorage::get::<String>("nexus_access_token").ok()
    }

    fn url(&self, path: &str) -> String {
        format!("{}/api{}", self.base_url, path)
    }

    fn build_request(&self, method: &str, path: &str) -> RequestBuilder {
        let url = self.url(path);
        let req = match method {
            "GET" => Request::get(&url),
            "POST" => Request::post(&url),
            "PATCH" => Request::patch(&url),
            "DELETE" => Request::delete(&url),
            "PUT" => Request::put(&url),
            _ => Request::get(&url),
        };
        if let Some(token) = Self::get_token() {
            req.header("Authorization", &format!("Bearer {}", token))
        } else {
            req
        }
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let resp = self
            .build_request("GET", path)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if resp.ok() {
            resp.json::<T>().await.map_err(|e| e.to_string())
        } else {
            let err = resp.json::<ApiError>().await.ok();
            Err(err
                .and_then(|e| e.errors.first().map(|e| e.message.clone()))
                .unwrap_or_else(|| format!("Request failed: {}", resp.status())))
        }
    }

    pub async fn post<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        let resp = self
            .build_request("POST", path)
            .header("Content-Type", "application/json")
            .json(body)
            .map_err(|e| e.to_string())?
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if resp.ok() {
            resp.json::<T>().await.map_err(|e| e.to_string())
        } else {
            let err = resp.json::<ApiError>().await.ok();
            Err(err
                .and_then(|e| e.errors.first().map(|e| e.message.clone()))
                .unwrap_or_else(|| format!("Request failed: {}", resp.status())))
        }
    }

    pub async fn patch<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        let resp = self
            .build_request("PATCH", path)
            .header("Content-Type", "application/json")
            .json(body)
            .map_err(|e| e.to_string())?
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if resp.ok() {
            resp.json::<T>().await.map_err(|e| e.to_string())
        } else {
            let err = resp.json::<ApiError>().await.ok();
            Err(err
                .and_then(|e| e.errors.first().map(|e| e.message.clone()))
                .unwrap_or_else(|| format!("Request failed: {}", resp.status())))
        }
    }

    pub async fn delete(&self, path: &str) -> Result<(), String> {
        let resp = self
            .build_request("DELETE", path)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if resp.ok() || resp.status() == 204 {
            Ok(())
        } else {
            let err = resp.json::<ApiError>().await.ok();
            Err(err
                .and_then(|e| e.errors.first().map(|e| e.message.clone()))
                .unwrap_or_else(|| format!("Request failed: {}", resp.status())))
        }
    }

    pub async fn login(
        &self,
        email: &str,
        password: &str,
    ) -> Result<AuthTokens, String> {
        #[derive(Serialize)]
        struct LoginBody<'a> {
            email: &'a str,
            password: &'a str,
            mode: &'a str,
        }

        let resp: ApiResponse<AuthTokens> = self
            .post(
                "/auth/login",
                &LoginBody {
                    email,
                    password,
                    mode: "json",
                },
            )
            .await?;

        // Store token
        let _ = gloo_storage::LocalStorage::set("nexus_access_token", &resp.data.access_token);
        let _ = gloo_storage::LocalStorage::set("nexus_refresh_token", &resp.data.refresh_token);

        Ok(resp.data)
    }

    pub async fn refresh_token(&self) -> Result<AuthTokens, String> {
        let refresh = gloo_storage::LocalStorage::get::<String>("nexus_refresh_token")
            .map_err(|_| "No refresh token".to_string())?;

        #[derive(Serialize)]
        struct RefreshBody {
            refresh_token: String,
            mode: String,
        }

        let resp: ApiResponse<AuthTokens> = self
            .post(
                "/auth/refresh",
                &RefreshBody {
                    refresh_token: refresh,
                    mode: "json".to_string(),
                },
            )
            .await?;

        let _ = gloo_storage::LocalStorage::set("nexus_access_token", &resp.data.access_token);
        let _ = gloo_storage::LocalStorage::set("nexus_refresh_token", &resp.data.refresh_token);

        Ok(resp.data)
    }

    pub async fn logout(&self) -> Result<(), String> {
        let refresh = gloo_storage::LocalStorage::get::<String>("nexus_refresh_token").ok();

        #[derive(Serialize)]
        struct LogoutBody {
            refresh_token: Option<String>,
        }

        let _ = self
            .build_request("POST", "/auth/logout")
            .header("Content-Type", "application/json")
            .json(&LogoutBody {
                refresh_token: refresh,
            })
            .map_err(|e| e.to_string())?
            .send()
            .await;

        gloo_storage::LocalStorage::delete("nexus_access_token");
        gloo_storage::LocalStorage::delete("nexus_refresh_token");

        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthTokens {
    pub access_token: String,
    pub expires: i64,
    pub refresh_token: String,
}

use gloo_storage::Storage;
