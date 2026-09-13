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

Reverse-proxy examples for Caddy and Nginx live in [docs/README.md](docs/README.md).

The production container and compose deployment notes live in [docs/deployment.md](docs/deployment.md).

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