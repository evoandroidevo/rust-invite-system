# Operations checklist

## Rotating credentials

- Update the LLDAP bind password in your secret store.
- Update the GHCR publish credentials if your registry setup changes.
- Recreate the app container so it reads the new secret values.
- Verify that invite creation and provisioning still succeed.

## Backing up data

- Back up the SQLite database volume regularly.
- Back up `config.toml` or its orchestrator-managed equivalent.
- Back up any proxy certificates and reverse-proxy config files.

## Upgrades

- Pull or build the new application image.
- Recreate the app container.
- Confirm migrations complete successfully.
- Check that the admin dashboard and invite flow still work.

## Recovery

- Restore the SQLite volume snapshot.
- Restore the config and certificate files.
- Restart the reverse proxy and then the app.
- Verify the app starts cleanly and the admin dashboard loads.
