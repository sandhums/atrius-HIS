# Platform Hardening — Phase 0

Checklist for production-grade infrastructure before HIS domain modules (registration, scheduling, ADT).

Architecture context: [atrius-hfs FHIR-native HIS plan](../../atrius-hfs/docs/his/fhir-native-his-plan.md)

## Goals

- PostgreSQL + Elasticsearch persistence for Clinical HFS
- SMART JWT auth via Keycloak (dev realm; production IdP later)
- Strict Atrius profile validation on writes
- HTS terminology linked to HFS
- Audit logging enabled
- Foundation seed data (Organization, Location, Practitioner, HealthcareService)
- his-server `/ready` probes HFS + HTS

## Checklist

### Infrastructure

- [ ] `./scripts/platform-up.sh` — postgres, elasticsearch, keycloak healthy
- [ ] Env files copied from `deploy/env/*.env.example`
- [ ] `atrius-hfs` release binaries built (`hfs`, `hts`)

### Clinical HFS

- [ ] `HFS_STORAGE_BACKEND=postgres-elasticsearch`
- [ ] `HFS_PROFILE_VALIDATION_MODE=strict`
- [ ] `HFS_AUTH_ENABLED=true` with Keycloak JWKS
- [ ] `HFS_TERMINOLOGY_SERVER` points at HTS
- [ ] `HFS_AUDIT_BACKEND=file` (or database in production)
- [ ] `./scripts/run-clinical-hfs.sh` starts without error
- [ ] `GET /metadata` returns CapabilityStatement (with bearer token)

### Terminology

- [ ] HTS running on configured port
- [ ] Core code systems imported (SNOMED, LOINC, etc.) — see [data-import.md](../../atrius-hfs/docs/clinical-reasoning/data-import.md)
- [ ] ValueSet binding checks pass during `$validate`

### HIS domain layer

- [ ] `cargo build --release` in this repo
- [ ] `his-server` `/health` and `/ready` return OK when HFS + HTS up
- [ ] `HIS_FHIR_BEARER_TOKEN` obtained via `deploy/keycloak/get-token.sh his-backend-client`

### Seed data

- [ ] `scripts/seed-hospital-foundation.py` transaction succeeds
- [ ] `GET /Organization/atrius-demo-hospital` returns seeded org

### Optional (before Phase 1)

- [ ] HFS subscriptions feature enabled for event-driven workflows
- [ ] AtriusIGDraft profiles for Appointment, EpisodeOfCare, bed Location
- [ ] Expanded manifest: `atrius-r4-profile-manifest-his.json`

## Smoke test

```bash
export HIS_FHIR_BEARER_TOKEN=$(./deploy/keycloak/get-token.sh his-backend-client)
./scripts/smoke-platform.sh
```

## Troubleshooting

| Symptom | Check |
|---------|-------|
| HFS fails on startup (postgres) | `docker compose ps`; connection string in `hfs-clinical.env` |
| HFS 401 on all requests | Token expired; re-run `get-token.sh`; `HFS_AUTH_JWKS_URL` reachable |
| Profile validation 422 | Resource missing `meta.profile`; manifest path correct |
| ES yellow/red | Single-node dev is OK at yellow; increase Docker memory if red |
| his-server /ready false | HFS or HTS not running; wrong URLs in `his-server.env` |

## After Phase 0

Proceed to **Phase 1 — Patient Registration** (`crates/his-registration`): MRN assignment, duplicate detection, BFF routes.
