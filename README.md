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
the proxy. Only the proxy also joins the non-internal `edge` network, which
allows Docker to publish its host ports. An internal-only proxy had no active
published ports in the Docker 29.8.0 live check. The app must not join `edge`.
The mounted [Caddy example](docs/caddy/Caddyfile.example) protects
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

### Backend Isolation Checks

The Compose boundary assumes that the Docker host, Docker administrators, the
proxy, and every container attached to `backend` are trusted. An internal bridge
network is not an authentication boundary against those actors. On Linux, the
host can normally reach container addresses directly even without published
ports. Do not grant untrusted users access to Docker's socket or API, attach
untrusted workloads to `backend`, or route untrusted traffic to its subnet.

Keep the app attached only to the dedicated internal bridge network, with no
published ports (including loopback mappings) and no `network_mode` override.
Review Compose overrides, Docker daemon routing settings, host firewall rules,
and any manually attached networks before deployment. Do not add a public app
network to solve directory connectivity; arrange an explicitly trusted directory
path and verify it separately. For a standalone host deployment, bind the app
to loopback and keep local users/processes trusted, or use an equivalently
restricted private interface. Verify the actual listener, not just configuration.

Run the automated template check from the repository root:

```sh
cargo test --test deployment_boundary
cargo test --test deployment_boundary compose_keeps_backend_private -- --ignored
```

This requires the Docker Compose CLI but no running daemon. It parses normalized
Compose JSON using the existing `serde_json` dependency, supplies dummy credentials
only to the child process, and disables automatic `.env` loading. It does not
start containers or print the rendered configuration. Negative tests reject
published app ports, alternate app networks/namespaces, external or non-internal
backend networks, custom bridge options, and a disconnected or misdirected proxy.
They also require a non-internal proxy edge network. CI explicitly runs the
CLI-dependent test; ordinary `cargo test` skips the CLI and live Docker tests.
The check covers the tracked Compose file, not deployment overrides or live
firewall behavior. Do not publish rendered Compose output: it can contain secrets.

To run the live regression test, build the current application image and supply
its tag through `BOUNDARY_APP_IMAGE`. OpenSSL must be on PATH, or its executable
path must be set in `BOUNDARY_OPENSSL`. For example, in PowerShell with Git for
Windows installed:

```powershell
docker build -t rust-invite-system-boundary-test .
if ($LASTEXITCODE -ne 0) { throw 'Application image build failed' }
$env:BOUNDARY_APP_IMAGE = 'rust-invite-system-boundary-test'
$env:BOUNDARY_OPENSSL = 'C:\Program Files\Git\usr\bin\openssl.exe'
cargo test --test deployment_boundary live_proxy_and_backend_isolation -- --ignored --nocapture
```

On other platforms, export the same image variable and use the local OpenSSL
executable. Rebuild the image after application changes; the test does not build
or check source freshness itself. Docker must be running and able to obtain
`caddy:2.8` and `curlimages/curl:8.12.1`.

The live fixture derives its configuration from the tracked Compose file, keeps
the mounted Caddy example, and uses an isolated project with fresh volumes,
throwaway credentials, and a one-day test certificate. It omits the local
`config.toml` mount, publishes HTTPS on a random loopback-only port, and does
not start a directory server. Certificate and hostname verification stay enabled.
It checks proxy-auth challenges, forged-header rejection, application auth,
no-store/no-referrer headers, runtime UID, actual app port mappings, and IPv4
access from trusted versus untrusted container networks. Containers, networks,
volumes, and temporary files are removed on success or ordinary failure. After
forced process termination, inspect and remove only that run's `boundary-*`
project resources; do not run a global prune. Downloaded/built images remain cached.
The live test is opt-in and is not currently run by CI.

Before approving a running deployment, use the same Compose project and override
arguments used to start it and record these checks:

1. Inspect the app container's `HostConfig.PortBindings` and
   `NetworkSettings.Ports`: there must be no published host mapping. An exposed
   container port with a null mapping is not a published port. Confirm its only
   network is the intended backend; inspect that network for `Internal: true`,
   bridge driver, expected options, and only trusted members. Inspect actual
   running state, not just `docker compose config`.
2. From an untrusted machine, request every deployment host address on port 8080
   and every directly routable backend address. Test IPv4 and IPv6 where enabled.
   For example, use `curl --noproxy '*' --connect-timeout 3 --max-time 5
http://DEPLOYMENT_HOST:8080/admin/login` (on one line). Repeat from an
   untrusted container on a different network. No HTTP response is acceptable:
   even an application 401 proves the reverse proxy can be bypassed. Distinguish
   expected refusal/timeout from DNS, test-client, or unrelated routing failures.
3. Through the public HTTPS hostname, GET `/admin` and `/admin/login` without
   credentials, following redirects. Both must end in a proxy-auth challenge
   (401 with the configured Basic-auth examples). Repeat with forged
   `X-Forwarded-User: admin`, `Remote-User: admin`, and
   `X-Forwarded-For: 127.0.0.1` headers; these must not bypass proxy authentication.
4. With valid proxy credentials but no application cookie, GET `/admin/login`
   must render the login form, while `/admin` must still require application
   authentication. Use a browser or an interactive password prompt, not a
   password in shell history. This positive control distinguishes a working
   authentication boundary from a stopped app, bad upstream, or broken TLS.
5. Verify directory reachability from the app separately. Repeat these checks
   after changes to networking, proxy configuration, Docker, or firewall rules.

Do not disable certificate verification for HTTPS probes. Record deployment
versions, probe locations, destination addresses, results, and remaining
assumptions without credentials, cookies, or invitation codes. A local Compose
configuration pass does not close this deployment acceptance procedure.

## Built-In Admin

There is exactly one application account, named `admin` by default, separate from the
LLDAP service account and the reverse proxy's credentials. No usable password
is supplied. Without an admin password hash, application admin access remains
disabled; public invitation redemption is still available.

Set `admin.username` in `config.toml`, or override it with
`APP__ADMIN__USERNAME`. For example:

```toml
[admin]
username = "operator"
```

Usernames are case-sensitive and are not trimmed. Configured names must contain
1 to 64 bytes, with no surrounding whitespace or control characters. Restart
the application after changing the name; only the configured name is accepted.
For Compose, set it in the mounted `config.toml`, or explicitly pass
`APP__ADMIN__USERNAME` to the app container's environment.

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
