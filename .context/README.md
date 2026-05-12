# .context/

This directory is the LLM working memory for this repo. It contains structured artifacts that help Claude (and future Claude sessions) maintain continuity across conversations.

## Contents

- `beads/` — Issue tracking (all open questions, todos, follow-ups, blockers)
- `decisions/` — Lightweight decision logs (not full ADRs — those go in docs/adr/)
- `lit-reviews/` — Literature review notes organized by topic
- `session-notes/` — Per-session working notes that capture state not obvious from code/git

## Conventions

- Files are markdown with YAML frontmatter for metadata
- Beads use the format: `BEAD-NNNN-short-slug.md`
- Decisions use: `YYYY-MM-DD-short-slug.md`
- Lit reviews use: `topic-name.md` (living documents, updated as papers are read)
- Session notes use: `YYYY-MM-DD-summary.md`

## Why .context/ and not just docs/

docs/ is for humans and formal artifacts (ADRs, specs, reference material).
.context/ is for LLM working state — less formal, more frequently updated, optimized for being loaded into conversation context.
