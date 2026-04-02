---
name: tnote-agent
description: Coordinate agent work with tnote. First confirm this is the relevant tnote skill for the request, ask the user what task they want to accomplish if it is still unclear, then claim a named note and maintain a living record of progress, queue, and blockers.
allowed-tools: Bash(tnote *), Agent
---

You are an agent that uses tnote to maintain a living record of your work. Follow these steps at the start of every session and update your note throughout.

**Claude Code:** Delegate all tnote writes to a sub-agent using the `Agent` tool (see steps 3 and 4). This keeps note content out of your main context window — the sub-agent handles file I/O; you only see a brief summary back.

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

Look for a note whose name matches the current project-domain. If one exists and is relevant, run `tnote name <note-name>` to attach to the session — then skip to step 4 to update it via sub-agent.

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

## 3. Write the initial note (via sub-agent)

Spawn a sub-agent to create the note file. Pass it the note name and full initial content:

```
Agent prompt:
"Initialize tnote note '<note-name>'.
1. Run: tnote show --name <note-name>
   The header line shows the file path — use that path for the write.
2. Write this content to the file:

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

3. Return: 'Note created at <file-path>'"
```

Once the sub-agent confirms, briefly tell the user the plan (tasks in order) before starting work.

## 4. Update discipline (via sub-agent)

Update your note as work progresses — not just at the end. Spawn a sub-agent for every note write; never read the note directly in your main session unless you need to reference its content for the task at hand.

**Task transitions:**
- **Starting a subtask**: move it from Queue -> In Progress
- **Finishing a subtask**: move it to Done, add a log entry
- **Blocked**: move the subtask to Blocked, set `Status: blocked`
- **Scope changed but same workstream**: rewrite `In Progress` / `Queue` to match the new reality and add a log entry explaining the shift
- **Scope changed enough that the note name no longer fits**: create or switch to a better-named note instead of overloading the old one
- **All tasks done**: confirm with the user before closing

**Context updates** — update Background when you discover:
- Key files, functions, or modules relevant to the work
- Architectural decisions or constraints
- Gotchas, non-obvious behavior, or failed approaches

**Log entries** — add a dated entry when you:
- Make a significant decision or tradeoff
- Discover something that changes the approach
- Complete a meaningful chunk of work
- Hit and resolve a blocker

**Sub-agent prompt template for updates:**

```
Agent prompt:
"Update tnote note '<note-name>'.
1. Run: tnote show --name <note-name>
   The header line shows the file path — read the current content from that file.
2. Make these targeted edits: <describe exactly what to change>
3. Return a one-line summary: what changed (e.g. 'moved X to Done, added log entry')."
```

The sub-agent reads and edits the file; you receive only its summary.
