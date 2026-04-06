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

Look for a note whose name matches the current project-domain. If one exists and is relevant, keep working against that note by name and run `tnote show -n <note-name>` to read the current state (the header includes the file path) - skip to step 4.

When resuming an existing note, reconcile it before starting fresh work:
- Move stale or completed items out of `In Progress`
- Add the current task to `In Progress` or `Queue`
- Before you start a task, make sure it already appears under `In Progress`
- Refresh `Context > Background` if the scope or constraints changed

Only proceed to create a new note if no relevant existing note is found.

Choose a name using `agent-<project>-<domain>-<task>`:
- **agent-**: required prefix for all agent notes
- **project**: top-level product or repo (`myapp`, `infra`, `tnote`)
- **domain**: workstream within the project (`auth`, `api`, `frontend`, `data`)
- **task**: specific focus (`session`, `search`, `dashboard`)

Examples: `agent-myapp-auth-session`, `agent-infra-postgres-migrate`, `agent-tnote-api-search`

Agents on the same workstream should share a project-domain prefix so summaries can group them. All agent notes use the `agent-` prefix; manager notes use `manager-`.

```
tnote name agent-<project>-<domain>-<task>
```

## 3. Write the initial note

After `tnote name`, immediately run `tnote show -n <project>-<domain>-<task>` so the header gives you the file path. Then write directly to that file - no need to read the body first.

The note is divided into four sections separated by `---`. Each section can be read independently using `awk`:

```bash
awk -v RS='---' 'NR==<n>' "$(tnote path -n <note-name>)"
```

| `awk` NR | Section | Contents | Update frequency |
|---|---|---|---|
| `NR==1` | Header | Status, Domain, Workspace, Goal, Started | Rarely |
| `NR==2` | Tasks | In Progress, Queue, Blocked, Done | Every task transition |
| `NR==3` | Context | Background, Files, Links, Dependencies | When scope or understanding changes |
| `NR==4` | Log | Dated decision/action entries | After meaningful work |

```markdown
## Status: in-progress
## Domain: <project>-<domain>
## Workspace: <absolute path to working directory>
## Goal: <one-line description of what this agent is doing>
## Started: <date>

---

## In Progress
- [ ] <first task>

## Queue
- [ ] <next task>

## Blocked

## Done

---

## Context

### Background
<what you already know: constraints, decisions, approach>

### Files
- `path/to/relevant/file`

### Links
- <URL to PR, ticket, doc, slack thread>

### Dependencies
- agent `<other-agent-name>`: <what you need from them>
- <external dependency>: <what you're waiting on>

---

## Log
### <date>
- Started work on <goal>
```

After writing the note, briefly tell the user the plan (tasks in order) before starting work.

## 4. Update discipline

Update your note as work progresses - not just at the end.

**Task transitions:**
- **Before starting any work**: add it to `In Progress` if it is not already listed there
- **Starting a subtask**: move it from Queue -> In Progress
- **Finishing a subtask**: move it to Done, add a log entry
- **Blocked**: move the subtask to Blocked, set `Status: blocked`
- **Scope changed but same workstream**: rewrite `In Progress` / `Queue` to match the new reality and add a log entry explaining the shift
- **Scope changed enough that the note name no longer fits**: create or switch to a better-named note instead of overloading the old one
- **All tasks done**: confirm with the user before closing

**Context updates** - update the Context section when you discover:
- Key files, functions, or modules relevant to the work (add to Files)
- URLs to PRs, tickets, docs, threads (add to Links)
- Dependencies on other agents or external blockers (add to Dependencies)
- Architectural decisions, constraints, gotchas, or failed approaches (add to Background)

**Log entries** - add a dated entry when you:
- Make a significant decision or tradeoff
- Discover something that changes the approach
- Complete a meaningful chunk of work
- Hit and resolve a blocker

To update your note, read the current state first, then make targeted edits - do not rewrite the whole file:

```
tnote show -n <your-note-name>
# the header shows the file path - use Edit tool on specific lines that changed
```
