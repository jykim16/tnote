---
name: tnote-summarize
description: Summarize the work of all active agents using tnote. Reads all named notes, groups them by project and domain, and produces a structured cluster health report with status, blockers, and coordination needs.
argument-hint: [project-prefix]
disable-model-invocation: true
allowed-tools: Bash(tnote *)
---

Summarize all active agent tnotes$ARGUMENTS.

## 1. Gather notes

```
tnote list
```

Collect all named notes. If a project prefix was given as an argument, filter to notes matching that prefix using:

```
tnote show --name '$ARGUMENTS-*'
```

Otherwise read each named note individually:

```
tnote show --name <agent-name>
```

## 2. Group by project, then domain

Agent names follow `<project>-<domain>-<task>`. Group first by project, then by domain within each project. Ungrouped names form their own section.

## 3. Write the summary

```
# Agent Cluster Summary
Date: <today>
Total agents: <N>

---

## <project> / <domain>
Agents: <name-1>, <name-2>

**Active work:**
- <name>: <what they are doing>

**Done:**
- <completed items>

**Blockers:**
- <name>: <blocker>

**Coordination needed:**
- <name-A> depends on <name-B> for <thing>

---

## Overall Health
- Domains making progress: <list>
- Domains blocked: <list>
- Agents idle or done: <list>
- Suggested next actions: <recommendations>
```

## 4. Persist the summary (optional)

```
tnote name <project>-cluster-summary
# write summary to: $(tnote path --name <project>-cluster-summary)
```
