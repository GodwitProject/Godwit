# Backlog — parité LiteLLM

Issues à traiter pour combler les écarts identifiés dans
[`litellm-parity-audit.md`](./litellm-parity-audit.md). Chaque entrée correspond à une
fonctionnalité présente dans LiteLLM et absente (ou partielle) dans Godwit.

## Bloqueur fondamental

- [ ] **Élargir le DTO partagé `ChatCompletionRequest`/`ChatMessage`** (`crates/godwit-core/src/lib.rs:70-82`)
      — ajouter `tools`, `tool_choice`, contenu structuré (multi-part : texte + images), `response_format`,
      `stop`, `logprobs`, `seed`, `n`, penalties. C'est le préalable à quasiment tous les points ci-dessous.

## Tool-calling & écosystème agentique

- [ ] Function/tool calling (requête : définitions de tools ; réponse : `tool_calls` ; tool choice forcé ; appels parallèles)
- [ ] Vision / entrée multimodale (images) par provider
- [ ] Web search natif — passthrough des tools serveur (OpenAI `web_search`, Anthropic `web_search_20250305`, Gemini `google_search`), citations/annotations dans la réponse
- [ ] MCP (Model Context Protocol) — client MCP (appeler des serveurs MCP et exposer leurs tools aux modèles) et/ou serveur MCP, config d'enregistrement de serveurs MCP

## Fonctionnalités de génération

- [ ] JSON mode / structured output (`response_format` json_schema, mode strict) pour le chat
- [ ] Reasoning/thinking passthrough (extended thinking Anthropic, reasoning effort OpenAI/o-series)
- [ ] Prompt caching (`cache_control` Anthropic, caching automatique OpenAI, context caching Gemini) + tracking des cache tokens dans `UsageReport`
- [ ] Paramètres manquants : `stop`, `logprobs`, `seed`, `n`, penalties (seuls `temperature`/`max_tokens` passent actuellement)

## Résilience proxy (fonctionnalité phare LiteLLM)

- [ ] Retry / fallback / failover entre modèles ou providers en cas d'erreur
- [ ] Load balancing entre déploiements dupliqués d'un même modèle (round-robin, least-busy, latency-based) — `model_router.rs` rejette actuellement les doublons comme "ambiguous" au lieu de répartir la charge
- [ ] Rate limiting RPM/TPM appliqué — la colonne DB `rate_limit_requests_per_minute` existe mais n'est jamais lue

## Endpoints manquants

- [ ] Moderation endpoint
- [ ] Rerank endpoint
- [ ] Batch API
- [ ] Audio/image generation pour les providers non-OpenAI (actuellement `CapabilityNotSupported` partout sauf OpenAI)

## Cost / usage tracking

- [ ] Extraire le usage réel du body JSON pour les 6 providers qui renvoient `UsageReport::default()` (OpenAI, Azure, llama.cpp, Ollama, vLLM, SGLang)
- [ ] Implémenter une couche de calcul de coût/spend dans `godwit-providers/src/usage.rs` (actuellement placeholder vide)

## Streaming

- [ ] Implémenter le streaming Gemini (`chat_stream` renvoie actuellement `CapabilityNotSupported`)
- [ ] Normaliser delta/finish_reason pour OpenAI/Azure/llama.cpp/Ollama/vLLM/SGLang (actuellement relai SSE brut)

## Non audité — à vérifier ensuite

- [ ] Cache backend (Redis, caching sémantique — voir aussi comparaison Bifrost ci-dessous)
- [ ] Guardrails (masquage PII, modération pre/post-call)
- [ ] Intégrations logging/observabilité (Langfuse, Helicone, callbacks custom, Prometheus)
- [ ] Alerting (Slack/webhook sur dépassement de budget)
- [ ] Health checks / circuit breaker sur les déploiements malsains
- [ ] Pass-through endpoints vers d'autres providers non listés

## Comparaison avec Bifrost (Maxim AI) — 2026-08-04

Bifrost est l'autre alternative open-source à LiteLLM (Go, Apache 2.0, ~20 providers,
overhead ~11 µs à 5000 req/s). Il a déjà quasiment tout ce qui manque dans ce backlog, plus
deux points qu'on n'avait pas encore identifiés :

- [ ] **Architecture de plugins/middleware extensible** — pour analytics, monitoring, logique
      custom ; aucun équivalent dans Godwit aujourd'hui (nouveau gap, pas dans l'audit initial)
- [ ] **Semantic caching** — cache basé sur la similarité sémantique des requêtes, pas juste
      exact-match (recoupe l'item "cache backend" ci-dessus)

Tout le reste (failover/fallback, load balancing, MCP client+serveur, multimodal/streaming
unifié, observabilité Prometheus/tracing, mode cluster) est déjà couvert par les sections
"Tool-calling", "Résilience proxy" et "Streaming" plus haut — Bifrost sert ici de preuve que
c'est un ensemble de fonctionnalités réalisable et déjà livré par un concurrent direct.

**Point de positionnement à retenir** ([[project_vision_litellm_alternative]]) : Bifrost gate lui
aussi une partie de ses fonctionnalités derrière une offre Enterprise — guardrails, adaptive
load balancing, clustering, SSO (Okta/Entra), RBAC, VPC, audit logs, exports de logs. Il
reproduit donc le même schéma que LiteLLM (cœur open + gouvernance/scale payantes). Godwit a
déjà RBAC + OIDC/SAML + budgets team/org en open — ça reste le vrai différenciateur, mais ça ne
compense pas le retard technique tant que fallback/MCP/load balancing/caching sémantique
manquent complètement.

## Non concerné (déjà couvert côté Godwit)

Gestion SSO, budgets team/org, spend tracking, audit logs — ce que LiteLLM (et Bifrost) réservent
à leur licence entreprise est déjà présent côté admin UI (voir mémoire `godwit-admin-ui-state`),
pas un gap ici.
