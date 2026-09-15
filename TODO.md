# Safety and Best-Practices TODO

Scope: rust-invite-system
Review date: 2026-09-13
Status: Deployment hardening started; built-in admin authentication implemented.
Fresh verification on 2026-09-13: ordinary Cargo tests passed (21 passed,
2 Docker smoke tests ignored); local compiler is Rust 1.98.1. LLDAP integration,
production deployment checks, and dependency scanning remain unverified.
Local Docker boundary verification on 2026-09-15 is recorded below.

Unchecked items include verification tasks, proposed designs, and regression
requirements, not just confirmed defects. Passing tests do not establish that
the deployment or complete redemption workflow is secure.

## Implementation Progress

- First deployment-hardening pass: Compose now sets Caddy's upstream to
  `app:8080`, with loopback retained as the standalone example default.
  Compose parsing and assertions passed for the upstream, shared internal
  backend network, and absence of app host ports. Caddy 2.8 adaptation passed
  and produced `app:8080`; this does not verify live proxying, certificates,
  authentication enforcement, or directory reachability.
- Added a `.env` ignore rule and a README deployment-boundary warning.
- Added the single built-in `admin` account with hidden-input Argon2id hash
  provisioning, disabled access when no hash is configured, bounded password
  verification, an exact HTTPS origin, and redacted admin configuration output.
- Added one active eight-hour in-memory session, secure host-only cookies,
  logout, and a global five-attempts-per-minute login throttle. Sessions and
  throttling reset on restart; multi-instance deployments are not supported.
- Protected all current admin operations and added CSRF tokens plus origin
  validation to login, generation, revocation, and logout. Core and real-router
  regression tests cover login, cookie flags, unauthorized access, forged proxy
  headers, CSRF rejection, mutations, rotation, expiry, throttling, and logout.
  This does not establish live proxy isolation or LLDAP group correctness.
- Admin implementation verification: 33 ordinary tests passed; two Docker
  LLDAP tests remain ignored. Clippy with all targets/features and warnings
  denied passed, as did the application build and Compose configuration checks.
  Firefox 155 checks passed against a fresh build at 1440x900, 390x844, and
  320x720: login rendering, keyboard focus, required-password validation,
  no horizontal overflow, no page-script errors, no-store/no-referrer headers,
  and unauthenticated admin rejection. Screenshots were visually inspected.
  This used a loopback HTTP preview with no admin credentials; authenticated
  browser sessions and deployed HTTPS/proxy behavior remain unverified.
- `docs/README.md` and `docs/deployment.md`, cited by the historical review
  below, are absent from this checkout. The README now provides the current
  deployment-boundary guidance; historical citations are not fresh evidence.

### Access-Boundary Static Review (2026-09-15)

- Reviewed current README deployment guidance and codebase conventions. Neither
  root `AGENTS.md` nor `.github/copilot-instructions.md` was present at the
  checked paths. Refreshed the source index before reviewing current Compose;
  the earlier cached configuration was stale.
- Compose requires separate application and proxy admin credentials, uses
  `app:8080` as Caddy's upstream, publishes only proxy ports 80/443, and attaches
  both services solely to an internal backend network. This is static evidence,
  not a live bypass test. External directory reachability remains unverified.
  The subsequent live review below corrects the proxy's internal-only attachment.
- Caddy protects `/admin*` with Basic authentication and sets no-referrer.
  Nginx protects `/admin/`, redirects exact `/admin` there, and configures
  request limits for admin paths and invite submission. Both examples require
  deployment-specific hostnames, certificates, and credentials. Neither proxy
  was run in this review; client-address trust and effective limits remain open.
- Dockerfile sets runtime UID 10001, creates an owned `/app/data` directory,
  and installs CA certificates. Its base-image tags are mutable. Actual mounted
  volume permissions, container privileges, and network isolation still need
  runtime verification.
