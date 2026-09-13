# LLDAP bootstrap data

This directory is mounted into the development LLDAP compose stack and seeded on startup.

The compose file at [docker-compose.lldap-dev.yml](docker-compose.lldap-dev.yml) runs the official `lldap/lldap:stable` image and then executes `/app/bootstrap.sh` against the running server.

Seeded demo groups:

- `engineering`
- `design`
- `support`

Seeded demo users:

- `alice@example.com`
- `bob@example.com`
- `carol@example.com`
