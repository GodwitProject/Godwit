# P2-B: Endpoints Manquants (Moderation, Rerank, Batch) — Design Spec

**Date:** 2026-08-07  
**Auteur:** Thomas (avec opencode)  
**Statut:** En review  
**Release cible:** v1.3.0

---

## 1. Vue d'Ensemble

Cette spec couvre les fonctionnalités P2-B pour compléter la parité LiteLLM sur les endpoints manquants : moderation, rerank, et batch API unifiée multi-provider.

### 1.1 Contexte

P1 + P2-A ont livré :
- ✅ Fallback/failover entre providers
- ✅ Usage tracking complet (9/9 providers)
- ✅ Tool-calling générique avec résolution MCP + web search
- ✅ Multimodal (images)
- ✅ JSON Schema / structured output
- ✅ Agentic loop (max 4 itérations)

**Ce qui manque :**
- ❌ Moderation endpoint (existe déjà, OpenAI-only)
- ❌ Rerank endpoint (existe déjà, Cohere-only)
- ❌ Batch API unifiée multi-provider (partiellement implémentée)

---

## 2. Fonctionnalités P2-B

### 2.1 Moderation Endpoint (Amélioré)

**Actuel :** `/v1/moderations` → OpenAI API only (passthrough)

**P2-B :** Fallback chain configurable + self-hosted

#### 2.1.1 Fallback Chain

```yaml
# config.yaml
moderation:
  provider_order: ["openai", "azure", "self_hosted"]
  self_hosted_url: "http://localhost:8000/moderate"
  timeout_per_provider_secs: 10
```

**Flux :**
```
Client → Godwit /v1/moderations
  ↓
Try OpenAI Moderations API
  ↓ (5xx/timeout)
Try Azure OpenAI Moderations API
  ↓ (5xx/timeout)
Try Self-Hosted Endpoint
  ↓ (success)
Return { flagged: bool, categories: {...}, category_scores: {...} }
```

#### 2.1.2 Response Normalization

Chaque provider a un format différent. Godwit normalise vers le format OpenAI :

```rust
// crates/godwit-core/src/lib.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationResponse {
    pub id: String,
    pub model: String,
    pub results: Vec<ModerationResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationResult {
    pub flagged: bool,
    pub categories: serde_json::Value,  // {"hate": false, "self-harm": false, ...}
    pub category_scores: serde_json::Value,  // {"hate": 0.001, "self-harm": 0.0001, ...}
}

// Provider-specific adapters
// - OpenAI: direct mapping
// - Azure: direct mapping (même format)
// - Self-hosted: adapter vers format OpenAI
```

---

### 2.2 Rerank Endpoint (Amélioré)

**Actuel :** `/v1/rerank` → Cohere API only (passthrough)

**P2-B :** Fallback chain configurable + self-hosted

#### 2.2.1 Fallback Chain

```yaml
# config.yaml
rerank:
  provider_order: ["cohere", "azure", "self_hosted"]
  self_hosted_url: "http://localhost:8000/rerank"
  timeout_per_provider_secs: 15
```

**Flux :**
```
Client → Godwit /v1/rerank
  ↓
Try Cohere Rerank API
  ↓ (5xx/timeout)
Try Azure AI Search Reranker
  ↓ (5xx/timeout)
Try Self-Hosted Endpoint
  ↓ (success)
Return { id, results: [{index, relevance_score}, ...] }
```

#### 2.2.2 Response Normalization

```rust
// crates/godwit-core/src/lib.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankRequest {
    pub query: String,
    pub documents: Vec<RerankDocument>,
    pub top_n: Option<usize>,
    pub return_documents: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RerankDocument {
    Text(String),
    Object { text: String, metadata: serde_json::Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResponse {
    pub id: String,
    pub results: Vec<RerankResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResult {
    pub index: usize,
    pub relevance_score: f64,
    pub document: Option<RerankDocument>,  // Si return_documents=true
}
```

---

### 2.3 Batch API Unifiée (NOUVEAU)

**Actuel :** Partiellement implémenté (retrieve/list/cancel existent, create manque)

**P2-B :** Batch API complète unifiée multi-provider

#### 2.3.1 Unified Format (Compatible LiteLLM)

```json
// Fichier JSONL (1 request par ligne)
{"custom_id": "req-1", "method": "POST", "url": "/v1/chat/completions", "body": {"model": "gpt-4o", "messages": [...]}}
{"custom_id": "req-2", "method": "POST", "url": "/v1/chat/completions", "body": {"model": "gpt-4o", "messages": [...]}}
{"custom_id": "req-3", "method": "POST", "url": "/v1/embeddings", "body": {"model": "text-embedding-3-small", "input": "..."}}
```

