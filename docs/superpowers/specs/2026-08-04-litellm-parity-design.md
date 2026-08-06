# Design — Parité LiteLLM (backend Godwit)

## Contexte

Le backlog dans `docs/backlog.md` et l’audit `docs/litellm-parity-audit.md` listent ~20 écarts entre Godwit et LiteLLM côté appels LLM. Le DTO partagé `ChatCompletionRequest` / `ChatMessage` est actuellement trop étroit (seulement `model`, `messages`, `stream`, `temperature`, `max_tokens` et `content: String`) et bloque l’implémentation de tool-calling, vision, JSON mode, reasoning, prompt caching, etc.

## Objectifs

1. Permettre l’usage de Godwit avec **OpenCode**, **Claude Code** et **LibreChat**.
2. Atteindre la parité fonctionnelle avec LiteLLM sur le backend (tool-calling, vision, résilience proxy, rate limiting, cost tracking, MCP, web search…).
3. Garder les clés API scopées aux modèles, comme dans LiteLLM.
4. Ne pas surprendre l’UI admin existante : adapter les pages de création de clés.

## Approche choisie : big-bang core DTO + sprints fonctionnels

On écarte le big-bang total (tout refaire en une seule fois). On part sur un **big-bang limité au DTO core** (sprint 1), suivi de sprints verticaux qui livrent de la valeur utilisateur progressivement.

## Découpage en sprints

### Sprint 1 — Core DTO big-bang

Étendre `godwit-core` avec les champs et types nécessaires à quasiment tous les autres sprints.

Livrables :
- `ChatContent` multimodal (`Text`, `ImageUrl`).
- Tool-calling : `Tool`, `ToolChoice`, `ToolCall`, `FunctionCall`.
- `ResponseFormat` (`text`, `json_object`, `json_schema`).
- Paramètres manquants : `top_p`, `top_k`, `frequency_penalty`, `presence_penalty`, `stop`, `seed`, `n`, `logprobs`, `top_logprobs`.
- Reasoning / thinking : `ReasoningConfig`, `ThinkingConfig`.
- Prompt caching : `CacheControl` sur `ChatMessage`, cache tokens dans `Usage` et `UsageReport`.
- Mise à jour des adapters existants pour compiler et transmettre les nouveaux champs quand le provider les supporte.
- Helpers dans `godwit-core` pour faciliter la migration (`ChatContent::as_text`, `ChatContent::has_images`, etc.).

### Sprint 2 — Pont Anthropic natif + clés scopées aux modèles

Objectif : Claude Code peut pointer sur Godwit et utiliser n’importe quel modèle (Anthropic, OpenAI, local…).

Livrables :
- Nouveau module `godwit-api/src/anthropic_proxy.rs` exposant `POST /v1/messages` et `POST /v1/messages?stream=true`.
- DTO Anthropic et conversion bidirectionnelle Anthropic ↔ Core.
- Le `model` passé par Claude Code est un `public_id` Godwit résolu par `DbModelRouter`.
- Ajout de `allowed_models: Vec<String>` sur `api_keys` (DB + modèle + migration).
- Middleware de vérification du scope modèle après `api_key_auth`.
- UI admin : multi-select des modèles autorisés lors de la création/édition d’une clé.

### Sprint 3 — Résilience proxy

Livrables :
- Retry / fallback / failover : policy configurable avec backoff exponentiel, codes retryables, liste de fallbacks dans `models.config`.
- Load balancing : quand plusieurs modèles partagent le même `public_id`, sélection via `round_robin`, `least_busy` ou `latency` au lieu de l’erreur “ambiguous”.
- Rate limiting RPM/TPM effectif : token bucket en mémoire par `(api_key_id, model)` et `(organization_id, model)`.

### Sprint 4 — Usage & cost tracking

Livrables :
- Extraction du usage réel dans les 6 adapters actuellement en `UsageReport::default()` (OpenAI, Azure, llama.cpp, Ollama, vLLM, SGLang).
- Couche de calcul de coût dans `godwit-providers/src/usage.rs` basée sur `models.pricing`.
- Remplissage de `request_logs.cost_usd` pour tous les providers et capabilities.

### Sprint 5 — Capacités manquantes

