# his-server

HTTP API for Atrius HIS domain services (registration, scheduling, ADT, documentation, orders, foundation). Sits between **atrius-bff** and **Clinical HFS**.

Full platform context: [docs/AUTH_ARCHITECTURE.md](../../../atrius-bff/docs/AUTH_ARCHITECTURE.md).

---

## Role in the stack

```
Clinical / Admin SPA  →  atrius-bff  →  his-server  →  Clinical HFS
                         (session)      (domain API)    (FHIR REST)
```

- **BFF** holds OAuth tokens and forwards `Authorization: Bearer <user access token>` on `/bff/his/*`.
- **his-server** optionally validates that JWT (**helios-auth**), then uses the **same bearer** for FHIR writes via `his-domain::FhirClient`.
- **HFS** validates the bearer again and enforces SMART v2 scopes on FHIR operations.

The BFF never sends `his-backend-client` tokens for interactive UI traffic (except dev smoke fallback when `enable_internal_launch=true` and no session).

---

## Endpoints

| Path | Auth required when `HIS_AUTH_ENABLED` |
|------|---------------------------------------|
| `GET /health` | No |
| `GET /ready` | No |
| `/api/v1/*` | Yes — `Authorization: Bearer` |

API surface is mounted at `/api/v1` (see `src/routes/`).

Examples:

- `POST /api/v1/encounters/start-visit`
- `POST /api/v1/consultation-notes`
- `GET /api/v1/patients/{id}`

BFF proxies these as `/bff/his/encounters/start-visit`, etc.

---

## Authentication architecture

### Inbound (browser → BFF → HIS)

When `HIS_AUTH_ENABLED=true`:

1. Axum middleware (`src/auth.rs`) runs on `/api/v1/*`.
2. Extracts `Authorization: Bearer <token>`.
3. Validates via **helios-auth** `JwksBearerAuthProvider` (JWKS, issuer, signature, JTI).
4. Builds `RequestAuth` extension: bearer string, tenant id, optional `Principal`.
5. Route handlers use `RequestAuth` extractor (`src/request_auth.rs`).

Exempt paths: `/health`, `/ready`.

When `HIS_AUTH_ENABLED=false` (default dev):

- Middleware not installed.
- `RequestAuth` is built from the incoming `Authorization` header (or `HIS_FHIR_BEARER_TOKEN` env fallback).
- Allows gradual rollout: BFF can forward user tokens before HIS validation is turned on.

### Outbound (HIS → HFS)

Each HTTP request gets a **request-scoped** `FhirClient`:

```rust
// src/state.rs — simplified
pub fn services(&self, auth: &RequestAuth) -> RequestServices {
    let fhir = self.fhir_template.clone()
        .with_tenant(auth.tenant_id.clone())
        .with_bearer(auth.bearer_token.clone());
    RequestServices::from_fhir(fhir, ...)
}
```

Domain crates (`his-adt`, `his-documentation`, …) receive this client per request — no static service-account token on the UI path.

See [his-domain README](../his-domain/README.md) for `FhirClient` details.

---

## Configuration

Copy [`deploy/env/his-server.env.example`](../../deploy/env/his-server.env.example).

### Core

| Variable | Default | Description |
|----------|---------|-------------|
| `HIS_SERVER_HOST` | `127.0.0.1` | Bind address |
| `HIS_SERVER_PORT` | `8096` | Listen port |
| `HIS_FHIR_BASE_URL` | `http://127.0.0.1:8082` | Clinical HFS base |
| `HIS_TERMINOLOGY_URL` | `http://127.0.0.1:9091` | HTS (readiness probe) |
| `HIS_DEFAULT_TENANT` | `atrius-hospital` | Default `X-Tenant-ID` when auth off |

### Inbound JWT (`helios-auth`)

| Variable | Fallback | Description |
|----------|----------|-------------|
| `HIS_AUTH_ENABLED` | `HFS_AUTH_ENABLED` | Require bearer on `/api/v1/*` |
| `HIS_AUTH_JWKS_URL` | `HFS_AUTH_JWKS_URL` | Keycloak JWKS |
| `HIS_AUTH_ISSUER` | `HFS_AUTH_ISSUER` | Expected JWT `iss` |
| `HIS_AUTH_AUDIENCE` | `HFS_AUTH_AUDIENCE` | Optional `aud` check |
| `HIS_AUTH_TENANT_CLAIM` | `HFS_AUTH_TENANT_CLAIM` | Default **`organization_id`** |
| `HIS_AUTH_JTI_BACKEND` | `HFS_AUTH_JTI_BACKEND` | `memory` or `disabled` |

Required when enabled: `HIS_AUTH_JWKS_URL`, `HIS_AUTH_ISSUER`.

### Offline / smoke only

| Variable | When to use |
|----------|-------------|
| `HIS_FHIR_BEARER_TOKEN` | curl/smoke scripts when `HIS_AUTH_ENABLED=false`; **not** for BFF UI path |

```bash
export HIS_FHIR_BEARER_TOKEN=$(./deploy/keycloak/get-token.sh his-backend-client)
./scripts/smoke-opd-lifecycle.sh
```

---

## Run

```bash
cd atrius-his
cargo run -p his-server

# With auth (match HFS Keycloak settings)
HIS_AUTH_ENABLED=true \
HIS_AUTH_JWKS_URL=https://localhost:8443/realms/fhir/protocol/openid-connect/certs \
HIS_AUTH_ISSUER=https://localhost:8443/realms/fhir \
HIS_AUTH_TENANT_CLAIM=organization_id \
cargo run -p his-server
```

Readiness (`GET /ready`) probes HFS `/metadata` and HTS availability.

---

## Code layout

```
src/
├── main.rs           # Router, optional auth middleware layer
├── auth.rs           # helios-auth init, JWT middleware
├── request_auth.rs   # RequestAuth extractor + extension
├── state.rs          # AppState, RequestServices (per-request FhirClient)
└── routes/           # adt, scheduling, registration, documentation, …
```

Dependency: `helios-auth` from `atrius-hfs/crates/auth` (path dependency in `Cargo.toml`).

---

## What HIS does *not* do

- **No SMART scope enforcement on REST paths** — helios-auth **authenticates** only. HIS domain authorization (future) would use claims like `roles` / `fhirUser`, not FHIR CRUDS.
- **No OAuth token issuance** — Keycloak only.
- **No duplicate JWKS in BFF** — validation happens here and again at HFS for FHIR writes.

Application-level permission checks (`encounter:start`, `consultation_note:write`, patient access) remain in **atrius-bff Postgres** before the request reaches his-server.

---

## Troubleshooting

| Symptom | Check |
|---------|-------|
| 401 from his-server | `HIS_AUTH_ENABLED=true` but missing/invalid bearer from BFF |
| 502 fhir_error from HIS | HFS down, or HFS auth rejected bearer (scope/expiry) |
| Smoke works, UI fails | UI should use user token; smoke may use `HIS_FHIR_BEARER_TOKEN` |
| Wrong tenant in HFS | `organization_id` claim or `X-Tenant-ID` from BFF |

---

## Related

- [deploy/keycloak/README.md](../../deploy/keycloak/README.md)
- [docs/platform-hardening.md](../../docs/platform-hardening.md)
- [helios-auth README](../../../atrius-hfs/crates/auth/README.md)