#### 2.3.2 Endpoints Batch

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/v1/batches` | Créer un batch (upload JSONL) |
| `GET` | `/v1/batches/{batch_id}` | Récupérer status/progress |
| `GET` | `/v1/batches` | Lister tous les batches (paginated) |
| `DELETE` | `/v1/batches/{batch_id}` | Annuler un batch en cours |
| `GET` | `/v1/batches/{batch_id}/results` | Télécharger les résultats |

#### 2.3.3 Provider Support

| Provider | Support | Implémentation |
|----------|---------|----------------|
| **OpenAI** | ✅ Natif | Utilise `/v1/batches` OpenAI API |
| **Azure OpenAI** | ✅ Natif | Utilise `/v1/batches` Azure API |
| **Anthropic** | ⚠️ Simulé | Boucle de requests async parallèles (max 10 concurrentes) |
| **Gemini** | ⚠️ Simulé | Boucle de requests async parallèles (max 10 concurrentes) |
| **llama.cpp/Ollama/vLLM** | ⚠️ Simulé | Boucle de requests async parallèles |

**"Simulé" =** Godwit gère la boucle, le polling, et l'agrégation des résultats.

#### 2.3.4 Database Schema

```sql
-- Table batches
CREATE TABLE batches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id),
    api_key_id UUID NOT NULL REFERENCES api_keys(id),
    
    -- Provider info
    provider VARCHAR(50) NOT NULL,  -- "openai", "azure", "anthropic", etc.
    provider_batch_id TEXT,  -- ID du batch chez le provider (null si simulé)
    
    -- Status
    status VARCHAR(20) NOT NULL DEFAULT 'pending',  -- pending, processing, completed, failed, cancelled
    total_requests INTEGER NOT NULL,
    completed_requests INTEGER DEFAULT 0,
    succeeded_requests INTEGER DEFAULT 0,
    failed_requests INTEGER DEFAULT 0,
    
    -- Progress (pour batches simulés)
    progress_percent INTEGER DEFAULT 0,
    
    -- Cost
    estimated_cost_usd DECIMAL(12,4),
    actual_cost_usd DECIMAL(12,4),
    
    -- Webhook
    webhook_url TEXT,
    webhook_sent_at TIMESTAMPTZ,
    
    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    
    -- Metadata
    input_file_id TEXT,  -- Optionnel : référence à un fichier uploadé
    output_file_id TEXT,  -- Optionnel : référence aux résultats
    metadata JSONB DEFAULT '{}'::jsonb
);

-- Table batch_requests (pour tracking individuel)
CREATE TABLE batch_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    batch_id UUID NOT NULL REFERENCES batches(id) ON DELETE CASCADE,
    custom_id TEXT NOT NULL,  -- custom_id du client
    request_body JSONB NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',  -- pending, succeeded, failed
    response_body JSONB,
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    
    UNIQUE(batch_id, custom_id)
);

-- Indexes
CREATE INDEX idx_batches_org_id ON batches(organization_id);
CREATE INDEX idx_batches_status ON batches(status);
CREATE INDEX idx_batch_requests_batch_id ON batch_requests(batch_id);
CREATE INDEX idx_batch_requests_status ON batch_requests(status);
```

#### 2.3.5 Retry Automatique

Pour les batches **simulés** (Anthropic, Gemini, etc.) :

```rust
// crates/godwit-api/src/batch_processor.rs

pub struct BatchProcessor {
    max_concurrent: usize,  // Default: 10
    max_retries: u32,  // Default: 2
    retry_delay_ms: u64,  // Default: 1000 (exponential backoff)
}

impl BatchProcessor {
    pub async fn process_batch(&self, batch_id: Uuid) -> Result<(), ApiError> {
        let requests = self.get_pending_requests(batch_id).await?;
        
        // Semaphore pour limiter le concurrent
        let semaphore = Semaphore::new(self.max_concurrent);
        
        let mut tasks = Vec::new();
        for req in requests {
            let permit = semaphore.clone().acquire_owned().await?;
            tasks.push(tokio::spawn(async move {
                self.process_request_with_retry(req, permit).await
            }));
        }
        
        // Attendre tous les tasks
        let results = futures::future::join_all(tasks).await;
        
        // Aggréger résultats
        self.update_batch_status(batch_id, &results).await?;
        
        // Webhook si configuré
        self.send_webhook(batch_id).await?;
        
        Ok(())
    }
    
