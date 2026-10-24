use obz_auth::{AuthConfig, AuthError};
use std::{env, net::SocketAddr};
use url::Url;

pub struct Config {
    pub bind: SocketAddr,
    pub origin: String,
    pub secure_cookies: bool,
    issuer: String,
    client_id: String,
    client_secret: String,
    callback: String,
    local_http: bool,
}
impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let origin = env::var("API_PUBLIC_URL").unwrap_or_else(|_| "http://localhost:3000".into());
        let url = Url::parse(&origin)?;
        if url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err("API_PUBLIC_URL must contain only scheme, host and optional port".into());
        }
        let local_http = match env::var("AUTH_ALLOW_LOCAL_HTTP").as_deref() {
            Ok("true") => true,
            Ok("false") | Err(_) => false,
            _ => return Err("AUTH_ALLOW_LOCAL_HTTP must be true or false".into()),
        };
        let config = Self {
            bind: env::var("API_BIND")
                .unwrap_or_else(|_| "127.0.0.1:3000".into())
                .parse()?,
            origin: url.origin().ascii_serialization(),
            secure_cookies: url.scheme() == "https",
            issuer: env::var("DEX_ISSUER").unwrap_or_else(|_| "http://localhost:5556".into()),
            client_id: env::var("DEX_CLIENT_ID").unwrap_or_else(|_| "rust-app".into()),
            client_secret: env::var("DEX_CLIENT_SECRET")
                .map_err(|_| "DEX_CLIENT_SECRET is required")?,
            callback: url.join("/api/auth/callback")?.to_string(),
            local_http,
        };
        config.auth_config()?;
        Ok(config)
    }
    pub fn auth_config(&self) -> Result<AuthConfig, AuthError> {
        AuthConfig::new(
            self.issuer.clone(),
            self.client_id.clone(),
            self.client_secret.clone(),
            self.callback.clone(),
            self.local_http,
        )
    }
}
