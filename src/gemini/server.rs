use std::io::{Read, Write};
use std::net::{IpAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::thread;

use anyhow::{anyhow, Context, Result};
use log;
use log::{error, info};
use openssl::ssl::{SslAcceptor, SslStream};
use path_clean::PathClean;

use crate::fs::read_file_as_bytes;
use crate::gemini::{GeminiUrl, tls};
use crate::nanoid::nanoid;
use crate::time::now_unix_millis;

struct GeminiSession {
    pub ip: IpAddr,
    pub tls_id: Option<String>,
    pub stream: SslStream<TcpStream>,
    pub id: String,
    pub timestamp: u128,
}

struct GeminiResponse {
    code: u8,
    mime_type: Option<String>,
    data: Option<Vec<u8>>,
}

impl GeminiResponse {
    pub fn new(
        srv_root: &str,
        gemini_session: &GeminiSession,
        path: &String,
    ) -> Result<GeminiResponse> {
        let filename = resolve_path(srv_root, path)?;
        if !filename.is_file() {
            return Ok(GeminiResponse {
                code: 51, // NOT FOUND
                mime_type: None,
                data: None,
            });
        }
        let mime_type = Self::guess_mime_type(PathBuf::from(&filename));
        if mime_type.is_none() {
            log_info(
                gemini_session,
                &format!("Cannot guess mime type {:?}", filename),
            );
            return Ok(GeminiResponse {
                code: 51,
                mime_type: None,
                data: None,
            });
        }
        log_info(gemini_session, &format!("Opening file {:?}", filename));
        let file_str = filename
            .to_str()
            .ok_or_else(|| anyhow!("Failed to convert path to string"))?;
        // let data = read_file_to_string(file_str)
        //     .with_context(|| format!("Failed to read file {}", path))?;
        let data = read_file_as_bytes(file_str)
            .with_context(|| format!("Failed to read file {}", path))?;
        Ok(GeminiResponse {
            code: 20,
            mime_type,
            data: Some(data),
        })
    }

    fn guess_mime_type(path: PathBuf) -> Option<String> {
        match path.extension() {
            Some(e) => match e.to_str().unwrap_or("") {
                "gmi" => Some(String::from("text/gemini")),
                "jpg" => Some(String::from("image/jpeg")),
                "jpeg" => Some(String::from("image/jpeg")),
                "png" => Some(String::from("image/png")),
                _ => None,
            },
            None => None,
        }
    }

    pub fn get_bytes(&self) -> Vec<u8> {
        let header = match &self.mime_type {
            Some(e) => format!("{}\t{}\r\n", self.code, e),
            None => format!("{}\r\n", self.code),
        };
        // Convert the header to bytes and prepare the response vector
        let mut response_bytes = header.into_bytes();

        // Extend the response with the data if it exists
        if let Some(e) = &self.data {
            response_bytes.extend(e);
        }
        response_bytes
    }
}

pub fn start_server(
    bind_address: &str,
    key_file: &str,
    cert_file: &str,
    srv_root: &str,
) -> Result<()> {
    let acceptor = tls::create_tls_acceptor(key_file, cert_file).context("TLS error")?;
    let listener = TcpListener::bind(bind_address).context("TCP bind error")?;
    info!("Gemini server listening to {}", &bind_address);

    for stream in listener.incoming() {
        // The stream will be dropped at the end of
        // each loop iteration, which will close
        // the stream automatically. No housekeeping
        // necessary.
        // Each stream is handled in a new thread.
        // Async is under consideration for now.
        match stream {
            Err(e) => {
                error!("Failed to accept connection: {:?}", e);
            }
            Ok(stream) => {
                let acceptor = acceptor.clone();
                let srv_root = srv_root.to_string();

                let handle = thread::spawn(move || {
                    if let Err(e) = initiate_connection(&acceptor, stream, &srv_root) {
                        error!("Connection handling error: {:?}", e);
                    }
                });
                if let Err(e) = handle.join() {
                    error!("Thread join error while handling stream: {:?}", e);
                }
            }
        }
    }
    Ok(())
}

fn log_info(session: &GeminiSession, message: &str) {
    info!("{} {} {}", session.id, session.ip.to_string(), message);
}

/// Resolve final absolute path
/// of the file to serve.
/// Final path must always begin with
/// root path (no escape outside root!).
/// If final path is a directory, append
/// index.gmi.
fn resolve_path(root_path: &str, input: &String) -> Result<PathBuf> {
    let path1 = PathBuf::from(root_path);
    let mut path2 = PathBuf::from(input);
    if path2.is_absolute() {
        path2 = PathBuf::from(path2.strip_prefix("/")?);
    }
    let mut final_path = path1.join(path2).clean();
    if !final_path.starts_with(path1) {
        return Err(anyhow!("Invalid path {:?} -> {:?}", input, final_path));
    }
    if final_path.is_dir() {
        final_path = final_path.join("index.gmi");
    }
    Ok(final_path)
}

fn handle_gemini_session(srv_root: &str, gemini_session: &mut GeminiSession) -> Result<()> {
    let mut buffer = [0; 1024];
    let bytes_read = gemini_session
        .stream
        .read(&mut buffer)
        .context("Invalid request (failed to read input stream)")?;
    let received = String::from_utf8(Vec::from(&buffer[..bytes_read]))
        .context("Invalid request (failed to convert input to UTF8)")?;
    let request_data = received.trim();
    let url = GeminiUrl::new(request_data)?;
    log_info(
        gemini_session,
        &format!(
            "New request from {}{} {}",
            gemini_session.ip,
            match &gemini_session.tls_id {
                Some(x) => format!(" TLS digest {}", x),
                None => String::from(""),
            },
            request_data
        ),
    );
    let response = GeminiResponse::new(srv_root, gemini_session, &url.path)?;
    let response_raw = response.get_bytes();
    log_info(
        gemini_session,
        &format!(
            "Reply Code {} Response length {} bytes",
            response.code,
            response_raw.len()
        ),
    );
    gemini_session
        .stream
        .write_all(&response_raw)
        .context("Failed to write response to stream")?;
    Ok(())
}

fn initiate_connection(acceptor: &SslAcceptor, stream: TcpStream, srv_root: &str) -> Result<()> {
    let timestamp1 = now_unix_millis();

    let ip = stream
        .peer_addr()
        .context("Failed to get peer IP address from TCP stream")?
        .ip();

    let stream = acceptor
        .accept(stream)
        .with_context(|| format!("{} Failed to establish SSL connection", ip))?;

    let mut gemini_session = GeminiSession {
        ip,
        tls_id: tls::get_peer_certificate_digest(&stream),
        stream,
        id: nanoid(),
        timestamp: now_unix_millis(),
    };

    handle_gemini_session(srv_root, &mut gemini_session)?;

    log_info(
        &gemini_session,
        &format!("Finished ({}ms)", now_unix_millis() - timestamp1),
    );

    Ok(())
}
