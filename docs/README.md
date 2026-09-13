# Proxy examples

The application is intended to listen on localhost and sit behind a reverse proxy.

The files in this directory show two deployment patterns:

- [docs/caddy/Caddyfile.example](docs/caddy/Caddyfile.example)
- [docs/nginx/rust-invite-system.conf.example](docs/nginx/rust-invite-system.conf.example)

Both examples terminate TLS at the proxy, protect `/admin` with authentication, and forward the usual client headers to the app.

The Nginx example also shows request rate limiting for the invite and admin-generation paths.

The app does not currently issue session cookies, so cookie hardening becomes relevant only if you add that layer later.
