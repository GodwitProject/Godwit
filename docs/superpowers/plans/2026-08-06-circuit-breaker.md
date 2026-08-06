# Circuit Breaker for Providers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a circuit breaker pattern to temporarily disable providers that fail repeatedly, preventing wasted requests and improving system resilience.

**Architecture:** Create a `CircuitBreaker` struct with three states (Closed, Open, HalfOpen), a concurrent registry mapping provider IDs to breakers, and integrate it into the existing retry/fallback flow in `proxy.rs`. Add a monitoring endpoint and configuration in `AppConfig`.

**Tech Stack:** Rust, `dashmap` for concurrent HashMap, `std::sync::atomic` for lock-free counters, axum for the monitoring endpoint, SQLx for persistence (request logging).

## Global Constraints

- Use `dashmap::DashMap` for the concurrent registry (same pattern as `godwit-cache` and `rate_limit.rs`)
- Follow existing error handling patterns (`ProviderError`, `ApiError`)
- Circuit breaker config goes in `AppConfig` under `circuit_breaker` section
- Monitoring endpoint: `GET /api/v1/circuit-breakers`
- Integration point: `with_retry` in `resilience.rs` and fallback loop in `proxy.rs`
- Tests must cover: state transitions, timeout behavior, monitoring endpoint

---

### Task 1: CircuitBreaker State Enum and Struct

**Files:**
- Create: `crates/godwit-api/src/circuit_breaker.rs`

**Interfaces:**
- Produces: `pub enum CircuitState`, `pub struct CircuitBreaker`
- Produces: Methods `can_execute()`, `record_success()`, `record_failure()`, `state()`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn starts_in_closed_state() {
        let cb = CircuitBreaker::new(5, Duration::from_secs(60), 3);
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.can_execute());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /home/thomas/work/Godwit && export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo test -p godwit-api circuit_breaker::tests::starts_in_closed_state -- --nocapture`
Expected: FAIL with "unresolved import" or "function not defined"

- [ ] **Step 3: Write minimal implementation**

```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{RwLock, Arc};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    failures: AtomicUsize,
    successes: AtomicUsize,
    last_failure: RwLock<Option<Instant>>,
    state: RwLock<CircuitState>,
    threshold: usize,
    timeout: Duration,
    half_open_max: usize,
    half_open_requests: AtomicUsize,
}

impl CircuitBreaker {
    pub fn new(threshold: usize, timeout: Duration, half_open_max: usize) -> Self {
        Self {
            failures: AtomicUsize::new(0),
            successes: AtomicUsize::new(0),
            last_failure: RwLock::new(None),
            state: RwLock::new(CircuitState::Closed),
            threshold,
            timeout,
            half_open_max,
            half_open_requests: AtomicUsize::new(0),
        }
    }

