# Agent notes for Godwit

## Toolchain path

Rust/Cargo are installed via rustup under `/usr/local/opt/rustup/bin`. Most commands will fail unless you prefix:

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
```

## Repository structure

- Rust workspace rooted in the current directory; members are all crates under `crates/*`.
- Root `Cargo.toml` is also a package (`godwit-integration-tests`) so that integration tests in `tests/` can depend on workspace crates.
- `docs/` is intentionally not version-controlled.

## Build / test commands

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"

# Compile everything
cargo check --workspace

# Run unit tests (database tests need DATABASE_URL)
DATABASE_URL=postgres://user:pass@localhost:5432/godwit cargo test --workspace

# Build the binary
cargo build --bin godwit

# Run locally
cp config.example.yaml config.yaml
# edit config.yaml
DATABASE_URL=postgres://user:pass@localhost:5432/godwit cargo run --bin godwit
```

## Database

- PostgreSQL 15+ is required. The binary runs SQLx migrations on startup.
- `godwit-db` uses SQLx compile-time checks; set `DATABASE_URL` before any build/test that touches it.
- Local dev DB name is conventionally `godwit`.

## Docker

```bash
docker compose build
docker compose up
```

The Dockerfile installs `xmlsec1` / `libclang` dependencies because the `samael` SAML crate requires them.

## Code conventions

- Crate prefix is `godwit_*` (e.g., `godwit_core`, `godwit_db`).
- `godwit-api` exposes `admin::router(state)` — it takes the shared `Arc<AppState>` because the JWT middleware needs state access.
- `godwit-providers` exposes a `Provider` trait; routing is currently by model-id prefix (`claude` → Anthropic, otherwise OpenAI).

## Testing quirks

- `cargo test --workspace` without `DATABASE_URL` will fail on `godwit-db` tests.
- Integration tests in `tests/` are marked `#[ignore]` because they require a running server.
- Compile them with `cargo test --test proxy_integration --no-run` and `cargo test --test admin_integration --no-run`.

## Benchmarks

```bash
cargo bench -p godwit-providers
./scripts/bench.sh   # requires oha and a running server on :3000
```

## Remote

Upstream is `https://github.com/GodwitProject/Godwit.git`. The local branch `pasteurllm-mvp` tracks `origin/main` because the `main` branch name is already used by another worktree.

## Dependency freshness

Always verify dependencies before a release. The MVP pins several crates that have newer major versions available:

| Crate | Current | Latest (checked 2026-08-02) | Notes |
|-------|---------|------------------------------|-------|
| axum | 0.7 | 0.8.9 | API changes likely |
| sqlx | 0.7 | 0.9.0 | Query macros and migration API may change |
| thiserror | 1.0 | 2.0.19 | Major API changes |
| reqwest | 0.12 | 0.13.4 | Used transitively via features |
| jsonwebtoken | 9 | 11.0.0 | JWT signing/validation |
| openidconnect | 3 | 4.0.1 | OIDC flow types |
| criterion | 0.5 | 0.8.2 | Benchmark harness |
| serde_yaml | 0.9 | 0.9.34+deprecated | Deprecated; consider `serde_yml` or alternatives |

Patch/minor updates are applied automatically by `cargo update`. Run it regularly:

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo update
cargo test --workspace
```

For major upgrades, bump versions in each `Cargo.toml`, fix compile errors, and re-run the full test suite before committing.
