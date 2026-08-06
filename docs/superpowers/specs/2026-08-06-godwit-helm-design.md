# Godwit Helm Chart — Design

## Status

Approved by user (all sections). Spec for a single Helm chart deploying the Godwit LLM proxy on Kubernetes.

## Goal

Package the Godwit backend for Kubernetes via a single Helm chart that deploys the three production core components (`api`, `ui`, and `db` via CloudNativePG) from the existing Dockerfiles, with secrets, `config.yaml`, persistent storage, probes, and an optional same-origin Ingress.

## Decision summary (user-confirmed)

1. **Scope:** `api` + `ui` + `db` only. Prometheus/Grafana and the legacy `admin` console are **out of scope** (typically managed by separate operator stacks).
2. **Structure:** one chart `godwit` with three app components — no umbrella/subcharts for the MVS.
3. **Images:** reuse the existing Dockerfiles via `values.image.repository:tag` + `imagePullPolicy`. No CI/script build+push in scope — operators push images themselves (documented).
4. **Postgres:** use the **CloudNativePG (CNPG)** operator — the chart deploys a `Cluster` CRD, not a manual Deployment/PVC. CNPG operator installation is a documented prerequisite (not managed by the chart).
5. **Secrets:** the chart creates the Secrets from `values.secrets` (or auto-generates when empty). An optional BYO pattern is supported.
6. **Access:** an optional, parameterized Ingress exposes the UI on a **single origin** (required for same-origin cookie auth). The `api` stays ClusterIP by default.

## Current state (verified)

- Components in the repo: `api` (binary `godwit`, port 3000, root `Dockerfile`), `ui` (Next.js `output:'standalone'`, `apps/ui/Dockerfile`, port 3000), `db` (`postgres:15-alpine`). Legacy `apps/admin` exists but is out of scope.
- Routing: the UI uses Next.js rewrites (`apps/ui/next.config.js`) — `NEXT_PUBLIC_API_ORIGIN` (default `http://localhost:3000`) is the rewrite destination for `/api/v1/*`, `/health`, `/metrics`, `/v1/utils/*`. It is a **build-time** value (read at next.config.js load, inlined at build; `output:'standalone'` ignores runtime env).
- API endpoints: exposes `/health` and `/health/ready` (port 3000) — suitable for readiness/liveness probes.
- DB bootstrap: `crates/godwit-bin/src/main.rs:36` calls `connect(&config.database.url).await?` then `run_migrations`; `connect` is `PgPool::connect(...).await` (`crates/godwit-db/src/lib.rs:16`) which **fails immediately** (no built-in retry) if Postgres is unavailable at boot. Therefore an `initContainer` that waits for Postgres is **required** before the API starts.
- API config: `config.yaml` (a copy of `config.example.yaml`) is loaded via `CONFIG_PATH`; secrets (JWT_SECRET, CREDENTIAL_ENCRYPTION_KEY, ADMIN_EMAIL/PASSWORD, provider keys, DATABASE_URL) are provided via env vars.
- No existing Helm/K8s assets in the repo.

## Design

### Chart layout

```
deploy/charts/godwit/
├── Chart.yaml                 # apiVersion v2, name godwit, appVersion, description
├── values.yaml                # defaults for api, ui, db, ingress, secrets, resources
├── README.md                  # usage, prereqs (CNPG operator), build-time image note, values table
├── templates/
│   ├── _helpers.tpl           # name/labels/chart.fullname helpers + secret-gen helpers
│   ├── secrets.yaml           # godwit-secrets (env secrets) + godwit-db-secret (basic-auth for CNPG)
│   ├── configmap.yaml         # API config.yaml (values.api.config) — non-secret only
│   ├── deployment-api.yaml
│   ├── deployment-ui.yaml
│   ├── postgresql.yaml        # CNPG Cluster CRD
│   ├── service-api.yaml       # ClusterIP
│   ├── service-ui.yaml        # ClusterIP
│   ├── ingress.yaml           # single-origin UI ingress (optional)
│   └── tests/
│       └── test-connection.yaml
```

### Component: db (CloudNativePG)

Deploy a CNPG `Cluster` CRD instead of a manual Deployment/PVC:

```yaml
# templates/postgresql.yaml
apiVersion: postgresql.cnpg.io/v1
kind: Cluster
metadata:
  name: {{ include "godwit.fullname" . }}-db
spec:
  instances: {{ .Values.db.instances }}
  imageName: {{ .Values.db.image }}
  storage:
    size: {{ .Values.db.storageSize | quote }}
    {{- with .Values.db.storageClass }}
    storageClass: {{ . | quote }}
    {{- end }}
  bootstrap:
    initdb:
      database: {{ .Values.db.database }}
      owner: {{ .Values.db.user }}
      secret:
        name: {{ include "godwit.fullname" . }}-db-secret
```

- **Credentials:** CNPG reads `username`/`password` from a Secret of type `kubernetes.io/basic-auth` referenced by `bootstrap.initdb.secret`. The chart creates `{fullname}-db-secret` with `data.username`/`data.password` (from `values.db.user` + a generated/built password).
- **Service:** CNPG auto-creates `<cluster-name>-rw` (read-write) and `-ro` services. The API connects to `<cluster>-rw:5432`.
- **`values.yaml` defaults:**