- Directly inspected `migrations/0001_invites.sql` with user permission because
  the index omits SQL. It declares non-null primary keys, unique token hashes,
  an expiry index, an event `(invite_id, id)` index, and an event foreign key
  with `ON DELETE CASCADE`. It has no persistent redemption claim or recovery
  state, and no CHECK constraints for JSON, timestamps, or lifecycle consistency.
  Foreign-key enforcement on runtime connections and direct constraint-rejection
  tests remain unverified; schema declarations alone do not establish these.
- Fresh verification: `cargo test invite_storage::tests` passed all four tests
  covering migration-backed creation/history, revocation, concurrent consumption,
  and stale cleanup. These tests do not cover LLDAP provisioning, deployed proxy
  behavior, backup/restore, or upgrades of an existing production database.

### Live Docker Boundary Review (2026-09-15)

- Built the current Dockerfile successfully and tested the real application with
  Caddy 2.8 on Docker Desktop's Linux engine 29.8.0. A controlled comparison found
  that an internal-only proxy had no active host port mapping; attaching a normal
  bridge immediately activated it. Compose now gives only the proxy a non-internal
  `edge` network. The app remains solely on the internal backend with no host ports.
- Seven deployment-boundary checks passed, including the new opt-in live test.
  Certificate-verified HTTPS challenged unauthenticated and forged-header requests
  to `/admin` and `/admin/login`. Valid proxy credentials reached the application's
  login form, but did not authorize `/admin` without an application session.
  Login responses retained no-store/no-referrer headers.
- Runtime inspection confirmed UID 10001, no app host port mappings, and a single
  internal bridge for the app. The same HTTP probe reached the running app from
  its trusted backend and failed to connect from a separate untrusted network.
- The fixture uses a random loopback HTTPS port, fresh named volumes, throwaway
  credentials/certificates, and no local config or directory server. Temporary
  containers, networks, volumes, and files are cleaned up; image caches remain.
  It does not verify an external machine's access, IPv6, real deployment overrides,
  Nginx, LLDAP reachability, or provisioning. Those acceptance checks remain open.

## Codebase Documentation Progress

- [x] Phase 1: Run scan, read intent documents.
- [x] Phase 2: Investigate each documentation area; mark unavailable evidence.
- [x] Phase 3: Populate all seven documents in `docs/codebase/`.
- [x] Phase 4: Validate required sections and evidence, present divergences,
      and resolve the remaining group-policy question.

Current references: [stack](docs/codebase/STACK.md),
[structure](docs/codebase/STRUCTURE.md),
[architecture](docs/codebase/ARCHITECTURE.md),
[conventions](docs/codebase/CONVENTIONS.md),
[integrations](docs/codebase/INTEGRATIONS.md),
[testing](docs/codebase/TESTING.md), and
[concerns](docs/codebase/CONCERNS.md).
The existing Phase 1 scan is preserved at `docs/.codebase-scan.txt`, ignored by
Git, outside the seven-document directory. Documentation completion does not
close the implementation or runtime verification tasks below.

## Review Gaps and Decisions

- Phase 2 intent review: `README.md`, `docs/README.md`, and
  `docs/deployment.md` describe proxy-owned admin authentication, localhost or
  internal-network app access, a non-root runtime container, and backup/recovery
  procedures. These are documented safeguards, not yet runtime-verified findings.
  The single built-in admin account is a new requirement beyond that design.
- Dockerfile and proxy examples were inspected on 2026-09-13. The runtime image
  defines a non-root user, and the examples define admin authentication and
  no-referrer headers. Compose mounts a Caddy upstream pointing to loopback
  inside the proxy container, not the separate app service. The upstream is now
  corrected and parser-checked; live deployment verification remains open.
- [x] Inspect migration constraints/indexes: directly reviewed the SQL file with
      user permission on 2026-09-15; see the static review above. Deployment behavior
      and direct constraint-rejection tests still require runtime checks.
- [x] Recheck source freshness: refreshed the index on 2026-09-15 before reviewing
      current Compose. This does not establish absence of source markers or make
      unsupported files available through the index.
