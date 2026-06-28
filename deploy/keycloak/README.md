# Keycloak — local dev (`https://localhost:8443`)

Atrius uses a **local Keycloak** server bound to `https://0.0.0.0:8443`, persisting to Postgres database **`keycloak_db`** on port `5432`.

Realm export / reference: [`realm.json`](./realm.json) (import or sync clients/users when the realm changes).

## Prerequisites

- Local Keycloak running on **https://localhost:8443**
- `fhir` realm present (check: `curl -sk https://localhost:8443/realms/fhir/.well-known/openid-configuration`)
- BFF uses default [`atrius-bff/config/dev.toml`](../../atrius-bff/config/dev.toml) — no `BFF_CONFIG` override needed

## Get a service token

```bash
export HIS_FHIR_BEARER_TOKEN=$(./deploy/keycloak/get-token.sh his-backend-client)
```

Defaults: `KEYCLOAK_URL=https://localhost:8443`, `curl -k` for self-signed TLS.

## Demo users

| User | Password | Realm role |
|------|----------|------------|
| `frontdesk.demo` | `demo` | `front_desk` |
| `dr.demo` | `demo` | `doctor` → `Practitioner/dr-patel` |
| `dr.sharma` | `demo` | `doctor` → `Practitioner/dr-sharma` |

Each clinician user needs Keycloak attribute **`fhirUser`** = `Practitioner/{id}` matching the FHIR Practitioner resource id (see seed script). The clinical UI loads that doctor's appointments after sign-in.

**Front desk / admin users** (`frontdesk.demo`, `admin.demo`) do not have `fhirUser` set in [`realm.json`](./realm.json) — the mapper is present but Keycloak **omits empty claims**, so the access token will not include `fhirUser` until you set the user attribute (e.g. `Practitioner/frontdesk` if you create that resource). Audit identity uses OIDC `sub` instead.

The **`fhirUser` client scope** in the token `scope` string (SMART authorization) is separate from the **`fhirUser` JWT claim** — you need both a client **protocol mapper** (Add to access token) and a **user attribute** value.

## OAuth clients (must exist in `fhir` realm)

| Client | Secret | Used by |
|--------|--------|---------|
| `atrius-clinical-bff` | `atrius-clinical-bff-secret` | Clinical SMART launch, BFF OAuth |
| `atrius-admin-bff` | `atrius-admin-bff-secret` | Staff OIDC login via `GET /bff/login` (PKCE); dev password grant when `enable_internal_launch=true` |
| `his-backend-client` | `his-backend-secret` | his-server / BFF HIS proxy |
| `hfs-backend-client` | `hfs-backend-secret` | HFS service account |

After editing `realm.json`, apply changes via Keycloak admin import or admin API. If `atrius-clinical-bff` is missing:

```bash
./deploy/keycloak/bootstrap-clinical-bff.sh
```

If staff **Sign in with hospital account** fails (OAuth / scope errors), patch the admin BFF client on a running Keycloak:

```bash
./deploy/keycloak/bootstrap-admin-bff.sh
```

Then restart **atrius-bff** after config changes.

### Access token must include `sub` (HFS audit / auth identity)

HFS and HIS validate the **user access token** forwarded by the BFF — not the id_token. Keycloak puts `sub` on the id_token by default, but **does not copy it to the access token** unless a client protocol mapper sets `access.token.claim=true`.

Symptom: audit events with empty `agent.who`, or `sub` missing in `/bff/dev/tokens` access_token claims while id_token has `sub`.

**Standard token exchange** does not fix this; add the **sub** mapper instead:

```bash
./deploy/keycloak/bootstrap-bff-scopes-logout.sh
```

Or in Keycloak Admin → `atrius-clinical-bff` / `atrius-admin-bff` → Client scopes → **sub** mapper → enable **Add to access token**.

After patching, log out and sign in again. Access token should include `"sub": "<user-uuid>"`.

### Front-channel logout (RP-initiated)

On each BFF client (`atrius-admin-bff`, `atrius-clinical-bff`):

1. Enable **Front channel logout**
2. Add **Valid post logout redirect URIs** (must match exactly what BFF sends):
   - `http://localhost:5174/login` (admin SPA)
   - `http://localhost:5173/login` (clinical SPA)
   - Or wildcards: `http://localhost:5174/*`, `http://localhost:5173/*`

Sign out in the SPA → BFF clears the session → browser visits Keycloak end-session URL → redirect back to the correct SPA login page.

## HFS SMART auth env

Point HFS at the same Keycloak (see `deploy/env/hfs-clinical.env.example`):

```bash
HFS_AUTH_ENABLED=true
HFS_AUTH_JWKS_URL=https://localhost:8443/realms/fhir/protocol/openid-connect/certs
HFS_AUTH_ISSUER=https://localhost:8443/realms/fhir
HFS_AUTH_TENANT_CLAIM=organization_id
HFS_SMART_TOKEN_ENDPOINT=https://localhost:8443/realms/fhir/protocol/openid-connect/token
HFS_SMART_AUTHORIZE_ENDPOINT=https://localhost:8443/realms/fhir/protocol/openid-connect/auth
```

## HIS inbound auth

`his-server` uses the same **helios-auth** crate as HFS. BFF forwards the **user** access token on `/bff/his/*`; HIS validates it and passes the same bearer through to HFS.

```bash
HIS_AUTH_ENABLED=true
HIS_AUTH_JWKS_URL=https://localhost:8443/realms/fhir/protocol/openid-connect/certs
HIS_AUTH_ISSUER=https://localhost:8443/realms/fhir
HIS_AUTH_TENANT_CLAIM=organization_id
```

(`HIS_AUTH_*` falls back to `HFS_AUTH_*` if unset.) Use `his-backend-client` only for seed/smoke scripts — not the clinical UI path.

Clinical staff login (`GET /bff/login?spa=clinical`) uses **`atrius-clinical-bff`** with write scopes (`user/Encounter.cruds`, `user/Composition.cruds`, …). Re-import `realm.json` after scope changes.

## Legacy docker Keycloak (`:8180`)

Optional — disabled by default. The HIS docker-compose Keycloak service uses an **ephemeral in-container DB**, not `keycloak_db`. To start it for isolated experiments only:

```bash
cd deploy && docker compose --profile legacy-docker-keycloak up -d keycloak
```

Do **not** run docker Keycloak and local Keycloak against the same `keycloak_db`.
