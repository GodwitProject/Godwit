# Catalogue unifié + refonte UI/backend de l'admin Godwit — Plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refondre l'interface admin (`apps/ui`) en un catalogue unifié des fournisseurs/modèles avec des formulaires riches, et aligner le backend (Azure enregistré, protocoles nettoyés, clés API enrichies, capabilities par protocole).

**Architecture:** Approche A incrémentale — backend fondations (Azure, protocoles, endpoint capabilities), backend clés riches, puis design system UI, page Catalogue unifiée, clés API riches, contract/coverage, vérification finale. Chaque tâche est TDD et livrable.

**Tech Stack:** Rust (axum, sqlx), Next.js 14 (App Router), TypeScript, Tailwind, vitest, Playwright.

---

## Conventions d'environnement (rappels critiques)

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
# Tests DB (godwit-db + tests sqlx::test) nécessitent PostgreSQL :
DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit
```

- `cargo test -p godwit-api --test route_contract` vérifie que chaque route de `contract/routes.json` existe dans le router réel. **Garder `contract/routes.json` à jour quand on ajoute/modifie des routes.**
- Les tests `sqlx::test` dans `crates/godwit-db` utilisent `migrations = "../godwit-db/migrations"`.
- Préfixe des crates : `godwit_*`. `godwit-api::app::build_app(Arc<AppState>)` est le router partagé. `build_test_state(pool)` / `build_test_state_with_auth(pool, auth, mailer)` construisent l'état de test.
- `Protocol` est `pub struct Protocol(pub String)` dans `crates/godwit-core/src/lib.rs:774`. Constructeurs : `Protocol::openai()`, etc.

---

## Plan de fichiers

**Backend (crates/):**
- `godwit-core/src/lib.rs` : liste des protocoles supportés + capabilities par protocole (helpers).
- `godwit-providers/src/azure_openai.rs` : pas de changement (adapter existe) — sera enregistré.
- `godwit-api/src/app.rs` + `godwit-bin/src/main.rs` : enregistrer Azure.
- `godwit-api/src/admin/provider_profiles.rs` : validation protocole à la création.
- `godwit-api/src/admin/provider_protocols.rs` (nouveau) : endpoint capabilities par protocole.
- `godwit-api/src/admin/api_keys.rs` : create/update enrichis (budget, expiration, team).
- `godwit-db/src/repositories/api_keys.rs` : méthode update.
- `godwit-api/src/admin/mod.rs` : router de `provider_protocols`.
- `contract/routes.json` : nouvelles routes.
- `docs/coverage/frontend-backend.md` : lignes coverage.

**UI (apps/ui/src/):**
- `lib/providers.ts` : types + fonctions (protocols, create/update profile).
- `lib/models.ts` : capabilities en tableau, pricing affiché.
- `lib/keys.ts` : create/update avec budget/expiration/scopes.
- `components/ui/Select.tsx` : amélioration.
- `components/ui/MultiSelect.tsx` (nouveau) : multi-sélection (modèles, capabilities).
- `components/ui/Badge.tsx` : badge coloré par protocole.
- `components/catalogue/*` (nouveaux) : CataloguePage, ProviderCard, ModelTable, ProviderForm, ModelForm.
- `app/(protected)/catalogue/page.tsx` (nouveau) : route catalogue.
- `app/(protected)/providers/page.tsx` : remplacé par catalogue.
- `app/(protected)/keys/page.tsx` : KeyForm enrichi.
- `components/layout/Sidebar.tsx` : nav "Catalogue".
- `i18n/translations.ts` : clés de traduction.
- `lib/auth.test.ts`, `lib/models.test.ts`, `lib/providers.test.ts`, `lib/keys.test.ts` : tests.

---

### Task 1: Backend — enregistrer l'adapter Azure + valider les protocoles

**Files:**
- Modify: `crates/godwit-api/src/app.rs:109-117`
- Modify: `crates/godwit-bin/src/main.rs:52-59`
- Modify: `crates/godwit-api/src/admin/provider_profiles.rs:58-83`
- Test: `crates/godwit-api/src/admin/provider_profiles.rs` (module tests)

- [ ] **Step 1: Enregistrer l'adapter Azure dans les deux registres**

Dans `crates/godwit-api/src/app.rs` (dans `build_test_state_with_auth`, après la ligne `registry.register(Protocol::ollama(), ...)`) :

```rust
    registry.register(Protocol::azure_openai(), Arc::new(AzureOpenAiAdapter::new()));
```

Vérifier que `AzureOpenAiAdapter` est importé dans `app.rs`. S'il ne l'est pas, ajouter à l'import :
```rust
use godwit_providers::{
    anthropic::AnthropicAdapter, azure_openai::AzureOpenAiAdapter, gemini::GeminiAdapter,
    llama_cpp::LlamaCppAdapter, ollama::OllamaAdapter, openai::OpenAiAdapter,
    sglang::SglangAdapter, vllm::VllmAdapter, AdapterRegistry,
};
```

Dans `crates/godwit-bin/src/main.rs`, même ajout dans la liste des `registry.register(...)` et dans l'import `use godwit_providers::{...}`.

- [ ] **Step 2: Ajouter la validation de protocole à la création de profil**

Dans `crates/godwit-api/src/admin/provider_profiles.rs`, ajouter une fonction qui valide le protocole contre les adapters supportés, et l'appeler dans `create_profile` :

```rust
fn validate_protocol(protocol: &str) -> Result<(), ApiError> {
    let supported = ["openai", "anthropic", "gemini", "azure_openai", "vllm", "sglang", "llama_cpp", "ollama"];
    if supported.contains(&protocol) {
        Ok(())
    } else {
        Err(ApiError::BadRequest(format!(
            "unsupported protocol '{protocol}'; supported: {}",
            supported.join(", ")
        )))
    }
}
```

Dans `create_profile`, après `require_super_admin(&claims)?;` :
```rust
    validate_protocol(&req.protocol)?;
```

- [ ] **Step 3: Écrire le test de validation (TDD)**

Dans le module `#[cfg(test)] mod tests` de `provider_profiles.rs`, ajouter :

```rust
    #[test]
    fn validate_protocol_accepts_registered_and_rejects_ghosts() {
        assert!(validate_protocol("openai").is_ok());
        assert!(validate_protocol("azure_openai").is_ok());
        assert!(validate_protocol("bedrock").is_err());
        assert!(validate_protocol("cohere").is_err());
        assert!(validate_protocol("not-a-protocol").is_err());
    }
```

- [ ] **Step 4: Vérifier**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-api --lib provider_profiles
DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo check --workspace
```

- [ ] **Step 5: Commit**

```bash
git add crates/godwit-api/src/app.rs crates/godwit-bin/src/main.rs crates/godwit-api/src/admin/provider_profiles.rs
git commit -m "feat(api): register azure adapter + validate provider protocols"
```

---

### Task 2: Backend — endpoint capabilities par protocole

**Files:**
- Create: `crates/godwit-api/src/admin/provider_protocols.rs`
- Modify: `crates/godwit-api/src/admin/mod.rs`
- Modify: `contract/routes.json`
- Test: `crates/godwit-api/src/admin/provider_protocols.rs`

- [ ] **Step 1: Créer le handler (TDD d'abord)**

Créer `crates/godwit-api/src/admin/provider_protocols.rs` :

```rust
use axum::{routing::get, Json, Router};
use godwit_core::Capability;
use std::sync::Arc;

use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/provider-protocols", get(list_protocols))
}

fn protocol_capabilities(protocol: &str) -> Vec<&'static str> {
    match protocol {
        "openai" => vec!["chat", "image_generation", "audio_tts", "audio_stt", "embedding"],
        "anthropic" => vec!["chat"],
        "gemini" => vec!["chat", "embedding"],
        "azure_openai" => vec!["chat", "image_generation", "audio_tts", "audio_stt", "embedding"],
        "vllm" => vec!["chat", "embedding"],
        "sglang" => vec!["chat", "embedding"],
        "llama_cpp" => vec!["chat", "embedding"],
        "ollama" => vec!["chat", "embedding"],
        _ => vec!["chat"],
    }
}

async fn list_protocols() -> Result<Json<serde_json::Value>, crate::error::ApiError> {
    let protocols = [
        "openai", "anthropic", "gemini", "azure_openai", "vllm", "sglang", "llama_cpp", "ollama",
    ];
    let data = protocols
        .iter()
        .map(|p| {
            serde_json::json!({
                "protocol": p,
                "label": p,
                "capabilities": protocol_capabilities(p),
            })
        })
        .collect::<Vec<_>>();
    Ok(Json(serde_json::json!({ "data": data })))
}
```

Note : `Capability` importé n'est pas utilisé ici si on garde `protocol_capabilities` en strings — **retirer `use godwit_core::Capability;`** pour éviter un warning (ou utiliser `Capability::as_str()`). Option propre : retourner des `Capability` via `as_str()` :

```rust
fn protocol_capabilities(protocol: &str) -> Vec<&'static str> {
    use godwit_core::Capability as C;
    match protocol {
        "openai" => vec![C::Chat.as_str(), C::ImageGeneration.as_str(), C::AudioTts.as_str(), C::AudioStt.as_str(), C::Embedding.as_str()],
        "anthropic" => vec![C::Chat.as_str()],
        "gemini" => vec![C::Chat.as_str(), C::Embedding.as_str()],
        "azure_openai" => vec![C::Chat.as_str(), C::ImageGeneration.as_str(), C::AudioTts.as_str(), C::AudioStt.as_str(), C::Embedding.as_str()],
        "vllm" => vec![C::Chat.as_str(), C::Embedding.as_str()],
        "sglang" => vec![C::Chat.as_str(), C::Embedding.as_str()],
        "llama_cpp" => vec![C::Chat.as_str(), C::Embedding.as_str()],
        "ollama" => vec![C::Chat.as_str(), C::Embedding.as_str()],
        _ => vec![C::Chat.as_str()],
    }
}
```

- [ ] **Step 2: Ajouter les tests**

Dans le même fichier, module tests :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn azure_and_ollama_capabilities() {
        assert!(protocol_capabilities("azure_openai").contains(&"image_generation"));
        assert!(protocol_capabilities("ollama").contains(&"chat"));
        assert!(!protocol_capabilities("anthropic").contains(&"embedding"));
    }

    #[test]
    fn unknown_protocol_defaults_to_chat() {
        assert_eq!(protocol_capabilities("nope"), vec!["chat"]);
    }
}
```

- [ ] **Step 3: Enregistrer le router dans admin/mod.rs**

Dans `crates/godwit-api/src/admin/mod.rs`, ajouter le module et monter le router. Repérer comment `auth::router(state)` / `users::router()` sont montés (probablement dans une fonction `pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>>`). Ajouter :

```rust
    pub mod provider_protocols;
```

et dans le `Router` monté sous `/api/v1` :
```rust
        .merge(super::admin::provider_protocols::router())
```
(adapter le chemin selon la structure réelle de `mod.rs` — vérifier comment `provider_profiles::router()` est monté et faire pareil).

- [ ] **Step 4: Ajouter la route au contract**

Dans `contract/routes.json`, ajouter (dans la section models/providers) :
```json
{ "id": "provider-protocols.list", "method": "GET", "path": "/api/v1/provider-protocols", "scope": "ui", "frontend": { "lib": "apps/ui/src/lib/providers.ts", "fn": "fetchProtocols" }, "backend": { "module": "crates/godwit-api/src/admin/provider_protocols.rs", "fn": "list_protocols" } }
```

- [ ] **Step 5: Vérifier**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-api --lib provider_protocols
DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-api --test route_contract
DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo check --workspace
```

- [ ] **Step 6: Commit**

```bash
git add crates/godwit-api/src/admin/provider_protocols.rs crates/godwit-api/src/admin/mod.rs contract/routes.json
git commit -m "feat(api): expose per-protocol capabilities endpoint"
```

---

### Task 3: Backend — clés API enrichies (budget, expiration, team) + édition

**Files:**
- Modify: `crates/godwit-api/src/admin/api_keys.rs`
- Modify: `crates/godwit-db/src/repositories/api_keys.rs`
- Test: `crates/godwit-api/src/admin/api_keys.rs`, `crates/godwit-db/tests/api_keys.rs` (ou tests existants)

- [ ] **Step 1: Étendre la création (budget, expiration, team_id)**

Lire `crates/godwit-api/src/admin/api_keys.rs` pour trouver le `CreateApiKeyRequest` et le handler de création. Étendre la requête :

```rust
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub allowed_models: Vec<String>,
    pub rate_limit_requests_per_minute: Option<i32>,
    pub rate_limit_tokens_per_minute: Option<i32>,
    pub budget_limit_usd: Option<rust_decimal::Decimal>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub team_id: Option<Uuid>,
}
```

Adapter le handler de création pour passer ces champs au repo `create` (si la signature du repo le permet) — sinon étendre la méthode `create` du repo dans `crates/godwit-db/src/repositories/api_keys.rs`.

- [ ] **Step 2: Ajouter une méthode update au repo**

Dans `crates/godwit-db/src/repositories/api_keys.rs`, ajouter :

```rust
    pub async fn update(
        &self,
        id: Uuid,
        name: Option<&str>,
        scopes: Option<&[String]>,
        allowed_models: Option<&[String]>,
        budget_limit_usd: Option<rust_decimal::Decimal>,
        rate_limit_requests_per_minute: Option<i32>,
        rate_limit_tokens_per_minute: Option<i32>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
        disabled: Option<bool>,
    ) -> Result<ApiKey, sqlx::Error> {
        // UPDATE api_keys SET name = COALESCE($2, name), scopes = COALESCE($3, scopes), ...
        //     WHERE id = $1 RETURNING *;
        // Utiliser sqlx::query_as! avec les colonnes existantes (voir les méthodes existantes
        // du repo pour le SELECT exact et le mapping ApiKey).
    }
```

(Suivre le pattern des requêtes `sqlx::query_as!` existantes dans ce fichier pour le mapping complet `ApiKey`.)

- [ ] **Step 3: Ajouter PATCH /api-keys/{id}**

Dans `crates/godwit-api/src/admin/api_keys.rs`, ajouter une route `patch(update_key)` sur `/api-keys/:id` et le handler :

```rust
#[derive(Debug, Deserialize)]
pub struct UpdateApiKeyRequest {
    pub name: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub allowed_models: Option<Vec<String>>,
    pub budget_limit_usd: Option<rust_decimal::Decimal>,
    pub rate_limit_requests_per_minute: Option<i32>,
    pub rate_limit_tokens_per_minute: Option<i32>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub disabled: Option<bool>,
}

async fn update_key(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateApiKeyRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // vérifier role.can_manage_api_keys() (comme create_key)
    let repo = ApiKeyRepository::new(state.pool.clone());
    let key = repo
        .update(
            id,
            req.name.as_deref(),
            req.scopes.as_deref(),
            req.allowed_models.as_deref(),
            req.budget_limit_usd,
            req.rate_limit_requests_per_minute,
            req.rate_limit_tokens_per_minute,
            req.expires_at,
            req.disabled,
        )
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": key })))
}
```

- [ ] **Step 4: Ajouter la route au contract**

Dans `contract/routes.json`, ajouter :
```json
{ "id": "api-keys.update", "method": "PATCH", "path": "/api/v1/api-keys/{id}", "scope": "ui", "frontend": { "lib": "apps/ui/src/lib/keys.ts", "fn": "updateKey" }, "backend": { "module": "crates/godwit-api/src/admin/api_keys.rs", "fn": "update_key" } }
```

- [ ] **Step 5: Vérifier**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-api --lib api_keys
DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-db --lib repositories
DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-api --test route_contract
DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo check --workspace
```

- [ ] **Step 6: Commit**

```bash
git add crates/godwit-api/src/admin/api_keys.rs crates/godwit-db/src/repositories/api_keys.rs contract/routes.json
git commit -m "feat(api): enrich api key create (budget/expiry/team) + PATCH update"
```

---

### Task 4: UI — design system (Select amélioré, MultiSelect, Badge protocole)

**Files:**
- Modify: `apps/ui/src/components/ui/Select.tsx`
- Create: `apps/ui/src/components/ui/MultiSelect.tsx`
- Modify: `apps/ui/src/components/ui/Badge.tsx`
- Test: `apps/ui/src/components/ui/__tests__/select.test.tsx`, `multiselect.test.tsx` (si pattern existant)

- [ ] **Step 1: Améliorer Select (avec placeholder + recherche optionnelle)**

Lire `apps/ui/src/components/ui/Select.tsx` existant. Ajouter une prop `placeholder` et un style cohérent (label, chevron). Exemple de base :

```tsx
'use client';
import { clsx } from 'clsx';

interface Option { value: string; label: string; }
interface SelectProps {
  label?: string;
  placeholder?: string;
  value: string;
  onChange: (value: string) => void;
  options: Option[];
  disabled?: boolean;
  error?: string;
}

export function Select({ label, placeholder, value, onChange, options, disabled, error }: SelectProps) {
  return (
    <div className="flex flex-col gap-1">
      {label && <label className="text-[12.5px] font-medium text-muted">{label}</label>}
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
        className={clsx(
          'appearance-none rounded-lg border bg-surface px-3 py-2 text-[13.5px] text-fg outline-none focus:border-accent',
          error ? 'border-danger' : 'border-border'
        )}
      >
        {placeholder && <option value="" disabled>{placeholder}</option>}
        {options.map((o) => (
          <option key={o.value} value={o.value}>{o.label}</option>
        ))}
      </select>
      {error && <span className="text-[11.5px] text-danger">{error}</span>}
    </div>
  );
}
```

- [ ] **Step 2: Créer MultiSelect (checkboxes en dropdown)**

`apps/ui/src/components/ui/MultiSelect.tsx` :

```tsx
'use client';
import { useState, useRef, useEffect } from 'react';
import { clsx } from 'clsx';

interface Option { value: string; label: string; }
interface MultiSelectProps {
  label?: string;
  selected: string[];
  onChange: (values: string[]) => void;
  options: Option[];
  placeholder?: string;
}

export function MultiSelect({ label, selected, onChange, options, placeholder }: MultiSelectProps) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onDoc = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener('mousedown', onDoc);
    return () => document.removeEventListener('mousedown', onDoc);
  }, []);

  const toggle = (v: string) => {
    onChange(selected.includes(v) ? selected.filter((x) => x !== v) : [...selected, v]);
  };

  return (
    <div className="flex flex-col gap-1" ref={ref}>
      {label && <label className="text-[12.5px] font-medium text-muted">{label}</label>}
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex min-h-[38px] flex-wrap items-center gap-1 rounded-lg border border-border bg-surface px-2 py-1.5 text-left text-[13.5px]"
      >
        {selected.length === 0 ? (
          <span className="text-muted">{placeholder ?? 'Sélectionner…'}</span>
        ) : (
          selected.map((s) => {
            const o = options.find((x) => x.value === s);
            return (
              <span key={s} className="inline-flex items-center gap-1 rounded-md bg-surface-2 px-2 py-0.5 text-[12px]">
                {o?.label ?? s}
                <button type="button" onClick={() => toggle(s)} className="text-muted hover:text-danger">×</button>
              </span>
            );
          })
        )}
      </button>
      {open && (
        <div className="absolute z-50 mt-10 max-h-56 w-full overflow-auto rounded-lg border border-border bg-surface p-1 shadow-lg">
          {options.map((o) => (
            <label key={o.value} className="flex cursor-pointer items-center gap-2 px-2 py-1.5 hover:bg-surface-2">
              <input type="checkbox" checked={selected.includes(o.value)} onChange={() => toggle(o.value)} />
              <span className="text-[13px]">{o.label}</span>
            </label>
          ))}
        </div>
      )}
    </div>
  );
}
```

(Note : le dropdown utilise `absolute` — le parent doit être `relative`. Adapter dans l'usage : wrapper `relative` autour du MultiSelect dans le formulaire.)

- [ ] **Step 3: Badge coloré par protocole**

Modifier `apps/ui/src/components/ui/Badge.tsx` pour accepter une couleur par protocole. Exemple d'ajout :

```tsx
const PROTOCOL_COLORS: Record<string, string> = {
  openai: 'bg-emerald-50 text-emerald-700 border-emerald-200',
  anthropic: 'bg-orange-50 text-orange-700 border-orange-200',
  gemini: 'bg-blue-50 text-blue-700 border-blue-200',
  azure_openai: 'bg-sky-50 text-sky-700 border-sky-200',
  vllm: 'bg-violet-50 text-violet-700 border-violet-200',
  sglang: 'bg-fuchsia-50 text-fuchsia-700 border-fuchsia-200',
  llama_cpp: 'bg-amber-50 text-amber-700 border-amber-200',
  ollama: 'bg-teal-50 text-teal-700 border-teal-200',
};
```

Vérifier le `Badge` existant (props/children) et ajouter un rendu qui applique `PROTOCOL_COLORS[protocol]` quand une prop `protocol` est passée, sinon le style par défaut.

- [ ] **Step 4: Tests + typecheck**

```bash
cd apps/ui && ./node_modules/.bin/tsc --noEmit
./node_modules/.bin/vitest run
```

Si des tests de composants existent dans `components/ui/__tests__/`, en ajouter pour MultiSelect (toggle ajoute/retire une valeur). Sinon, le typecheck + vitest existant suffisent.

- [ ] **Step 5: Commit**

```bash
git add apps/ui/src/components/ui/
git commit -m "feat(ui): premium form primitives (select, multiselect, protocol badges)"
```

---

### Task 5: UI — page Catalogue unifiée (profils + modèles imbriqués)

**Files:**
- Create: `apps/ui/src/lib/providers.ts` (enrichir)
- Modify: `apps/ui/src/lib/models.ts`
- Create: `apps/ui/src/app/(protected)/catalogue/page.tsx`
- Create: `apps/ui/src/components/catalogue/ProviderCard.tsx`
- Create: `apps/ui/src/components/catalogue/ModelTable.tsx`
- Create: `apps/ui/src/components/catalogue/ProviderForm.tsx`
- Create: `apps/ui/src/components/catalogue/ModelForm.tsx`
- Modify: `apps/ui/src/components/layout/Sidebar.tsx`
- Modify: `apps/ui/src/i18n/translations.ts`
- Test: `apps/ui/src/lib/providers.test.ts`, `apps/ui/src/lib/models.test.ts`

- [ ] **Step 1: Enrichir `lib/providers.ts`**

Ajouter types + fonctions :

```ts
export interface ProviderProtocol {
  protocol: string;
  label: string;
  capabilities: string[];
}

export interface Provider {
  id: string;
  name: string;
  protocol: string;
  base_url: string | null;
  allow_wildcard: boolean;
  enabled: boolean;
  has_credentials: boolean;
  created_at: string;
}

export interface ProviderInput {
  name: string;
  protocol: string;
  base_url?: string;
  api_key?: string;
  allow_wildcard?: boolean;
}

export async function fetchProtocols(): Promise<ProviderProtocol[]> {
  const res = await apiFetch('/api/v1/provider-protocols');
  if (!res.ok) throw new Error('Failed to fetch protocols');
  const body = await res.json();
  return body.data ?? [];
}

export async function createProvider(input: ProviderInput): Promise<Provider> {
  const res = await apiFetch('/api/v1/provider-profiles', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(input),
  });
  if (!res.ok) throw new Error('Failed to create provider');
  return res.json();
}

export async function updateProvider(id: string, patch: Partial<ProviderInput>): Promise<Provider> {
  const res = await apiFetch(`/api/v1/provider-profiles/${id}`, {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(patch),
  });
  if (!res.ok) throw new Error('Failed to update provider');
  return res.json();
}
```

(Adapter `apiFetch` au helper réel de l'app — vérifier `apps/ui/src/lib/http.ts` ou comment les autres libs font leurs appels ; `providers.ts` existant utilise sûrement un helper commun.)

- [ ] **Step 2: Enrichir `lib/models.ts`**

Adapter `CreateModelRequest` pour accepter `capabilities: string[]` et ajouter un helper de formatting pricing :

```ts
export interface CreateModelRequest {
  public_id: string;
  provider: string;
  provider_profile_id: string;
  provider_model_id: string;
  capabilities: string[];
  pricing: { input_price_per_million: number; output_price_per_million: number };
}

export function formatPrice(value: unknown): string {
  if (typeof value === 'number') {
    return `$${value.toFixed(4)}`;
  }
  return '—';
}
```

(Adapter au shape réel de `ApiModel`/`parseModel` existants dans `models.ts`.)

- [ ] **Step 3: Créer la page catalogue**

`apps/ui/src/app/(protected)/catalogue/page.tsx` — client component qui :
- charge `fetchProviders()` + `fetchModels()` + `fetchProtocols()`
- rend une liste de `ProviderCard`
- gère l'état : créer un provider (bouton), éditer (Configure), toggle enable

**ProviderCard** : carte avec en-tête (nom, Badge protocole, badge état credentials/disabled, toggle), table de modèles du profil (via `ModelTable`), boutons "Nouveau modèle" et "Configure".

**ProviderForm** : modal `Modal` avec `Select` protocole (options = `fetchProtocols()`), Input nom/base_url/api_key, toggle allow_wildcard.

**ModelForm** : modal avec Input public_id, Input provider_model_id, cases à cocher capabilities (7 valeurs filtrées selon le protocole du profil parent via `fetchProtocols()`), Inputs pricing.

- [ ] **Step 4: Sidebar + i18n**

Dans `Sidebar.tsx`, remplacer l'item `/providers` par `/catalogue` avec label `nav.catalogue` (ou réutiliser `nav.models` avec nouvelle clé). Ajouter les clés de traduction dans `translations.ts` (fr+en) pour : `nav.catalogue`, `catalogue.*`, `provider.*`, `model.*` selon les besoins des nouveaux composants.

- [ ] **Step 5: Tests lib + typecheck**

```bash
cd apps/ui && ./node_modules/.bin/tsc --noEmit
./node_modules/.bin/vitest run
```

Ajouter des tests pour `fetchProtocols`/`createProvider`/`updateProvider` (mock fetch) et pour `formatPrice` en suivant le pattern de `apps/ui/src/lib/auth.test.ts`.

- [ ] **Step 6: Commit**

```bash
git add apps/ui/src/
git commit -m "feat(ui): unified catalogue page (providers + nested models)"
```

---

### Task 6: UI — clés API riches

**Files:**
- Modify: `apps/ui/src/lib/keys.ts`
- Modify: `apps/ui/src/app/(protected)/keys/page.tsx`
- Modify: `apps/ui/src/i18n/translations.ts`
- Test: `apps/ui/src/lib/keys.test.ts`

- [ ] **Step 1: Enrichir `lib/keys.ts`**

```ts
export interface CreateKeyInput {
  name: string;
  scopes: string[];
  allowed_models: string[];
  rate_limit_requests_per_minute?: number;
  rate_limit_tokens_per_minute?: number;
  budget_limit_usd?: number;
  expires_at?: string; // ISO
}

export interface UpdateKeyInput {
  name?: string;
  scopes?: string[];
  allowed_models?: string[];
  budget_limit_usd?: number;
  expires_at?: string | null;
  rate_limit_requests_per_minute?: number;
  rate_limit_tokens_per_minute?: number;
  disabled?: boolean;
}

export async function updateKey(id: string, patch: UpdateKeyInput): Promise<ApiKey> {
  const res = await apiFetch(`/api/v1/api-keys/${id}`, {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(patch),
  });
  if (!res.ok) throw new Error('Failed to update key');
  const body = await res.json();
  return body.data ?? body;
}
```

Adapter `createKey` pour accepter `CreateKeyInput` (avec budget/expiration).

- [ ] **Step 2: Enrichir le KeyForm**

Dans `apps/ui/src/app/(protected)/keys/page.tsx`, enrichir `KeyForm` :
- `Select` scopes (options : proxy:write, proxy:read, admin:read, admin:write) ou MultiSelect
- `MultiSelect` allowed_models (options = public_id des modèles)
- Input budget (USD), Input date expiration (type="date"), RPM/TPM
- ajouter un mode édition (bouton par ligne) qui appelle `updateKey`

- [ ] **Step 3: Tests + typecheck**

```bash
cd apps/ui && ./node_modules/.bin/tsc --noEmit
./node_modules/.bin/vitest run
```

Tests pour `updateKey`/`createKey` (mock fetch).

- [ ] **Step 4: Commit**

```bash
git add apps/ui/src/
git commit -m "feat(ui): rich api key form (budget, expiry, scopes, models)"
```

---

### Task 7: Contract + coverage + vérification finale

**Files:**
- Modify: `contract/routes.json`
- Modify: `apps/ui/tests/route-contract.test.ts`
- Modify: `docs/coverage/frontend-backend.md`

- [ ] **Step 1: S'assurer que toutes les routes sont dans le contract**

Vérifier que les routes suivantes existent dans `contract/routes.json` (créées dans les tâches précédentes) :
- `GET /api/v1/provider-protocols`
- `PATCH /api/v1/api-keys/{id}`
- les routes existantes `/api/v1/provider-profiles` (GET/POST/PATCH), `/api/v1/models` (GET/POST), `/api/v1/models/{id}` (GET/PATCH/DELETE)

S'il manque la route `/api/v1/models/{id}` PATCH/DELETE dans le contract avec les bons frontend.fn, l'ajouter.

- [ ] **Step 2: Mettre à jour le test route-contract UI**

Dans `apps/ui/tests/route-contract.test.ts`, ajouter les cas `invoke` pour les nouvelles fns :
```ts
case 'fetchProtocols': await providers.fetchProtocols(); break;
case 'createProvider': await providers.createProvider({ name: 'p', protocol: 'openai' }); break;
case 'updateProvider': await providers.updateProvider(ZERO_UUID, { enabled: true }); break;
case 'updateKey': await keys.updateKey(ZERO_UUID, { name: 'k' }); break;
```
(Importer `providers`/`keys` comme les autres `import * as ...`.)

- [ ] **Step 3: Mettre à jour docs/coverage**

Ajouter les lignes correspondantes dans `docs/coverage/frontend-backend.md` (status `covered`), alignées sur le format existant.

- [ ] **Step 4: Vérification backend complète**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-api --test route_contract
DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo check --workspace
DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-api --lib
```

- [ ] **Step 5: Vérification frontend complète**

```bash
cd apps/ui && ./node_modules/.bin/tsc --noEmit && ./node_modules/.bin/vitest run
cd ../admin && ./node_modules/.bin/vitest run tests/route-contract.test.ts
```

- [ ] **Step 6: Commit**

```bash
git add contract/routes.json apps/ui/tests/route-contract.test.ts docs/coverage/frontend-backend.md
git commit -m "feat(contract): declare catalogue + api-key routes, update coverage"
```

---

## Self-Review Notes

- **Spec coverage** : chaque décision du spec a une tâche (Azure+validation → T1, endpoint capabilities → T2, clés riches → T3+T6, design system → T4, catalogue unifié → T5, contract/coverage → T7). La direction visuelle premium/calme est portée par T4 + T5.
- **Hors périmètre** (du spec) : pas d'adapters bedrock/cohere/mistral/groq/together, pas de mode sombre, pas de refonte Logs/Settings/Dashboard.
- **Attention** : les signatures exactes des handlers (`api_keys.rs`, repo `create`) doivent être lues avant chaque modification — le plan donne les shapes attendues mais l'implémenteur doit vérifier les signatures réelles (comme indiqué dans chaque task).
- **Route contract** : le test `route_contract` échouera si une route du contract n'existe pas dans le router réel — garder `contract/routes.json` et le router synchronisés.
