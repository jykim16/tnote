---
name: tnote-plan
description: Inject tasks into agent tnotes based on their current trajectory. Reads existing agent notes, matches tasks to agents by project-domain alignment, and creates new notes for new workstreams.
argument-hint: [project-prefix]
disable-model-invocation: true
allowed-tools: Bash(tnote *)
---

Plan and inject tasks into agent tnotes$ARGUMENTS.

## 1. Read the cluster state

```
tnote list
tnote show --name '<project>-*'   # quote the glob
```

For each relevant agent understand:
- Their project-domain prefix
- What is in their Queue and In Progress
- Whether they are blocked or done
- Their Handoff notes

## 2. Match tasks to agents

Assign by **trajectory alignment**:
- Same project-domain prefix → strong fit
- Agent's In Progress is a prerequisite or neighbor → strong fit
- Agent is blocked or done → do not queue; create a new agent instead
- Agent already has 3+ items in Queue → spawn a new agent with the same project-domain prefix

## 3. Inject tasks

For each assignment:

```
# Read current note
tnote show --name <agent-name>

# Get path, then write updated note with new Queue items
tnote path --name <agent-name>
```

Add to the Queue section:
```markdown
- [ ] <new task>   <!-- injected by planner: <date> -->
```

Add a log entry:
```markdown
### <date>
- [planner] injected: <task description>
```

## 4. Create notes for new workstreams

**CRITICAL: Do NOT use `tnote name` when creating task notes as a planner.**

Write directly to the tnote directory with `named-` prefix to avoid changing the session's pinned note:

```
TNOTE_DIR=$(dirname $(tnote path --name $(tnote list | head -1 | awk '{print $1}')))
cat > "$TNOTE_DIR/named-<project>-<domain>-<task>.md" << 'EOF'
<note content>
EOF
```

Use Status `queued` and pre-populate the Queue. Match the project prefix of related agents.

## 5. Record the plan (optional)

```
tnote name <project>-plan-<date>
# write plan to: $(tnote path --name <project>-plan-<date>)
```

```markdown
## Plan: <date>

## Assignments
- <agent>: <task> — reason: <why>

## New agents created
- <agent>: <initial queue>

## Deferred
- <task>: <reason>
```