    pub fn can_execute(&self) -> bool {
        let current_state = self.state();
        match current_state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                let last_failure = self.last_failure.read().unwrap();
                if let Some(last) = *last_failure {
                    last.elapsed() >= self.timeout
                } else {
                    true
                }
            }
            CircuitState::HalfOpen => {
                self.half_open_requests.load(Ordering::SeqCst) < self.half_open_max
            }
        }
    }

    pub fn record_success(&self) {
        self.successes.fetch_add(1, Ordering::SeqCst);
        let mut state = self.state.write().unwrap();
        match *state {
            CircuitState::HalfOpen => {
                *state = CircuitState::Closed;
                self.failures.store(0, Ordering::SeqCst);
                self.half_open_requests.store(0, Ordering::SeqCst);
            }
            CircuitState::Closed => {
                self.failures.store(0, Ordering::SeqCst);
            }
            CircuitState::Open => {}
        }
    }

    pub fn record_failure(&self) {
        self.failures.fetch_add(1, Ordering::SeqCst);
        *self.last_failure.write().unwrap() = Some(Instant::now());
        
        let mut state = self.state.write().unwrap();
        match *state {
            CircuitState::Closed => {
                if self.failures.load(Ordering::SeqCst) >= self.threshold {
                    *state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                *state = CircuitState::Open;
                self.half_open_requests.store(0, Ordering::SeqCst);
            }
            CircuitState::Open => {}
        }
    }

    pub fn state(&self) -> CircuitState {
        let current = *self.state.read().unwrap();
        if current == CircuitState::Open {
            let last_failure = self.last_failure.read().unwrap();
            if let Some(last) = *last_failure {
                if last.elapsed() >= self.timeout {
                    return CircuitState::HalfOpen;
                }
            }
        }
        current
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /home/thomas/work/Godwit && export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo test -p godwit-api circuit_breaker::tests::starts_in_closed_state -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
cd /home/thomas/work/Godwit
git add crates/godwit-api/src/circuit_breaker.rs
git commit -m "feat: add CircuitBreaker struct with state management"
```

### Task 2: CircuitBreaker State Transitions Tests

**Files:**
- Modify: `crates/godwit-api/src/circuit_breaker.rs:100-150`

**Interfaces:**
- Consumes: `CircuitBreaker::new()`, `can_execute()`, `record_success()`, `record_failure()`, `state()`
- Produces: Test functions for state transitions

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn circuit_opens_after_threshold_failures() {
    let cb = CircuitBreaker::new(3, Duration::from_secs(60), 2);
    for _ in 0..3 {
        cb.record_failure();
    }
    assert_eq!(cb.state(), CircuitState::Open);
    assert!(!cb.can_execute());
}

#[test]
fn circuit_half_opens_after_timeout() {
    let cb = CircuitBreaker::new(2, Duration::from_millis(100), 2);
    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);
    
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(cb.state(), CircuitState::HalfOpen);
    assert!(cb.can_execute());
}

#[test]
fn success_in_half_open_closes_circuit() {
    let cb = CircuitBreaker::new(2, Duration::from_millis(50), 2);
    cb.record_failure();
    cb.record_failure();
    std::thread::sleep(Duration::from_millis(60));
    
    assert_eq!(cb.state(), CircuitState::HalfOpen);
    cb.record_success();
    assert_eq!(cb.state(), CircuitState::Closed);
    assert!(cb.can_execute());
}

#[test]
fn failure_in_half_open_reopens_circuit() {
    let cb = CircuitBreaker::new(2, Duration::from_millis(50), 2);
    cb.record_failure();
    cb.record_failure();
    std::thread::sleep(Duration::from_millis(60));
    
    assert_eq!(cb.state(), CircuitState::HalfOpen);
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /home/thomas/work/Godwit && export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo test -p godwit-api circuit_breaker::tests -- --nocapture`
Expected: Some tests FAIL (timeout-related tests may have timing issues initially)

- [ ] **Step 3: Fix implementation if needed**

Adjust the `can_execute()` and `state()` methods to properly handle the timeout check.

- [ ] **Step 4: Run test to verify all pass**

Run: `cd /home/thomas/work/Godwit && export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo test -p godwit-api circuit_breaker::tests -- --nocapture`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
cd /home/thomas/work/Godwit
git add crates/godwit-api/src/circuit_breaker.rs
git commit -m "test: add circuit breaker state transition tests"
```

### Task 3: CircuitBreakerRegistry

**Files:**
- Modify: `crates/godwit-api/src/circuit_breaker.rs` (add at end)

**Interfaces:**
- Consumes: `CircuitBreaker`
- Produces: `pub struct CircuitBreakerRegistry` with methods `get()`, `record_success()`, `record_failure()`, `all_states()`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod registry_tests {
    use super::*;

    #[test]
    fn registry_creates_breaker_on_demand() {
        let registry = CircuitBreakerRegistry::new(5, Duration::from_secs(60), 3);
        let breaker = registry.get("provider-1");
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[test]
    fn registry_records_per_provider() {
        let registry = CircuitBreakerRegistry::new(2, Duration::from_secs(60), 2);
        registry.record_failure("provider-1");
        registry.record_failure("provider-1");
        
        let p1 = registry.get("provider-1");
        let p2 = registry.get("provider-2");
        
        assert_eq!(p1.state(), CircuitState::Open);
        assert_eq!(p2.state(), CircuitState::Closed);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /home/thomas/work/Godwit && export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo test -p godwit-api circuit_breaker::registry_tests -- --nocapture`
Expected: FAIL with "unresolved import" or "function not defined"

- [ ] **Step 3: Write minimal implementation**

```rust
use dashmap::DashMap;

pub struct CircuitBreakerRegistry {
    breakers: DashMap<String, Arc<CircuitBreaker>>,
    threshold: usize,
    timeout: Duration,
    half_open_max: usize,
}

impl CircuitBreakerRegistry {
    pub fn new(threshold: usize, timeout: Duration, half_open_max: usize) -> Self {
        Self {
            breakers: DashMap::new(),
            threshold,
            timeout,
            half_open_max,
        }
    }

    pub fn get(&self, provider_id: &str) -> Arc<CircuitBreaker> {
        self.breakers
            .entry(provider_id.to_string())
            .or_insert_with(|| {
                Arc::new(CircuitBreaker::new(
                    self.threshold,
                    self.timeout,
                    self.half_open_max,
                ))
            })
            .clone()
    }

    pub fn record_success(&self, provider_id: &str) {
        let breaker = self.get(provider_id);
        breaker.record_success();
    }

    pub fn record_failure(&self, provider_id: &str) {
        let breaker = self.get(provider_id);
        breaker.record_failure();
    }

    pub fn all_states(&self) -> Vec<(String, CircuitState)> {
        self.breakers
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().state()))
            .collect()
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /home/thomas/work/Godwit && export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo test -p godwit-api circuit_breaker::registry_tests -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
cd /home/thomas/work/Godwit
git add crates/godwit-api/src/circuit_breaker.rs
git commit -m "feat: add CircuitBreakerRegistry with DashMap"
```

### Task 4: Add CircuitBreakerConfig to AppConfig

**Files:**
- Modify: `crates/godwit-core/src/lib.rs:24-36` (add CircuitBreakerConfig struct and field in AppConfig)

**Interfaces:**
- Produces: `pub struct CircuitBreakerConfig`
- Modifies: `pub struct AppConfig` to include `circuit_breaker: Option<CircuitBreakerConfig>`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn circuit_breaker_config_parses_from_yaml() {
    let yaml = r#"
server:
  host: 127.0.0.1
  port: 3000
  request_timeout_seconds: 60
database:
  url: postgres://user:pass@localhost/pasteurllm
auth:
  jwt_secret: supersecret
  access_token_ttl_minutes: 15
  refresh_token_ttl_days: 7
  oidc_providers: []
  saml_providers: []
circuit_breaker:
  failure_threshold: 5
  recovery_timeout_secs: 60
  half_open_max_requests: 3
"#;
    let config: AppConfig = serde_yaml::from_str(yaml).expect("parse yaml");
    let cb = config.circuit_breaker.expect("circuit_breaker config present");
    assert_eq!(cb.failure_threshold, 5);
    assert_eq!(cb.recovery_timeout_secs, 60);
    assert_eq!(cb.half_open_max_requests, 3);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /home/thomas/work/Godwit && export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo test -p godwit-core circuit_breaker_config_parses_from_yaml -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Write minimal implementation**

Add after `CompatConfig` (around line 112):

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: usize,
    pub recovery_timeout_secs: u64,
    pub half_open_max_requests: usize,
}
```

Modify `AppConfig` (around line 34):

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    /// Agentic ecosystem wiring: MCP tool servers and the SearXNG web-search backend.
    #[serde(default)]
    pub agentic: AgenticConfig,
    /// Compatibility flags for wire-format interoperability.
    #[serde(default)]
    pub compat: Option<CompatConfig>,
    /// Circuit breaker configuration for provider resilience.
    #[serde(default)]
    pub circuit_breaker: Option<CircuitBreakerConfig>,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /home/thomas/work/Godwit && export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo test -p godwit-core circuit_breaker_config_parses_from_yaml -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
cd /home/thomas/work/Godwit
git add crates/godwit-core/src/lib.rs
git commit -m "feat: add CircuitBreakerConfig to AppConfig"
```

### Task 5: Update config.example.yaml

**Files:**
- Modify: `config.example.yaml`

**Interfaces:**
- Produces: Example circuit breaker configuration

- [ ] **Step 1: Add circuit breaker section to config.example.yaml**

```yaml
server:
  host: 0.0.0.0
  port: 3000
  request_timeout_seconds: 120

database:
  url: postgres://user:pass@localhost:5432/godwit

auth:
  jwt_secret: change-me-in-production
  access_token_ttl_minutes: 15
  refresh_token_ttl_days: 7
  oidc_providers: []
  saml_providers: []

# Circuit breaker configuration for provider resilience
circuit_breaker:
  failure_threshold: 5
  recovery_timeout_secs: 60
  half_open_max_requests: 3

# Agentic ecosystem wiring (optional). MCP servers are exposed as chat tools and SearXNG
# backs web_search-style tool calls when the selected adapter has no native web search.
agentic:
  mcp_servers: []
  # searxng:
  #   base_url: http://localhost:8080
```

- [ ] **Step 2: Verify config parses**

Run: `cd /home/thomas/work/Godwit && export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo test -p godwit-core circuit_breaker_config_parses_from_yaml -- --nocapture`
Expected: PASS (update test to match actual values if needed)

- [ ] **Step 3: Commit**

```bash
cd /home/thomas/work/Godwit
git add config.example.yaml
git commit -m "docs: add circuit_breaker example config"
```

### Task 6: Integrate Circuit Breaker into AppState

**Files:**
- Modify: `crates/godwit-api/src/state.rs`
- Modify: `crates/godwit-api/src/lib.rs` (where AppState is constructed)

**Interfaces:**
- Consumes: `CircuitBreakerRegistry`, `AppConfig::circuit_breaker`
- Produces: `AppState` with `circuit_breaker_registry: Arc<CircuitBreakerRegistry>`

- [ ] **Step 1: Modify state.rs to add circuit_breaker_registry field**

```rust
use crate::{circuit_breaker::CircuitBreakerRegistry, model_router::DbModelRouter, rate_limit::RateLimiter};
// ... rest of imports

pub struct AppState {
    pub config: AppConfig,
    pub pool: PgPool,
    pub adapter_registry: Arc<AdapterRegistry>,
    pub model_router: DbModelRouter,
    pub mcp: Arc<McpRegistry>,
    pub searxng: Option<SearxngProvider>,
    pub searxng_profile: Option<ResolvedProfile>,
    pub user_repo: UserRepository,
    pub org_repo: OrganizationRepository,
    pub team_repo: TeamRepository,
    pub team_membership_repo: TeamMembershipRepository,
    pub api_key_repo: ApiKeyRepository,
    pub refresh_token_repo: RefreshTokenRepository,
    pub end_user_repo: EndUsersRepository,
    pub api_key_cache: MemoryCache<String, ApiKey>,
    pub credential_master_key: [u8; 32],
    pub rate_limiter: RateLimiter,
    pub circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
}
```

- [ ] **Step 2: Find where AppState is constructed in lib.rs**

Run: `cd /home/thomas/work/Godwit && export PATH="/usr/local/opt/rustup/bin:$PATH" && grep -n "AppState {" crates/godwit-api/src/lib.rs | head -5`

- [ ] **Step 3: Modify AppState construction to initialize circuit_breaker_registry**

Use default values if `config.circuit_breaker` is None:
- threshold: 5
- timeout: 60 seconds
- half_open_max: 3

- [ ] **Step 4: Verify compilation**

Run: `cd /home/thomas/work/Godwit && export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo check -p godwit-api`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
cd /home/thomas/work/Godwit
git add crates/godwit-api/src/state.rs crates/godwit-api/src/lib.rs
git commit -m "feat: wire CircuitBreakerRegistry into AppState"
```

### Task 7: Integrate Circuit Breaker into Retry/Fallback Flow

**Files:**
- Modify: `crates/godwit-api/src/proxy.rs` (call_chat_agentic, call_embedding, call_image_generation, etc.)
- Modify: `crates/godwit-api/src/resilience.rs` (with_retry function)

**Interfaces:**
- Consumes: `AppState::circuit_breaker_registry`
- Produces: Circuit breaker checks before provider calls

- [ ] **Step 1: Modify with_retry to accept circuit breaker**

In `resilience.rs`, add optional circuit breaker parameter:

```rust
pub async fn with_retry<F, Fut, T>(
    policy: &RetryPolicy,
    circuit_breaker: Option<&CircuitBreaker>,
    f: F,
) -> Result<T, ProviderError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, ProviderError>>,
{
    // Check circuit breaker before attempting
    if let Some(cb) = circuit_breaker {
        if !cb.can_execute() {
            return Err(ProviderError::Provider("circuit breaker is open".to_string()));
        }
    }
    
    let mut last_err = None;
    for attempt in 0..=policy.max_retries {
        match f().await {
            Ok(v) => {
                if let Some(cb) = circuit_breaker {
                    cb.record_success();
                }
                return Ok(v);
            }
            Err(e) => {
                if let Some(cb) = circuit_breaker {
                    cb.record_failure();
                }
                // ... rest of retry logic
```

- [ ] **Step 2: Update call_chat_agentic to pass circuit breaker**

In `proxy.rs`, modify `call_chat_agentic` and related functions to extract provider ID from `resolved` and pass the circuit breaker to `with_retry`.

- [ ] **Step 3: Update fallback chain to check circuit breaker**

In the fallback loop (around line 558-579), skip providers whose circuit breaker is open:

```rust
for fallback_id in fallback_chain {
    // Check circuit breaker before attempting fallback
    let cb = state.circuit_breaker_registry.get(&fallback_id);
    if !cb.can_execute() {
        tracing::info!("skipping {} due to open circuit breaker", fallback_id);
        continue;
    }
    // ... rest of fallback logic
```

- [ ] **Step 4: Verify compilation**

Run: `cd /home/thomas/work/Godwit && export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo check -p godwit-api`
Expected: May have errors - fix them

- [ ] **Step 5: Commit**

```bash
cd /home/thomas/work/Godwit
git add crates/godwit-api/src/resilience.rs crates/godwit-api/src/proxy.rs
git commit -m "feat: integrate circuit breaker into retry/fallback flow"
```

### Task 8: Create Monitoring Endpoint

**Files:**
- Create: `crates/godwit-api/src/admin/circuit_breakers.rs`
- Modify: `crates/godwit-api/src/admin/mod.rs`

**Interfaces:**
- Produces: `GET /api/v1/circuit-breakers` endpoint
- Response: `{"breakers": [{"provider_id": "...", "state": "closed|open|half_open"}]}`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    
    // Test that the endpoint returns correct structure
}
```

- [ ] **Step 2: Create circuit_breakers.rs**

```rust
use axum::{extract::State, http::StatusCode, Json};
use std::sync::Arc;

use crate::state::AppState;

#[derive(serde::Serialize)]
pub struct CircuitBreakerStatus {
    pub provider_id: String,
    pub state: String,
}

#[derive(serde::Serialize)]
pub struct CircuitBreakersResponse {
    pub breakers: Vec<CircuitBreakerStatus>,
}

pub async fn list_circuit_breakers(
    State(state): State<Arc<AppState>>,
) -> Json<CircuitBreakersResponse> {
    let breakers = state
        .circuit_breaker_registry
        .all_states()
        .into_iter()
        .map(|(provider_id, state)| CircuitBreakerStatus {
            provider_id,
            state: match state {
                crate::circuit_breaker::CircuitState::Closed => "closed".to_string(),
                crate::circuit_breaker::CircuitState::Open => "open".to_string(),
                crate::circuit_breaker::CircuitState::HalfOpen => "half_open".to_string(),
            },
        })
        .collect();

    Json(CircuitBreakersResponse { breakers })
}

pub fn router() -> axum::Router<Arc<AppState>> {
    axum::Router::new()
        .route("/circuit-breakers", get(list_circuit_breakers))
}
```

- [ ] **Step 3: Wire into admin/mod.rs**

Add the module and merge the router into the admin router.

- [ ] **Step 4: Verify compilation**

Run: `cd /home/thomas/work/Godwit && export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo check -p godwit-api`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
cd /home/thomas/work/Godwit
git add crates/godwit-api/src/admin/circuit_breakers.rs crates/godwit-api/src/admin/mod.rs
git commit -m "feat: add GET /api/v1/circuit-breakers monitoring endpoint"
```

### Task 9: Integration Tests

**Files:**
- Modify: `crates/godwit-api/src/circuit_breaker.rs` (add more comprehensive tests)

**Interfaces:**
- Tests all circuit breaker behaviors

- [ ] **Step 1: Add half-open request limiting test**

```rust
#[test]
fn half_open_limits_concurrent_requests() {
    let cb = CircuitBreaker::new(2, Duration::from_millis(50), 2);
    cb.record_failure();
    cb.record_failure();
    std::thread::sleep(Duration::from_millis(60));
    
    assert_eq!(cb.state(), CircuitState::HalfOpen);
    assert!(cb.can_execute()); // First request allowed
    assert!(cb.can_execute()); // Second request allowed
    assert!(!cb.can_execute()); // Third request blocked
}
```

- [ ] **Step 2: Run all circuit breaker tests**

Run: `cd /home/thomas/work/Godwit && export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo test -p godwit-api circuit_breaker -- --nocapture`
Expected: All tests PASS

- [ ] **Step 3: Commit**

```bash
cd /home/thomas/work/Godwit
git add crates/godwit-api/src/circuit_breaker.rs
git commit -m "test: add comprehensive circuit breaker integration tests"
```

### Task 10: Documentation

**Files:**
- Create: `docs/circuit-breaker.md`

**Interfaces:**
- Documents the circuit breaker feature

- [ ] **Step 1: Write documentation**

```markdown
# Circuit Breaker

The circuit breaker pattern prevents repeated failures by temporarily disabling providers that fail repeatedly.

## How It Works

1. **Closed State**: Normal operation. Requests flow through to providers.
2. **Open State**: After `failure_threshold` consecutive failures, the circuit opens. All requests are immediately rejected.
3. **Half-Open State**: After `recovery_timeout_secs`, the circuit transitions to half-open. A limited number of test requests (`half_open_max_requests`) are allowed through.
   - If a test request succeeds, the circuit closes (resets).
   - If a test request fails, the circuit re-opens.

## Configuration

```yaml
circuit_breaker:
  failure_threshold: 5        # Number of failures before opening
  recovery_timeout_secs: 60   # Seconds before half-open
  half_open_max_requests: 3   # Max test requests in half-open state
```

## Monitoring

Check the state of all circuit breakers:

```bash
curl http://localhost:3000/api/v1/circuit-breakers
```

Response:
```json
{
  "breakers": [
    {"provider_id": "openai-gpt4", "state": "closed"},
    {"provider_id": "anthropic-claude", "state": "open"},
    {"provider_id": "ollama-local", "state": "half_open"}
  ]
}
```

## States

- `closed`: Provider is healthy, requests flow normally
- `open`: Provider is failing, requests are blocked
- `half_open`: Provider is being tested with limited requests

## Integration with Retry/Fallback

The circuit breaker integrates with the existing retry and fallback mechanism:
- Before attempting a provider, the circuit breaker is checked
- If open, the request immediately falls back to the next provider
- Success/failure is recorded after each attempt
```

- [ ] **Step 2: Commit**

```bash
cd /home/thomas/work/Godwit
git add docs/circuit-breaker.md
git commit -m "docs: add circuit breaker documentation"
```

---

## Summary

**Files to create:**
- `crates/godwit-api/src/circuit_breaker.rs`
- `crates/godwit-api/src/admin/circuit_breakers.rs`
- `docs/circuit-breaker.md`

**Files to modify:**
- `crates/godwit-core/src/lib.rs` (add CircuitBreakerConfig)
- `crates/godwit-api/src/state.rs` (add circuit_breaker_registry field)
- `crates/godwit-api/src/lib.rs` (initialize circuit_breaker_registry)
- `crates/godwit-api/src/resilience.rs` (integrate with with_retry)
- `crates/godwit-api/src/proxy.rs` (integrate into fallback chain)
- `crates/godwit-api/src/admin/mod.rs` (wire in monitoring endpoint)
- `config.example.yaml` (add example config)

**Tests:**
- Unit tests for CircuitBreaker state transitions
- Unit tests for CircuitBreakerRegistry
- Integration test for monitoring endpoint
- Config parsing test

**Total commits:** 10
