---
name: tnote-track
description: Track your work as an agent using tnote. Invoke at the start of any multi-step task to claim a named note and maintain a living record of tasks, progress, and blockers. Use when starting an autonomous task, when working alongside other agents, or when asked to track progress.
allowed-tools: Bash(tnote *)
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

## Queue
- [ ] <next task>

## In Progress
- [ ] <current task>

## Done
- [x] <completed task>

## Log
### <date>
- <decisions, findings, actions>

## Blocked
- <blocker and what is needed>

## Handoff
- <what the next agent or human needs to know>
- <dependencies on other agents by name>
```

## 3. Update discipline

- **Starting a task**: move Queue → In Progress
- **Finishing a task**: move In Progress → Done, add log entry
- **Blocked**: add to Blocked, set Status to `blocked`
- **Session end**: set Status to `done` or `handoff`, fill Handoff

```
# Read current note
tnote show --name <your-note-name>

# Get path to write updates
tnote path --name <your-note-name>
```

## 4. Coordinate with other agents

```
tnote list                          # see all active agents
tnote show --name '<project>-*'     # see all notes for a project (quote the glob)
tnote show --name <agent-name>      # read a specific agent's note
```

Look for agents with the same project-domain prefix. Note dependencies in your Handoff section.
