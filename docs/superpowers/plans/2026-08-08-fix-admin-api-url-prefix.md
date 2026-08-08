# Corriger le préfixe des routes API admin de l'UI

> **Pour les workers agentiques :** SOUS-SKILL REQUIS : utilisez `superpowers:subagent-driven-development` (recommandé) ou `superpowers:executing-plans` pour implémenter ce plan tâche par tâche. Les étapes utilisent la syntaxe de case à cocher (`- [ ]`) pour le suivi.

**Objectif :** Rendre le flux "déclarer un modèle" (et toute la couche de données admin) fonctionnel en corrigeant le préfixe d'URL manquant (`/api/v1`) dans les libs de l'UI `apps/ui`.

**Architecture :** Le backend Rust monte le routeur admin sous `/api/v1` (`.nest("/api/v1", admin::router(...))` dans `main.rs:137`). Le rewrite Next (`next.config.js`) ne mappe que `/api/v1/:path*` vers le backend. Or les libs admin de l'UI (`api.ts`, `keys.ts`, `providers.ts`, `models.ts`, `logs.ts`) appellent des chemins nus (`/models`, `/api-keys`, ...) via `API_BASE = ''`, qui ne sont ni servis ni rewrités → **404**. Le correctif consiste à préfixer `API_BASE = '/api/v1'` dans ces 5 libs, en alignant sur la convention déjà correcte utilisée par `auth.ts`, `http.ts`, `websocket.ts` et l'app legacy `apps/admin`.

**Tech Stack :** Next.js 14 (App Router), React Query, Vitest (mock de `global.fetch` via `vi.stubGlobal`), Rust/axum/SQLx (backend).

---

## Carte de correspondance routes (source de vérité)

| UI (lib) | Chemin actuel (cassé) | Chemin backend réel (sous `/api/v1`) | Endpoint backend |
|---|---|---|---|
| `lib/api.ts` | `/admin/stats` | `/api/v1/admin/stats` | `stats::router()` |
| `lib/api.ts` | `/spend?days=` | `/api/v1/spend` | `spend::router()` |
| `lib/keys.ts` | `/api-keys` | `/api/v1/api-keys` | `api_keys::router()` |
| `lib/keys.ts` | `/api-keys/${id}/block` | `/api/v1/api-keys/:id/block` | idem |
| `lib/keys.ts` | `/api-keys/${id}/unblock` | `/api/v1/api-keys/:id/unblock` | idem |
| `lib/keys.ts` | `/api-keys/${id}` | `/api/v1/api-keys/:id` | idem |
| `lib/providers.ts` | `/provider-profiles` | `/api/v1/provider-profiles` | `provider_profiles::router()` |
| `lib/providers.ts` | `/provider-profiles/${id}` | `/api/v1/provider-profiles/:id` | idem |
| `lib/models.ts` | `/models` | `/api/v1/models` | `models::router()` |
| `lib/logs.ts` | `/spend/logs` | `/api/v1/spend/logs` | `spend_logs::router()` |

**Ne PAS toucher** (déjà corrects) : `lib/auth.ts` (`/api/v1/auth/*`), `lib/http.ts` (`/api/v1/auth/refresh`), `lib/websocket.ts` (`ws://.../api/v1/ws/metrics`), et la route `/metrics` (servie à la racine par `metrics_endpoint::router`, rewrite Next `/metrics` → backend).

---

## Structure des fichiers du plan

- **Modifier** (préfixe `API_BASE`) :
  - `apps/ui/src/lib/api.ts`
  - `apps/ui/src/lib/keys.ts`
  - `apps/ui/src/lib/providers.ts`
  - `apps/ui/src/lib/models.ts`
  - `apps/ui/src/lib/logs.ts`
