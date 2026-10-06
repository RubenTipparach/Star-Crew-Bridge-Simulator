#!/bin/bash
# SessionStart hook for Claude Code cloud sessions: make the repository's checks
# runnable (CLAUDE.md section 12). The only tool a fresh cloud container lacks is
# the OpenSpec CLI; python3 (stdlib only) and node with Playwright are already
# there. Idempotent and synchronous, so the checks work from the first prompt.
set -euo pipefail

if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

OPENSPEC_VERSION="1.14.0"
if ! command -v openspec >/dev/null 2>&1 || [ "$(openspec --version 2>/dev/null)" != "$OPENSPEC_VERSION" ]; then
  npm install -g "@fission-ai/openspec@${OPENSPEC_VERSION}" >/dev/null 2>&1
fi
openspec --version >/dev/null