    async fn process_request_with_retry(
        &self,
        req: BatchRequest,
        _permit: SemaphorePermit,
    ) -> Result<BatchResponse, ApiError> {
        let mut last_error = None;
        
        for attempt in 0..self.max_retries {
            match self.execute_request(&req).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.max_retries - 1 {
                        tokio::time::sleep(
                            Duration::from_millis(self.retry_delay_ms * 2u64.pow(attempt))
                        ).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap())
    }
}
```

#### 2.3.6 Webhook on Completion

```rust
// crates/godwit-api/src/batch_webhook.rs

#[derive(Debug, Clone, Serialize)]
pub struct BatchWebhookPayload {
    pub batch_id: Uuid,
    pub status: String,  // "completed", "failed", "cancelled"
    pub total_requests: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub actual_cost_usd: Option<Decimal>,
    pub completed_at: DateTime<Utc>,
}

pub async fn send_webhook(
    webhook_url: &str,
    payload: BatchWebhookPayload,
) -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();
    
    client
        .post(webhook_url)
        .json(&payload)
        .header("X-Godwit-Event", "batch.completed")
        .header("X-Godwit-Batch-Id", payload.batch_id.to_string())
        .send()
        .await?;
    
    Ok(())
}
```

---

## 3. Architecture

### 3.1 Diagramme de Flux — Batch API

```
┌─────────────┐
│   Client    │
│ (POST JSONL)│
└──────┬──────┘
       │
       ▼
┌─────────────────────────────────────────┐
│         /v1/batches                     │
│  1. Parse JSONL                         │
│  2. Validate requests                   │
│  3. Estimate cost                       │
│  4. Create batch in DB (status=pending) │
└──────────────┬──────────────────────────┘
               │
               ▼
      ┌────────────────┐
      │ Provider = ?   │
      └───────┬────────┘
              │
    ┌─────────┼─────────┐
    │                   │
    ▼                   ▼
┌─────────┐       ┌─────────────┐
│ OpenAI  │       │ Anthropic/  │
│ Azure   │       │ Gemini/etc. │
│ (natif) │       │ (simulé)    │
└────┬────┘       └──────┬──────┘
     │                   │
     ▼                   ▼
┌─────────────────────────────────┐
│  Submit to Provider API         │
│  OR                             │
│  Spawn BatchProcessor (async)   │
└──────────────┬──────────────────┘
               │
               ▼
      ┌────────────────┐
      │ Poll Progress  │
      │ (natif) OR     │
      │ Track locally  │
      │ (simulé)       │
      └───────┬────────┘
              │
              ▼
      ┌────────────────┐
      │ Update DB      │
      │ completed_requests,
      │ succeeded, failed
      └───────┬────────┘
              │
              ▼
      ┌────────────────┐
      │ Completed?     │
      │ Yes → Webhook  │
      └───────┬────────┘
              │
              ▼
      ┌────────────────┐
      │ Client GET     │
      │ /batches/{id}  │
      │ or             │
      │ /batches/{id}/ │
      │ results        │
      └────────────────┘
```

### 3.2 Fichiers à Créer/Modifier

**Nouveaux Fichiers :**
- `crates/godwit-db/migrations/20260811000001_batches.up.sql` — Tables batches + batch_requests
- `crates/godwit-db/migrations/20260811000001_batches.down.sql` — Rollback
- `crates/godwit-api/src/batch_processor.rs` — Batch processor avec retry + concurrent limit
- `crates/godwit-api/src/batch_webhook.rs` — Webhook sender
- `crates/godwit-api/src/moderation_fallback.rs` — Moderation fallback chain
- `crates/godwit-api/src/rerank_fallback.rs` — Rerank fallback chain

**Fichiers à Modifier :**
- `crates/godwit-api/src/proxy.rs` — Ajouter endpoints batch (/v1/batches, /v1/batches/{id}, etc.)
- `crates/godwit-api/src/admin/moderation.rs` — Remplacer passthrough par fallback chain
- `crates/godwit-api/src/admin/rerank.rs` — Remplacer passthrough par fallback chain
- `crates/godwit-core/src/lib.rs` — DTOs : ModerationRequest/Response, RerankRequest/Response, Batch structs
- `crates/godwit-providers/src/openai.rs` — Ajouter batch methods (create, retrieve, cancel)
- `crates/godwit-providers/src/azure_openai.rs` — Idem
- `crates/godwit-providers/src/anthropic.rs` — Idem (simulé)
- `crates/godwit-providers/src/gemini.rs` — Idem (simulé)

---

## 4. Tests

### 4.1 Tests Unitaires

```rust
// batch_processor_tests.rs
#[tokio::test]
async fn test_batch_processor_retry_on_failure() {
    // Mock: request échoue 2 fois, réussit à la 3ème
    // Assert: retry_count = 2, status = succeeded
}

#[tokio::test]
async fn test_batch_processor_max_retries_exceeded() {
    // Mock: request échoue 3 fois (max_retries=2)
    // Assert: status = failed, error_message set
}

