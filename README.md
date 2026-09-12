# rust-invite-system

## Configuration

Copy `config.example.toml` to `config.toml` for local overrides. The file is optional; defaults are used when it is absent.

Environment variables override file values using the `APP__` prefix and double underscores for nesting:

```sh
APP_SERVER__PORT=9090 cargo run
```

The default database URL is `sqlite://./data/invites.db`. The application creates the `data/` directory, runs migrations during startup, and stores invite records plus append-only invite events from `migrations/0001_invites.sql`.

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