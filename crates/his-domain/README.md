# his-domain

Shared infrastructure for Atrius HIS crates: configuration, **Clinical HFS HTTP client**, patient builders, platform health probes.

Domain services (`his-adt`, `his-scheduling`, `his-registration`, …) depend on this crate; **his-server** wires them per request with a caller-scoped bearer token.

Auth context: [AUTH_ARCHITECTURE.md](../../../atrius-bff/docs/AUTH_ARCHITECTURE.md) · [his-server README](../his-server/README.md).

---

## `HisConfig`

Loaded from environment via `HisConfig::from_env()`.

| Field / env | Description |
|-------------|-------------|
| `HIS_FHIR_BASE_URL` → `fhir_base_url` | Clinical HFS root (e.g. `http://127.0.0.1:8082`) |
| `HIS_TERMINOLOGY_URL` → `terminology_url` | HTS base for readiness |
| `HIS_DEFAULT_TENANT` → `default_tenant` | Default tenant when inbound auth is off |
| `HIS_FHIR_BEARER_TOKEN` → `fhir_bearer_token` | **Optional static bearer** — smoke/seed scripts only |

```rust
let config = HisConfig::from_env()?;
config.validate()?; // http(s) URLs
```

When **his-server** auth is disabled and no `Authorization` header is present, `RequestAuth` may fall back to `fhir_bearer_token` for local scripts. Interactive UI traffic from the BFF always supplies a user bearer.

---

## `FhirClient`

Thin reqwest wrapper for HFS REST (`src/fhir_client.rs`).

### Construction

```rust
let client = FhirClient::new(&config)?;
```

Uses `config.fhir_bearer_token` if set (static dev/smoke mode).

### Per-request bearer (production path)

his-server clones a template client and overrides bearer + tenant per request:

```rust
let fhir = template
    .clone()
    .with_tenant("atrius-hospital")
    .with_bearer(user_access_token);
```

| Method | Purpose |
|--------|---------|
| `with_tenant(id)` | Sets `X-Tenant-ID` header |
| `with_bearer(token)` | Sets `Authorization: Bearer …` |
| `with_bearer_opt(Option<String>)` | Clear or set bearer |

All GET/POST/PUT calls attach headers from `request_headers()`:

- `X-Tenant-ID` — must match HFS tenant when auth enabled (`organization_id` from JWT)
- `Authorization` — user or service token; HFS validates when `HFS_AUTH_ENABLED=true`

### Operations

| Method | HFS path |
|--------|----------|
| `metadata()` | `GET /metadata` |
| `read_resource(type, id)` | `GET /{type}/{id}` |
| `create_resource(type, body)` | `POST /{type}` |
| `update_resource(type, id, body)` | `PUT /{type}/{id}` |
| `search_resources(type, query)` | `GET /{type}?…` |
| `post_transaction(bundle)` | `POST /` (Bundle) |

Errors surface as `anyhow::Error` with upstream status/body; domain crates map these to `*Error::Fhir`.

---

## Token flow (UI path)

```
Keycloak  →  access token in BFF Redis
           →  BFF POST /bff/his/encounters/start-visit
           →  his-server RequestAuth.bearer_token
           →  FhirClient.with_bearer(...)
           →  HFS POST /Encounter (helios-auth + SMART scopes)
```

The **same** JWT transits BFF → HIS → HFS. Do not configure `HIS_FHIR_BEARER_TOKEN` for this path.

---

## `PlatformProbe`

Used by `his-server` `GET /ready`:

- HFS `GET /metadata` (optionally with static bearer from config)
- HTS reachability

Ensures the domain layer’s upstream dependencies are alive before accepting traffic.

---

## Other modules

| Module | Purpose |
|--------|---------|
| `patient_builder` | Profile-compliant Patient resources for registration |
| `clinical/` | Shared FHIR builders, specs, entry helpers |
| `bundle` | Search bundle helpers |

---

## HFS auth expectations

When `HFS_AUTH_ENABLED=true`, writes from HIS require the user token to carry appropriate SMART scopes, e.g.:

| HIS operation | Typical HFS resources | SMART scope |
|---------------|----------------------|-------------|
| Register patient | `Patient` | `user/Patient.c` |
| Start visit | `Encounter`, `Appointment` | `user/Encounter.c`, `user/Appointment.u` |
| Consultation note | `Composition`, `DocumentReference` | `user/Composition.c` |
| Lab order | `ServiceRequest` | `user/ServiceRequest.c` |

Scopes are issued by Keycloak on **`atrius-clinical-bff`**. See `atrius-his/deploy/keycloak/realm.json`.

Set `HFS_AUTH_TENANT_CLAIM=organization_id` to match Keycloak’s org mapper.

---

## Related

- [helios-auth README](../../../atrius-hfs/crates/auth/README.md) — JWT validation on HFS
- [deploy/env/hfs-clinical.env.example](../../deploy/env/hfs-clinical.env.example)