- Decision (2026-09-13): Support exactly one built-in admin account. Clearly
  document that direct public exposure is not safe and strongly recommend a
  trusted reverse proxy with an additional authentication solution.
- Decision (2026-09-13): The planned admin may assign any pre-existing LLDAP
  group, not only an application allowlist. Invitation provisioning must not
  create missing groups. Validate existence before account-creation side effects.
- [TODO] Verify missing-group behavior against the supported LLDAP version;
  do not assume membership assignment creates a group. The previously reviewed
  application rejects unknown groups locally rather than creating them.
- Decision (2026-09-13): Plaintext directory connections are acceptable on
  explicitly configured isolated networks with proper firewalls. Recommend
  verified TLS elsewhere; network isolation is a deployment responsibility.
- Seven-document workflow completed with evidence gaps explicitly marked
  `[TODO]`; the group-policy question was answered. See the progress section.

## Phase 2 - Investigation Handoff

Investigation date: 2026-09-13. This records the seven documentation areas for
Phase 3; it does not mark security changes implemented or Phase 4 complete.
This is the historical Phase 2 snapshot; the completed documents linked above
supersede its earlier inventory, compiler, proxy, and container evidence gaps.
The Phase 1 scan script ran on 2026-09-13 under the updated instruction's
explicit permission exception. It completed the file inventory, recent-history
ranking, and production marker scan. Remaining deployment-file inspection is
still open; scan completion alone does not close those verification tasks.
Indexed reads are used for source, and Markdown intent documents are read under
the explicit exception.

### Stack and Structure

- [Cargo.toml](Cargo.toml) declares one package, version 0.1.0, Rust edition 2024,
  using Cargo. The [README](README.md) requires Rust 1.98 or newer for smoke tests;
  the manifest has no `rust-version`. Actual compiler and resolved dependency
  versions remain [TODO]; manifest versions are dependency requirements.
- Runtime dependencies: config, serde, sha2, sqlx, chrono, rand, reqwest, ldap3,
  serde_json, uuid, topcoat, topcoat-asset, rustls, rustls-pemfile, and tokio.
  Development dependencies: dotenvy, reqwest, and testcontainers. SQLx enables
  SQLite/migrations; the HTTP and LDAP clients enable rustls-related features.
- Source lives in `src/`, with startup in [src/main.rs](src/main.rs), application
  context in [src/app_state.rs](src/app_state.rs), and modules for routes, storage,
  directory integration, configuration, validation, views, and versioning.
  Indexed supporting areas are `tests/`, `.github/workflows/`, and
  `lldap-bootstrap/`. Documentation lives in `docs/`.
- [TODO] Complete the non-indexed file inventory, inspect migrations and the
  container base image, and confirm repository-local guidance.

### Architecture and Conventions

- [Startup](src/main.rs) bundles assets, loads configuration, creates `data/`,
  initializes shared state and migrations, starts cleanup, discovers routes,
  attaches application context, and starts Topcoat.
- [AppState](src/app_state.rs) holds configuration, an invite repository backed
  by a SQLite pool with at most five connections, and an LLDAP client.
- [Redemption](src/routes.rs) validates form fields, checks directory uniqueness,
  validates the stored invite, provisions the account, then consumes the invite.
  Failed consumption attempts account deletion and ignores deletion errors.
- [Storage](src/invite_storage.rs) uses parameterized SQL and transactions for
  invite mutations and event insertion. Consumption is atomic within SQLite,
  not across SQLite and LLDAP. Cleanup runs every 24 hours with a 120-day cutoff
  for eligible expired/revoked invites and their events; consumed invites are
  retained by the current cleanup predicate.
- Observed Rust naming uses snake_case files/functions and PascalCase types.
  SQLx errors propagate from storage; the LLDAP client uses `LldapError`; routes
  render user-facing messages and sometimes discard errors. Startup uses
  `expect`/`unwrap`. Request logging emits JSON through stderr, not a dedicated
  tracing layer. These are observations, not endorsements of every pattern.
