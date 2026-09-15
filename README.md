# rust-invite-system

## Configuration

Copy `config.example.toml` to `config.toml` for local overrides. The file is optional; defaults are used when it is absent.

Environment variables override file values using the `APP__` prefix and double underscores for nesting:

```sh
APP__SERVER__PORT=9090 cargo run
```

The default database URL is `sqlite://./data/invites.db`. The application creates the `data/` directory, runs migrations during startup, and stores invite records plus append-only invite events from `migrations/0001_invites.sql`.

## Local LLDAP compose stack

For a reproducible local identity directory with demo users and groups, start the bundled compose stack:

```sh
docker compose -f docker-compose.lldap-dev.yml up -d
```

That stack exposes LLDAP on `127.0.0.1:17170` for the web UI and `127.0.0.1:3890` for LDAP, then runs the official bootstrap script against the live server to seed:

- Groups: `engineering`, `design`, `support`
- Users: `alice@example.com`, `bob@example.com`, `carol@example.com`

When running the app locally, you can keep the default LLDAP settings or set them explicitly:

```sh
APP__LDAP__HTTP_URL=http://127.0.0.1:17170 \
APP__LDAP__LDAP_URL=ldap://127.0.0.1:3890 \
APP__LDAP__USE_TLS=false \
APP__LDAP__TLS_INSECURE_SKIP_VERIFY=false \
APP__LDAP__TLS_CA_FILE=/path/to/custom-ca.pem \
APP__LDAP__BASE_DN=dc=example,dc=com \
cargo run
```

Set `APP__LDAP__USE_TLS=true` only when your LDAP endpoint is available over TLS, for example with an `ldaps://` URL.
If your LDAP server uses a private or self-signed certificate, set `APP__LDAP__TLS_CA_FILE` to the CA PEM file path. For local testing you can also set `APP__LDAP__TLS_INSECURE_SKIP_VERIFY=true`, but that should stay off for real deployments.

Use a dedicated LLDAP service account for provisioning instead of an all-powerful admin account. The account only needs the permissions required to create users, set passwords, and manage group membership for invited users. For local development, keep LDAP traffic on the loopback/private network; for remote LDAP endpoints, enable TLS and trust the server certificate explicitly.

## Deployment Boundary

Direct public exposure of the application is not safe. Built-in admin
authentication alone does not make direct public exposure safe. Use a trusted
reverse proxy with an additional authentication solution, and prevent clients
from reaching the backend directly.

The [Compose deployment](docker-compose.yml) publishes only the proxy ports;
the app has no published host port and shares an internal backend network with
the proxy. The mounted [Caddy example](docs/caddy/Caddyfile.example) protects
`/admin*` with HTTP Basic authentication. Configure the hostname, certificate
files, and `CADDY_ADMIN_PASSWORD_HASH` before deployment. Verify unauthenticated
admin requests are rejected and the backend is unreachable from untrusted
networks; the configuration alone is not proof of isolation.

Compose sets `APP_UPSTREAM=app:8080` in the proxy container. When using the same
Caddy example on the host, its default upstream remains `127.0.0.1:8080`.
The directory must be reachable from the app's internal network; external LLDAP
connectivity and the complete deployment workflow still require runtime checks.

Local `.env`, `dev.env`, and `config.toml` files are ignored by Git. Keep real
credentials out of tracked files. An ignore rule does not protect secrets that
have already been committed.

## Built-In Admin

There is exactly one application account, named `admin`, separate from the
LLDAP service account and the reverse proxy's credentials. No usable password
is supplied. Without an admin password hash, application admin access remains
disabled; public invitation redemption is still available.

Generate an Argon2id hash locally using hidden password prompts:

```sh
cargo run -- --hash-admin-password
```

The command requires a password of 16 to 1024 bytes, asks for confirmation,
and prints only the resulting hash to stdout. It does not initialize the
database or start the server. Passwords are not trimmed. The `argon2` crate
provides hashing and verification, `rpassword` provides hidden terminal input,
and `subtle` provides constant-time token comparison.

Store the hash in ignored `config.toml` as `admin.password_hash`, or supply
`APP__ADMIN__PASSWORD_HASH` through your service's protected environment. Set
`admin.origin` or `APP__ADMIN__ORIGIN` to the exact browser-facing HTTPS origin,
such as `https://invite.example.com`, with no trailing slash or path. Nonempty
invalid hashes or non-HTTPS origins prevent startup. Accepted hashes use
Argon2id v19, 19,456 to 65,536 KiB memory, two to five iterations, one to four
lanes, a salt of at least 16 bytes, and a 32-byte output. The generator uses
19,456 KiB, two iterations, and one lane.

For Compose, supply `APP_ADMIN_PASSWORD_HASH` and `APP_ADMIN_ORIGIN`; these are
mapped to the application's environment. In a local Compose `.env` file,
single-quote the hash value so its `$` characters are preserved. This hash is
not interchangeable with `CADDY_ADMIN_PASSWORD_HASH`, which protects the proxy.

Open `/admin/login` through the HTTPS reverse proxy. Preserve the original
Host and Origin headers, and leave proxy authentication enabled on `/admin*`,
including the login page. The login page requires no application session.
Admin reads return 401 without a valid session; generation, revocation, and
logout also require a session-bound CSRF token and the configured Origin.
Forwarded user or IP headers do not authorize application admin access.

The session cookie is host-only, Secure, HttpOnly, SameSite=Strict, and expires
after eight hours. Admin pages and generated invitation responses use no-store
and no-referrer headers. Only one session is active: a new login invalidates
the previous session, and logout invalidates it on the server. Sessions and the
global five-attempts-per-minute login throttle are in memory and reset on
restart; run one application instance. Password verification is limited to one
concurrent operation. The throttle can temporarily block legitimate login, so
keep the additional proxy protection in place.

To rotate or recover the admin password, generate a new hash, replace the
configured hash, and restart the application. Restarting invalidates all
sessions. There is no public account-registration or password-reset endpoint.

## Versioning

The application uses semantic versioning from `Cargo.toml` as the source of truth.

- App version: `x.y.z` from `CARGO_PKG_VERSION`
- Git release tags: `vX.Y.Z`
- GHCR image tags: `vX.Y.Z`, `X.Y.Z`, `latest` on the default branch, and commit SHA tags from CI

GitHub Actions also publishes a GitHub Release for each `vX.Y.Z` tag push with autogenerated release notes.

## LLDAP smoke test

The ignored Docker integration test starts `lldap/lldap:latest` with the values in `dev.env` and waits for its HTTP service.

```sh
cp dev.env.example dev.env
cargo test --test lldap_smoke -- --ignored --nocapture
```

Requirements:

- Rust 1.98 or newer
- Docker running with the current user able to access its socket

The test verifies container startup, GraphQL user creation, LDAP password modification, and cleanup of the temporary user.
