# Atrius HIS — Domain Services

FHIR-native Hospital Information System **domain layer** (Layer 2). Orchestrates hospital operations—registration, scheduling, ADT, staffing—against [Clinical HFS](https://github.com/HeliosSoftware/hfs) (Layer 1).

Full architecture plan: [atrius-hfs/docs/his/fhir-native-his-plan.md](../atrius-hfs/docs/his/fhir-native-his-plan.md)

## Repository layout

```
atrius-his/
├── crates/
│   ├── his-domain/       # Shared FHIR client, patient builder, platform probes
│   ├── his-registration/ # Patient registration service
│   └── his-server/       # HTTP API (/api/v1/patients, health, ready)
├── deploy/
│   ├── docker-compose.yml          # PostgreSQL, Elasticsearch, Keycloak
│   ├── env/*.env.example           # HFS, HTS, his-server templates
│   └── keycloak/                   # Dev SMART realm + get-token.sh
├── scripts/                        # Platform lifecycle + seed data
└── docs/platform-hardening.md      # Phase 0 checklist
```

## Prerequisites

| Dependency | Purpose |
|------------|---------|
| [atrius-hfs](../atrius-hfs) (sibling clone) | `hfs` and `hts` binaries |
| Docker | PostgreSQL, Elasticsearch, Keycloak |
| Rust 1.90+ | his-server / his-domain |

Set `ATRIUS_HFS_PATH` if your clone is not at `../atrius-hfs`:

```bash
export ATRIUS_HFS_PATH=/path/to/atrius-hfs
```

## Quick start — Phase 0 platform hardening

### 1. Start infrastructure

```bash
chmod +x scripts/*.sh deploy/keycloak/get-token.sh
./scripts/platform-up.sh
```

### 2. Configure environment

```bash
cp deploy/env/hfs-clinical.env.example deploy/env/hfs-clinical.env
cp deploy/env/hts.env.example deploy/env/hts.env
cp deploy/env/his-server.env.example deploy/env/his-server.env
```

Edit `hfs-clinical.env` if `ATRIUS_HFS_PATH` differs from the default.

### 3. Build platform binaries (in atrius-hfs)

```bash
cd "${ATRIUS_HFS_PATH:-../atrius-hfs}"
cargo build --release -p helios-hfs -p helios-hts
```

### 4. Run services (three terminals)

```bash
# Terminal 1 — terminology
./scripts/run-hts.sh

# Terminal 2 — clinical FHIR store (postgres + ES + auth + strict validation)
./scripts/run-clinical-hfs.sh

# Terminal 3 — HIS domain API
export HIS_FHIR_BEARER_TOKEN=$(./deploy/keycloak/get-token.sh his-backend-client)
./scripts/run-his-server.sh
```

First HFS start against PostgreSQL creates schema automatically.

### 5. Verify

```bash
export HIS_FHIR_BEARER_TOKEN=$(./deploy/keycloak/get-token.sh his-backend-client)
./scripts/smoke-platform.sh
```

### 6. Seed foundation data

```bash
export HIS_FHIR_BEARER_TOKEN=$(./deploy/keycloak/get-token.sh his-backend-client)
python3 scripts/seed-hospital-foundation.py --tenant atrius-hospital --token "$HIS_FHIR_BEARER_TOKEN"
```

## Phase 1 — Patient registration

With platform services running and `his-server` started:

```bash
export HIS_FHIR_BEARER_TOKEN=$(./deploy/keycloak/get-token.sh his-backend-client)
./scripts/smoke-registration.sh
```

### API (`/api/v1`)

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/patients` | Register patient (assigns MRN, creates `atrius-in-patient`) |
| `GET` | `/patients/{id}` | Read patient from Clinical HFS |
| `POST` | `/patients/check-duplicates` | Duplicate check by name + birth date |

**Register example:**

```bash
curl -s -X POST http://127.0.0.1:8096/api/v1/patients \
  -H "Content-Type: application/json" \
  -d '{
    "family_name": "Sharma",
    "given_names": ["Priya"],
    "gender": "female",
    "birth_date": "1965-06-15"
  }' | python3 -m json.tool
```

Set `"allow_duplicates": true` to register despite duplicate warnings (default is `false` → HTTP 409).

### Profile validation note

Registration builds **profile-compliant** `atrius-in-patient` resources (identifier with MR type, name, gender). If Clinical HFS has `HFS_PROFILE_VALIDATION_MODE=strict`, writes are validated against the manifest from `./scripts/build-atrius-profile-manifest.sh` in atrius-hfs. Terminology binding checks also require HTS imports — if validation fails, try `warn` mode first while bringing HTS online, or run `$validate` manually:

```bash
curl -s -X POST http://127.0.0.1:8082/Patient/\$validate \
  -H "Authorization: Bearer $HIS_FHIR_BEARER_TOKEN" \
  -H "X-Tenant-ID: atrius-hospital" \
  -H "Content-Type: application/fhir+json" \
  -d @patient.json
```

## Port map (local dev)

| Service | Port | Notes |
|---------|------|-------|
| Clinical HFS | 8082 | Authoritative FHIR store |
| HTS | 9091 | Terminology |
| his-server | 8096 | Domain API |
| Keycloak | 8180 | SMART dev (`admin` / `admin`) |
| PostgreSQL | 5432 | `atrius` / `atrius` / `hfs_clinical` |
| Elasticsearch | 9200 | Search index |

## Next implementation phases

| Phase | Crate / module | Status |
|-------|----------------|--------|
| 0 | Platform hardening | Done (smoke test) |
| 1 | `his-registration` | **Done** — register, read, duplicate check |
| 2 | `his-scheduling` | Planned |
| 3 | `his-adt` | Planned |
| 4 | `his-staffing` | Planned |

See [docs/platform-hardening.md](./docs/platform-hardening.md) for the Phase 0 checklist.

## Related repos

| Repo | Role |
|------|------|
| atrius-hfs | FHIR platform (HFS, HTS, validation, CDS) |
| AtriusIGDraft | Profile / IG authoring |
| atrius-bff | SMART gateway + prefetch |
| atrius-clinical-ui | Clinician UI |
