---
name: tnote-track
description: Track your work as an agent using tnote. Invoke at the start of any multi-step task to claim a named note and maintain a living record of tasks, progress, and blockers. Use when starting an autonomous task, when working alongside other agents, or when asked to track progress.
---

You are an agent that uses tnote to maintain a living record of your work. Follow these steps at the start of every session and update your note throughout.

## 1. Claim your note

Choose a name using `<project>-<domain>-<task>`:
- **project**: top-level product or repo (`myapp`, `infra`, `tnote`)
- **domain**: workstream within the project (`auth`, `api`, `frontend`, `data`)
- **task**: specific focus (`session`, `search`, `dashboard`)

Examples: `myapp-auth-session`, `infra-postgres-migrate`, `tnote-api-search`

Agents on the same workstream should share a project-domain prefix so summaries can group them.

```
tnote name <project>-<domain>-<task>
NOTE_PATH=$(tnote path --name <project>-<domain>-<task>)
```

## 2. Note format

Write this structure to your note path:

```markdown
## Status: in-progress
## Domain: <project>-<domain>
## Started: <date>

## In Progress
- [ ] <current task>

## Queue
- [ ] <next task>

## Blocked
- <blocker and what is needed>

## Done
- [x] <completed task>

---

## Context
### Goal
<what this agent was asked to accomplish>

### Background
<relevant context, decisions, dependencies — seeded by manager, extended by agent>

---

## Log
### <date>
- <decisions, findings, actions>

```

The `---` separator divides the note into three zones:
- **Status zone** (above first `---`): In Progress / Queue / Done / Blocked — updated frequently, quick to scan
- **Context zone**: Goal / Background — detailed context for doing the work
- **Context zone** (below last `---`): Log — Daily notes of work accomplished

## 3. Update discipline

- **Starting a task**: move subtask from Queue → In Progress
- **Finishing a subtask**: move subtask to Done and add log entry. If there are no more tasks ask if the overall task is complete.
- **Blocked**: move subtask to Blocked, set Status to `blocked`
- **Finishing a task**: move In Progress → Done, add log entry

To update your note, read it first then make targeted edits to the file — do not rewrite the whole file:

```
# Read current note
tnote show --name <your-note-name>

# Get the file path, then edit only the lines that changed
tnote path --name <your-note-name>
```
