# Design: Catalogue unifié + refonte UI/backend de l'admin Godwit

Date : 2026-08-10
Statut : Validé (brainstorming)
Branche : `feat/ui-catalogue-refonte`

## Contexte et problème

L'interface admin (`apps/ui`) est jugée "très basique" et ne reflète pas les capacités réelles du backend :

- La page `/providers` sert en réalité de page "Modèles" (confusion de nommage : le label nav dit "Models", la route dit `providers`).
- Le champ "provider" d'un modèle est un texte libre dérivé du `protocol` du profil.
- Les **capabilities** d'un modèle sont saisies dans un champ texte "comma separated" au lieu d'un sélecteur.
- Le **pricing** et les **capabilities** ne sont jamais affichés dans le catalogue.
- Les **provider profiles** ne sont pas gérables : bouton "Configure" mort, pas de création/édition/clé API.
- Les **clés API** : pas de budget/expiration/scope réglables ; scopes/allowed_models en texte libre.
- Backend : **Azure** a un adapter écrit mais non enregistré ; **bedrock, cohere, mistral, groq, together** n'existent qu'en string sans adapter ; le protocole d'un profil n'est pas validé.
- Le composant primitif de l'UI est un simple `<select>` natif : pas de combobox, pas de multi-select, pas de dropdown menu.

## Décisions (issue du brainstorming)

1. **Périmètre** : refonte complète **UI + backend**.
2. **Navigation** : une **page Catalogue unifiée** — les provider profiles avec leurs modèles imbriqués.
3. **Capabilities** : **cases à cocher** pour les 7 valeurs, avec **filtrage backend** selon les capabilities réellement supportées par l'adapter choisi.
4. **Provider profiles** : **CRUD complet** (nom, protocole en sélecteur, base_url, clé API, allow_wildcard, enable/disable).
5. **Clés API** : formulaire riche (budget USD, expiration, scopes en cases à cocher, allowed_models en multi-sélecteur, RPM/TPM).
6. **Backend adaptateurs** : enregistrer l'adapter **Azure**, nettoyer/expliciter les protocoles sans adapter pour éviter les profils qui cassent au runtime. Le sélecteur UI n'offre que les protocoles réellement fonctionnels.
7. **Direction visuelle** : **premium et calme** (hiérarchie claire, badges par protocole, états visuels, espacement généreux).

## Inventaire backend (source de vérité)

### Protocoles
- `Protocol` est un newtype `pub struct Protocol(pub String)` (godwit-core/src/lib.rs:774-829).
- Valeurs définies : `openai`, `anthropic`, `gemini`, `ollama`, `azure_openai`, `bedrock`, `cohere`, `mistral`, `groq`, `together`, `vllm`, `sglang`, `llama_cpp`.
- **Adapters enregistrés au runtime** (app.rs + main.rs, identiques) : `openai`, `anthropic`, `gemini`, `vllm`, `sglang`, `llama_cpp`, `ollama`.
- **Azure** : `AzureOpenAiAdapter` écrit (`azure_openai.rs`) mais **non enregistré**.
- **bedrock, cohere, mistral, groq, together** : string seulement, pas d'adapter.

### Capabilities
- 7 valeurs valides (DB CHECK) : `chat`, `image_generation`, `image_edit`, `video_generation`, `audio_tts`, `audio_stt`, `embedding`.
- Capacités supportées par adapter : openai (chat, image_generation, audio_tts, audio_stt, embedding), anthropic (chat), gemini/vllm/sglang/llama_cpp/ollama (chat, embedding).

### Provider profiles (table + API)
- Colonnes : `id, name (unique global), protocol (TEXT libre, pas de CHECK), base_url, auth (JSONB chiffré), config, enabled, allow_wildcard, created_at`.
- API `/api/v1/provider-profiles` : GET/POST/PATCH. Create : `{name, protocol, base_url, api_key, allow_wildcard}`. Update : `{base_url, api_key, allow_wildcard, enabled}` (nom/protocole non modifiables).
- Response list : `{id, name, protocol, base_url, allow_wildcard, enabled, has_credentials, created_at}`.

### Modèles (table + API)
- Colonnes : `id, public_id (unique par profil), provider (legacy), provider_profile_id, provider_model_id, capabilities (TEXT[]), pricing (JSONB), config (JSONB), created_at`.
- API `/api/v1/models` : GET/POST ; `/api/v1/models/{id}` : GET/PATCH/DELETE.
- Create : `{public_id, provider, provider_profile_id, provider_model_id, capabilities (comma-separated), pricing}`. Pricing requiert `input_price_per_million` + `output_price_per_million`.
- Update : `{public_id, capabilities}` (pas de pricing/modèle/profil).

### Clés API (table + API)
- Colonnes : `id, user_id, team_id, organization_id, name, key_prefix, key_hash (jamais sérialisé), scopes TEXT[], allowed_models TEXT[], budget_limit_usd, budget_spent_usd, rate_limit_requests_per_minute, rate_limit_tokens_per_minute, expires_at, disabled, created_at`.
- API `/api/v1/api-keys` : GET/POST ; `/api/v1/api-keys/{id}` : GET/DELETE ; block/unblock/regenerate/reset_spend.
- Create : `{name, scopes, allowed_models, rate_limit_requests_per_minute, rate_limit_tokens_per_minute}` — **budget_limit_usd, team_id, expires_at NON réglables actuellement**.

## Design

### 1. Backend — adaptateurs et protocoles

