# P1 Core Resilience & Usage Tracking — Design Spec

**Date:** 2026-08-07  
**Auteur:** Thomas (avec opencode)  
**Statut:** En review  
**Release cible:** v1.1.0

---

## 1. Vue d'Ensemble

Cette spec couvre les fonctionnalités P1 manquantes pour une parité complète avec LiteLLM au-delà du MVP. L'objectif est de transformer Godwit d'un proxy fonctionnel en une **plateforme de production résiliente** avec visibilité complète sur les coûts et l'usage.

### 1.1 Contexte

Le MVP (v1.0.0-liteLLM-parity) a livré :
- ✅ 13 endpoints core + 1 Anthropic-native
- ✅ 20 endpoints admin
- ✅ Budget enforcement (team + end-user)
- ✅ Streaming tool resolution
- ✅ Model aliasing
- ✅ Circuit breaker
- ✅ Tags personnalisés

**Ce qui manque pour la production :**
- ❌ Fallback automatique entre providers/modèles
- ❌ Usage tracking incomplet (certains providers ne remontent pas le usage)
- ❌ Cost layer à consolider

**Ce qui est déjà là (contre-intuitif) :**
- ✅ Load balancing (RoundRobin/LeastBusy/Latency) — déjà implémenté dans `model_router.rs`
- ✅ Rate limiting RPM/TPM — déjà implémenté dans `rate_limit.rs` et utilisé dans `proxy.rs:check_rate_limit()`

---

## 2. Fonctionnalités P1 à Implémenter

### 2.1 Fallback / Failover (PRIORITÉ MAX)

**Problème :** Si un provider échoue, la requête échoue. LiteLLM permet de configurer des chaînes de fallback.

**Solution :** Wrapper retry/fallback autour des appels providers.

#### 2.1.1 Design

```rust
// Dans godwit-api/src/resilience.rs (existe déjà pour retry)
pub struct FallbackConfig {
    pub models: Vec<String>,  // Chaîne de fallback par modèle
    pub max_retries: u32,
    pub timeout_per_attempt: Duration,
}

// Usage dans proxy.rs
async fn call_chat_with_fallback(
    state: &Arc<AppState>,
    initial_model: &str,
    req: ChatCompletionRequest,
) -> Result<(Response, UsageReport), ApiError> {
    let fallback_chain = state.model_router.get_fallback_chain(initial_model);
    
    for (attempt, model_ref) in fallback_chain.iter().enumerate() {
        match call_chat(state, model_ref, req.clone()).await {
            Ok(result) => return Ok(result),
            Err(e) if attempt < fallback_chain.len() - 1 => {
                log::warn!("Fallback: {} failed, trying {}", model_ref, fallback_chain[attempt + 1]);
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}
```

#### 2.1.2 Configuration

```yaml
# config.yaml
models:
  - public_id: gpt-4o
    provider_profile_id: uuid-openai
    provider_model_id: gpt-4o
    config:
      fallbacks:
        - anthropic/claude-sonnet-4-20250514
        - gemini/gemini-2.5-pro
      max_fallback_attempts: 3
```

#### 2.1.3 Critères de Fallback

Fallback déclenché sur :
- ✅ HTTP 5xx (provider down)
- ✅ Timeout
- ✅ Rate limit (429)
- ❌ Pas sur 4xx client error (mauvaise requête)

#### 2.1.4 Logging

Chaque tentative de fallback doit être loggée dans `request_logs` :
- `attempt_number`
- `model_attempted`
- `error_message`
- `fallback_triggered: bool`

---

### 2.2 Usage Tracking Complet

**Problème :** Certains providers ne remontent pas le usage réel, cassant le cost tracking.

#### 2.2.1 État Actuel

| Provider | Chat | Embed | Image | Audio | Statut |
|----------|------|-------|-------|-------|--------|
| OpenAI | ✅ | ✅ | ❌ | ❌ | OK chat |
| Azure | ❌ | ❌ | ❌ | ❌ | TODO |
| Anthropic | ❌ (stream: ✅) | N/A | N/A | N/A | TODO |
| Gemini | ❌ (stream: ✅) | ❌ | N/A | N/A | TODO |
| Bedrock | ✅ | ✅ | N/A | N/A | OK |
| llama.cpp | ❌ | ❌ | N/A | N/A | TODO |
| Ollama | ❌ | ❌ | N/A | N/A | TODO |
| vLLM | ❌ | ❌ | N/A | N/A | TODO |
| SGLang | ❌ | ❌ | N/A | N/A | TODO |

#### 2.2.2 Solution

**Pour les providers open-source (llama.cpp, Ollama, vLLM, SGLang) :**
Ces providers suivent le format OpenAI. Il faut parser le champ `usage` dans la réponse.

```rust
// Dans openai.rs:27 (adapter pour chaque provider)
let body: ChatCompletionResponse = res.json().await?;
let usage = crate::usage::chat_usage_report(&body.usage);  // DÉJÀ FAIT POUR OPENAI
```