Livrables :
- Streaming Gemini (`chat_stream` actuellement non implémenté).
- Normalisation des deltas et `finish_reason` pour OpenAI, Azure, llama.cpp, Ollama, vLLM, SGLang.
- Embeddings pour Anthropic, Gemini, Bedrock.
- Audio / image génération pour les providers non-OpenAI qui le supportent.
- Endpoints `/v1/moderations`, `/v1/rerank`, `/v1/batches`.

### Sprint 6 — MCP, web search & SearXNG

Livrables :
- Web search natif : passthrough des tools serveur (`web_search` OpenAI, `web_search_20250305` Anthropic, `google_search` Gemini) avec citations/annotations.
- **SearXNG** comme provider de recherche fallback et/ou endpoint `/v1/search`.
- Client MCP : enregistrement de serveurs MCP en config, exposition de leurs tools dans les requêtes de chat.
- Serveur MCP optionnel : Godwit peut s’exposer comme serveur MCP.
- Guardrails & observabilité (PII, Langfuse, Helicone, Prometheus) : non prioritaire, à planifier plus tard.

## DTO core détaillé (sprint 1)

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ChatContent {
    Text(String),
    Parts(Vec<ChatContentPart>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageUrl {
    pub url: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Tool {
    pub r#type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    None,
    Auto,
    Required,
    Function { function: FunctionName },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FunctionName {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ResponseFormat {
    Text,
    JsonObject,
    JsonSchema { json_schema: JsonSchema },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JsonSchema {
    pub name: String,
    pub schema: Option<serde_json::Value>,
    pub strict: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReasoningConfig {
    pub effort: Option<String>,
    pub thinking: Option<ThinkingConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThinkingConfig {
    pub r#type: String,
    pub budget_tokens: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheControl {
    pub r#type: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: ChatContent,
    pub name: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
    pub cache_control: Option<CacheControl>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Stop {
    String(String),
    Array(Vec<String>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: Option<bool>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub top_p: Option<f32>,
    pub top_k: Option<i32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub stop: Option<Stop>,
    pub seed: Option<i64>,
    pub n: Option<i32>,
    pub logprobs: Option<bool>,
    pub top_logprobs: Option<i32>,
    pub tools: Option<Vec<Tool>>,
    pub tool_choice: Option<ToolChoice>,
    pub response_format: Option<ResponseFormat>,
    pub reasoning: Option<ReasoningConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatCompletionChoice {
    pub index: i32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
    pub logprobs: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Usage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub prompt_tokens_details: Option<TokenDetails>,
    pub completion_tokens_details: Option<TokenDetails>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenDetails {
    pub cached_tokens: Option<i32>,
    pub audio_tokens: Option<i32>,
    pub image_tokens: Option<i32>,
}
```

`UsageReport` dans `godwit-providers/src/adapter.rs` est enrichi avec :
- `cache_read_tokens: Option<i32>`
- `cache_write_tokens: Option<i32>`

## Migration des adapters (sprint 1)

- Utiliser `ChatContent::as_text()` dans les adapters text-only (Anthropic texte, local engines).
- Propager `tools`, `tool_choice`, `response_format`, `stop`, `seed`, `n`, penalties, etc. quand le provider les supporte.
- Ignorer silencieusement les champs non supportés à ce stade, sauf cas explicitement incompatibles (ex. `tool_choice=required` sur un modèle text-only renvoie `CapabilityNotSupported`).
- Mettre à jour les tests existants qui construisent `ChatMessage { content: String }` en `ChatMessage { content: ChatContent::Text(...), ... }`.

## SearXNG (sprint 6)

- Nouveau type de profil `searxng` : `base_url`, `categories`, `language`, `safe_search`.
- Endpoint interne `/v1/search` ou tool `web_search` exposé aux modèles.
- Appel API SearXNG : `GET /search?q={query}&format=json`.
- Résultats normalisés : `title`, `url`, `content`, `source`.
- Utilisé comme fallback quand le provider cible ne supporte pas le web search natif.

## Non-objectifs de ce design

- Refonte de l’auth SSO/SAML.
- Refonte de la gestion des organisations/teams/budgets (déjà présente côté admin).
- Front-end marketing.
- Multi-région / multi-cloud.

## Prochaine étape

Passer à l’écriture du plan d’implémentation détaillé (skill `writing-plans`).
