# Codebase Structure

Verified September 13, 2026. This map describes source and configuration, not generated build conventions.

## Core Sections (Required)

### 1) Top-Level Map

| Path                    | Purpose                                                                | Evidence                                                                                                                         |
| ----------------------- | ---------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `src/`                  | Rust application and library modules                                   | [src/lib.rs](../../src/lib.rs), [src/main.rs](../../src/main.rs)                                                                 |
| `tests/`                | Ignored Docker-backed LLDAP smoke tests                                | [tests/lldap_smoke.rs](../../tests/lldap_smoke.rs)                                                                               |
| `migrations/`           | Embedded SQLx migration input                                          | [src/invite_storage.rs](../../src/invite_storage.rs), [0001_invites.sql](../../migrations/0001_invites.sql)                      |
| `assets/icons/`         | Theme SVG assets bundled with the executable                           | [src/routes.rs](../../src/routes.rs), [src/main.rs](../../src/main.rs)                                                           |
| `docs/`                 | Configuration, deployment, lifecycle, operations, and proxy examples   | [docs/README.md](../README.md)                                                                                                   |
| `docs/codebase/`        | Seven evidence-backed codebase reference documents                     | [STACK.md](STACK.md), this document                                                                                              |
| `lldap-bootstrap/`      | Local demo group/user configuration                                    | [lldap-bootstrap/README.md](../../lldap-bootstrap/README.md), [README.md](../../README.md)                                       |
| `.github/workflows/`    | CI and release automation                                              | [ci.yml](../../.github/workflows/ci.yml), [release.yml](../../.github/workflows/release.yml)                                     |
| Cargo manifest/lockfile | Package metadata, direct requirements, dependency resolution           | [Cargo.toml](../../Cargo.toml), [Cargo.lock](../../Cargo.lock)                                                                   |
| Container/config files  | Application image, proxy deployment, local directory, example settings | [Dockerfile](../../Dockerfile), [docker-compose.yml](../../docker-compose.yml), [config.example.toml](../../config.example.toml) |
| `TODO.md`               | Proposed safety work and accepted future requirements                  | [TODO.md](../../TODO.md)                                                                                                         |

The Phase 1 scan found no monorepo signals. `target/` is generated output, and `data/` is created at runtime; neither defines source conventions. The raw scan is kept outside this directory at `docs/.codebase-scan.txt` and ignored by Git. Evidence: scan directory/monorepo sections, [.gitignore](../../.gitignore), [src/main.rs](../../src/main.rs).

### 2) Entry Points

- Main runtime: [src/main.rs](../../src/main.rs), selected by Cargo's binary layout and the container entrypoint in [Dockerfile](../../Dockerfile).
- Library: [src/lib.rs](../../src/lib.rs), exposing eight application modules for the binary and integration tests.
- Background work: `spawn_invite_cleanup_task` inside the main process, not a separate worker executable.
- HTTP handlers: discovered by the router from [src/routes.rs](../../src/routes.rs).
- Test executable: [tests/lldap_smoke.rs](../../tests/lldap_smoke.rs), selected with `cargo test --test lldap_smoke`.

### 3) Module Boundaries

These are observed responsibilities. The repository does not establish enforced "must not" dependency rules; [TODO] define those before a structural refactor.

| Boundary                                                           | What belongs here today                                                               | Boundary limitation                                          |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| [main.rs](../../src/main.rs)                                       | Asset bundling, config load, state initialization, cleanup scheduling, server startup | Not a separate job service                                   |
| [app_state.rs](../../src/app_state.rs)                             | Shared config, repository, directory client; database initialization                  | Concrete service types, not a mock interface                 |
| [routes.rs](../../src/routes.rs)                                   | HTTP handling, orchestration, request logging, markup, CSS, scripts                   | Presentation is not confined to `views.rs`                   |
| [invite_storage.rs](../../src/invite_storage.rs)                   | Invite/event records, SQL queries, migration runner, transaction boundaries           | Does not make LLDAP operations transactional                 |
| [lldap.rs](../../src/lldap.rs)                                     | LLDAP HTTP/GraphQL and LDAP operations                                                | LLDAP-specific management API, not generic LDAP provisioning |
| [configuration.rs](../../src/configuration.rs)                     | Typed defaults and file/environment loading                                           | Loaded server settings need binding verification             |
| [validation.rs](../../src/validation.rs)                           | Username/email/password checks                                                        | Trimming here does not normalize handler-owned values        |
| [views.rs](../../src/views.rs), [version.rs](../../src/version.rs) | Page title/version helpers                                                            | Not the main rendering layer                                 |

### 4) Naming and Organization Rules

- Observed across all eleven Rust files: lowercase/snake_case filenames such as `app_state.rs`, `invite_storage.rs`, and `lldap_smoke.rs`. Single-word names include `routes.rs`, `validation.rs`, and `main.rs`.
- Modules are grouped by technical responsibility, not repeated feature directories. Documentation uses descriptive hyphenated names; bootstrap fixtures are divided into `group-configs/` and `user-configs/`.
- Internal imports use `crate::...`; the binary and integration tests use `rust_invite_system::...`; colocated tests use `super::...`. See [src/lib.rs](../../src/lib.rs), [src/main.rs](../../src/main.rs), [src/validation.rs](../../src/validation.rs), [tests/lldap_smoke.rs](../../tests/lldap_smoke.rs).
- [TODO] SQL migration constraints/indexes remain uninspected: the migration is in the scan inventory but absent from the available code index. Its presence is not evidence of its schema details.

### 5) Evidence

- Phase 1 scan: `docs/.codebase-scan.txt`, DIRECTORY TREE and MONOREPO SIGNALS; generated local evidence, not required for committed links.
- [src/lib.rs](../../src/lib.rs), [src/main.rs](../../src/main.rs): module exports and entrypoint.
- [src/routes.rs](../../src/routes.rs), [src/invite_storage.rs](../../src/invite_storage.rs), [src/lldap.rs](../../src/lldap.rs): responsibilities.
- [tests/lldap_smoke.rs](../../tests/lldap_smoke.rs), [Dockerfile](../../Dockerfile), [ci.yml](../../.github/workflows/ci.yml): supporting execution paths.
