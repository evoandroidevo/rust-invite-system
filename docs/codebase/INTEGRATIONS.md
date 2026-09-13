# External Integrations

Verified September 13, 2026. Configuration examples are not verification of a running deployment.

## Core Sections (Required)

### 1) Integration Inventory

| System                   | Type                   | Purpose                                                                 | Auth model                                                                  | Criticality                           | Evidence                                                                                     |
| ------------------------ | ---------------------- | ----------------------------------------------------------------------- | --------------------------------------------------------------------------- | ------------------------------------- | -------------------------------------------------------------------------------------------- |
| LLDAP management service | HTTP/GraphQL API       | Look up/create/delete users; list/assign existing groups                | Service credentials at `/auth/simple/login`; bearer token at `/api/graphql` | Required for redemption               | [src/lldap.rs](../../src/lldap.rs)                                                           |
| LLDAP directory endpoint | LDAP/LDAPS             | Set initial password via password-modify operation                      | Service-account simple bind                                                 | Required for redemption               | [src/lldap.rs](../../src/lldap.rs#L292)                                                      |
| SQLite                   | Embedded database      | Invitation state and event history                                      | Filesystem access                                                           | Required at startup and for invites   | [src/app_state.rs](../../src/app_state.rs), [storage](../../src/invite_storage.rs)           |
| Caddy / Nginx examples   | Reverse proxy          | TLS termination, admin authentication, forwarding; Nginx request limits | Caddy password hash / Nginx htpasswd                                        | Documented admin access boundary      | [Caddy](../caddy/Caddyfile.example), [Nginx](../nginx/rust-invite-system.conf.example)       |
| GHCR / GitHub Releases   | Build/release services | Publish images and release notes                                        | Workflow token                                                              | Delivery, not request-time dependency | [ci.yml](../../.github/workflows/ci.yml), [release.yml](../../.github/workflows/release.yml) |

No queue, event bus, or monitoring backend is configured in the inspected application startup or production Compose. [TODO] External infrastructure may exist outside this repository; do not infer its absence in production.

### 2) Data Stores

| Store                                | Role                                                           | Access layer                                 | Key risk                                                                             | Evidence                                                                                                             |
| ------------------------------------ | -------------------------------------------------------------- | -------------------------------------------- | ------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------- |
| `sqlite://./data/invites.db` default | Hashed tokens, group JSON, lifecycle timestamps, invite events | SQLx repository; up to five pool connections | SQLite and LLDAP do not share a transaction; cleanup deletes eligible invite history | [configuration](../../src/configuration.rs), [state](../../src/app_state.rs), [storage](../../src/invite_storage.rs) |
| External LLDAP                       | Provisioned users/passwords/group membership                   | HTTP/GraphQL plus LDAP                       | Partial provisioning and failed compensation                                         | [src/lldap.rs](../../src/lldap.rs)                                                                                   |

[TODO] Inspect exact SQL constraints/indexes in [0001_invites.sql](../../migrations/0001_invites.sql), absent from the available code index. Repository queries establish the accessed fields, not every schema guarantee. No application cache store is present in the inspected state definition.

### 3) Secrets and Credentials Handling

- [AppConfig](../../src/configuration.rs) loads defaults, an optional TOML file, then environment overrides. Development credential literals and plaintext endpoints are present; production rejection of those defaults is not implemented in that loader.
- [Compose](../../docker-compose.yml) requires directory credentials and the Caddy password hash through environment interpolation. [Deployment guidance](../deployment.md) recommends environment injection or a secrets manager and documents credential rotation by recreating the app.
- The ignore file covers local `dev.env`, `config.toml`, and `data/`. It does not include a general `.env` rule, despite the deployment guide suggesting an ignored local `.env`. Verify exclusion before using that workflow. Evidence: [.gitignore](../../.gitignore), [deployment guide](../deployment.md).
- Custom CA loading and skip-verification settings are applied to the LDAP client. The HTTP client uses `reqwest::Client::new()`; do not assume the LDAP CA option also configures HTTPS. See [src/lldap.rs](../../src/lldap.rs).
- Accepted intent from [TODO.md](../../TODO.md): one future built-in admin; strongly recommend authenticated proxy protection and warn against direct public exposure; plaintext directory traffic is acceptable only on explicitly isolated, properly firewalled networks. On September 13, 2026, the user also selected any pre-existing LLDAP group as assignable, not an application allowlist.

### 4) Reliability and Failure Behavior

- No explicit retry/backoff, circuit breaker, application deadline, or concurrency limiter appears in the inspected LLDAP wrapper. [TODO] Verify library-default timeouts separately; no numeric timeout guarantee is asserted here.
- Each uniqueness lookup authenticates independently; provisioning authenticates again. Group membership calls are sequential. See [src/lldap.rs](../../src/lldap.rs#L244).
- GraphQL errors and failed HTTP status codes become errors, but membership/deletion `ok` values are discarded. Cleanup errors are also discarded. See [src/lldap.rs](../../src/lldap.rs#L225).
- Unknown groups are rejected locally after creating the account, without group creation and without taking the deletion branch. [TODO] Test the supported server's response to missing group IDs separately from this application's local name-resolution failure.
- The production proxy example has a container-topology mismatch: Caddy targets its own loopback address rather than the separate app service. The backend is marked `internal: true`; [TODO] verify reachability to external directory endpoints and intended firewall rules. See [Compose](../../docker-compose.yml), [Caddy](../caddy/Caddyfile.example).
- Backup/restore and credential rotation are documented in [operations](../operations.md) and [deployment](../deployment.md); no restore exercise was run during this documentation pass.

### 5) Observability for Integrations

- Route-level JSON logs record request start/completion, but the logger has no response-status field or distinct per-integration timing. See [RequestLog](../../src/routes.rs#L102).
- Failed background cleanup emits a generic message; its actual error is discarded. Directory compensation failures are not logged or persisted by the inspected code. See [main](../../src/main.rs), [lldap](../../src/lldap.rs).
- [TODO] Metrics, distributed tracing, external log retention, alerts, and service-level objectives require deployment evidence. Referer redaction and trusted proxy-header handling remain safety tasks in [TODO.md](../../TODO.md).

### 6) Evidence

- [src/lldap.rs](../../src/lldap.rs): endpoints, authentication, TLS, and failure handling.
- [src/configuration.rs](../../src/configuration.rs), [docker-compose.yml](../../docker-compose.yml), [.gitignore](../../.gitignore): configuration and local credential handling.
- [src/invite_storage.rs](../../src/invite_storage.rs), [src/app_state.rs](../../src/app_state.rs): database access.
- [Caddyfile.example](../caddy/Caddyfile.example), [Nginx example](../nginx/rust-invite-system.conf.example): proxy safeguards and upstreams.
- [operations.md](../operations.md), [lldap-requirements.md](../lldap-requirements.md): operational intent and service-account requirements.
