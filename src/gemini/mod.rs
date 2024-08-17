use std::fmt;
use std::fmt::Debug;

use anyhow::{anyhow, Context, Result};
use url::Url;

pub mod server;
pub mod tls;

#[derive(Debug)]
pub struct GeminiUrl {
    pub scheme: String,
    pub hostname: String,
    pub port: u16,
    pub path: String,
}

impl fmt::Display for GeminiUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}://{}:{}{}",
            self.scheme, self.hostname, self.port, self.path
        )
    }
}

impl GeminiUrl {
    /// Default scheme: "gemini://"
    /// Port: 1965
    /// Path: "/"
    pub fn new(input: &str) -> Result<GeminiUrl> {
        let input_with_scheme = if input.contains("://") {
            input.to_string()
        } else {
            format!("gemini://{}", input)
        };
        let parsed = Url::parse(&input_with_scheme)
            .with_context(|| format!("Invalid request URL {}", &input_with_scheme))?;
        if parsed.scheme() != "gemini" {
            return Err(anyhow!(
                "Invalid request (URL protocol should be gemini://)"
            ));
        }
        let hostname = parsed
            .host_str()
            .ok_or_else(|| anyhow!("Invalid request (URL must have a host)"))?;
        let mut parsed = GeminiUrl {
            scheme: parsed.scheme().to_string(),
            hostname: hostname.to_string(),
            port: parsed.port().unwrap_or(1965),
            path: parsed.path().to_string(),
        };
        if parsed.path.is_empty() {
            parsed.path = String::from("/")
        };
        Ok(parsed)
    }

    pub fn to_url(&self) -> Url {
        Url::parse(&self.to_string()).unwrap()
    }
}
