# TOOL_SPLIT.md — CodeGuards as two tools over one engine

> **Status:** DRAFT — design note, not a committed release decision.
> Captured 2026-08-31 from discussion. Owner: user. Do not implement
> until the two open decisions below are made.

---

## The split

One shared engine, two thin MCP entrypoints. The boundary is
**contract writer vs contract reader**; the files on disk
(`intent.json`, `.planning/ARCHITECTURE.md`, `.planning/PROJECT_PLAN.md`,
structural baseline, `.codeguards.yaml`) are the interface between them.

| Tool | Role | Handlers (current server.py) | Cadence |
|---|---|---|---|
| **Authoring** | Writes contracts | `probe`, `declare_intent`, `plan`, `update_task`, `list_tasks`, `save_baseline` | Rare, project start / deliberate re-architecture; conversational |
| **Enforcement** | Reads contracts, never writes them | `check_project`, `check_file`, `detect_languages`, `list_guards` | Continuous, per change; mechanical |

Shared engine (stays ONE library, not split):
`guards/`, `import_analyzer.py`, `config.py`, `constants.py`, `plugins/`,
`detectors.py`, `intent.py`, `planning.py`, `fixes.py`.

## What changes with the split

1. **The no-intent gate moves from checker to authoring.**
   Today `check_project` hard-blocks without `intent.json`.
   Post-split, enforcement degrades per the v0.2 spec:
   no contract → baseline hygiene only + advisory "declare architecture".
   The ritual gate lives in the authoring flow, not the checker.
2. **Enforcement is structurally read-only against contracts.**
   Gives ANTI_DRIFT.md's threat model (model edits guards/arch docs to make
   checks pass) a natural enforcement point: only the authoring tool, with
   human confirmation, writes contract files.
3. **Distribution.** Throwaway sessions register enforcement only;
   new-project sessions get authoring. Contract files outlive the agent
   (disposable-agent model).

## Open decisions (block implementation)

1. **No-contract semantics for `check_project`:**
   - (a) v0.1: hard block — "no intent, no check" (the confidence-to-build
     identity feature).
   - (b) v0.2 spec: baseline hygiene + advisory prompt to declare
     architecture.
   - (c) configurable, default to one of the above.
2. **Naming.** Keep `probe` exclusive to the authoring ritual
   (questions before building). The checker's file inspection stays
   `check_file`. Do not overload "probe" across both tools.

## Non-goals

- No engine split (one guard set, one registry, one config loader).
- No new guard rules as part of the split.
- No changes to the file contracts themselves (that's v0.2 territory,
  see `design_v0.2.md` — PARKED).

## Effort estimate

Mostly unmixing `server.py`: two entrypoints + two `TOOL_HANDLERS` /
`TOOL_DEFINITIONS` tables over the existing modules. No guard logic
changes beyond the no-contract behavior (open decision #1).
