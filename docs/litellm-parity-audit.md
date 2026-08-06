# Audit de parité LiteLLM — cœur Rust (2026-08-04)

Objectif : vérifier si `crates/godwit-providers` / `godwit-core` / `model_router.rs` couvrent
les fonctionnalités d'appel LLM que propose LiteLLM.

## Bloqueur principal

`ChatCompletionRequest` / `ChatMessage` (`crates/godwit-core/src/lib.rs:70-82`) ne portent que
`model, messages, stream, temperature, max_tokens`, et `ChatMessage.content` est une simple
`String`. Tant que ce DTO partagé reste aussi étroit, aucun adapter ne peut exposer tools,
images, JSON schema, `cache_control`, `stop`/`logprobs`/`seed`/`n` — quelle que soit la
richesse réelle de l'API du provider sous-jacent. C'est le fix à plus fort effet de levier.

## Verdicts par fonctionnalité

| # | Fonctionnalité | Verdict | Détail |
|---|---|---|---|
| 1 | Chat non-streaming | Full | Les 9 providers implémentent `chat()` (openai.rs:52, anthropic.rs:230, gemini.rs:197, azure_openai.rs:57, bedrock.rs:333, + llama.cpp/ollama/vllm/sglang) |
| 2 | Streaming (SSE) | Partial | OpenAI/Azure/llama.cpp/Ollama/vLLM/SGLang relaient le SSE brut sans normaliser delta/finish_reason (streaming.rs). Anthropic normalise correctement (anthropic.rs:178-221). Bedrock décode l'eventstream AWS (bedrock.rs:232-286). **Gemini streaming est Missing** — `chat_stream` renvoie `CapabilityNotSupported` (gemini.rs:259-268) |
| 3 | Tool/function calling | Missing | Aucun champ `tools`/`tool_choice`/`tool_calls` dans le DTO partagé ; zéro occurrence dans le crate |
| 4 | Vision / multimodal | Missing | `ChatMessage.content: String` (lib.rs:81) ne peut pas porter de parties image ; tous les adapters (anthropic.rs:42-46, gemini.rs:44-46) ne construisent que du texte |
| 5 | JSON mode / structured output | Missing (chat) | `response_format` existe seulement sur `ImageEditRequest`/`AudioTtsRequest`/`AudioSttRequest` (lib.rs:252,260,268), pas sur `ChatCompletionRequest` |
| 6 | Embeddings | Partial | OK sur OpenAI, Azure, llama.cpp, Ollama, vLLM, SGLang, openai_compatible. `CapabilityNotSupported` pour Anthropic (anthropic.rs:408-417), Gemini (gemini.rs:331-340), Bedrock (bedrock.rs:501-512) |
| 7 | Reasoning / thinking | Missing | Aucun champ `reasoning_effort`/`thinking`/`reasoning_content` ni parsing associé |
| 8 | Prompt caching | Missing | Aucun `cache_control`, aucun compteur de cache tokens dans `UsageReport` (adapter.rs:22-31) |
| 9 | Retry / fallback / failover | Missing | `DbModelRouter::resolve` fait une résolution unique, sans wrapper retry/backoff ; aucun code de retry dans le repo |
| 10 | Load balancing multi-déploiement | Missing | `resolve()` (model_router.rs:99) renvoie une erreur "ambiguous" dès que >1 modèle partage un `public_id` (lignes 150-154) au lieu de répartir la charge ; pas de round-robin/least-busy |
| 11 | Rate limiting (RPM/TPM) | Missing (non appliqué) | `rate_limit_requests_per_minute` existe en colonne DB mais n'est jamais lu dans un chemin de requête ; `PasteurError::RateLimited` n'est jamais construit |
| 12 | Cost / usage tracking | Partial | `UsageReport` capture bien les tokens pour Anthropic/Gemini/Bedrock, mais **6 providers sur 9** (OpenAI, Azure, llama.cpp, Ollama, vLLM, SGLang) renvoient `UsageReport::default()` dans `chat()` — le usage réel du body JSON n'est jamais extrait. `usage.rs` est un placeholder vide : aucune couche de calcul de coût/spend dans ce crate |
| 13 | Timeouts / cancellation | Partial | Timeout HTTP de 120s codé en dur par adapter (ex. openai.rs:27) ; pas d'override par requête, pas de propagation d'annulation ; `AppConfig.request_timeout_seconds` (lib.rs:35) n'est lu que dans un test, jamais câblé au client HTTP |
| 14 | Error normalization | Partial | Tous les adapters mappent vers `ProviderError::{Http,Serialization,Provider,CapabilityNotSupported}` (adapter.rs:39-48), mais pas de parsing du corps d'erreur spécifique au provider (ex. `error.type`/`error.code` OpenAI) — texte brut seulement |
| 15 | Moderation endpoint | Missing | Aucun code |
| 16 | Rerank endpoint | Missing | Aucun code |
| 17 | Audio / image generation | Partial, OpenAI-only | `image_generation`/`image_edit`/`audio_tts`/`audio_stt` implémentés uniquement dans openai.rs ; tous les autres providers renvoient `CapabilityNotSupported` |
| 18 | Batch API | Missing | Aucun code |
| 19 | stop / logprobs / seed / n / penalties | Missing | Aucun de ces champs n'existe sur `ChatCompletionRequest` (lib.rs:70-76) ; seuls `temperature` et `max_tokens` passent |
| 20 | Web search (tool natif OpenAI `web_search`, Anthropic `web_search_20250305`, Gemini `google_search`) | Missing | Zéro occurrence de `web_search`/`websearch`/`google_search`/`search_retrieval` dans tout le repo (`crates/`, `apps/`) ; dépend du tool-calling générique (#3), lui-même absent |
| 21 | MCP (Model Context Protocol) | Missing | Zéro occurrence réelle de `mcp` dans `crates/`/`apps/` ; le seul hit (`.gitignore:4` → `.playwright-mcp/`) est lié aux outils de test Playwright, sans rapport. Pas de routes MCP dans `godwit-api`, pas de clé `mcp_servers` dans `config.yaml`/`config.example.yaml` |

## Plus gros écarts (par impact sur la parité LiteLLM)

1. **DTO `ChatCompletionRequest`/`ChatMessage` trop étroit** — bloque simultanément tools, vision, JSON mode, reasoning, cache_control, stop/logprobs/seed/n/penalties, web search natif et MCP. Fix à traiter en premier.
2. **Aucun retry/fallback/load-balancing** — fonctionnalité phare du proxy LiteLLM (liste de modèles + fallback + stratégie de routage) sans équivalent ; `model_router.rs` rejette activement les doublons ambigus au lieu de répartir la charge.
3. **Rate limiting non appliqué** — le champ de config existe mais est inerte.
4. **Capture d'usage inégale** — 6 providers sur 9 jettent le usage réel, ce qui casse le cost tracking même si l'UI admin a déjà des pages de spend.
5. **Streaming Gemini totalement absent**, et couverture des capacités (embeddings/audio/image) très centrée OpenAI — la plupart des providers non-OpenAI sont chat-only.
6. **Moderation, rerank, batch API, web search, MCP** — absents, sans scaffolding partiel ; les deux derniers ne peuvent pas être construits avant l'ajout d'une couche de tool-calling générique.

## Contexte

Voir mémoire projet `project-vision-litellm-alternative` : Godwit vise une alternative
pleinement open-source à LiteLLM, notamment sur les fonctionnalités que LiteLLM réserve à sa
licence entreprise (SSO, budgets d'équipe, spend tracking, audit logs). Ces écarts côté
fonctionnalités d'appel LLM (tools, vision, fallback/retry, load balancing...) sont un axe de
travail distinct de l'UI admin, mais tout aussi structurant pour la parité annoncée.