- [CI](.github/workflows/ci.yml) enforces formatting and Clippy. [TODO] Confirm
  any additional local formatting/import rules before restructuring source.

### Integrations and Testing

- [LLDAP client](src/lldap.rs) authenticates over HTTP, sends GraphQL requests,
  and uses LDAP password modification. This is an LLDAP-specific management
  integration, not evidence of compatibility with arbitrary LDAP servers.
- [Deployment documentation](docs/deployment.md) describes environment-injected
  secrets, a non-root container, internal networking, GHCR publishing, and
  backup/recovery procedures. [Proxy documentation](docs/README.md) describes
  authenticated Caddy/Nginx examples and Nginx rate limits. Verify actual config
  and deployed behavior before classifying these safeguards as absent or complete.
- [LLDAP requirements](docs/lldap-requirements.md) explicitly includes permission
  to delete users during compensation. [TODO] Verify the least-privilege role
  that supports every required operation on the supported LLDAP version.
- [CI](.github/workflows/ci.yml) runs `cargo fmt --check`, Clippy with warnings
  denied, and `cargo test`. Unit tests are embedded in source; the storage suite
  includes concurrent consumption. No coverage threshold appears in this workflow.
- [Smoke tests](tests/lldap_smoke.rs) are ignored by ordinary test runs, require
  Docker and `dev.env`, and use `lldap/lldap:latest`. The account test calls
  GraphQL creation and LDAP password modification directly; it does not exercise
  the application's complete `provision_user` or redemption flow, group assignment,
  or compensation failures. Its cleanup does not assert the returned `ok` value.
- [TODO] Establish a deterministic fault-injection strategy and execute the full
  workflow tests; the supplied passing ordinary test run is not that evidence.
- [TODO] Verify monitoring, external log collection, and any operational queues;
  no such deployment guarantees can be established from the reviewed docs.

### Concerns and Intent vs. Reality

- [Lifecycle documentation](docs/invite-lifecycle.md) puts invite validation
  before username/email lookups; [the handler](src/routes.rs) performs lookups
  first. Correct the implementation and align the documentation.
- The lifecycle's atomic-consumption wording must distinguish the SQLite
  transaction from the non-atomic provisioning workflow. An unused invite after
  a provisioning error does not guarantee a clean directory or safe retry.
- [Configuration documentation](docs/configuration.md) says `server.host` and
  `server.port` control binding, but [startup](src/main.rs) does not visibly pass
  them to the server. [TODO] Test the effective bind address before declaring the
  environment-variable example functional or choosing a framework-specific fix.
- The [README](README.md) describes append-only invite events, while
  [cleanup](src/invite_storage.rs) deletes events for eligible stale invites.
  Document this retention exception; permanent audit retention is not guaranteed.
- The documented proxy-only authentication model predates the user's decision
  to add one built-in admin account. That is planned functionality, not proof
  that the previous deployment intentionally exposed unauthenticated admin routes.
- [Routes](src/routes.rs) has 1,206 indexed lines mixing HTTP handling, validation
  orchestration, logging, markup, CSS, and scripts. [Storage](src/invite_storage.rs)
  has 565 lines including tests; length alone is not proof of a design defect.
  Avoid a broad refactor unless it supports the scoped safety changes.
- Indexed 90-day git churn samples: `src/routes.rs` has six commits and
  `src/lldap.rs` has two, both first seen on 2026-09-12. These are sampled counts,
  not a full-repository ranking or proof of instability. [TODO] Complete ranking
  and production/test TODO-marker counts with fresh, permitted scan evidence.
- Existing high-priority findings remain: unknown-group cleanup bypass,
  provisioning-before-consumption races, discarded mutation outcomes, pre-invite
  directory enumeration, and full Referer logging. Runtime reproductions and
  fixes remain unchecked in the sections below.

## P0 - Verify the Access Boundary

- [x] Review repository guidance, proxy configuration, Dockerfile, and migrations
      (static review completed 2026-09-15; runtime verification remains separate).
- [x] Implement exactly one built-in admin account with secure credential
      provisioning, password hashing, and no default usable credentials.
