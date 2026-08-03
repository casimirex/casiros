//! Typed Rust client for the CASIROS REST API.
//!
//! This crate provides a thin, asynchronous HTTP client that mirrors the
//! `OpenAPI` contract exposed by [`casiros_api`]. All request and response types
//! are re-exported from the API crate so the client and server share a single
//! source of truth.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::pedantic)]
#![deny(warnings)]
#![allow(clippy::needless_return)]

use std::time::Duration;

pub use casiros_api::models::{
    BindingRequest, DistributionRequest, EdgeRequest, ErrorResponse, EvaluateRequest,
    EvaluateResponse, FormulaRequest, HealthzResponse, NodeRequest, PortRequest, SimulateRequest,
    SimulateResponse,
};
use reqwest::{Client, Url};

/// Errors that can occur when calling the CASIROS API.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// The base URL provided could not be parsed.
    #[error("Invalid base URL: {0}")]
    InvalidUrl(String),

    /// An HTTP transport error occurred.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// The API returned an error response.
    #[error("API error: {error}")]
    Api {
        /// Error message returned by the server.
        error: String,
    },

    /// The response body could not be decoded.
    #[error("Failed to decode response: {0}")]
    Decode(String),
}

/// Asynchronous client for the CASIROS REST API.
#[derive(Debug, Clone)]
pub struct CasirosClient {
    client: Client,
    base_url: Url,
}

impl CasirosClient {
    /// Creates a new client pointing at the given base URL.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::InvalidUrl`] if `base_url` cannot be parsed.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_api_client::CasirosClient;
    ///
    /// let client = CasirosClient::new("http://localhost:8080").unwrap();
    /// assert_eq!(client.base_url().as_str(), "http://localhost:8080/");
    /// ```
    pub fn new(base_url: impl AsRef<str>) -> Result<Self, ClientError> {
        let base_url = Url::parse(base_url.as_ref())
            .map_err(|err| ClientError::InvalidUrl(err.to_string()))?;
        return Self::new_with_url(base_url);
    }

    /// Creates a new client with a parsed base URL and default timeout.
    fn new_with_url(base_url: Url) -> Result<Self, ClientError> {
        let client = Client::builder().timeout(Duration::from_secs(30)).build()?;
        return Ok(Self { client, base_url });
    }

    /// Returns the configured base URL.
    #[must_use]
    pub fn base_url(&self) -> &Url {
        return &self.base_url;
    }

    /// Calls `GET /healthz`.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::Http`] on transport failure or [`ClientError::Api`]
    /// if the server returns an error status.
    pub async fn healthz(&self) -> Result<HealthzResponse, ClientError> {
        return self.get("healthz").await;
    }

    /// Calls `POST /evaluate`.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::Http`] on transport failure or [`ClientError::Api`]
    /// if the server returns an error status.
    pub async fn evaluate(
        &self,
        request: &EvaluateRequest,
    ) -> Result<EvaluateResponse, ClientError> {
        return self.post("evaluate", request).await;
    }

    /// Calls `POST /simulate`.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::Http`] on transport failure or [`ClientError::Api`]
    /// if the server returns an error status.
    pub async fn simulate(
        &self,
        request: &SimulateRequest,
    ) -> Result<SimulateResponse, ClientError> {
        return self.post("simulate", request).await;
    }

    async fn get<R: serde::de::DeserializeOwned>(&self, path: &str) -> Result<R, ClientError> {
        let url = self.resolve(path);
        let response = self.client.get(url).send().await?;
        return Self::decode(response).await;
    }

    async fn post<B: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<R, ClientError> {
        let url = self.resolve(path);
        let response = self.client.post(url).json(body).send().await?;
        return Self::decode(response).await;
    }

    fn resolve(&self, path: &str) -> Url {
        return self.base_url.join(path).expect("path is always valid");
    }

    async fn decode<R: serde::de::DeserializeOwned>(
        response: reqwest::Response,
    ) -> Result<R, ClientError> {
        let status = response.status();
        if !status.is_success() {
            let error = response
                .json::<ErrorResponse>()
                .await
                .map_err(|err| ClientError::Decode(err.to_string()))?;
            return Err(ClientError::Api { error: error.error });
        }
        return Ok(response.json::<R>().await?);
    }
}
