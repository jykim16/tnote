# tnote agent skills

Two skills for agentic tnote use:

| Skill | Invocation | Purpose |
|---|---|---|
| `tnote-agent` | auto + manual | Individual agent task tracking |
| `tnote-manager` | auto + manual | Primary planning agent — priorities, status, and task delegation |

---

## Claude Code

`tnote-agent` has a Claude Code-specific variant (`SKILL-claude.md`) that delegates all tnote writes to a forked sub-agent via the `Agent` tool. This keeps note content out of the main context window — the sub-agent handles file I/O and returns only a brief summary. Kiro and Codex use the generic `SKILL.md` since neither supports session-level sub-agent spawning.

```
Install the tnote agent skills into this project. Run these commands:

mkdir -p .claude/skills/tnote-agent .claude/skills/tnote-manager

curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-agent/SKILL-claude.md -o .claude/skills/tnote-agent/SKILL.md
curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-manager/SKILL.md      -o .claude/skills/tnote-manager/SKILL.md
```

**Claude Code-specific frontmatter fields** — add these to `tnote-manager`'s frontmatter after installing (`tnote-agent` already includes them in `SKILL-claude.md`):

| Skill | Additional fields |
|---|---|
| `tnote-manager` | `argument-hint: [project-name]`, `allowed-tools: Bash(tnote *)` |

---

## Kiro

```
Install the tnote agent skills into this project as Kiro steering files. Run these commands:

mkdir -p .kiro/steering

curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-agent/SKILL.md   -o .kiro/steering/tnote-agent.md
curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-manager/SKILL.md -o .kiro/steering/tnote-manager.md

Then prepend the following frontmatter to tnote-manager.md:
---
inclusion: manual
---

Then read the root and worspace directory in .kiro/agents. If agent specs exist, add the following resources to the configurations:
{
  "resources": ["file://.kiro/steering/**/*.md"]
}
```

> **Custom agents:** Steering files are only auto-loaded by the default agent. If you use a custom agent, add the steering files to your agent's `resources` in `.kiro/agents/<agent>.json`:
>
> ```json
> {
>   "resources": ["file://.kiro/steering/**/*.md"]
> }
> ```

---

## Codex

``` 
Install the tnote agent skills into this project. Run these commands:

mkdir -p .agents/skills/tnote-agent .agents/skills/tnote-manager

curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-agent/SKILL.md   -o .agents/skills/tnote-agent/SKILL.md
curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-manager/SKILL.md -o .agents/skills/tnote-manager/SKILL.md
 
For live tnote note updates, also make sure Codex can write to the default tnote note store by adding `~/.tnote` to `sandbox_workspace_write.writable_roots`.

If `~/.codex/config.toml` already exists, update that file.

Only create or update a project-scoped `.codex/config.toml` if `~/.codex/config.toml` does not already exist.

Add:

[sandbox_workspace_write]
writable_roots = ["~/.tnote"]
```

If you set `sandbox_workspace_write.writable_roots` in a project-scoped `.codex/config.toml`, include every extra writable root you still want for that project when setting that key.