**a) Enregistrer Azure.**
- Ajouter `registry.register(Protocol::azure_openai(), Arc::new(AzureOpenAiAdapter::new()))` dans `crates/godwit-api/src/app.rs` (build_test_state) et `crates/godwit-bin/src/main.rs`.
- Vérifier les capabilities de `AzureOpenAiAdapter` (aligner sur `openai`).

**b) Nettoyer les protocoles fantômes.**
- `bedrock`, `cohere`, `mistral`, `groq`, `together` : pas d'adapter. Décision : les retirer des constructeurs de `Protocol` (ou les garder mais les marquer explicitement non supportés).
- Ajouter une validation de `protocol` à la création d'un provider profile : rejeter les protocoles sans adapter enregistré (erreur claire au lieu d'un échec runtime).
- Le sélecteur UI n'offre que les 8 protocoles fonctionnels : `openai`, `anthropic`, `gemini`, `azure_openai`, `vllm`, `sglang`, `llama_cpp`, `ollama`.

**c) Exposer un endpoint "capabilities par protocole"** (ou enrichir GET /provider-profiles).
- Pour le filtrage backend des capabilities : l'UI doit savoir quelles capabilities un protocole supporte. Ajouter un endpoint `GET /api/v1/provider-protocols` (ou `GET /provider-profiles/protocols`) retournant `[{protocol, capabilities, label}]`. Source : les 8 adapters enregistrés (`adapter.supported_capabilities()`).

### 2. Backend — clés API enrichies

- Étendre `POST /api/v1/api-keys` pour accepter : `budget_limit_usd: Option<Decimal>`, `expires_at: Option<DateTime>`, `team_id: Option<Uuid>` (si le rôle le permet).
- Étendre `PATCH /api/v1/api-keys/{id}` (ou l'update existant) pour : `budget_limit_usd`, `expires_at`, `rate_limit_*`, `allowed_models`, `scopes`, `name`, `disabled`.
- Valider `scopes` contre une liste connue fixe : `proxy:write`, `proxy:read`, `admin:read`, `admin:write`. Valeur par défaut du backend actuelle : `proxy:write`. `allowed_models` : libre (chaînes de `public_id`) mais documenté dans l'UI comme une sélection des modèles déclarés.

### 3. Backend — capabilities des modèles

- Remplacer le champ "capabilities comma-separated" par un tableau dans l'API create (`capabilities: Vec<String>` accepté, en plus du string pour compat).
- Ne pas changer le format DB (TEXT[]), juste l'API request.
- Filtrage : à la création, si l'UI envoie une capability non supportée par le protocole du profil, soit rejeter soit ignorer avec warning. Décision : **rejeter** avec message clair.

### 4. UI — Catalogue unifié (`/catalogue` ou `/providers` rénové)

**Structure de page :**
- Nouvelle route `/catalogue` (label nav "Catalogue"), remplaçant l'actuelle `/providers` (label "Modèles").
- Une page : liste des **provider profiles** (accordéons ou cartes), chacun avec :
  - En-tête : nom, badge protocole (couleur par protocole), état (connecté / credentials manquants / disabled), toggle enable/disable, bouton Configure.
  - Modèles imbriqués : table de modèles du profil (public_id, provider_model_id, capabilities en badges, pricing formaté, statut), actions créer/éditer/supprimer.
  - Section "Déclarer un modèle" par profil (formulaire riche).

**Formulaire provider (création/édition) :**
- Nom, protocole (sélecteur des 8), base_url, clé API (masquée, optionnelle, avec test de présence "has_credentials"), allow_wildcard (toggle).

**Formulaire modèle :**
- public_id, provider_model_id, capabilities (cases à cocher des 7, filtrées selon protocole), pricing (input_price/output_price per million).

**Design system :**
- Nouveaux composants : `Select` amélioré (searchable si besoin), `Combobox`/`MultiSelect` (pour allowed_models et capabilities), `Badge` coloré par protocole, `Card`/accordéon, `Modal`/`Drawer` réutilisables (existent déjà).
- Direction premium/calme : hiérarchie typographique claire, espacement généreux, états visuels cohérents, cohérence oklch actuelle.

### 5. UI — Clés API riches

- `KeyForm` : name, scopes (cases à cocher des scopes connus), allowed_models (multi-select des public_id), budget_limit_usd, expires_at (date picker), RPM/TPM.
- Affichage : budget/spent dans la table, expiration, scopes en badges, allowed_models en tags.
- Édition (PATCH) d'une clé (budget, expiration, rate limits, scopes, allowed_models).

## Implémentation (ordre des étapes — Approche A)

1. **Backend fondations** : enregistrer Azure, nettoyer protocoles, validation protocol, endpoint capabilities-par-protocole. Tests backend.
2. **Backend clés riches** : budget/expiration/team en création + édition. Tests.
3. **UI design system** : composants premium (Select amélioré, MultiSelect, badges protocole, cartes accordéon). 
4. **UI Catalogue unifié** : page /catalogue, profils imbriqués, formulaires provider/modèle riches, capabilities en cases à cocher filtrées, pricing affiché.
5. **UI Clés API riches** : KeyForm enrichi + édition.
6. **Contract + coverage** : mise à jour `contract/routes.json`, tests route-contract (ui + admin), docs/coverage.
7. **Vérification finale** : cargo test, vitest, E2E, typecheck, docs.

## Hors périmètre (YAGNI)

- Nouveaux adapters (bedrock/cohere/mistral/groq/together) : implémentation future, pas cette refonte.
- Capabilities "tool_calling"/"vision"/"multimodal" : pas dans les 7 valeurs backend actuelles ; on les ajoutera si le backend évolue.
- Refonte des pages Logs/Settings/Dashboard : hors périmètre (sauf impact direct du design system).
- Mode sombre : pas demandé.
