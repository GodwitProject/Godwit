# P2-B: Moderation, Rerank, Batch API Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement moderation/rerank fallback chains et batch API unifiée multi-provider avec retry automatique, webhook, et cost tracking.

**Architecture:** 
- Moderation/Rerank : Fallback chain avec timeout par provider, response normalisée
- Batch API : Unified JSONL format, DB tracking, processor async avec retry + concurrent limit
- Providers : OpenAI/Azure (natif via leur batch API), autres (simulé via boucle async)

**Tech Stack:** Rust, SQLx, serde_json, tokio (semaphore, spawn), futures, reqwest

## Global Constraints

- Moderation fallback : OpenAI → Azure → Self-hosted (configurable dans `config.yaml`)
- Rerank fallback : Cohere → Azure → Self-hosted (configurable)
- Timeout per provider : 10s (moderation), 15s (rerank), configurable
- Batch API : JSONL uniquement (pas de JSON array)
- Batch retry : max 2 retries, exponential backoff (1s, 2s, 4s)
- Batch concurrent limit : max 10 requests parallèles (configurable)
- Batch webhook : optionnel, configurable par batch
- Batch cost tracking : estimé avant submit, réel après completion
- Use Decimal for all cost calculations (no floats)
- Follow existing code patterns in `godwit-api/src/proxy.rs`, `godwit-providers/src/*.rs`

---

## Task Decomposition

**Task 1:** Database migrations for batches  
**Task 2:** Moderation fallback chain  
**Task 3:** Rerank fallback chain  
**Task 4:** Batch unified format parsing (JSONL)  
**Task 5:** Batch OpenAI/Azure native integration  
**Task 6:** Batch simulated processor (Anthropic/Gemini/etc.)  
**Task 7:** Batch retry + concurrent limit  
**Task 8:** Batch webhook + cost tracking  
**Task 9:** Batch endpoints integration (POST/GET/DELETE)  
**Task 10:** Config updates (moderation/rerank fallback chains)  
**Task 11:** Integration tests and documentation  

---

## Task Briefs (Summaries)

### Task 1: Database Migrations for Batches
- **Files:** Create `crates/godwit-db/migrations/20260811000001_batches.{up,down}.sql`
- **Changes:** Tables `batches` et `batch_requests` avec indexes
- **Tests:** Run `sqlx migrate run`, verify tables exist
- **Commit:** "db: add batches and batch_requests tables"

### Task 2: Moderation Fallback Chain
- **Files:** Create `crates/godwit-api/src/moderation_fallback.rs`, modify `crates/godwit-api/src/admin/moderation.rs`
- **Changes:** `ModerationFallback` struct avec provider chain, timeout per provider, response normalization
- **Tests:** Fallback triggered, response normalized, timeout respected
- **Commit:** "feat: add moderation fallback chain (OpenAI → Azure → Self-hosted)"

### Task 3: Rerank Fallback Chain
- **Files:** Create `crates/godwit-api/src/rerank_fallback.rs`, modify `crates/godwit-api/src/admin/rerank.rs`
- **Changes:** `RerankFallback` struct avec provider chain, timeout per provider, response normalization
- **Tests:** Fallback triggered, response normalized, timeout respected
- **Commit:** "feat: add rerank fallback chain (Cohere → Azure → Self-hosted)"

### Task 4: Batch Unified Format Parsing
- **Files:** Create `crates/godwit-api/src/batch_parser.rs`
- **Changes:** Parse JSONL, validate format, estimate cost avant submit
- **Tests:** Valid JSONL parsed, invalid JSONL rejected, cost estimation correct
- **Commit:** "feat: add batch JSONL parser with cost estimation"

### Task 5: Batch OpenAI/Azure Native Integration
- **Files:** Modify `crates/godwit-providers/src/{openai,azure_openai}.rs`
- **Changes:** `create_batch()`, `retrieve_batch()`, `cancel_batch()` methods using provider batch API
- **Tests:** Batch created, retrieved, cancelled successfully
- **Commit:** "feat: add native batch support for OpenAI/Azure"

### Task 6: Batch Simulated Processor
- **Files:** Create `crates/godwit-api/src/batch_processor.rs`
- **Changes:** Async processor avec semaphore (max 10 concurrent), boucle de requests parallèles
- **Tests:** Concurrent limit respected, all requests processed
- **Commit:** "feat: add simulated batch processor for Anthropic/Gemini/etc."

### Task 7: Batch Retry + Concurrent Limit
- **Files:** Modify `crates/godwit-api/src/batch_processor.rs`
- **Changes:** Retry logic (max 2, exponential backoff), semaphore for concurrent limit
- **Tests:** Retry on failure, max retries exceeded → failed, concurrent limit enforced
- **Commit:** "feat: add retry logic and concurrent limit to batch processor"

### Task 8: Batch Webhook + Cost Tracking
- **Files:** Create `crates/godwit-api/src/batch_webhook.rs`, modify `batch_processor.rs`
- **Changes:** Webhook sender, actual cost calculation post-completion
- **Tests:** Webhook sent on completion, cost tracked correctly
- **Commit:** "feat: add batch webhook and actual cost tracking"

### Task 9: Batch Endpoints Integration
- **Files:** Modify `crates/godwit-api/src/proxy.rs`
- **Changes:** Add routes: `POST /v1/batches`, `GET /v1/batches/{id}`, `GET /v1/batches`, `DELETE /v1/batches/{id}`, `GET /v1/batches/{id}/results`
- **Tests:** Endpoints return correct responses, batch status updated
- **Commit:** "feat: add batch API endpoints"

### Task 10: Config Updates
- **Files:** Modify `crates/godwit-core/src/lib.rs` (AppConfig), `config.example.yaml`
- **Changes:** Add `moderation.provider_order`, `rerank.provider_order`, `batch.max_concurrent`, `batch.max_retries`, `batch.webhook_url`
- **Tests:** Config parsing, defaults correct, overrides work
- **Commit:** "feat: add moderation/rerank/batch config options"

### Task 11: Integration Tests + Documentation
- **Files:** Create `tests/{moderation,rerank,batch}_integration.rs`, create `docs/moderation-rerank-batch.md`
- **Changes:** Integration tests (ignored, require server+DB), documentation with examples
- **Tests:** Compile integration tests, manual run with server
- **Commit:** "docs: add P2-B integration tests and guide"

---

## Execution Notes

- **Task briefs:** Use `scripts/task-brief <plan-file> <task-N>` to generate detailed briefs with exact code snippets
- **Task reports:** Each implementer writes to `.superpowers/sdd/<plan-basename>/task-N-report.md`
- **Review package:** Use `scripts/review-package <plan-file> <base> <head>` after each task
- **Ledger:** Track progress in `.superpowers/sdd/<plan-basename>/progress.md`

---

## Success Criteria

- [ ] All 11 tasks complete and reviewed
- [ ] Moderation: fallback chain working, response normalized
- [ ] Rerank: fallback chain working, response normalized
- [ ] Batch API: JSONL parsing, native (OpenAI/Azure) + simulated (others) working
- [ ] Batch: retry (max 2), concurrent limit (10), webhook, cost tracking all functional
- [ ] Integration tests compile (marked `#[ignore]`)
- [ ] Documentation complete

---

## Timeline Estimée

- Tasks 1-3 (Migrations + Fallbacks): 2 jours
- Tasks 4-8 (Batch core): 4-5 jours
- Tasks 9-11 (Integration + docs): 2-3 jours
- **Total:** 8-10 jours