- [x] Add secure session handling, logout, and admin login throttling.
- [x] Protect admin pages and mutations with application authentication and
      authorization; keep the login endpoint accessible without a session.
- [x] Document that built-in authentication alone is not safe for direct public
      exposure; strongly recommend a trusted reverse proxy with additional auth.
- [ ] Document and test backend isolation so deployments using proxy auth cannot
      bypass it by reaching the application directly.
      Progress (2026-09-15): README now defines trusted-host/network assumptions
      and live bypass probes. Seven deployment-boundary checks passed, including
      live Caddy HTTPS/authentication and cross-network IPv4 probes; the proxy-only
      edge network fixes the observed host-port publishing failure. CI runs the
      CLI-dependent configuration check; the live Docker test is opt-in.
      This remains open for actual-deployment/external-machine acceptance and
      IPv6 verification where enabled; local fixture success is not that evidence.
- [x] Verify CSRF protection for invite generation and revocation.
- [ ] Enforce the chosen group policy: any pre-existing LLDAP group, with
      existence checked before provisioning and no implicit group creation.
- [x] Test unauthenticated application access, forged proxy identity headers,
      and cross-origin admin submissions.
- [ ] Test deployed proxy bypass and attempts to assign nonexistent groups.

## P1 - Make Redemption Safe and Recoverable

Confirmed in the reviewed source: directory lookups precede invite validation;
lookup errors are treated as not found; provisioning precedes invite consumption;
unknown-group errors can bypass cleanup; mutation result flags are discarded.
SQLite consumption itself is transactional and has a concurrency test.

- [ ] Validate invite format and active state before directory lookups.
- [ ] Fail closed on directory lookup errors instead of treating them as not found.
- [ ] Design exclusive redemption before provisioning; an atomic persistent
      claim is the proposed approach, not an existing requirement from project intent.
- [ ] Implement the chosen claim, completion, and recovery model without holding
      a database transaction open across network calls.
- [ ] Define behavior for concurrent redemption, expiry, and revocation.
- [ ] Test membership assignment to a nonexistent group on the supported LLDAP
      version and record the actual error or side effects. Separately test the
      application's unknown-group handling; it currently fails before sending
      a membership request for that group, but after creating the account.
- [ ] Resolve and authorize all requested groups before creating an account.
- [ ] Check directory mutation results instead of discarding their ok values.
- [ ] Handle every post-creation failure consistently.
- [ ] Persist failed cleanup for reconciliation; never silently abandon it.
- [ ] Reconcile ambiguous outcomes before retrying account creation.
- [ ] Test concurrent submissions, unknown groups, partial membership failures,
      database failures, process interruption, and failed account deletion.
- [ ] Verify one invite cannot leave multiple usable accounts.

## P1 - Protect Secrets and Bound Resource Usage

Confirmed in the reviewed source: request logging retains the full Referer;
secret-bearing types derive Debug; expiration arithmetic has no upper bound.
Debug derives create an accidental-disclosure risk, not proof of a logged password.
Proxy headers, framework defaults, and deployment rate limits remain unverified.

- [ ] Redact invitation codes from application and proxy logs, including Referer.
- [ ] Verify effective no-referrer and no-store policies on sensitive responses;
      configure them where missing.
- [ ] Redact passwords from debug output for configuration and form types.
- [ ] Add explicit HTTP/LDAP deadlines and bounded provisioning concurrency.
- [ ] Verify public submission and admin mutation rate limits; add them where missing.
- [ ] Trust forwarded client addresses only from configured proxies.
- [ ] Bound request bodies, field lengths, group counts, and invite lifetimes.
- [ ] Use checked expiration arithmetic.
- [ ] Test log redaction, oversized input, timeout handling, and rate limits.

## P2 - Configuration and Operational Reliability

Development transport and credential defaults are present; whether a production
deployment uses them is unverified. Validators trim usernames and emails locally,
but the handler passes the original values to directory operations.

