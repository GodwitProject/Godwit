# Load Balancing Implementation

## Overview

Godwit supports load balancing across multiple provider profiles for the same model. When a model `public_id` matches multiple entries in the catalog, the load balancer selects one based on the configured strategy.

## Strategies

### Round Robin (default)

Distributes requests evenly across all providers. Uses atomic counter for thread-safety.

```json
{
  "load_balance": "round_robin"
}
```

**Behavior:** Cycles through providers sequentially (0, 1, 2, 0, 1, 2...). Wrap-around is automatic.

### Least Busy

Selects the provider with fewest in-flight requests. Useful for providers with rate limits.

```json
{
  "load_balance": "least_busy"
}
```

**Behavior:** Scans all candidates, chooses the one with minimum `in_flight` counter. Ties break by index (prefers first).

### Latency-based

Selects the provider with lowest EWMA latency (alpha=0.2). Adapts to network conditions.

```json
{
  "load_balance": "latency"
}
```

**Behavior:** Tracks exponential weighted moving average of latency. First sample is raw value, subsequent samples use: `ewma = 0.2 * sample + 0.8 * old_ewma`.

## Configuration

Set strategy in model config:

```sql
-- Set least-busy strategy
UPDATE models 
SET config = jsonb_set(config, '{load_balance}', '"least_busy"')
WHERE public_id = 'gpt-4o';

-- Set latency-based strategy
UPDATE models 
SET config = jsonb_set(config, '{load_balance}', '"latency"')
WHERE public_id = 'gpt-4o';
```

All models with the same `public_id` should have the same strategy for predictable behavior.

## Implementation Details

### Architecture

- **State is per-model** (not global): Each model ID has its own `LoadBalancerState`
- **Thread-safe**: Uses `DashMap` for concurrent access, atomic operations for counters
- **In-flight tracking**: Atomic counters with RAII guard (`InFlightGuard`)
- **EWMA latency**: Stored as `f64` bits in `AtomicU64` for lock-free updates
- **No external dependencies**: Only std and `dashmap`

### Data Structures

```rust
LoadBalancerState {
    rr_index: AtomicUsize,      // Round-robin counter
    in_flight: AtomicUsize,     // Current in-flight requests
    latency_ewma: AtomicU64,    // EWMA latency (f64 bits)
}

LoadBalancer {
    states: DashMap<Uuid, Arc<LoadBalancerState>>,
}
```

### InFlightGuard

RAII pattern ensures in-flight counter is decremented even on panic:

```rust
let guard = load_balancer.increment_in_flight(model_id);
// ... make request ...
// guard.drop() automatically decrements counter
```

### EWMA Formula

```
alpha = 0.2 (fixed)
first_sample: ewma = sample
subsequent: ewma = alpha * sample + (1 - alpha) * old_ewma
```

Example: samples [1000, 200] → EWMA = 0.2×200 + 0.8×1000 = 840ms

## Testing

Unit tests in `load_balancer.rs`:

- `round_robin_cycles_through_indices`: Verifies sequential cycling and wrap-around
- `least_busy_prefers_idle_model`: Verifies selection of idle over busy
- `least_busy_ties_break_by_index`: Verifies tie-breaking behavior
- `latency_prefers_lower_ewma`: Verifies lower latency preference
- `latency_ewma_updates_correctly`: Verifies EWMA calculation formula
- `stress_concurrent_round_robin`: 10 threads × 100 iterations, no panic/deadlock
- `stress_concurrent_in_flight`: Verifies counter returns to 0 under concurrency

Integration tests in `model_router.rs`:

- `bare_public_id_load_balances_when_duplicated`: Full resolve() flow
- `load_balance_least_busy_prefers_idle_model`: End-to-end least-busy
- `load_balance_latency_prefers_lower_latency`: End-to-end latency-based

## API

### LoadBalancer Methods

```rust
LoadBalancer::new() -> Self
LoadBalancer::select_provider(strategy, model_ids) -> Option<usize>
LoadBalancer::increment_in_flight(model_id) -> InFlightGuard
LoadBalancer::record_latency(model_id, duration_ms: f64)
```

### Strategy Enum

```rust
enum LoadBalanceStrategy {
    RoundRobin,
    LeastBusy,
    Latency,
}
```

## Usage in DbModelRouter

The `DbModelRouter::resolve()` method automatically uses load balancing when multiple models match:

1. Query finds N models with same `public_id`
2. Strategy read from model config (default: RoundRobin)
3. `select_load_balanced()` calls `LoadBalancer::select_provider()`
4. Selected model's in-flight counter incremented
5. Request processed
6. In-flight counter decremented (RAII), latency recorded

## Performance Considerations

- **Atomic operations**: All counters use `SeqCst` ordering for correctness
- **DashMap**: Concurrent hash map with fine-grained locking
- **No blocking**: All operations are lock-free except DashMap access
- **Memory**: One `LoadBalancerState` per model (24 bytes + Arc overhead)
