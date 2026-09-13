# Configuration keys and deployment commands

## File and environment configuration

Copy `config.example.toml` to `config.toml` for local overrides. Every key can also be overridden with `APP__` environment variables.

Examples:

```sh
APP__SERVER__PORT=9090 cargo run
APP__LDAP__HTTP_URL=http://127.0.0.1:17170 cargo run
```

Key groups:

- `database.url` for the SQLite database path
- `server.host` and `server.port` for the bind address
- `ldap.http_url`, `ldap.ldap_url`, `ldap.use_tls`, `ldap.tls_insecure_skip_verify`, `ldap.tls_ca_file`, `ldap.base_dn`, `ldap.username`, and `ldap.password`
- `invites.expiration_hours`
- `appearance.default_theme`
- `password_policy.*`

## Local development commands

Start the local LLDAP stack:

```sh
docker compose -f docker-compose.lldap-dev.yml up -d
```

Run the application:

```sh
cargo run
```

Run tests:

```sh
cargo test
```

Optional Docker smoke test:

```sh
cp dev.env.example dev.env
cargo test --test lldap_smoke -- --ignored --nocapture
```