- **Créer** (tests de non-régression du préfixe) :
  - `apps/ui/src/lib/api.test.ts` (ajouter des cas de test fetch)
  - `apps/ui/src/lib/keys.test.ts` (nouveau)
  - `apps/ui/src/lib/providers.test.ts` (nouveau)
  - `apps/ui/src/lib/models.test.ts` (nouveau — inclut `fetchModels` et `createModel`, le cœur du flux "déclarer un modèle")
  - `apps/ui/src/lib/logs.test.ts` (nouveau)
- **Modifier** (test backend de cohérence) :
  - `crates/godwit-api/tests/router_integration.rs` (POST `/models` : ajouter `pricing`)

---

### Tâche 1 : Test de non-régression sur le préfixe de `createModel`

**Fichiers :**
- Créer : `apps/ui/src/lib/models.test.ts`

Le test vérifie que `createModel` POST vers `/api/v1/models` (et non `/models`). C'est le garde-fou direct du bug.

- [ ] **Étape 1 : écrire le test qui échoue**

```ts
// apps/ui/src/lib/models.test.ts
import { describe, it, expect, vi, afterEach } from 'vitest';
import { createModel, fetchModels } from './models';

// NOTE: vi.stubGlobal() returns void — return the mock explicitly so it can be
// inspected via .mock.calls in the assertions.
function mockFetch(data: unknown) {
  const fetchMock = vi.fn().mockResolvedValue({
    ok: true,
    json: async () => data,
  });
  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('models API', () => {
  it('posts to /api/v1/models when creating a model', async () => {
    const fetchMock = mockFetch({ data: { id: 'm1', public_id: 'gpt-4o' } });
    await createModel({
      public_id: 'gpt-4o',
      provider: 'openai',
      provider_profile_id: '11111111-1111-1111-1111-111111111111',
      provider_model_id: 'gpt-4o-2024-11-20',
      capabilities: 'chat',
      pricing: { input_price_per_million: 2.5, output_price_per_million: 10 },
    });
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe('/api/v1/models');
    expect(init.method).toBe('POST');
  });

  it('fetches models from /api/v1/models', async () => {
    const fetchMock = mockFetch({
      data: [{ id: 'm1', public_id: 'gpt-4o', provider_model_id: 'gpt-4o', capabilities: ['chat'] }],
    });
    await fetchModels();
    const [url] = fetchMock.mock.calls[0];
    expect(url).toBe('/api/v1/models');
  });
});
```

- [ ] **Étape 2 : lancer le test pour vérifier qu'il échoue**

Run: `cd apps/ui && npx vitest run src/lib/models.test.ts`
Attendu : FAIL — `expect(url).toBe('/api/v1/models')` reçoit `'/models'`.

- [ ] **Étape 3 : corriger `lib/models.ts`**

Changer dans `apps/ui/src/lib/models.ts:13` :

```ts
const API_BASE = ''; // same-origin via next rewrites
```

en :

```ts
const API_BASE = '/api/v1';
```

- [ ] **Étape 4 : relancer le test**

Run: `cd apps/ui && npx vitest run src/lib/models.test.ts`
Attendu : PASS (2 tests).

- [ ] **Étape 5 : commit**

```bash
git add apps/ui/src/lib/models.ts apps/ui/src/lib/models.test.ts
git commit -m "fix(ui): prefix model API calls with /api/v1"
```

---

### Tâche 2 : Test + correctif sur `lib/api.ts` (`/admin/stats`, `/spend`)

**Fichiers :**
- Modifier : `apps/ui/src/lib/api.test.ts:1` (ajouter des cas)
- Modifier : `apps/ui/src/lib/api.ts:4`

- [ ] **Étape 1 : ajouter les tests de préfixe**

Ajouter en fin de `apps/ui/src/lib/api.test.ts` :

