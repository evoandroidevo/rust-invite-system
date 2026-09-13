# Technology Stack

Verified September 13, 2026. Versions below are manifest requirements, not resolved lockfile versions.

## Core Sections (Required)

### 1) Runtime Summary

| Area                | Value                                                                                            | Evidence                                                         |
| ------------------- | ------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------- |
| Primary language    | Rust, edition 2024; package version 0.1.0                                                        | [Cargo.toml](../../Cargo.toml)                                   |
| Runtime + version   | Native binary with Tokio; README requires Rust 1.98+ for smoke tests; no manifest `rust-version` | [Cargo.toml](../../Cargo.toml), [README.md](../../README.md)     |
| Package manager     | Cargo; lockfile present                                                                          | [Cargo.toml](../../Cargo.toml), [Cargo.lock](../../Cargo.lock)   |
| Module/build system | Library and binary entry points; release container build                                         | [src/main.rs](../../src/main.rs), [Dockerfile](../../Dockerfile) |
| Container           | Builder `rust:1-bookworm`; runtime `debian:bookworm-slim`; UID 10001                             | [Dockerfile](../../Dockerfile)                                   |

Local verification on September 13, 2026: `rustc --version` returned `rustc 1.98.1 (48a229cea 2026-09-01)`. [TODO] Confirm the effective minimum compiler and resolved dependency versions. Builder and CI toolchain tags are floating, not exact compiler pins.

### 2) Production Frameworks and Dependencies

All direct runtime dependencies from [Cargo.toml](../../Cargo.toml):

| Dependency     | Version requirement | Role / enabled features                                                   |
| -------------- | ------------------- | ------------------------------------------------------------------------- |
| topcoat        | 0.8.0               | HTTP routing and server-rendered views                                    |
| topcoat-asset  | 0.8.0               | Asset bundling; `bundler`                                                 |
| tokio          | 1.0                 | Async runtime; `macros`, `rt-multi-thread`, `time`                        |
| config         | 0.15                | TOML and environment configuration                                        |
| serde          | 1.0                 | Serialization types; `derive`                                             |
| serde_json     | 1.0                 | JSON API payloads, group lists, request logs                              |
| sqlx           | 0.8                 | SQLite; defaults disabled; `runtime-tokio`, `sqlite`, `migrate`, `macros` |
| chrono         | 0.4                 | Timestamps and expiry; `clock`                                            |
| rand           | 0.9                 | Invite token randomness                                                   |
| sha2           | 0.10                | SHA-256 invite token hashing                                              |
| uuid           | 1.0                 | Record identifiers; `v4`                                                  |
| reqwest        | 0.13                | LLDAP HTTP/GraphQL; defaults disabled; `json`, `rustls`                   |
| ldap3          | 0.12                | LDAP password modification; defaults disabled; `tls-rustls-ring`          |
| rustls         | 0.23                | LDAP TLS configuration                                                    |
| rustls-pemfile | 2                   | Custom CA PEM loading                                                     |

Roles are grounded in [startup](../../src/main.rs), [configuration](../../src/configuration.rs), [routes](../../src/routes.rs), [storage](../../src/invite_storage.rs), and [LLDAP client](../../src/lldap.rs). Transitive packages are recorded in the lockfile, not enumerated as direct application dependencies here.

### 3) Development Toolchain

| Tool                | Purpose                                    | Evidence                                                     |
| ------------------- | ------------------------------------------ | ------------------------------------------------------------ |
| rustfmt             | CI formatting check                        | [ci.yml](../../.github/workflows/ci.yml)                     |
| Clippy              | All targets/features; warnings denied      | [ci.yml](../../.github/workflows/ci.yml)                     |
| Cargo test          | Ordinary Rust tests                        | [ci.yml](../../.github/workflows/ci.yml)                     |
| dotenvy 0.15        | Local smoke-test environment               | [Cargo.toml](../../Cargo.toml)                               |
| reqwest 0.13        | Smoke-test HTTP requests; `json`, `rustls` | [Cargo.toml](../../Cargo.toml)                               |
| testcontainers 0.28 | Docker integration-test lifecycle          | [Cargo.toml](../../Cargo.toml), [README.md](../../README.md) |
| Docker Buildx       | Container build/publish to GHCR            | [ci.yml](../../.github/workflows/ci.yml)                     |

### 4) Key Commands

Run from the repository root. These are documented/configured commands, not a claim that each was executed during this documentation pass.

```sh
cargo run
cargo build --release
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo test --test lldap_smoke -- --ignored --nocapture
docker compose -f docker-compose.lldap-dev.yml up -d
```

Evidence: [README.md](../../README.md), [Dockerfile](../../Dockerfile), [ci.yml](../../.github/workflows/ci.yml). Cargo handles dependency fetching during builds. Smoke tests require Docker and a local environment file.

### 5) Environment and Config

- Optional local `config.toml`, based on [config.example.toml](../../config.example.toml), is overlaid by `APP__` nested environment settings; missing values use defaults in [src/configuration.rs](../../src/configuration.rs).
- Compose requires `APP_LDAP_HTTP_URL`, `APP_LDAP_LDAP_URL`, `APP_LDAP_BASE_DN`, `APP_LDAP_USERNAME`, `APP_LDAP_PASSWORD`, and `CADDY_ADMIN_PASSWORD_HASH`. It maps LDAP inputs to the application's double-underscore names. See [docker-compose.yml](../../docker-compose.yml).
- Compose and the Dockerfile also set `HOST` and `PORT`. [TODO] Verify effective binding: startup does not explicitly pass `AppConfig.server` to Topcoat. See [src/main.rs](../../src/main.rs).
- SQLite defaults to `sqlite://./data/invites.db`; startup creates `data/` and runs migrations. Container deployment requires writable `/app/data`, configuration, certificates, and directory connectivity. See [src/app_state.rs](../../src/app_state.rs), [Dockerfile](../../Dockerfile), [deployment guide](../deployment.md).
- Development directory credentials and plaintext endpoints are defaults, not a production security guarantee. The accepted isolated-network exception and planned single-admin requirement are recorded in [TODO.md](../../TODO.md).

### 6) Evidence

- [Cargo.toml](../../Cargo.toml): direct runtime/development dependencies and edition.
- [Dockerfile](../../Dockerfile), [ci.yml](../../.github/workflows/ci.yml): build images and configured gates.
- [src/main.rs](../../src/main.rs), [src/app_state.rs](../../src/app_state.rs): runtime initialization.
- [src/configuration.rs](../../src/configuration.rs), [docker-compose.yml](../../docker-compose.yml): configuration inputs.
- [README.md](../../README.md): documented local commands and compiler requirement.