```yaml
db:
  enabled: true
  instances: 1
  image: ghcr.io/cloudnative-pg/postgresql:15
  user: godwit
  database: godwit
  storageSize: 8Gi
  storageClass: ""
```

- **Prerequisite:** the CNPG operator must be installed in the cluster (documented in README; not managed by the chart — analogous to a Prometheus stack).
- A `db.external` DSN fallback is noted for future non-CNPG use but is not part of this MVS.

### Component: api

Deployment `{fullname}-api`:
- Image `{values.api.image.repository:tag}`, port 3000, `imagePullPolicy`.
- **initContainer `wait-for-db`** (required — `PgPool::connect().await` fails at boot without retry) using `postgres:15-alpine` and `pg_isready` against the CNPG `-rw` service:

```yaml
initContainers:
  - name: wait-for-db
    image: {{ .Values.db.waitImage | default "postgres:15-alpine" }}
    command: ["/bin/sh", "-c"]
    args:
      - |
        until pg_isready -h {{ include "godwit.fullname" . }}-db-rw -p 5432 -U "$DB_USER" -d "$DB_NAME"; do
          echo "waiting for postgres..."; sleep 2;
        done
    env:
      - name: DB_USER
        valueFrom: { secretKeyRef: { name: {fullname}-db-secret, key: username } }
      - name: DB_NAME
        value: {{ .Values.db.database }}
```

- **ConfigMap `config.yaml`**: generated from `values.api.config` (a YAML blob matching `config.example.yaml` keys), mounted at `/app/config.yaml`, with `CONFIG_PATH=/app/config.yaml`.
- **Env from Secret `{fullname}-secrets` (via `secretKeyRef`, never plaintext in ConfigMap):** `DATABASE_URL` (derived `postgres://{user}:{pass}@{cluster}-rw:5432/{db}`), `JWT_SECRET`, `CREDENTIAL_ENCRYPTION_KEY`, `ADMIN_EMAIL`, `ADMIN_PASSWORD`, `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`.
- **Probes:** readiness + liveness `httpGet { path: /health, port: 3000 }`. (Optionally `/health/ready` for readiness.)
- **Resources:** `requests`/`limits` paramétrés with modest defaults.

### Component: ui

Deployment `{fullname}-ui`:
- Image `{values.ui.image.repository:tag}`, port 3000.
- **Build-time note (critical):** the `NEXT_PUBLIC_API_ORIGIN` rewrite destination is inlined at image build time (`apps/ui/Dockerfile` ARG). In K8s the image must be built with:
  `docker build --build-arg NEXT_PUBLIC_API_ORIGIN=http://{fullname}-api:3000` (the cluster-internal api Service name). This is documented in the README and as a `values.ui.buildArgsNote` comment, because it cannot be changed at runtime by the chart.
  `NEXT_PUBLIC_WS_URL` (metrics WS) is likewise a build-time value pointing at the api Service for completeness and documented; cookie-auth flows all go through the same-origin rewrite.
- **Probes:** readiness + liveness `httpGet { path: /, port: 3000 }`.
- **Resources:** paramétrés.

### Secrets

- `{fullname}-secrets` (opaque): holds the app runtime secrets listed under api.
- `{fullname}-db-secret` (`kubernetes.io/basic-auth`): `username`/`password` for CNPG.
- **Provisioning:** values come from `values.secrets.*`; when a value is empty, the chart generates it with Helm `randAlphaNum`. **Stability mechanism (explicit):** the secret templates use Helm `lookup "v1" "Secret" ...` to read any existing cluster Secret value and reuse it; the random value is only computed for a key that is both empty in values **and** absent from the existing Secret (via a `generateSecret` helper in `_helpers.tpl`). This guarantees `helm upgrade` does not rotate already-created secrets. Documented in the README.

### Ingress

- Parametrized Ingress exposing **only** the `ui` Service on one origin:

```yaml
ingress:
  enabled: true
  className: ""          # e.g. nginx
  annotations: {}
  hosts:
    - host: godwit.example.com
      paths: ["/"]
  tls:
    - secretName: godwit-tls
      hosts: [godwit.example.com]
```

- The `api` remains ClusterIP (not exposed to the browser) to preserve the same-origin cookie-auth security property (a documented migration from the docker-compose host-port-8000 exposure). A future `api.expose` toggle is out of MVS scope.

### Validation

- `helm lint deploy/charts/godwit`
- `helm template deploy/charts/godwit -f minimal-values.yaml` — render check (no cluster needed).
- `kubectl apply --dry-run=client -f <rendered>` if a kubectl/context is available; otherwise render-only.
- Real deployment is a documented manual operator step (prereqs: CNPG operator + image push). No live cluster in the dev environment is assumed.

## Out of scope

- Prometheus, Grafana, and the legacy `admin` console charts (managed separately).
- CI/script to build+push images.
- NetworkPolicies, ServiceAccounts/RBAC, autoscaling, backups/DR for CNPG (default CNPG storage only).
- Cross-cluster/multi-tenant values packaging.
- DB `external` non-CNPG mode (documented as future).
