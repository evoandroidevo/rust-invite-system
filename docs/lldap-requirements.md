# Required LLDAP permissions and attributes

The invite service uses a dedicated LLDAP service account for provisioning. That account needs only the permissions required to:

- authenticate to the LLDAP management API
- create users
- set initial passwords
- add users to groups
- delete a user if invite consumption fails after provisioning

The provisioning flow sends the following user attributes to LLDAP:

- `id`
- `email`
- `displayName`
- `firstName`
- `lastName`
- `avatar` as `null`
- `attributes` as `null`

The app also queries LLDAP for existing users by `id` and `email` before it provisions a new account.
