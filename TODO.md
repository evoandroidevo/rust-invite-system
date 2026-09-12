# Project TODO

## 1. Confirm the foundation

- [x] Initialize the Rust binary crate.
- [x] Add Topcoat `0.8.0`.
- [x] Verify the project with `cargo check`.
- [x] Confirm the current Topcoat `0.8.0` routing, view, and server-start APIs with a compiling starter page.
- [x] Choose LLDAP GraphQL as the provisioning path; reserve `ldap3` for directory queries and compatibility checks.
- [ ] Confirm the LLDAP user-creation and password-handling flow with a real integration test before writing provisioning code.

Foundation decisions:

- Topcoat `0.8.0` uses `topcoat::router::{page, Router, RouterBuilderDiscoverExt}` and `topcoat::view::{view, View}` for the starter route.
- Topcoat query parameters are read through request context helpers, not the older `Query(...)` extractor shown in the imported chat.
- Existing workspace deployments use LLDAP's LDAP listener on port `3890`; the invite service should use the LLDAP management API for writes so password handling follows LLDAP's supported path.
- LDAP bind credentials, base DN, and API settings must be loaded from runtime configuration or secrets.

## 2. Establish the application structure

- [ ] Create modules for configuration, application state, routes, invite storage, LLDAP provisioning, validation, and views.
- [ ] Add structured application errors and user-safe error responses.
- [ ] Add graceful startup and shutdown handling.
- [ ] Add a health-check endpoint.

## 3. Configuration and secrets

- [ ] Define typed configuration for server, database, LLDAP, invite expiry, and password policy.
- [ ] Load non-secret defaults from a local configuration file.
- [ ] Support environment-variable overrides for deployment.
- [ ] Keep LDAP bind passwords and other secrets out of committed files.
- [ ] Add a documented example configuration with placeholder values.

## 4. Invite storage

- [ ] Add SQLite and migration support.
- [ ] Create an invites table with a hashed invite token, groups, creation time, expiry time, and used time.
- [ ] Generate cryptographically secure invite tokens.
- [ ] Store only a hash of each invite token when practical.
- [ ] Validate expiry and one-time use.
- [ ] Consume an invite atomically after successful account provisioning.
- [ ] Add cleanup for expired invites.

## 5. User invite flow

- [ ] Build the admin form for selecting groups and generating an invite.
- [ ] Return a complete invite URL to the administrator.
- [ ] Build `/invite?code=...` with username, email, first name, last name, and password fields.
- [ ] Preserve the invite token through form submission without trusting hidden fields alone.
- [ ] Validate the invite before provisioning.
- [ ] Validate username and email format and uniqueness.
- [ ] Validate the configured password policy.
- [ ] Return clear success and failure views.

## 6. LLDAP provisioning

- [ ] Implement a dedicated LLDAP client service.
- [ ] Use a service account with the minimum required permissions.
- [ ] Provision the user with the required attributes.
- [ ] Assign the groups stored on the invite.
- [ ] Use TLS or a private trusted network for LDAP traffic.
- [ ] Avoid blocking synchronous LDAP calls on async request threads.
- [ ] Define rollback behavior when provisioning succeeds but invite consumption fails.
- [ ] Never log passwords, bind credentials, or complete invite tokens.

## 7. Admin protection and HTTP security

- [ ] Keep admin authentication and TLS termination in the reverse proxy as planned.
- [ ] Restrict admin routes so they are not publicly reachable through an unprotected path.
- [ ] Validate forwarded headers and trusted proxy configuration.
- [ ] Add CSRF protection for state-changing browser forms where required.
- [ ] Add rate limiting for invite generation and registration attempts.
- [ ] Set secure cookies and security-related response headers.

## 8. Tests and verification

- [ ] Unit-test token generation, hashing, expiry, password policy, and email validation.
- [ ] Test atomic invite consumption under concurrent requests.
- [ ] Add route tests for valid, expired, used, and invalid invites.
- [ ] Add an LLDAP integration test environment or mock client.
- [ ] Run `cargo fmt --check`, `cargo clippy`, and `cargo test` in CI.
- [ ] Test configuration loading with file values and environment overrides.

## 9. Deployment

- [ ] Add a multi-stage Dockerfile using a current Rust builder image.
- [ ] Add a non-root runtime container.
- [ ] Add Docker Compose with persistent SQLite storage and read-only configuration mounts.
- [ ] Bind the application only to the reverse proxy network or localhost.
- [ ] Add a reverse proxy configuration with HTTPS and admin route protection.
- [ ] Document database backups, secret injection, upgrades, and recovery.

## 10. Documentation

- [ ] Update the README with local development setup.
- [ ] Document the invite lifecycle and security assumptions.
- [ ] Document required LLDAP permissions and attributes.
- [ ] Document configuration keys and deployment commands.
- [ ] Add an operations checklist for rotating credentials and backing up data.
