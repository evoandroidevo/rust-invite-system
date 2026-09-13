# Architecture

Verified September 13, 2026. Current behavior is distinct from planned work in [TODO.md](../../TODO.md).

## Core Sections (Required)

### 1) Architectural Style

- One Rust process, with modules organized by responsibility: HTTP routes orchestrate a SQLite repository and an LLDAP client. This classification follows [src/lib.rs](../../src/lib.rs), [src/routes.rs](../../src/routes.rs), and [src/app_state.rs](../../src/app_state.rs); it is not a claim of Clean Architecture.
- Server-rendered views, CSS, and scripts mostly live in the routes module. The views module supplies title/version helpers.
- Constraints: invitations span two independently failing systems; HTTP handlers share concrete application state; the documented admin security boundary is the reverse proxy. A single built-in admin account is accepted future work, not present application authentication. See [lifecycle intent](../invite-lifecycle.md) and [TODO.md](../../TODO.md).

### 2) System Flow

```text
main -> assets/config -> SQLite + migrations + LLDAP client
     -> cleanup task -> discovered Topcoat routes + shared AppState

invite submission -> field validation -> directory uniqueness lookups
                  -> stored invite validation -> LLDAP provisioning
                  -> SQLite consumption/event -> success view
```

1. [Startup](../../src/main.rs) bundles assets, loads optional configuration, creates `data/`, and initializes state. [AppState](../../src/app_state.rs) opens a SQLite pool with at most five connections, constructs the directory client, and runs migrations before serving.
2. The router discovers handlers and attaches shared state. A Tokio task periodically removes eligible stale invitations; it runs on a 24-hour interval using a 120-day cutoff.
3. [submit_invite](../../src/routes.rs#L916) checks that a code exists and validates fields, then checks username/email availability. Lookup errors become `false`, before stored invite validation.
4. The handler hashes the token, retrieves and checks the invitation, and parses its group list. [provision_user](../../src/lldap.rs#L82) logs in, creates the user, sets the password, lists groups, and assigns memberships sequentially.
5. Only after directory provisioning does [consume_invite](../../src/invite_storage.rs#L108) use `BEGIN IMMEDIATE` to check active state, update the record, and append a consumption event in one SQLite transaction.
6. Failed consumption triggers a best-effort account deletion whose error is discarded. A success view is returned only after successful consumption. This is not a transaction across the directory and SQLite.

### 3) Layer/Module Responsibilities

No enforced dependency prohibitions were found in the inspected modules; the "Must not own" column records observed separation, not invented team policy.

| Layer or module  | Owns                                                | Must not own / observed boundary                    | Evidence                                                             |
| ---------------- | --------------------------------------------------- | --------------------------------------------------- | -------------------------------------------------------------------- |
| Startup/state    | Initialization and shared services                  | No separate worker deployment is defined here       | [main.rs](../../src/main.rs), [app_state.rs](../../src/app_state.rs) |
| Routes           | Request orchestration, logging, UI responses        | SQL mutations are delegated, but rendering is local | [routes.rs](../../src/routes.rs)                                     |
| Validation       | Local field/password checks                         | No directory or database lookups                    | [validation.rs](../../src/validation.rs)                             |
| Repository       | Invite/event SQL and transactions                   | No LLDAP transaction or compensation mechanism      | [invite_storage.rs](../../src/invite_storage.rs)                     |
| Directory client | HTTP authentication, GraphQL, LDAP password changes | No durable invite/recovery state                    | [lldap.rs](../../src/lldap.rs)                                       |

### 4) Reused Patterns

| Pattern                       | Where found                                                                | Observed role                                                             |
| ----------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Shared application context    | [AppState](../../src/app_state.rs), [routes](../../src/routes.rs)          | Handlers obtain the configured repository and directory client            |
| Repository wrapper            | [InviteRepository](../../src/invite_storage.rs)                            | Centralizes SQL and invite/event mutations                                |
| Integration wrapper           | [LldapClient](../../src/lldap.rs)                                          | Groups HTTP/GraphQL and LDAP operations                                   |
| Transactional event insertion | [create/consume/revoke](../../src/invite_storage.rs)                       | Persists invite mutations and their event together                        |
| Best-effort compensation      | [provision_user](../../src/lldap.rs), [submit_invite](../../src/routes.rs) | Attempts account deletion after some later failures; not durable rollback |

The startup/state path defines no message queue or separate event consumer. [TODO] External operational queues or scheduled services, if any, require deployment evidence.

### 5) Known Architectural Risks

- Provisioning precedes exclusive consumption, allowing concurrent requests or crashes to leave directory state inconsistent with invite state. Deletion failures are not persisted for recovery. Source: [routes](../../src/routes.rs#L1049), [directory client](../../src/lldap.rs#L82).
- Unknown-group resolution returns through `?` after account creation/password assignment, bypassing deletion. Other cleanup branches also discard errors. Resolve groups before side effects and design explicit recovery before changing this flow.
- Cleanup removes events for stale expired/revoked invitations. The README's "append-only" wording does not describe this retention exception. Source: [cleanup_stale](../../src/invite_storage.rs#L218), [README.md](../../README.md).
- Compose mounts a host-local Caddy example into a separate proxy container, whose upstream remains `127.0.0.1:8080`. The app service is a different container. Source: [Compose](../../docker-compose.yml), [Caddy example](../caddy/Caddyfile.example). [TODO] Verify a corrected container-aware upstream and actual reachability.
- Loaded `server.host`/`server.port` are not explicitly forwarded at startup. [TODO] Verify the framework's effective bind configuration rather than assuming the documented override works.

### 6) Evidence

- [src/main.rs](../../src/main.rs), [src/app_state.rs](../../src/app_state.rs): initialization and background work.
- [src/routes.rs](../../src/routes.rs), [src/validation.rs](../../src/validation.rs): request flow.
- [src/invite_storage.rs](../../src/invite_storage.rs), [src/lldap.rs](../../src/lldap.rs): transaction and network boundaries.
- [docs/invite-lifecycle.md](../invite-lifecycle.md), [README.md](../../README.md): intent comparison.
- [docker-compose.yml](../../docker-compose.yml), [Caddyfile.example](../caddy/Caddyfile.example): deployment topology.
