# G2: OpenAI Wire Compatibility Flag

## Status

✅ Implemented

## Summary

Added `compat.openai_wire_streaming` configuration flag to enable native OpenAI SSE streaming format.

## Changes

- `godwit-core`: Added `CompatConfig` struct with `openai_wire_streaming` field
- `godwit-api`: Threaded config through `AppState`, conditional translation in proxy streaming
- `docs`: Added streaming API documentation

## Testing

- Unit test: `openai_wire_flag_changes_sse_format`
- Verified: `cargo test -p godwit-api --lib proxy`

## Files Modified

- `crates/godwit-core/src/lib.rs`
- `crates/godwit-api/src/proxy.rs`
- `docs/api/streaming.md` (not version-controlled per repo convention)
