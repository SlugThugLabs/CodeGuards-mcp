# Engineering Workspace Standard (.planning)

This repository follows the **SlugPlan** convention: `.planning/` serves as the in-repo Confluence, Jira, and Tool Workspace.

## Quick Orientation for AI Agents & Engineers:
1. **Source Code (`src/` or `lib/`):** Pure product code only. No planning notes, temporary test scratchpads, or loose docs.
2. **End-User Docs (`docs/`):** Public / user-facing documentation only (guides, installation, API references). Never put internal developer notes or architecture plans in `docs/`.
3. **Architecture RFC / Confluence (`.planning/ARCHITECTURE.md`):** The canonical source of truth for system layers, module boundaries, allowed dependencies, and invariants. Read this BEFORE writing or modifying code.
4. **Task Board / Jira (`.planning/jira/PROJECT_PLAN.md`):** Active epics, sprint milestones, and tasks. Check your active task here, and update progress as work is completed.
5. **Decisions / ADRs (`.planning/adr/`):** Architecture Decision Records documenting why specific trade-offs or technologies were chosen.
6. **Tool Workspaces (`.planning/<tool>/`):** Dedicated workspaces for development tools (e.g. `.planning/slugaudit/`, `.planning/codeguards/`). These store tool evidence and caches.

## Operating Rules for AI Agents:
- **Never guess architecture:** Always read `.planning/ARCHITECTURE.md` first.
- **Never clutter root or `docs/`:** Internal planning belongs strictly in `.planning/`.
- **Stay scoped:** Work only on the active task assigned in `.planning/jira/PROJECT_PLAN.md`.\n