#[tokio::test]
async fn test_batch_processor_concurrent_limit() {
    // Mock: 20 requests, max_concurrent=10
    // Assert: jamais plus de 10 requests en parallèle
}

// moderation_fallback_tests.rs
#[tokio::test]
async fn test_moderation_fallback_openai_to_azure() {
    // Mock: OpenAI retourne 503, Azure réussit
    // Assert: fallback triggered, Azure response returned
}

// rerank_fallback_tests.rs
#[tokio::test]
async fn test_rerank_fallback_cohere_to_self_hosted() {
    // Mock: Cohere timeout, self-hosted réussit
    // Assert: fallback triggered, self-hosted response returned
}
```

### 4.2 Tests d'Intégration

```bash
# Batch API
curl -X POST http://localhost:3000/v1/batches \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/jsonl" \
  --data-binary @requests.jsonl

curl http://localhost:3000/v1/batches/{batch_id}
# Response: { status: "processing", progress_percent: 45, ... }

curl http://localhost:3000/v1/batches/{batch_id}/results
# Response: JSONL avec résultats

# Moderation
curl -X POST http://localhost:3000/v1/moderations \
  -H "Authorization: Bearer $KEY" \
  -d '{"input": "I want to kill myself."}'
# Response: { flagged: true, categories: {self-harm: true}, ... }

# Rerank
curl -X POST http://localhost:3000/v1/rerank \
  -H "Authorization: Bearer $KEY" \
  -d '{"query": "machine learning", "documents": ["deep learning", "cooking recipes", "neural networks"]}'
# Response: { results: [{index: 0, relevance_score: 0.95}, {index: 2, relevance_score: 0.87}, ...] }
```

---

## 5. Critères de Succès

### 5.1 Moderation

- [ ] Fallback chain configurable (OpenAI → Azure → Self-hosted)
- [ ] Response normalisée (format OpenAI)
- [ ] Timeout per provider (configurable)
- [ ] Tests : fallback triggered, response normalized

### 5.2 Rerank

- [ ] Fallback chain configurable (Cohere → Azure → Self-hosted)
- [ ] Response normalisée (format Cohere-like)
- [ ] Timeout per provider (configurable)
- [ ] Tests : fallback triggered, response normalized

### 5.3 Batch API

- [ ] Endpoints : POST /batches, GET /batches/{id}, GET /batches, DELETE /batches/{id}, GET /batches/{id}/results
- [ ] Unified format (JSONL, compatible LiteLLM)
- [ ] Providers : OpenAI/Azure (natif), Anthropic/Gemini/autres (simulé)
- [ ] Retry automatique (max 2 retries, exponential backoff)
- [ ] Concurrent limit (max 10 requests parallèles)
- [ ] Webhook on completion (configurable)
- [ ] Cost tracking (estimé + réel)
- [ ] Progress tracking (temps réel pour simulé, polling pour natif)
- [ ] Tests : batch creation, retry, webhook, cost tracking

---

## 6. Risques & Mitigations

| Risque | Impact | Mitigation |
|--------|--------|------------|
| Batch simulé trop lent | Timeout client | Async processing, webhook notify |
| Retry infini | Coût explosif | Max 2 retries, logging |
| Webhook échoue | Client pas notifié | Retry webhook (3x), fallback polling |
| JSONL invalide | Batch rejeté | Validation avant submit, erreur claire |
| Provider down pendant batch | Batch bloqué | Fallback chain, timeout per request |

---

## 7. Timeline Estimée

| Feature | Complexité | Temps |
|---------|------------|-------|
| Moderation fallback | Faible | 0.5 jour |
| Rerank fallback | Faible | 0.5 jour |
| Batch DB schema + migrations | Moyenne | 0.5 jour |
| Batch unified format parsing | Moyenne | 1 jour |
| Batch OpenAI/Azure (natif) | Moyenne | 1-2 jours |
| Batch simulé (Anthropic/Gemini) | Haute | 2-3 jours |
| Retry + concurrent limit | Moyenne | 1 jour |
| Webhook + cost tracking | Faible | 0.5 jour |
| Tests + docs | Moyenne | 2 jours |
| **Total** | | **9-11 jours** |

---

## 8. Notes

- **JSONL uniquement** — pas de support JSON array (YAGNI)
- **Batch simulé** = Godwit gère la boucle, pas le provider
- **Webhook optionnel** — configurable par batch
- **Cost tracking** : estimé avant submit (prix catalog), réel après completion

---

## 9. Prochaines Étapes

1. ✅ Review de cette spec par l'utilisateur
2. ⏳ Créer le plan d'implémentation (writing-plans skill)
3. ⏳ Implémenter moderation/rerank fallback
4. ⏳ Implémenter batch API (DB, parsing, providers)
5. ⏳ Tests + docs
6. ⏳ Release v1.3.0
