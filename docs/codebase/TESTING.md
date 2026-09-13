# Testing Patterns

Verified September 13, 2026. Ordinary tests passed; live deployment and ignored Docker tests were not run.

## Core Sections (Required)

### 1) Test Stack and Commands

- Primary framework: Rust's built-in test harness with Tokio async tests; local compiler `rustc 1.98.1 (48a229cea 2026-09-01)`.
- Assertions: standard `assert!` and `assert_eq!`. Docker support: testcontainers 0.28; local environment: dotenvy 0.15; HTTP: reqwest 0.13. These are requirements from [Cargo.toml](../../Cargo.toml), not independently verified lockfile resolutions.

```sh
cargo test
cargo test --lib
cargo test invite_storage::tests
cargo test --test lldap_smoke -- --ignored --nocapture
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

The library/storage selectors follow the test layout. CI configures the ordinary test, formatting, and Clippy commands; the README documents the ignored smoke command. [TODO] No coverage command is configured in the inspected CI.

Fresh terminal verification on September 13, 2026: `cargo test --quiet --manifest-path <repository>/Cargo.toml` exited 0: **21 passed, 0 failed; 2 Docker tests ignored**. Formatting, Clippy, coverage collection, and Docker smoke tests were not rerun in this documentation pass.

### 2) Test Layout

- Colocated `#[cfg(test)]` modules exist in [validation](../../src/validation.rs), [configuration](../../src/configuration.rs), [storage](../../src/invite_storage.rs), [routes](../../src/routes.rs), and [lldap](../../src/lldap.rs).
- Test names describe behavior, for example `consume_invite_is_atomic_under_concurrency` and `rejects_a_username_that_is_too_short`.
- [tests/lldap_smoke.rs](../../tests/lldap_smoke.rs) contains two `#[tokio::test]` cases with explicit `#[ignore]` attributes. Both load `dev.env` from the repository root and start `lldap/lldap:latest` containers.
- Smoke setup requires `LLDAP_LDAP_USER_DN`, `LLDAP_LDAP_USER_PASS`, `LLDAP_LDAP_BASE_DN`, and `LLDAP_JWT_SECRET`; use the documented local template workflow, not production credentials. See [README.md](../../README.md) and the smoke-test setup.

### 3) Test Scope Matrix

| Scope                   | Covered?                           | Typical target                                                                                   | Notes                                                                   |
| ----------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------- |
| Unit                    | Yes                                | Field/password validation, config precedence, invite state helpers, directory endpoint accessors | Existing ordinary tests; not all error paths                            |
| Storage integration     | Yes                                | Real SQLite migrations, create/consume/revoke/history/cleanup                                    | Includes concurrent consumption; not concurrent end-to-end provisioning |
| Directory integration   | Present, ignored                   | HTTP startup, GraphQL create, LDAP password change, GraphQL delete                               | Uses real Docker service; image version floats                          |
| Complete redemption E2E | Not present in inspected tests     | HTTP submission through provisioning and consumption                                             | [TODO] Add handler-level/full-workflow coverage                         |
| Deployment/security     | Not established by inspected suite | Proxy auth/isolation, headers, limits, TLS, restore                                              | [TODO] Verify independently                                             |

Evidence: [storage tests](../../src/invite_storage.rs#L272), [route tests](../../src/routes.rs#L1120), [smoke tests](../../tests/lldap_smoke.rs), [ci.yml](../../.github/workflows/ci.yml).

### 4) Mocking and Isolation Strategy

- Storage tests create timestamp-suffixed temporary SQLite files, run real migrations, and use a five-connection pool. The concurrency test uses a Tokio barrier. See [src/invite_storage.rs](../../src/invite_storage.rs).
- Configuration tests declare a process-local environment mutex; their scope is local test isolation, not production configuration synchronization. See [src/configuration.rs](../../src/configuration.rs).
- Docker tests use real containers with mapped ports and locally supplied environment values. No dedicated mocking framework appears in the manifest. [TODO] Deterministic network/database failure injection is not established.
- The HTTP availability test polls; the account test logs in after a fixed container wait without that polling loop. This and `latest` create reproducibility/readiness risks, not evidence that failures have occurred. See [tests/lldap_smoke.rs](../../tests/lldap_smoke.rs).

### 5) Coverage and Quality Signals

- CI runs ordinary tests but does not explicitly opt into ignored tests or set a coverage threshold. [TODO] Coverage percentage, target, and tooling remain unknown.
- The account smoke test calls GraphQL creation and LDAP password modification directly. It does not call `provision_user`, exercise group assignment, or submit a full invitation. Its delete call does not assert the returned `ok` flag.
- Missing-group behavior, partial membership failures, compensation failures, crashes, and concurrent full redemption remain verification gaps. See [TODO.md](../../TODO.md).
- Test coverage gaps are separate from production-code debt. The Phase 1 scan reported no production TODO/FIXME/HACK markers; that does not prove complete coverage or absence of defects.

### 6) Evidence

- [Cargo.toml](../../Cargo.toml), [ci.yml](../../.github/workflows/ci.yml): tools and configured gates.
- [src/validation.rs](../../src/validation.rs), [src/configuration.rs](../../src/configuration.rs), [src/routes.rs](../../src/routes.rs): representative unit tests.
- [src/invite_storage.rs](../../src/invite_storage.rs): SQLite integration and concurrency tests.
- [tests/lldap_smoke.rs](../../tests/lldap_smoke.rs), [README.md](../../README.md): real-service tests and prerequisites.
- September 13, 2026 terminal output: `rustc --version`; ordinary Cargo test run, 21 passed and 2 ignored.