- [ ] Reject development credentials in production and validate transport settings
      against the documented deployment mode.
- [x] Ensure local `.env` files are ignored before following deployment guidance.
- [x] Correct the containerized Caddy upstream to address the app service rather
      than proxy-container loopback; validate Compose and Caddy parsing.
- [ ] Verify directory reachability from the internal backend network and
      complete the end-to-end deployment check.
- [ ] Support explicitly configured plaintext HTTP/LDAP on isolated, properly
      firewalled networks; document that this exposes credentials to anyone able
      to observe that traffic and that the app cannot verify firewall isolation.
- [ ] Recommend TLS outside that exception and verify certificates when TLS is used.
- [ ] Normalize usernames and emails once before validation and use.
- [ ] Preserve current password handling: do not trim or silently transform passwords.
- [ ] Reconcile server host/port configuration with actual startup behavior.
- [ ] Report revocation and database failures accurately.
- [ ] Align lifecycle documentation with actual validation order and transaction
      boundaries; document audit-event retention exceptions.
- [ ] Add sanitized operational logs and attributable admin audit events.
- [ ] Verify database permissions, migration behavior, backup, and restore.

## P2 - Automated Verification

Reviewed CI already runs formatting, Clippy, and ordinary tests. Both LLDAP
smoke tests are marked ignored, so an ordinary passing test run does not verify
those integrations. Existing storage concurrency coverage is not end-to-end
provisioning coverage.

- [ ] Preserve existing formatting, Clippy, unit, and storage concurrency checks.
- [ ] Add handler-level tests covering the complete redemption workflow.
- [ ] Exercise `provision_user` itself, group membership, missing-group behavior,
      and compensation outcomes rather than only direct protocol calls.
- [ ] Make LLDAP integration tests independent of a developer's local env file.
- [ ] Pin the integration image and explicitly run ignored smoke tests in CI.
- [ ] Add dependency-advisory scanning and pin CI actions to reviewed commits.
- [ ] Review container privileges, exposed ports, and runtime filesystem access.
- [ ] Run the full verification suite and record remaining deployment assumptions.

## Evidence

- [Project README](README.md), [proxy overview](docs/README.md), and
  [deployment guide](docs/deployment.md): stated deployment intent, proxy auth,
  container isolation, secrets, and backup/recovery procedures.
- [Lifecycle](docs/invite-lifecycle.md), [configuration guide](docs/configuration.md),
  [operations](docs/operations.md), and [LLDAP requirements](docs/lldap-requirements.md):
  guarantees and deployment assumptions compared with source during Phase 2.
- [Manifest](Cargo.toml) and [application state](src/app_state.rs): dependency
  requirements, database initialization, and shared service wiring.
- [Request handlers and logging](src/routes.rs): admin handlers, lookup order,
  provisioning/consumption order, compensation, Referer logging, and expiry input.
- [Directory client](src/lldap.rs): group resolution, cleanup paths, mutation
  result handling, and client construction.
- [Invite storage](src/invite_storage.rs): transactional consumption and storage
  concurrency tests.
- [Configuration](src/configuration.rs) and [startup](src/main.rs): development
  defaults and server configuration wiring to verify.
- [Validation](src/validation.rs): local trimming and current password handling.
- [Compose](docker-compose.yml): proxy service and internal backend network;
  this alone does not verify proxy authorization or effective runtime isolation.
- [CI](.github/workflows/ci.yml) and [LLDAP smoke tests](tests/lldap_smoke.rs):
  current gates and explicitly ignored integration tests.
- Fresh terminal verification on 2026-09-13: ordinary Cargo tests, exit 0,
  21 passed and 2 Docker tests ignored; `rustc --version` reports Rust 1.98.1.
- User decisions on 2026-09-13: one built-in admin account, warning against direct
  public exposure, strong recommendation for authenticated reverse-proxy
  protection, and acceptance of plaintext directory traffic on isolated networks
  with proper firewalls. The admin may assign any pre-existing LLDAP group.
  Missing-group server behavior remains a verification task.