```ts
import { afterEach, vi } from 'vitest';
import { fetchStats, fetchSpend } from './api';

afterEach(() => {
  vi.unstubAllGlobals();
});

function mockJson(data: unknown) {
  const fetchMock = vi.fn().mockResolvedValue({
    ok: true,
    json: async () => data,
  });
  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
}

describe('admin stats API', () => {
  it('calls /api/v1/admin/stats', async () => {
    const m = mockJson({ organizations: 1, teams: 1, users: 2, apiKeys: 3 });
    await fetchStats();
    const [url] = m.mock.calls[0];
    expect(url).toBe('/api/v1/admin/stats');
  });

  it('calls /api/v1/spend with days', async () => {
    const m = mockJson({ data: [{ date: '2026-08-01', cost: '1.2' }] });
    await fetchSpend(30);
    const [url] = m.mock.calls[0];
    expect(url).toBe('/api/v1/spend?days=30');
  });
});
```

- [ ] **Étape 2 : lancer pour vérifier l'échec**

Run: `cd apps/ui && npx vitest run src/lib/api.test.ts`
Attendu : FAIL — l'URL reçue est `/admin/stats` / `/spend?days=30`.

- [ ] **Étape 3 : corriger `lib/api.ts`**

Dans `apps/ui/src/lib/api.ts:4` :

```ts
const API_BASE = ''; // same-origin via next rewrites
```

en :

```ts
const API_BASE = '/api/v1';
```

- [ ] **Étape 4 : relancer**

Run: `cd apps/ui && npx vitest run src/lib/api.test.ts`
Attendu : PASS.

- [ ] **Étape 5 : commit**

```bash
git add apps/ui/src/lib/api.ts apps/ui/src/lib/api.test.ts
git commit -m "fix(ui): prefix admin stats and spend API calls with /api/v1"
```

---

### Tâche 3 : Test + correctif sur `lib/keys.ts`

**Fichiers :**
- Créer : `apps/ui/src/lib/keys.test.ts`
- Modifier : `apps/ui/src/lib/keys.ts:39`

- [ ] **Étape 1 : écrire le test**

```ts
// apps/ui/src/lib/keys.test.ts
import { describe, it, expect, vi, afterEach } from 'vitest';
import { fetchKeys, blockKey, unblockKey, deleteKey } from './keys';

function mockFetch(data: unknown) {
  const fetchMock = vi.fn().mockResolvedValue({
    ok: true,
    json: async () => data,
  });
  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('keys API', () => {
  it('fetches keys from /api/v1/api-keys', async () => {
    const m = mockFetch({ data: [] });
    await fetchKeys();
    const [url] = m.mock.calls[0];
    expect(url).toBe('/api/v1/api-keys');
  });

  it('blocks a key at /api/v1/api-keys/:id/block', async () => {
    const m = mockFetch({ data: { id: 'k1' } });
    await blockKey('k1');
    const [url, init] = m.mock.calls[0];
    expect(url).toBe('/api/v1/api-keys/k1/block');
    expect(init.method).toBe('POST');
  });

  it('unblocks a key at /api/v1/api-keys/:id/unblock', async () => {
    const m = mockFetch({ data: { id: 'k1' } });
    await unblockKey('k1');
    const [url] = m.mock.calls[0];
    expect(url).toBe('/api/v1/api-keys/k1/unblock');
  });

  it('deletes a key at /api/v1/api-keys/:id', async () => {
    const m = vi.fn().mockResolvedValue({ ok: true, json: async () => ({}) });
    vi.stubGlobal('fetch', m);
    await deleteKey('k1');
    const [url, init] = m.mock.calls[0];
    expect(url).toBe('/api/v1/api-keys/k1');
    expect(init.method).toBe('DELETE');
  });
});
```

- [ ] **Étape 2 : lancer pour vérifier l'échec**

Run: `cd apps/ui && npx vitest run src/lib/keys.test.ts`
Attendu : FAIL — URLs sans `/api/v1`.

- [ ] **Étape 3 : corriger `lib/keys.ts`**

Dans `apps/ui/src/lib/keys.ts:39` :

```ts
const API_BASE = ''; // same-origin via next rewrites
```

en :

```ts
const API_BASE = '/api/v1';
```

- [ ] **Étape 4 : relancer**

Run: `cd apps/ui && npx vitest run src/lib/keys.test.ts`
Attendu : PASS.

