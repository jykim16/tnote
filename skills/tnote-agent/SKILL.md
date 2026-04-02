---
name: tnote-agent
description: Coordinate agent work with tnote. First confirm this is the relevant tnote skill for the request, ask the user what task they want to accomplish if it is still unclear, then claim a named note and maintain a living record of progress, queue, and blockers.
---

You are an agent that uses tnote to maintain a living record of your work. Follow these steps at the start of every session and update your note throughout.

## 1. Confirm the task before creating a note

Before you run any `tnote` command:

- Check whether this is the relevant tnote skill for the request.
- If another tnote skill is more appropriate, say so and switch to that skill instead of proceeding here.
- If the user's concrete task is still unclear, ask what they want to accomplish before setting up a note.
- Only continue once the task is specific enough to name and track.

## 2. Claim your note

First, scan existing notes to check for a relevant one you should resume instead of creating a new one:

```
tnote ls
```

Look for a note whose name matches the current project-domain. If one exists and is relevant, run `tnote name <note-name>` to attach to the session, then `tnote show --name <note-name>` to read the current state (the header includes the file path) - skip to step 4.

When resuming an existing note, reconcile it before starting fresh work:
- Move stale or completed items out of `In Progress`
- Add the current task to `In Progress` or `Queue`
- Refresh `Context > Background` if the scope or constraints changed

Only proceed to create a new note if no relevant existing note is found.

Choose a name using `<project>-<domain>-<task>`:
- **project**: top-level product or repo (`myapp`, `infra`, `tnote`)
- **domain**: workstream within the project (`auth`, `api`, `frontend`, `data`)
- **task**: specific focus (`session`, `search`, `dashboard`)

Examples: `myapp-auth-session`, `infra-postgres-migrate`, `tnote-api-search`

Agents on the same workstream should share a project-domain prefix so summaries can group them.

```
tnote name <project>-<domain>-<task>
```

## 3. Write the initial note

After `tnote name`, immediately run `tnote show --name <project>-<domain>-<task>` so the header gives you the file path. Then write directly to that file - no need to read the body first. Seed the Background section with context you already know from the conversation: relevant files, constraints, decisions, and dependencies.

```markdown
## Status: in-progress
## Domain: <project>-<domain>
## Started: <date>

## In Progress
- [ ] <first task>

## Queue
- [ ] <next task>

## Blocked

## Done

---

## Context
### Goal
<what this agent was asked to accomplish>

### Background
<what you already know: relevant files, tech, constraints, decisions, dependencies>

---

## Log
### <date>
- Started work on <goal>
```

The `---` separator divides the note into three zones:
- **Status zone** (above first `---`): In Progress / Queue / Done / Blocked - updated frequently, quick to scan
- **Context zone**: Goal / Background - detailed context for doing the work
- **Log zone** (below second `---`): Dated entries for decisions, findings, and actions

After writing the note, briefly tell the user the plan (tasks in order) before starting work.

## 4. Update discipline

Update your note as work progresses - not just at the end.

**Task transitions:**
- **Starting a subtask**: move it from Queue -> In Progress
- **Finishing a subtask**: move it to Done, add a log entry
- **Blocked**: move the subtask to Blocked, set `Status: blocked`
- **Scope changed but same workstream**: rewrite `In Progress` / `Queue` to match the new reality and add a log entry explaining the shift
- **Scope changed enough that the note name no longer fits**: create or switch to a better-named note instead of overloading the old one
- **All tasks done**: confirm with the user before closing

**Context updates** - update Background when you discover:
- Key files, functions, or modules relevant to the work
- Architectural decisions or constraints
- Gotchas, non-obvious behavior, or failed approaches

**Log entries** - add a dated entry when you:
- Make a significant decision or tradeoff
- Discover something that changes the approach
- Complete a meaningful chunk of work
- Hit and resolve a blocker

To update your note, read the current state first, then make targeted edits - do not rewrite the whole file:

```
tnote show --name <your-note-name>
# the header shows the file path - use Edit tool on specific lines that changed
```
