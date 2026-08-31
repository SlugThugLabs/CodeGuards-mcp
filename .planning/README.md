# .planning/

Internal project docs: plans, working notes, decisions.
Not for end users — that's `docs/`.

Tool artifacts (e.g. the slugaudit index) live under `.planning/slugaudit/`
and are gitignored; everything else here is committed.

## Contents

- `ARCHITECTURE.md` — **source of truth**: goal, layers, modules, constraints, quality goals.
- `V0.2_ARCHITECTURE_SPEC.md` — v0.2 mental model and system spec (target state).
- `design_v0.2.md` — v0.2 "architecture-as-source-of-truth" design (PARKED).
- `ANTI_DRIFT.md` — design for token-protected guard/architecture files.
- `THINKING_PAUSE.md` — deliberation-signaling UX principle (implemented in server.py).
