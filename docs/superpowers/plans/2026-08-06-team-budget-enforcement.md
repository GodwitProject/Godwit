# Team Budget Enforcement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement budget enforcement for teams by adding `check_team_budget()` function and integrating it into the proxy middleware to block requests when team budget is exceeded.

**Architecture:** Follow the existing pattern used for `check_end_user_budget()` in `rate_limit.rs`. Add a new function that queries team budget and spend, then call it in proxy handlers after checking user budget.

**Tech Stack:** Rust, SQLx, PostgreSQL, Axum

## Global Constraints

- Follow existing code style (no comments unless necessary)
- Use SQLx compile-time queries
- Return `ApiError::BudgetExceeded` when budget is exceeded
- Tests require `DATABASE_URL` environment variable
- Update documentation in `docs/end-user-budgets.md`

---

### Task 1: Add `check_team_budget()` function to rate_limit.rs

**Files:**
- Modify: `crates/godwit-api/src/rate_limit.rs`
- Test: `crates/godwit-api/src/rate_limit.rs` (inline tests)

**Interfaces:**
- Consumes: `PgPool`, `Uuid` (team_id)
- Produces: `Result<(), ApiError>`

- [ ] **Step 1: Write the failing test**

```rust
#[sqlx::test]
async fn budget_check_team_blocks_when_exceeded(pool: PgPool) {
    use godwit_db::models::UserRole;
    use godwit_db::repositories::organizations::OrganizationRepository;
    use godwit_db::repositories::users::UserRepository;
    use godwit_db::repositories::teams::TeamsRepository;
    use crate::error::ApiError;

    let orgs = OrganizationRepository::new(pool.clone());
    let org = orgs.create("test-org", None).await.expect("create org");
    
    let users = UserRepository::new(pool.clone());
    let user = users.create("test@example.com", None, UserRole::User, Some(org.id))
        .await.expect("create user");
    
    let teams = TeamsRepository::new(pool.clone());
    let max_budget = rust_decimal::Decimal::from_str("100.00").unwrap();
    let team = teams.create(org.id, "test-team", None, Some(max_budget))
        .await.expect("create team budget");
    
    sqlx::query(
        "INSERT INTO request_logs (api_key_id, user_id, organization_id, team_id, model, provider, provider_model_id, capability, duration_ms, streamed, status, cost_usd)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"
    )
    .bind(Uuid::new_v4())
    .bind(user.id)
    .bind(org.id)
    .bind(team.id)
    .bind("gpt-4o")
    .bind("openai")
    .bind("gpt-4o")
    .bind("chat")
    .bind(100)
    .bind(false)
    .bind("success")
    .bind(rust_decimal::Decimal::from_str("150.00").unwrap())
    .execute(&pool)
    .await
    .expect("insert request log");
    
    let result = check_team_budget(&pool, team.id).await;
    assert!(matches!(result, Err(ApiError::BudgetExceeded)));
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
DATABASE_URL=postgres://user:pass@localhost:5432/godwit cargo test -p godwit-api --lib rate_limit::tests::budget_check_team_blocks_when_exceeded -- --exact
```
Expected: FAIL with "function not defined"

- [ ] **Step 3: Write minimal implementation**

Add to `crates/godwit-api/src/rate_limit.rs` after `check_end_user_budget()`:

```rust
pub async fn check_team_budget(
    pool: &PgPool,
    team_id: Uuid,
) -> Result<(), crate::error::ApiError> {
    use crate::error::ApiError;
    
    let team = sqlx::query_as::<_, godwit_db::models::Team>(
        "SELECT * FROM teams WHERE id = $1",
    )
    .bind(team_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Core(PasteurError::Database(e.to_string())))?;
    
    let team = match team {
        Some(t) => t,
        None => return Ok(()),
    };
    
    let max_budget = match team.max_budget_usd {
        Some(budget) => budget,
        None => return Ok(()),
    };
    
    let spent: Option<rust_decimal::Decimal> = sqlx::query_scalar(
        "SELECT COALESCE(SUM(cost_usd), 0) FROM request_logs WHERE team_id = $1",
    )
    .bind(team_id)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Core(PasteurError::Database(e.to_string())))?;
    
    let spent = spent.unwrap_or(rust_decimal::Decimal::ZERO);
    
    if spent >= max_budget {
        return Err(ApiError::BudgetExceeded);
    }
    
    Ok(())
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
DATABASE_URL=postgres://user:pass@localhost:5432/godwit cargo test -p godwit-api --lib rate_limit::tests::budget_check_team_blocks_when_exceeded -- --exact
```
Expected: PASS

