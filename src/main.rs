use std::env;

use anyhow::{Context, Result};
use log::error;

use gemini_grs::gemini;

#[derive(Debug)]
pub struct EnvConfig {
    pub hostname: String,
    pub key_path: String,
    pub cert_path: String,
    pub server_root: String,
}

impl EnvConfig {
    pub fn from_env() -> Result<Self> {
        let hostname = env::var("GEMINI_SERVER_HOSTNAME").unwrap_or("127.0.0.1:1965".to_string());
        let key_path = env::var("GEMINI_SERVER_TLS_KEY_FILENAME").with_context(|| {
            "Missing environment variable GEMINI_SERVER_TLS_KEY_FILENAME".to_string()
        })?;
        let cert_path = env::var("GEMINI_SERVER_TLS_CERT_FILENAME").with_context(|| {
            "Missing environment variable GEMINI_SERVER_TLS_CERT_FILENAME".to_string()
        })?;
        let server_root = env::var("GEMINI_SERVER_ROOT_DIRECTORY").with_context(|| {
            "Missing environment variable GEMINI_SERVER_ROOT_DIRECTORY".to_string()
        })?;
        Ok(Self {
            hostname,
            key_path,
            cert_path,
            server_root,
        })
    }
}

fn main() {
    let exit_code = run();
    std::process::exit(exit_code);
}

fn run() -> i32 {
    env_logger::init();
    let env_config = match EnvConfig::from_env() {
        Err(e) => {
            error!("{:#?}", e);
            return 1;
        }
        Ok(x) => x,
    };
    if let Err(e) = gemini::server::start_server(
        &env_config.hostname,
        &env_config.key_path,
        &env_config.cert_path,
        &env_config.server_root,
    ) {
        error!("{:#?}", e);
        return 1;
    }
    0
}
