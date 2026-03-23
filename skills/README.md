# tnote agent skills

Three skills for agentic tnote use:

| Skill | Invocation | Purpose |
|---|---|---|
| `tnote-track` | auto + manual | Individual agent task tracking |
| `tnote-summarize` | manual only | Summarize all agent notes across a cluster |
| `tnote-plan` | manual only | Inject tasks into agent notes |

---

## Claude Code

```
Install the tnote agent skills into this project. Run these commands:

mkdir -p .claude/skills/tnote-track .claude/skills/tnote-summarize .claude/skills/tnote-plan

curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-track/SKILL.md    -o .claude/skills/tnote-track/SKILL.md
curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-summarize/SKILL.md -o .claude/skills/tnote-summarize/SKILL.md
curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-plan/SKILL.md     -o .claude/skills/tnote-plan/SKILL.md
```

---

## Kiro

```
Install the tnote agent skills into this project as Kiro steering files. Run these commands:

mkdir -p .kiro/steering

curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-track/SKILL.md    -o .kiro/steering/tnote-track.md
curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-summarize/SKILL.md -o .kiro/steering/tnote-summarize.md
curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-plan/SKILL.md     -o .kiro/steering/tnote-plan.md

Then prepend the following frontmatter to tnote-summarize.md and tnote-plan.md:
---
inclusion: manual
---
```

---

## Codex

```
Install the tnote agent skills into this project. Run these commands:

mkdir -p .codex/skills/tnote-track .codex/skills/tnote-summarize/agents .codex/skills/tnote-plan/agents

curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-track/SKILL.md    -o .codex/skills/tnote-track/SKILL.md
curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-summarize/SKILL.md -o .codex/skills/tnote-summarize/SKILL.md
curl -sL https://raw.githubusercontent.com/jykim16/tnote/main/skills/tnote-plan/SKILL.md     -o .codex/skills/tnote-plan/SKILL.md

printf 'policy:\n  allow_implicit_invocation: false\n' | tee .codex/skills/tnote-summarize/agents/openai.yaml .codex/skills/tnote-plan/agents/openai.yaml
```