- [ ] **Step 5: Add additional tests**

Add two more tests to `rate_limit.rs`:

```rust
#[sqlx::test]
async fn budget_check_team_allows_when_under_budget(pool: PgPool) {
    use godwit_db::models::UserRole;
    use godwit_db::repositories::organizations::OrganizationRepository;
    use godwit_db::repositories::users::UserRepository;
    use godwit_db::repositories::teams::TeamsRepository;

    let orgs = OrganizationRepository::new(pool.clone());
    let org = orgs.create("test-org", None).await.expect("create org");
    
    let users = UserRepository::new(pool.clone());
    let user = users.create("test2@example.com", None, UserRole::User, Some(org.id))
        .await.expect("create user");
    
    let teams = TeamsRepository::new(pool.clone());
    let max_budget = rust_decimal::Decimal::from_str("100.00").unwrap();
    let team = teams.create(org.id, "test-team", None, Some(max_budget))
        .await.expect("create team budget");
    
    sqlx::query(
        "INSERT INTO request_logs (api_key_id, user_id, organization_id, team_id, model, provider, provider_model_id, capability, duration_ms, streamed, status, cost_usd)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"
    )
    .bind(Uuid::new_v4())
    .bind(user.id)
    .bind(org.id)
    .bind(team.id)
    .bind("gpt-4o")
    .bind("openai")
    .bind("gpt-4o")
    .bind("chat")
    .bind(100)
    .bind(false)
    .bind("success")
    .bind(rust_decimal::Decimal::from_str("50.00").unwrap())
    .execute(&pool)
    .await
    .expect("insert request log");
    
    let result = check_team_budget(&pool, team.id).await;
    assert!(result.is_ok());
}

#[sqlx::test]
async fn budget_check_team_allows_when_no_max_budget(pool: PgPool) {
    use godwit_db::models::UserRole;
    use godwit_db::repositories::organizations::OrganizationRepository;
    use godwit_db::repositories::users::UserRepository;
    use godwit_db::repositories::teams::TeamsRepository;

    let orgs = OrganizationRepository::new(pool.clone());
    let org = orgs.create("test-org", None).await.expect("create org");
    
    let users = UserRepository::new(pool.clone());
    let user = users.create("test3@example.com", None, UserRole::User, Some(org.id))
        .await.expect("create user");
    
    let teams = TeamsRepository::new(pool.clone());
    let team = teams.create(org.id, "test-team", None, None)
        .await.expect("create team budget without max");
    
    let result = check_team_budget(&pool, team.id).await;
    assert!(result.is_ok());
}
```

- [ ] **Step 6: Run all rate_limit tests**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
DATABASE_URL=postgres://user:pass@localhost:5432/godwit cargo test -p godwit-api --lib rate_limit
```
Expected: All tests PASS

- [ ] **Step 7: Commit**

```bash
git add crates/godwit-api/src/rate_limit.rs
git commit -m "feat: add check_team_budget function with tests"
```

---

### Task 2: Integrate team budget check into proxy handlers

**Files:**
- Modify: `crates/godwit-api/src/proxy.rs`

**Interfaces:**
- Consumes: `check_team_budget()` from rate_limit module
- Produces: Budget enforcement in proxy handlers

- [ ] **Step 1: Add helper function for team budget check**

Add to `proxy.rs` after `check_user_budget()`:

```rust
async fn check_team_budget(
    state: &Arc<AppState>,
    api_key: &ApiKey,
) -> Result<(), crate::error::ApiError> {
    if let Some(team_id) = api_key.team_id {
        rate_limit::check_team_budget(&state.pool, team_id).await
    } else {
        Ok(())
    }
}
```

- [ ] **Step 2: Add team budget check to chat_completions handler**

Modify `chat_completions()` function to add team budget check after user budget check:

```rust
check_rate_limit(&state, &api_key, &primary_resolved.model, estimated_tokens).await?;
check_user_budget(&state, &api_key).await?;
check_team_budget(&state, &api_key).await?;  // Add this line
```

- [ ] **Step 3: Add team budget check to embeddings handler**

Modify `embeddings()` function similarly:

```rust
check_rate_limit(&state, &api_key, &primary_resolved.model, estimated_tokens).await?;
check_user_budget(&state, &api_key).await?;
check_team_budget(&state, &api_key).await?;  // Add this line
```

- [ ] **Step 4: Add team budget check to image_generations handler**

Modify `image_generations()` function similarly.

- [ ] **Step 5: Add team budget check to audio_speech handler**

Modify `audio_speech()` function similarly.

- [ ] **Step 6: Add team budget check to audio_transcriptions handler**

Modify `audio_transcriptions()` function similarly.

- [ ] **Step 7: Add team budget check to image_edits handler**

Modify `image_edits()` function similarly.

- [ ] **Step 8: Run cargo check**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo check --workspace --tests
```
Expected: No errors