**Pour Anthropic non-streaming :**
Anthropic renvoie `usage` dans la réponse (même en non-streaming).

```rust
// anthropic.rs:~280
let body: AnthropicResponse = res.json().await?;
let usage = UsageReport {
    prompt_tokens: Some(body.usage.input_tokens),
    completion_tokens: Some(body.usage.output_tokens),
    ..Default::default()
};
```

**Pour Gemini non-streaming :**
Gemini renvoie `usageMetadata` dans la réponse.

```rust
// gemini.rs:~400
let body: GeminiResponse = res.json().await?;
let usage = UsageReport {
    prompt_tokens: Some(body.usage_metadata.prompt_token_count),
    completion_tokens: Some(body.usage_metadata.candidates_token_count),
    ..Default::default()
};
```

#### 2.2.3 Image & Audio (OpenAI)

OpenAI ne renvoie PAS de usage pour image/audio dans l'API. On doit estimer :

```rust
// Image : compteur simple
let usage = UsageReport {
    image_count: Some(body.data.len() as i32),
    ..Default::default()
};

// Audio TTS : compter les caractères de l'input
let usage = UsageReport {
    tts_characters: Some(request.input.chars().count() as i32),
    ..Default::default()
};

// Audio STT : durée du fichier (nécessite metadata audio)
let duration_secs = get_audio_duration(&audio_bytes)?;
let usage = UsageReport {
    audio_seconds: Some(duration_secs),
    ..Default::default()
};
```

---

### 2.3 Cost Layer — Consolidation

**Problème :** Le cost tracking est fragmenté entre `godwit-providers/src/usage.rs` et `godwit-api/src/admin/spend.rs`.

#### 2.3.1 Architecture Cible

```
godwit-providers/src/usage.rs
├── compute_chat_cost()       # Utilise pricing du modèle
├── compute_embedding_cost()
├── compute_image_cost()
├── compute_audio_tts_cost()
└── compute_audio_stt_cost()

godwit-api/src/admin/spend.rs
├── compute_spend_by_org()
├── compute_spend_by_team()
├── compute_spend_by_api_key()
└── compute_spend_by_tag()    # Utilise les fonctions de godwit-providers
```

#### 2.3.2 Pricing Storage

Le pricing est déjà stocké dans `models.pricing: JSONB` :

```json
{
  "input_price_per_million": 2.5,
  "output_price_per_million": 10.0,
  "image_price_per_image": 0.005,
  "tts_price_per_character": 0.00001,
  "stt_price_per_second": 0.0001
}
```

**Action requise :** S'assurer que TOUS les modèles ont un pricing valide (migration DB ou validation à la création).

---

## 3. Architecture

### 3.1 Diagramme de Flux

```
┌─────────────┐
│   Client    │
└──────┬──────┘
       │
       ▼
┌─────────────────────────────────────┐
│         proxy.rs                    │
│  ┌─────────────────────────────┐    │
│  │  call_chat_with_fallback()  │    │
│  │  - Fallback loop            │    │
│  │  - Retry policy             │    │
│  └──────────────┬──────────────┘    │
└─────────────────┼───────────────────┘
                  │
        ┌─────────┴─────────┐
        │                   │
        ▼                   ▼
┌──────────────┐    ┌──────────────┐
│  Model A     │    │  Model B     │
│  (primary)   │    │  (fallback)  │
└──────┬───────┘    └──────┬───────┘
       │                   │
       ▼                   ▼
┌─────────────────────────────────────┐
│      godwit-providers               │
│  ┌──────────────────────────────┐   │
│  │  Adapter::chat()             │   │
│  │  - Call upstream             │   │
│  │  - Parse usage               │   │
│  │  - Return (response, usage)  │   │
│  └──────────────────────────────┘   │
└─────────────────────────────────────┘
                  │
                  ▼
        ┌─────────────────┐
        │  UsageReport    │
        │  - prompt_tokens│
        │  - completion   │
        │  - cache_read   │
        │  - image_count  │
        │  - audio_secs   │
        └────────┬────────┘
                 │
                 ▼
        ┌─────────────────────────┐
        │  compute_cost()         │
        │  - pricing * usage      │
        │  - Decimal precision    │
        └────────┬────────────────┘
                 │
                 ▼
        ┌─────────────────────────┐
        │  request_logs + spend   │
        │  - Insert log entry     │
        │  - Update budget spent  │
        └─────────────────────────┘
```

### 3.2 Modifications Requises

#### 3.2.1 `godwit-api/src/proxy.rs`

**Ajouter :**
- `call_chat_with_fallback()` — wrapper fallback
- `call_embeddings_with_fallback()` — idem pour embeddings
- `call_image_with_fallback()` — idem pour images
- `call_audio_with_fallback()` — idem pour audio

