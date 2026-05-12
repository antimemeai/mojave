#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT=$(git rev-parse --show-toplevel)
HOOKS_DIR="$REPO_ROOT/.git/hooks"
SCRIPTS_DIR="$REPO_ROOT/scripts"

echo "Installing git hooks..."

cp "$SCRIPTS_DIR/pre-commit" "$HOOKS_DIR/pre-commit"
chmod +x "$HOOKS_DIR/pre-commit"

echo "Done. Pre-commit hook installed."
echo ""
echo "What it enforces:"
echo "  Rust: rustfmt + clippy -D warnings (zero tolerance)"
echo "  Python: ruff format + ruff lint + mypy"
echo "  General: no secrets, no .env files, no large binaries"
