# Codex Session Workflow (Milestone-by-Milestone)

Use this exact process for every milestone to keep context handoff clean between sessions.

1. Ask Codex to implement only one milestone.
2. When implementation is done, ask Codex to summarize in this exact structure:
- current architecture
- files changed
- commands to run
- validations passed
- remaining warnings/issues
3. Save that summary into `PROJECT_STATE.md`.
4. Commit all milestone changes (code + `PROJECT_STATE.md`) to git.
5. Start a new Codex session for the next milestone and begin by sharing `PROJECT_STATE.md`.

Notes:
- Keep scope strict: no work from future milestones.
- Keep run commands copy-paste ready.
- Track temporary assumptions and local defaults in `PROJECT_STATE.md`.