**Modifier :**
- `chat_completions()` — utiliser `call_chat_with_fallback()`
- `embeddings()` — utiliser `call_embeddings_with_fallback()`
- `image_generations()` — utiliser `call_image_with_fallback()`
- `audio_speech()` — utiliser `call_audio_with_fallback()`
- `audio_transcriptions()` — utiliser `call_audio_with_fallback()`

#### 3.2.2 `godwit-api/src/resilience.rs`

**Ajouter :**
- `FallbackConfig` struct
- `get_fallback_chain()` helper
- Logique de fallback (déjà partiellement là avec `with_retry()`)

#### 3.2.3 `godwit-providers/src/*.rs` (providers)

**Modifier :**
- `anthropic.rs` — parser `usage` en non-streaming
- `gemini.rs` — parser `usageMetadata` en non-streaming
- `azure_openai.rs` — parser `usage` (format OpenAI)
- `llama_cpp.rs` — parser `usage` (format OpenAI)
- `ollama.rs` — parser `usage` (format OpenAI)
- `vllm.rs` — parser `usage` (format OpenAI)
- `sglang.rs` — parser `usage` (format OpenAI)

#### 3.2.4 `godwit-db/migrations/`

**Ajouter :**
- Migration pour ajouter `request_logs.attempt_number`
- Migration pour ajouter `request_logs.fallback_triggered`
- Migration optionnelle : valider/peupler `models.pricing`

---

## 4. Tests

### 4.1 Tests Unitaires

```rust
// proxy_fallback.rs
#[tokio::test]
async fn fallback_succeeds_on_first_failure() {
    // Mock: Model A échoue, Model B réussit
    // Assert: fallback triggered, B's response returned
}

#[tokio::test]
async fn fallback_exhausted_returns_last_error() {
    // Mock: Tous les modèles échouent
    // Assert: dernière erreur retournée, tous les attempts loggés
}

// usage_tracking.rs
#[test]
fn anthropic_usage_parsed_correctly() {
    // Assert: input_tokens, output_tokens extraits
}

#[test]
fn gemini_usage_parsed_correctly() {
    // Assert: prompt_token_count, candidates_token_count extraits
}
```

### 4.2 Tests d'Intégration

```bash
# Fallback
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer $KEY" \
  -d '{"model": "gpt-4o", "messages": [{"role": "user", "content": "test"}]}'

# Si OpenAI down → fallback Anthropic → fallback Gemini

# Usage tracking
curl http://localhost:3000/api/v1/spend/logs?api_key_id=$KEY_ID
# Doit montrer tokens, coût, modèle utilisé (après fallback)
```

---

## 5. Critères de Succès

### 5.1 Fallback

- [ ] Fallback configurable par modèle (YAML + DB)
- [ ] Fallback déclenché sur 5xx/timeout/429
- [ ] Pas de fallback sur 4xx
- [ ] Logging complet des attempts
- [ ] Tests : fallback success, fallback exhausté

### 5.2 Usage Tracking

- [ ] TOUS les providers remontent le usage chat
- [ ] Image/audio remontent des estimateurs
- [ ] Tests : parsing usage par provider
- [ ] Tests : compute_cost() pour chaque modalité

### 5.3 Cost Layer

- [ ] Pricing requis pour tous les modèles (validation)
- [ ] `compute_cost()` unifié entre providers et admin
- [ ] Tests : coûts cohérents avec pricing

---

## 6. Risques & Mitigations

| Risque | Impact | Mitigation |
|--------|--------|------------|
| Fallback infini | Boucle, coût explosif | Max 3 attempts, logging |
| Usage parsing break | Cost tracking faux | Tests par provider, fallback à 0 si parse échoue |
| Pricing manquant | Coûts à 0 | Validation à la création de modèle |
| Latence fallback | UX dégradée | Timeout par attempt, circuit breaker |

---

## 7. Timeline Estimée

| Feature | Complexité | Temps |
|---------|------------|-------|
| Fallback core | Moyenne | 1-2 jours |
| Fallback logging | Faible | 0.5 jour |
| Usage tracking (7 providers) | Moyenne | 2-3 jours |
| Cost layer consolidation | Faible | 0.5 jour |
| Tests + docs | Moyenne | 1-2 jours |
| **Total** | | **6-10 jours** |

---

## 8. Notes

- **Load balancing et rate limiting sont DÉJÀ IMPLÉMENTÉS** — l'audit était obsolète
- **Circuit breaker est DÉJÀ IMPLÉMENTÉ** (P0.4)
- **Cette spec se concentre sur les vrais gaps : fallback + usage tracking**

---

## 9. Prochaines Étapes

1. ✅ Review de cette spec par l'utilisateur
2. ⏳ Créer le plan d'implémentation (writing-plans skill)
3. ⏳ Implémenter fallback
4. ⏳ Implémenter usage tracking par provider
5. ⏳ Tests + docs
6. ⏳ Release v1.1.0
