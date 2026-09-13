# Codebase Concerns

Verified September 13, 2026. Findings below are static unless a terminal check is explicitly named. Suggested fixes are not implemented changes.

## Core Sections (Required)

### 1) Top Risks (Prioritized)

| Severity                   | Concern                                                                                    | Evidence                                                                                                       | Impact                                                                            | Suggested action                                                                       |
| -------------------------- | ------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| High                       | Account provisioning precedes exclusive invite consumption; deletion failure is ignored    | [submit_invite](../../src/routes.rs#L1049)                                                                     | Concurrent requests or interruption can leave inconsistent directory/invite state | Design durable claim/recovery semantics and test the full workflow                     |
| High                       | Unknown-group lookup exits after account/password creation without cleanup                 | [provision_user](../../src/lldap.rs#L82)                                                                       | Failed redemption can leave a usable account                                      | Resolve every existing group before creating the account; cover all compensation paths |
| High, deployment-dependent | Admin mutations have no application auth checks                                            | [routes](../../src/routes.rs#L709), [proxy intent](../README.md)                                               | Direct backend access bypasses the intended proxy boundary                        | Preserve proxy isolation; implement the accepted single-admin requirement              |
| High, operational          | Containerized Caddy targets `127.0.0.1:8080`, not the separate app service                 | [Compose](../../docker-compose.yml), [Caddy example](../caddy/Caddyfile.example)                               | Supplied upstream does not address the app container                              | Provide a container-aware upstream and verify deployment end to end                    |
| Medium                     | Username/email lookups occur before invite validity checks; lookup errors become not-found | [submit_invite](../../src/routes.rs#L916)                                                                      | Directory enumeration and fail-open uniqueness handling                           | Validate invite first and fail closed on lookup errors                                 |
| Medium                     | Delete/membership `ok` values and revocation results are discarded                         | [lldap](../../src/lldap.rs#L225), [revoke handler](../../src/routes.rs#L709)                                   | False success and invisible cleanup/mutation failures                             | Check operation results; surface sanitized failures                                    |
| Medium                     | Full Referer logging; secret-bearing `Debug` types                                         | [RequestLog](../../src/routes.rs#L102), [form](../../src/routes.rs#L805), [config](../../src/configuration.rs) | Bearer-link disclosure or accidental credential disclosure                        | Redact secrets at the application boundary and verify proxy logs                       |

### 2) Technical Debt

The Phase 1 scan's production-marker section reported **0 TODO/FIXME/HACK matches**. This is the scanner's result, not a proof of no debt; test gaps are listed in [TESTING.md](TESTING.md).

| Debt item                         | Why it exists / evidence limit                                                                     | Where                                                                     | Risk if ignored                                          | Suggested fix                                                                        |
| --------------------------------- | -------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- | -------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| Mixed handler/presentation module | Current source combines orchestration, logging, HTML, CSS, and scripts; original rationale unknown | [src/routes.rs](../../src/routes.rs), 1,206 indexed lines                 | Safety changes touch a broad file                        | Add behavior tests, then extract only what the safety work needs                     |
| Config/bind mismatch              | Loader has server fields; startup does not explicitly pass them to server                          | [configuration](../../src/configuration.rs), [main](../../src/main.rs)    | Documented override may not control listening address    | Verify effective binding before choosing a framework-specific fix                    |
| Identifier normalization mismatch | Validators trim copies; handler sends originals                                                    | [validation](../../src/validation.rs), [routes](../../src/routes.rs)      | Validation and directory operations see different values | Normalize username/email once; do not transform passwords                            |
| Audit-retention wording           | README says append-only; cleanup deletes eligible events                                           | [README.md](../../README.md), [cleanup](../../src/invite_storage.rs#L218) | Operators may assume permanent audit history             | Document retention precisely; verify backup/retention requirements                   |
| Incomplete schema evidence        | SQL migration exists but is absent from available code index                                       | [0001_invites.sql](../../migrations/0001_invites.sql)                     | Constraints/indexes remain unverified                    | [TODO] Obtain permitted indexed migration content before asserting schema guarantees |

### 3) Security Concerns

| Risk                            | OWASP category, if applicable                    | Evidence                                                                             | Current mitigation                                                               | Gap                                                                                                              |
| ------------------------------- | ------------------------------------------------ | ------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Admin boundary bypass           | Broken access control                            | [Compose](../../docker-compose.yml), [admin handlers](../../src/routes.rs#L709)      | Internal backend with no app host-port publication; authenticated proxy examples | Deployment isolation, app auth, and CSRF protection require verification/implementation                          |
| Directory privilege delegation  | Broken access control                            | [group input](../../src/routes.rs#L722), [client](../../src/lldap.rs#L82)            | Only existing group records are used for membership calls                        | Accepted policy permits any pre-existing group; admin credentials therefore carry broad directory authority      |
| Secret-bearing logs             | Cryptographic failures / sensitive-data exposure | [logging](../../src/routes.rs#L102), [config](../../src/configuration.rs)            | Request path excludes query; proxy examples set `no-referrer`                    | Full Referer still retained; no proven application redaction                                                     |
| Plaintext/default credentials   | Security misconfiguration                        | [defaults](../../src/configuration.rs), [TLS client](../../src/lldap.rs#L324)        | LDAP supports TLS, custom CA, verification enabled by default                    | Plaintext is accepted only on explicitly isolated/firewalled networks; app cannot verify isolation               |
| Local environment-file exposure | Security misconfiguration                        | [.gitignore](../../.gitignore), [deployment guide](../deployment.md)                 | `dev.env` and `config.toml` are ignored                                          | General `.env` is not ignored by this file despite deployment guidance; verify exclusions before writing secrets |
| Unbounded expiry/resource usage | Insecure design                                  | [generate handler](../../src/routes.rs#L722), [directory client](../../src/lldap.rs) | Minimum invite lifetime check; Nginx example rate limits                         | No upper expiry bound in handler; verify body/field bounds, deadlines, concurrency and rate limits               |

Proxy headers are trusted by the application's IP logger without validating the sending peer. Caddy overwrites forwarded-for while the Nginx example appends it. [TODO] Verify the actual trust chain before using logged client IPs for attribution or enforcement. Source: [request_client_ip](../../src/routes.rs#L81), proxy examples.

CI uses floating action tags/toolchains; the builder uses `rust:1-bookworm`, and smoke tests use `lldap/lldap:latest`. Dependency-advisory scanning is not configured in the inspected workflows. These are supply-chain/reproducibility concerns, not evidence of known vulnerable dependencies. See [ci.yml](../../.github/workflows/ci.yml), [Dockerfile](../../Dockerfile), [tests](../../tests/lldap_smoke.rs).

### 4) Performance and Scaling Concerns

| Concern                                        | Evidence                                                                                            | Current symptom                            | Scaling risk                                                   | Suggested improvement                                                               |
| ---------------------------------------------- | --------------------------------------------------------------------------------------------------- | ------------------------------------------ | -------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| Serial directory operations and repeated login | [lldap](../../src/lldap.rs#L244), [submission](../../src/routes.rs#L916)                            | No measured latency evidence               | Added round trips and load with group count/concurrency        | Measure first; reduce redundant auth and bound work without unsafe mutation retries |
| SQLite write contention                        | [BEGIN IMMEDIATE](../../src/invite_storage.rs#L108), [five-connection pool](../../src/app_state.rs) | Concurrency test passes; no load benchmark | Serialized writes may constrain throughput                     | Load-test expected workload before changing storage                                 |
| Consumed-record retention and cleanup work     | [cleanup](../../src/invite_storage.rs#L218), [schedule](../../src/main.rs)                          | No size/cleanup-duration measurements      | Consumed history grows; cleanup deletes other eligible history | Establish retention/backup objectives and measure; indexes remain [TODO]            |
| Unbounded external operation duration          | [client construction/settings](../../src/lldap.rs)                                                  | No timeout failure measurements            | Requests may occupy resources longer than intended             | Define explicit deadlines and concurrency limits after verifying client defaults    |

The scan detected no benchmark/performance-test configuration. Do not describe any of these potential bottlenecks as measured regressions.

### 5) Fragile/High-Churn Areas

Phase 1 scan, September 13, 2026, last 90 days; counts are commits touching each path, not a fragility score:

| Area                                                                                                 | Why it deserves care                   | Churn signal                                 | Safe change strategy                                              |
| ---------------------------------------------------------------------------------------------------- | -------------------------------------- | -------------------------------------------- | ----------------------------------------------------------------- |
| [routes](../../src/routes.rs)                                                                        | Mixes UI, HTTP and side effects        | 6 commits; 1,206 indexed lines               | Focused handler tests before orchestration changes                |
| [README](../../README.md), [TODO](../../TODO.md)                                                     | Intent/evidence can drift              | 6 commits each                               | Compare every claimed guarantee with implementation               |
| [configuration](../../src/configuration.rs), [main](../../src/main.rs), [manifest](../../Cargo.toml) | Runtime wiring and build behavior      | 4 commits each                               | Test configuration precedence and startup behavior                |
| [storage](../../src/invite_storage.rs)                                                               | Transaction/retention boundary         | 3 commits; 565 indexed lines including tests | Preserve concurrency and cleanup regression checks                |
| [lldap](../../src/lldap.rs)                                                                          | External side effects and compensation | 2 commits                                    | Test missing groups, mutation false results, and cleanup failures |

Evidence: local `docs/.codebase-scan.txt`, HIGH-CHURN FILES and GIT RECENT COMMITS. This sample had eleven recent commits; high relative churn alone does not prove instability.

### 6) `[ASK USER]` Questions

1. [ASK USER] Which directory groups may the planned administrator assign? **Resolved September 13, 2026:** the user selected **any pre-existing LLDAP group**, not a configurable allowlist. Group creation is outside that selection. Record and enforce the chosen policy during implementation; missing-group server behavior is still a verification task.

No unresolved intent question remains from this documentation pass. Previously accepted decisions are one built-in admin with a direct-public-exposure warning and strong authenticated-proxy recommendation, plus plaintext directory transport only on explicitly isolated/firewalled networks. See [TODO.md](../../TODO.md). Durable redemption recovery remains proposed design work, not an approved implementation design.

Intent versus reality:

- [Lifecycle docs](../invite-lifecycle.md) put invite validation before uniqueness checks; the handler does the reverse.
- The lifecycle's atomic wording does not distinguish SQLite consumption from non-atomic directory provisioning; a still-usable invite after failure does not guarantee clean directory state.
- [README](../../README.md) says append-only events, but cleanup deletes eligible history.
- [Configuration docs](../configuration.md) describe server bind keys whose effect is not established by startup wiring.
- [Deployment docs](../deployment.md) describe a usable container/proxy stack, but the mounted Caddy upstream is host-local rather than container-aware; `.env` exclusion is also not supplied by the repository ignore file.
- Proxy-owned authentication is the existing documented model. The accepted single built-in admin is future functionality, not evidence that the previous design intended unauthenticated public admin access.

### 7) Evidence

- [src/routes.rs](../../src/routes.rs), [src/lldap.rs](../../src/lldap.rs): side effects, failures, logging and authority.
- [src/invite_storage.rs](../../src/invite_storage.rs), [src/main.rs](../../src/main.rs): transactions, retention and initialization.
- [docker-compose.yml](../../docker-compose.yml), [Caddyfile.example](../caddy/Caddyfile.example), [Nginx example](../nginx/rust-invite-system.conf.example), [.gitignore](../../.gitignore): supplied safeguards and gaps.
- [TODO.md](../../TODO.md), [invite-lifecycle.md](../invite-lifecycle.md), [configuration.md](../configuration.md): intent and accepted requirements.
- Local scan `docs/.codebase-scan.txt`: marker, churn, metrics, security and performance sections.
- September 13, 2026 terminal test result: 21 passed, 2 ignored; no live security reproduction or deployment test performed.
