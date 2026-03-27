# Technology Stack

**Analysis Date:** 2026-03-27

## Languages

**Primary:**
- Rust 2024 edition - CLI application, JMAP client, HTTP handlers

## Runtime

**Environment:**
- Rust toolchain (stable)
- Tokio async runtime (1.49.0)

**Build:**
- Cargo (Rust package manager)
- Lockfile: Present (`Cargo.lock`)

## Frameworks

**Core:**
- Tokio 1.49.0 - Async runtime with full feature set for concurrent operations
- Reqwest 0.13.1 - HTTP client with TLS/rustls support and JSON serialization

**CLI:**
- Clap 4.5.54 - Command-line argument parsing with derive macros
- Clap_complete 4.5.65 - Shell completion generation (bash, zsh, fish, powershell)

**GraphQL & MCP:**
- async-graphql 7 - GraphQL server implementation for MCP
- rmcp 0.12 - Model Context Protocol server framework with transport-io support

**Serialization:**
- Serde 1.0.228 - Serialization framework with derive macros
- Serde_json 1.0.149 - JSON support
- Toml 0.8 - TOML configuration parsing

**Document Processing:**
- Kreuzberg 4.4 - Multi-format text extraction (56 formats)
  - Features: archives, bundled-pdfium, email, excel, html, language-detection, office, pdf, xml
- Image 0.25 - Image manipulation (gif, jpeg, png, webp formats)
- roxmltree 0.21.1 - XML parsing for vCard/CardDAV responses

**Observability:**
- Tracing 0.1.44 - Structured logging framework
- Tracing-subscriber 0.3.22 - Tracing backend with env-filter support

## Key Dependencies

**Critical:**
- Reqwest 0.13.1 - Why it matters: Handles all HTTP communication with Fastmail JMAP API and CardDAV endpoints
- Tokio 1.49.0 - Why it matters: Enables concurrent async operations for email operations, downloads, and server mode
- async-graphql 7 - Why it matters: Powers the MCP server's GraphQL schema for composable queries

**API & Serialization:**
- Serde 1.0.228 - Polymorphic JSON/TOML serialization for API responses and config
- Schemars 0.8 - JSON Schema generation for MCP tool parameter validation

**Error Handling:**
- Thiserror 2.0.17 - Error type derivation for custom error handling
- Anyhow 1.0.100 - Error context and wrapping

**File & Directory:**
- Dirs 6.0.0 - Platform-aware config directory paths (`~/.config/fastmail-cli`)

**Utilities:**
- Base64 0.22 - Base64 encoding/decoding for email content

## Configuration

**Environment:**
- Configuration file: `~/.config/fastmail-cli/config.toml`
- File permissions: 0600 (Unix) for secure credential storage
- Env vars take precedence over config file

**Build:**
- Release profile optimizations:
  - `strip = true` - Remove debug symbols
  - `lto = true` - Link-time optimization
  - `codegen-units = 1` - Single compilation unit for maximum optimization
- Targets: x86_64-linux-gnu, x86_64-darwin, aarch64-darwin

## Platform Requirements

**Development:**
- Rust stable toolchain
- Cargo for building
- Unix-like environment preferred (directory handling, permissions)

**Production:**
- Deployment: Standalone binary or via Mise version manager
- Installation: Prebuilt releases on GitHub or `cargo install --git`
- Platforms: Linux (x86_64), macOS (x86_64, aarch64)

## Feature Flags

**Kreuzberg:** archives, bundled-pdfium, email, excel, html, language-detection, office, pdf, xml
- Enables text extraction from 56 file formats

**Image:** gif, jpeg, png, webp
- Disables default features, adds specific image format support

**Reqwest:** json, rustls
- JSON serialization for API requests
- Rustls for TLS (no OpenSSL dependency)

**Tokio:** full
- Complete feature set including io-util, rt, sync, time, macros

**Tracing-subscriber:** env-filter
- Runtime log level filtering via `RUST_LOG` environment variable

---

*Stack analysis: 2026-03-27*
