# tnote agent skills

Three skills for agentic tnote use:

| Skill | Invocation | Purpose |
|---|---|---|
| `tnote-track` | auto + manual | Individual agent task tracking |
| `tnote-summarize` | manual only | Summarize all agent notes across a cluster |
| `tnote-manager` | manual only | Primary planning agent — priorities, status, and task delegation |

---

## Claude Code

```
Install the tnote agent skills into this project. Run these commands:

mkdir -p .claude/skills/tnote-track .claude/skills/tnote-summarize .claude/skills/tnote-manager

curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-track/SKILL.md    -o .claude/skills/tnote-track/SKILL.md
curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-summarize/SKILL.md -o .claude/skills/tnote-summarize/SKILL.md
curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-manager/SKILL.md  -o .claude/skills/tnote-manager/SKILL.md
```

---

## Kiro

```
Install the tnote agent skills into this project as Kiro steering files. Run these commands:

mkdir -p .kiro/steering

curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-track/SKILL.md    -o .kiro/steering/tnote-track.md
curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-summarize/SKILL.md -o .kiro/steering/tnote-summarize.md
curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-manager/SKILL.md  -o .kiro/steering/tnote-manager.md

Then prepend the following frontmatter to tnote-summarize.md and tnote-manager.md:
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

mkdir -p .codex/skills/tnote-track .codex/skills/tnote-summarize/agents .codex/skills/tnote-manager/agents

curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-track/SKILL.md    -o .codex/skills/tnote-track/SKILL.md
curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-summarize/SKILL.md -o .codex/skills/tnote-summarize/SKILL.md
curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-manager/SKILL.md  -o .codex/skills/tnote-manager/SKILL.md

printf 'policy:\n  allow_implicit_invocation: false\n' | tee .codex/skills/tnote-summarize/agents/openai.yaml .codex/skills/tnote-manager/agents/openai.yaml
```
