# Project TODO

## 1. Confirm the foundation

- [x] Initialize the Rust binary crate.
- [x] Add Topcoat `0.8.0`.
- [x] Verify the project with `cargo check`.
- [x] Confirm the current Topcoat `0.8.0` routing, view, and server-start APIs with a compiling starter page.
- [x] Choose LLDAP GraphQL as the provisioning path; reserve `ldap3` for directory queries and compatibility checks.
- [x] Confirm the LLDAP user-creation and password-handling flow with a real integration test before writing provisioning code.

Foundation decisions:

- Topcoat `0.8.0` uses `topcoat::router::{page, Router, RouterBuilderDiscoverExt}` and `topcoat::view::{view, View}` for the starter route.
- Topcoat query parameters are read through request context helpers, not the older `Query(...)` extractor shown in the imported chat.
- Existing workspace deployments use LLDAP's LDAP listener on port `3890`; the invite service should use the LLDAP management API for writes so password handling follows LLDAP's supported path.
- LDAP bind credentials, base DN, and API settings must be loaded from runtime configuration or secrets.
- The local LLDAP smoke test uses `dev.env`, pulls `lldap/lldap:latest`, and is run explicitly with `cargo test --test lldap_smoke -- --ignored`.

## 2. Establish the application structure

- [x] Create modules for configuration, application state, routes, invite storage, LLDAP provisioning, validation, and views.
- [x] Add structured application errors and user-safe error responses.
- [x] Add graceful startup and shutdown handling.
- [x] Add a health-check endpoint.

## 3. Configuration and secrets

- [x] Define typed configuration for server, database, LLDAP, invite expiry, and password policy.
- [x] Load non-secret defaults from an optional local TOML configuration file.
- [x] Support `APP__` environment-variable overrides for deployment.
- [ ] Keep LDAP bind passwords and other secrets out of committed files.
- [x] Add a documented example configuration with placeholder values.

## 4. Invite storage

- [x] Add SQLite and migration support.
- [x] Create invite and invite-event tables with a hashed invite token, groups, creation time, expiry time, and consumption state.
- [x] Generate cryptographically secure invite tokens.
- [x] Store only a hash of each invite token when practical.
- [x] Validate expiry and one-time use.
- [x] Consume an invite atomically after successful account provisioning.
- [x] Add cleanup for expired/revoked invites older than 120 days (background task, runs daily).

## 5. User invite flow

- [x] Build the initial admin form for selecting groups and expiration.
- [x] Return a complete invite URL to the administrator.
- [x] View invite history in the admin dashboard, with the ability to disable (revoke) active invites.
- [x] Build `/invite?code=...` with username, email, first name, last name, and password fields.
- [x] Preserve the invite token through form submission without trusting hidden fields alone.
- [x] Validate the invite before provisioning.
- [x] Validate username and email format and uniqueness.
- [x] Validate the configured password policy.
- [x] Return clear success and failure views.

## 6. LLDAP provisioning

- [x] Implement a dedicated LLDAP client service.
- [x] Add LDAP TLS configuration with CA file and verify-skip support.
- [x] Use a service account with the minimum required permissions.
- [x] Provision the user with the required attributes.
- [x] Assign the groups stored on the invite.
- [x] Use TLS or a private trusted network for LDAP traffic.
- [x] Avoid blocking synchronous LDAP calls on async request threads.
- [x] Define rollback behavior when provisioning succeeds but invite consumption fails.
- [x] Never log passwords, bind credentials, or complete invite tokens.

## 7. Admin protection and HTTP security

- [x] Keep admin authentication and TLS termination in the reverse proxy as planned.
- [x] Restrict admin routes so they are not publicly reachable through an unprotected path.
- [x] Validate forwarded headers and trusted proxy configuration.
- [ ] Add CSRF protection for state-changing browser forms where required.
- [x] Add rate limiting for invite generation and registration attempts.
- [ ] Set secure cookies and security-related response headers.

## 8. Tests and verification

- [x] Unit-test token generation, hashing, expiry, password policy, and email validation.
- [x] Test atomic invite consumption under concurrent requests.
- [x] Add route tests for valid, expired, used, and invalid invites.
- [x] Add an LLDAP user-provisioning integration test or mock client.
- [x] Add an opt-in LLDAP Docker smoke test using `lldap/lldap:latest` and `dev.env`.
- [x] Run `cargo fmt --check`, `cargo clippy`, and `cargo test` in CI.
- [x] Test configuration loading with file values and environment overrides.

## 9. Deployment

- [x] Add a multi-stage Dockerfile using a current Rust builder image.
- [x] Add a non-root runtime container.
- [x] Add Docker Compose with persistent SQLite storage and read-only configuration mounts.
- [x] Bind the application only to the reverse proxy network or localhost.
- [x] Add a reverse proxy configuration with HTTPS and admin route protection.
- [x] Document database backups, secret injection, upgrades, and recovery.

## 10. Documentation

- [x] Update the README with local development setup.
- [x] Document the invite lifecycle and security assumptions.
- [x] Document required LLDAP permissions and attributes.
- [x] Document configuration keys and deployment commands.
- [x] Add an operations checklist for rotating credentials and backing up data.