- [ ] **Step 9: Commit**

```bash
git add crates/godwit-api/src/proxy.rs
git commit -m "feat: integrate team budget check into proxy handlers"
```

---

### Task 3: Update documentation

**Files:**
- Modify: `docs/end-user-budgets.md`

- [ ] **Step 1: Add Team Budget Enforcement section**

Add to `docs/end-user-budgets.md` after the "Budget Enforcement" section:

```markdown
## Team Budget Enforcement

Teams can also have budget limits enforced at request time. When an API key is associated with a team, the gateway checks both the user's budget (if set) and the team's budget (if set) before processing the request.

### Team Budget Check Logic

The team budget check in `crates/godwit-api/src/rate_limit.rs::check_team_budget()`:

```rust
pub async fn check_team_budget(
    pool: &PgPool,
    team_id: Uuid,
) -> Result<(), ApiError> {
    // 1. Fetch team budget record
    let team = sqlx::query_as::<_, Team>(
        "SELECT * FROM teams WHERE id = $1",
    )
    .bind(team_id)
    .fetch_optional(pool)
    .await?;
    
    // 2. No team record = no enforcement
    let Some(team) = team else { return Ok(()) };
    let Some(max_budget) = team.max_budget_usd else { return Ok(()) };
    
    // 3. Sum all request_logs for this team
    let spent: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(cost_usd), 0) FROM request_logs WHERE team_id = $1",
    )
    .bind(team_id)
    .fetch_one(pool)
    .await?;
    
    // 4. Block if spent >= max_budget
    if spent >= max_budget {
        return Err(ApiError::BudgetExceeded);
    }
    
    Ok(())
}
```

### Integration with User Budgets

When both user and team budgets are configured:
1. User budget is checked first (via `check_user_budget()`)
2. Team budget is checked second (via `check_team_budget()`)
3. Either budget being exceeded results in HTTP 429

The check order matters: a user with their own budget limit will be blocked before the team budget is evaluated, even if the team has remaining budget.

### Response

When team budget is exceeded:
- **HTTP Status**: 429 Too Many Requests
- **Error Type**: `https://api.godwit.local/errors/budget-exceeded`
- **Detail**: "End-user budget has been exceeded."

**Note:** The same error response is used for both user and team budget exceeded scenarios.
```

- [ ] **Step 2: Commit**

```bash
git add docs/end-user-budgets.md
git commit -m "docs: add team budget enforcement section"
```

---

### Task 4: Final verification

- [ ] **Step 1: Run full workspace check**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo check --workspace --tests
```

- [ ] **Step 2: Run rate_limit tests**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
DATABASE_URL=postgres://user:pass@localhost:5432/godwit cargo test -p godwit-api --lib rate_limit
```

Expected: All tests pass (including 3 new team budget tests + existing end-user budget tests)

- [ ] **Step 3: Build the binary**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo build --bin godwit
```

Expected: No errors

---

## Test Summary

**New tests added:**
1. `budget_check_team_blocks_when_exceeded` - Verifies blocking when team spend >= max_budget
2. `budget_check_team_allows_when_under_budget` - Verifies allowing when team spend < max_budget  
3. `budget_check_team_allows_when_no_max_budget` - Verifies allowing when max_budget is NULL

**Total test count:** 3 new tests in `rate_limit.rs`

**Files modified:**
1. `crates/godwit-api/src/rate_limit.rs` - Add `check_team_budget()` function + tests
2. `crates/godwit-api/src/proxy.rs` - Integrate team budget check into 6 proxy handlers
3. `docs/end-user-budgets.md` - Add "Team Budget Enforcement" section
