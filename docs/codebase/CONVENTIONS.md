# Coding Conventions

Verified September 13, 2026. Observed patterns are not automatically recommended practices.

## Core Sections (Required)

### 1) Naming Rules

| Item                     | Observed rule                                                         | Example                                               | Evidence                                                                                           |
| ------------------------ | --------------------------------------------------------------------- | ----------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| Files                    | Lowercase/snake_case across the eleven Rust files                     | `app_state.rs`, `invite_storage.rs`, `lldap_smoke.rs` | [STRUCTURE.md](STRUCTURE.md), [src/lib.rs](../../src/lib.rs)                                       |
| Functions/methods/fields | snake_case                                                            | `consume_invite`, `token_hash`                        | [src/invite_storage.rs](../../src/invite_storage.rs)                                               |
| Types                    | PascalCase                                                            | `AppState`, `LldapClient`, `PasswordPolicy`           | [src/app_state.rs](../../src/app_state.rs), [src/validation.rs](../../src/validation.rs)           |
| Constants                | SCREAMING_SNAKE_CASE                                                  | `STALE_INVITE_RETENTION_DAYS`                         | [src/invite_storage.rs](../../src/invite_storage.rs)                                               |
| Environment              | `APP__` prefix with nested `__` keys                                  | `APP__LDAP__HTTP_URL`                                 | [src/configuration.rs](../../src/configuration.rs), [docker-compose.yml](../../docker-compose.yml) |
| Private members          | Rust visibility, no special private-name prefix in inspected examples | `InviteRepository.pool`, `LldapClient.http`           | [storage](../../src/invite_storage.rs), [directory client](../../src/lldap.rs)                     |

### 2) Formatting and Linting

- Formatter: rustfmt, enforced by `cargo fmt --check` in [ci.yml](../../.github/workflows/ci.yml).
- Linter: `cargo clippy --all-targets --all-features -- -D warnings` in the same workflow.
- CI selects stable Rust and installs both components. The Phase 1 scan detected no root-specific formatter/linter configuration. [TODO] Do not infer additional team style rules from that negative scan.
- Rust edition 2024 is declared in [Cargo.toml](../../Cargo.toml). TypeScript strictness is not applicable to this Rust package.

### 3) Import and Module Conventions

- [src/lib.rs](../../src/lib.rs) explicitly exports eight modules using `pub mod`.
- Internal code uses `crate::...`; binary/integration code uses `rust_invite_system::...`; nested unit tests use `super::...`. See [main](../../src/main.rs), [validation](../../src/validation.rs), [smoke tests](../../tests/lldap_smoke.rs).
- Standard-library, external-crate, and internal imports are often separated, but ordering is not uniform: compare [main](../../src/main.rs) with [lldap](../../src/lldap.rs). Follow rustfmt rather than claiming a stricter import-order policy.
- [TODO] No project-specific alias/public-export policy beyond the observed module declarations has been established.

### 4) Error and Logging Conventions

- Storage returns typed SQLx errors and propagates with `?`; the directory adapter has `LldapError` with HTTP, JSON, and GraphQL variants. LDAP errors are converted to its GraphQL string variant. See [storage](../../src/invite_storage.rs), [directory client](../../src/lldap.rs).
- Startup uses `expect`/`unwrap` for configuration, assets, initialization, and server termination. Routes render user-facing errors but also use `unwrap_or(false)` or discard results. Those are current failure-handling gaps, not conventions to copy. See [main](../../src/main.rs), [routes](../../src/routes.rs).
- `RequestLog` emits JSON via `eprintln!` on construction and drop. Fields include timestamp, level, configurable message key (default `msg`), method, path, client IP, forwarded-for, user agent, Referer, and completion duration. It does not record the response status. See [RequestLog](../../src/routes.rs#L102), [LoggingConfig](../../src/configuration.rs).
- Request paths omit the query, but Referer is retained in full. Proxy examples set `no-referrer`; application-side redaction remains proposed. Config and submitted-password types derive `Debug`, creating accidental disclosure risk without proving any password has been logged. See [routes](../../src/routes.rs#L805), [configuration](../../src/configuration.rs), [Caddy](../caddy/Caddyfile.example).
- Username/email validators trim local copies; handlers pass original values to LLDAP. Password validation does not trim. Normalize identifiers consistently before future changes; preserve password bytes. See [validation](../../src/validation.rs), [submit_invite](../../src/routes.rs#L916).

### 5) Testing Conventions

- Unit/storage tests use nested `#[cfg(test)] mod tests`, `#[test]` or `#[tokio::test]`, descriptive snake_case names, and standard `assert!`/`assert_eq!` macros. See [validation](../../src/validation.rs), [storage](../../src/invite_storage.rs).
- Storage tests use temporary SQLite files and real migrations. Docker tests live in [tests/lldap_smoke.rs](../../tests/lldap_smoke.rs) and are explicitly ignored by default.
- No dedicated mocking dependency appears in [Cargo.toml](../../Cargo.toml). The inspected integration tests exercise real services, not a reusable fault-injection interface.
- [TODO] Coverage target, coverage tool, and full-handler test policy are not established by the inspected CI. See [TESTING.md](TESTING.md).

### 6) Evidence

- [ci.yml](../../.github/workflows/ci.yml), [Cargo.toml](../../Cargo.toml): enforced tooling and edition.
- [src/lib.rs](../../src/lib.rs), [src/main.rs](../../src/main.rs): module/import examples.
- [src/routes.rs](../../src/routes.rs), [src/lldap.rs](../../src/lldap.rs), [src/invite_storage.rs](../../src/invite_storage.rs): error/logging patterns.
- [src/validation.rs](../../src/validation.rs), [tests/lldap_smoke.rs](../../tests/lldap_smoke.rs): test conventions.
- Phase 1 scan: `docs/.codebase-scan.txt`, LINTING AND FORMATTING CONFIG and DIRECTORY TREE.
