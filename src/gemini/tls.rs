use std::net::TcpStream;

use hex::ToHex;
use openssl::hash::MessageDigest;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod, SslStream, SslVerifyMode};

pub fn create_tls_acceptor(key_filename: &str, cert_filename: &str) -> anyhow::Result<SslAcceptor> {
    let mut acceptor = SslAcceptor::mozilla_intermediate(SslMethod::tls())?;
    acceptor.set_private_key_file(key_filename, SslFiletype::PEM)?;
    acceptor.set_certificate_chain_file(cert_filename)?;

    // Enable peer verification
    acceptor.set_verify(SslVerifyMode::PEER);

    // Custom verification callback
    acceptor.set_verify_callback(SslVerifyMode::PEER, |preverify_ok, x509_ctx| {
        if preverify_ok {
            return true;
        }
        // Check if it's a self-signed certificate
        let cert = match x509_ctx.current_cert() {
            Some(cert) => cert,
            None => return false,
        };

        let issuer_name = cert.issuer_name().to_der();
        let subject_name = cert.subject_name().to_der();

        let is_self_signed = match (issuer_name, subject_name) {
            (Ok(issuer), Ok(subject)) => issuer == subject,
            _ => return false,
        };

        if is_self_signed {
            let public_key = match cert.public_key() {
                Ok(key) => key,
                Err(_) => return false,
            };

            if cert.verify(&public_key).is_ok() {
                let now = match openssl::asn1::Asn1Time::days_from_now(0) {
                    Ok(time) => time,
                    Err(_) => return false,
                };

                if cert.not_before() <= now && cert.not_after() >= now {
                    return true;
                }
            }
        }
        false
    });
    // Initialize the session ID context
    let context = &crate::nanoid::nanoid().into_bytes();
    acceptor.set_session_id_context(context)?;
    acceptor.check_private_key()?;
    Ok(acceptor.build())
}

pub fn get_peer_certificate_digest(stream: &SslStream<TcpStream>) -> Option<String> {
    if let Some(cert) = stream.ssl().peer_certificate() {
        if let Ok(digest) = cert.digest(MessageDigest::ripemd160()) {
            Some(digest.encode_hex::<String>())
        } else {
            None
        }
    } else {
        None
    }
}