- [ ] **Étape 5 : commit**

```bash
git add apps/ui/src/lib/keys.ts apps/ui/src/lib/keys.test.ts
git commit -m "fix(ui): prefix api-keys calls with /api/v1"
```

---

### Tâche 4 : Test + correctif sur `lib/providers.ts`

**Fichiers :**
- Créer : `apps/ui/src/lib/providers.test.ts`
- Modifier : `apps/ui/src/lib/providers.ts:41`

- [ ] **Étape 1 : écrire le test**

```ts
// apps/ui/src/lib/providers.test.ts
import { describe, it, expect, vi, afterEach } from 'vitest';
import { fetchProviders, setProviderEnabled } from './providers';

function mockFetch(data: unknown) {
  const fetchMock = vi.fn().mockResolvedValue({
    ok: true,
    json: async () => data,
  });
  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('providers API', () => {
  it('fetches providers from /api/v1/provider-profiles', async () => {
    const m = mockFetch({ data: [] });
    await fetchProviders();
    const [url] = m.mock.calls[0];
    expect(url).toBe('/api/v1/provider-profiles');
  });

  it('patches enabled at /api/v1/provider-profiles/:id', async () => {
    const m = mockFetch({ id: 'p1', enabled: false });
    await setProviderEnabled('p1', false);
    const [url, init] = m.mock.calls[0];
    expect(url).toBe('/api/v1/provider-profiles/p1');
    expect(init.method).toBe('PATCH');
  });
});
```

- [ ] **Étape 2 : lancer pour vérifier l'échec**

Run: `cd apps/ui && npx vitest run src/lib/providers.test.ts`
Attendu : FAIL.

- [ ] **Étape 3 : corriger `lib/providers.ts`**

Dans `apps/ui/src/lib/providers.ts:14` :

```ts
const API_BASE = ''; // same-origin via next rewrites
```

en :

```ts
const API_BASE = '/api/v1';
```

- [ ] **Étape 4 : relancer**

Run: `cd apps/ui && npx vitest run src/lib/providers.test.ts`
Attendu : PASS.

- [ ] **Étape 5 : commit**

```bash
git add apps/ui/src/lib/providers.ts apps/ui/src/lib/providers.test.ts
git commit -m "fix(ui): prefix provider-profile calls with /api/v1"
```

---

### Tâche 5 : Test + correctif sur `lib/logs.ts`

**Fichiers :**
- Créer : `apps/ui/src/lib/logs.test.ts`
- Modifier : `apps/ui/src/lib/logs.ts:37`

- [ ] **Étape 1 : écrire le test**

```ts
// apps/ui/src/lib/logs.test.ts
import { describe, it, expect, vi, afterEach } from 'vitest';
import { fetchLogs } from './logs';

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('logs API', () => {
  it('fetches logs from /api/v1/spend/logs', async () => {
    const m = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ data: [], offset: 0, limit: 50 }),
    });
    vi.stubGlobal('fetch', m);
    await fetchLogs({ limit: 50 });
    const [url] = m.mock.calls[0];
    expect(url).toBe('/api/v1/spend/logs?limit=50');
  });
});
```

- [ ] **Étape 2 : lancer pour vérifier l'échec**

Run: `cd apps/ui && npx vitest run src/lib/logs.test.ts`
Attendu : FAIL — `/spend/logs?limit=50`.

- [ ] **Étape 3 : corriger `lib/logs.ts`**

Dans `apps/ui/src/lib/logs.ts:37` :

```ts
const API_BASE = ''; // same-origin via next rewrites
```

en :

```ts
const API_BASE = '/api/v1';
```

- [ ] **Étape 4 : relancer**

Run: `cd apps/ui && npx vitest run src/lib/logs.test.ts`
Attendu : PASS.

- [ ] **Étape 5 : commit**

```bash
git add apps/ui/src/lib/logs.ts apps/ui/src/lib/logs.test.ts
git commit -m "fix(ui): prefix spend/logs calls with /api/v1"
```

