# Load Balancing Implementation - Brainstorming Session

Date: 2026-08-06
Topic: Active Load Balancing for Multi-Provider Models

## Context Exploration

### Current State
- `LoadBalanceStrategy` enum exists in `model_router.rs:25` but is not actively used
- Routing is currently catalog-driven (single provider per model)
- Need to support N providers per model with active selection

### Requirements Summary
1. **RoundRobin**: Cycle through providers evenly
2. **LeastBusy**: Choose provider with fewest in-flight requests
3. **Latency**: Choose provider with lowest EWMA latency

## Clarifying Questions

### 1. Provider Selection Granularity

Where should provider selection happen in the request flow?

**Option A**: At `DbModelRouter::resolve()` time - select provider when resolving the model
- Pros: Clean separation, ResolvedModel contains chosen provider
- Cons: Need to track state per (model, provider) pair

**Option B**: At provider call time - resolve returns all providers, selection happens in godwit-providers
- Pros: More flexibility, can react to real-time conditions
- Cons: Blurs responsibility boundaries

**My recommendation**: Option A - keeps routing logic centralized in model_router.

### 2. State Storage Location

Where should load balancer state live?

**Option A**: Per-model state in `ModelState` struct
- Each model has its own counters/indices
- Simple, isolated

**Option B**: Global `LoadBalancer` struct with HashMap<model_id, State>
- Centralized management
- Better for cross-model analytics

**Option C**: Per-(model, provider) state
- Most granular control
- More complex indexing

**My recommendation**: Option A for MVP - per-model state is simpler and sufficient.

### 3. EWMA Parameters for Latency Strategy

What parameters for exponential weighted moving average?

- Alpha (smoothing factor): 0.1-0.3 typical
- Initial value: First sample, or configured default?

**Recommendation**: alpha=0.2, initialize with first latency sample.

### 4. Thread Safety Approach

Requirements say "no locking hot path" - how to achieve this?

**Option A**: All atomics (AtomicUsize for counts, AtomicU64 for latency bits)
- Truly lock-free
- Complex for EWMA (need atomic float or bit-cast)

**Option B**: Arc<Mutex> for EWMA only, atomics for counts
- Simpler EWMA math
- Mutex on latency update path only

**Option C**: Use `crossbeam` or `parking_lot` for better lock performance
- External dependency
- Better contention characteristics

**My recommendation**: Option A with `AtomicU64` + `f64::to_bits()` for lock-free EWMA.

### 5. Provider Health/Availability

Should load balancer consider provider health?

**Current understanding**: No - providers are either active or not, catalog determines this.
**Future consideration**: Could add circuit breaker pattern.

### 6. Test Strategy

How to test concurrency-sensitive code?

**Unit tests**: Deterministic tests for each strategy with mocked providers
**Integration tests**: Stress test with concurrent requests (optional for MVP)

**Recommendation**: Start with unit tests proving correctness, add stress tests later.

## Proposed Approaches

### Approach 1: Minimal Invasive Change
- Add `load_balancer_state: Option<Arc<LoadBalancerState>>` to `ModelState`
- Modify `resolve()` to select provider based on strategy
- Wrap provider call with in_flight increment/decrement
- **Pros**: Small changes, backward compatible
- **Cons**: State management scattered

### Approach 2: Dedicated LoadBalancer Component
- New `godwit-loadbalancer` crate (or module in godwit-api)
- `LoadBalancer` struct manages all state
- `ModelRouter` delegates provider selection to `LoadBalancer`
- **Pros**: Clean separation, testable, reusable
- **Cons**: More files, larger refactor

### Approach 3: Inline State in ModelRouter
- Add atomics directly to `ModelRouter` struct
- Selection logic in `resolve()` method
- **Pros**: No new types, minimal code
- **Cons**: Harder to test, less modular

**My Recommendation**: Approach 2 - Dedicated LoadBalancer component. The extra modularity pays off in testability and clarity, especially for concurrent code.

## Architecture Diagram

```
┌─────────────────┐
│   ModelRouter   │
│                 │
│  resolve() ─────┼──► LoadBalancer::select_provider(model_id, strategy)
│                 │                      │
└─────────────────┘                      ▼
                              ┌─────────────────────┐
                              │ LoadBalancerState   │
                              │                     │
                              │  - rr_index         │
                              │  - in_flight[N]     │
                              │  - latency_ewma[N]  │
                              └─────────────────────┘
```

## Success Criteria

1. ✅ RoundRobin distributes requests evenly (provable in tests)
2. ✅ LeastBusy selects idle provider under concurrent load
3. ✅ Latency EWMA converges to average within 10 samples
4. ✅ Zero mutex contention on hot path
5. ✅ Backward compatible: single-provider models unchanged
6. ✅ Thread-safe: passes `cargo test --workspace` with no data races

## Open Questions for User

1. **Approach preference**: Minimal (1), Modular (2), or Inline (3)?
2. **State scope**: Per-model or global LoadBalancer?
3. **EWMA alpha**: 0.1, 0.2, or configurable?
4. **Test depth**: Unit tests only, or add stress tests for MVP?
5. **Crate structure**: New `godwit-loadbalancer` crate, or module in `godwit-api`?

---

**Next Steps**:
- User answers questions above
- Present detailed design based on answers
- Write spec to `docs/load-balancing-design.md`
- Invoke `writing-plans` skill for implementation
