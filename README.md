# gemini-grs

A Gemini protocol server written in Rust.

## TODO

- [x] Configuration via environment variables
- [x] Proper logging
- [x] Serve images
- [ ] Read and send file in chunks instead of serving whole file from memory
- [ ] Expand to serve other protocols (spartan, scroll, titan etc.)
- [ ] Async I/O?

## External dependencies

OpenSSL

## Building

```shell
cargo build --release
```

## Running

Generate your TLS keys:

```shell
openssl genrsa -out key.pem 2048
openssl req -new -key key.pem -out request.pem
openssl x509 -req -days 36500 -in request.pem -signkey key.pem -out cert.pem
```

Environment variables with examples:

- GEMINI_SERVER_HOSTNAME="0.0.0.0:1965"
- GEMINI_SERVER_TLS_KEY_FILENAME="/where/you/put/key.pem"
- GEMINI_SERVER_TLS_CERT_FILENAME="/where/you/put/cert.pem"
- GEMINI_SERVER_ROOT_DIRECTORY="/files/to/serve"
- RUST_LOG="debug"

Example command:

```shell
GEMINI_SERVER_TLS_KEY_FILENAME=/home/user/server/key.pem \
GEMINI_SERVER_TLS_CERT_FILENAME=/home/user/server/cert.pem \
GEMINI_SERVER_ROOT_DIRECTORY=/gemini-root \
RUST_LOG=debug \
./target/release/gemini-grs
```

