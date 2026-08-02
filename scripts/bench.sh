#!/bin/bash
set -e
which oha || cargo install oha
oha -z 30s -c 50 --no-tui \
  -H "Authorization: Bearer sk-godwit-test" \
  -d '{"model":"gpt-4o","messages":[{"role":"user","content":"hi"}]}' \
  http://localhost:3000/v1/chat/completions
