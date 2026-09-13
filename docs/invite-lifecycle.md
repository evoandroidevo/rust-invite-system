# Invite lifecycle and security assumptions

## Lifecycle

1. An administrator creates an invite from the admin dashboard.
2. The app stores a hashed invite token, the selected groups, the creation time, and the expiry time.
3. The administrator shares the invite URL with the recipient.
4. The recipient opens the invite link and submits username, email, name, and password.
5. The app validates the invite, checks the requested username and email, provisions the account in LLDAP, and consumes the invite atomically.
6. If provisioning fails, the invite remains usable until it expires or is revoked.

## Security assumptions

- Invite tokens are treated as bearer secrets and are never stored in plaintext.
- The invite URL must be delivered over a trusted channel.
- Admin routes are expected to be protected by the reverse proxy.
- LDAP traffic should stay on a private network or use TLS with a trusted certificate.
- LLDAP credentials should be injected from the environment or another secret store, not committed to the repository.
- The app does not currently issue session cookies; browser hardening becomes relevant only if session state is added later.