---

### Tâche 6 : Aligner le test d'intégration backend POST `/models`

**Fichiers :**
- Modifier : `crates/godwit-api/tests/router_integration.rs:607-616`

Le DTO `CreateModelRequest` exige `pricing` (pas de `#[serde(default)]`) et `validate_pricing` exige `input_price_per_million` + `output_price_per_million`. Le test actuel POST sans `pricing` échouerait s'il était exécuté. L'UI envoie bien `pricing` ; alignons le test sur ce contrat.

- [ ] **Étape 1 : ajouter `pricing` au payload du test**

Dans `crates/godwit-api/tests/router_integration.rs:608-616`, remplacer le `serde_json::json!({...})` du POST (qui contient `public_id`, `provider`, `provider_profile_id`, `provider_model_id`, `capabilities`) par le même objet **plus** :

```rust
                        "pricing": {
                            "input_price_per_million": 2.5,
                            "output_price_per_million": 10.0
                        }
```

- [ ] **Étape 2 : vérifier que le test compile et qu'il s'exécute (nécessite une base PostgreSQL)**

Run (avec une DB `godwit` locale) :

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
DATABASE_URL=postgres://user:pass@localhost:5432/godwit cargo test -p godwit-api --test router_integration super_admin_can_create_a_vllm_backed_catalog_model -- --nocapture
```

Attendu : PASS — `POST /api/v1/models` réussit, `data.public_id == "llama-3-70b"`.

> Si aucune base locale n'est disponible, se limiter à :
> `cargo check -p godwit-api` (compilation seule).

- [ ] **Étape 3 : commit**

```bash
git add crates/godwit-api/tests/router_integration.rs
git commit -m "test(api): send pricing when creating a catalog model"
```

---

### Tâche 7 : Vérification globale

- [ ] **Étape 1 : typecheck UI**

Run: `cd apps/ui && npx tsc --noEmit`
Attendu : aucune sortie (succès).

- [ ] **Étape 2 : test UI**

Run: `cd apps/ui && npm test`
Attendu : tous les tests passent (>= 62 précédents + les nouveaux). Les composants (`ModelsTable`, `ProviderList`, etc.) utilisent des fixtures locales et ne sont pas afectés par le changement d'`APP_BASE`.

- [ ] **Étape 3 : build Next**

Run: `cd apps/ui && npx next build`
Attendu : build OK, route `/providers` générée.

- [ ] **Étape 4 : check Rust**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo check -p godwit-api`
Attendu : compilation réussie (warnings préexistants acceptés).

- [ ] **Étape 5 : revue du diff final**

Run: `git log --oneline -8`
Attendu : les commits de correction `fix(ui): ...` + `test(api): ...`.

---

## Auto-revue du plan

**1. Couverture de la spec :** Tous les chemins cassés cartographiés dans la table de correspondance sont traités : `api.ts` (Tâche 2), `keys.ts` (Tâche 3), `providers.ts` (Tâche 4), `models.ts` (Tâche 1), `logs.ts` (Tâche 5). Le test d'intégration backend est aligné (Tâche 6), vérification globale (Tâche 7). Aucun manque.

**2. Scan des placeholders :** aucune "TBD"/"TODO". Chaque étape donne le code exact et la commande avec la sortie attendue.

**3. Cohérence des types :** les fonctions testées (`createModel`, `fetchModels`, `fetchStats`, `fetchSpend`, `fetchKeys`, `blockKey`, `unblockKey`, `deleteKey`, `fetchProviders`, `setProviderEnabled`, `fetchLogs`) existent toutes dans les libs respectives, avec les signatures exactes utilisées dans les tests. `API_BASE` est déclaré dans chaque lib à la ligne indiquée.

**Note d'exécution :** exécuter `afterEach(() => vi.unstubAllGlobals())` dans chaque fichier de test évite toute fuite du mock `fetch` entre fichiers Vitest.
