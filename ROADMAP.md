# Roadmap

Feature ideas for tnote, roughly ordered by term. This is a brainstorm, not a
commitment — nothing here is scheduled. See `CHANGELOG.md` for what's shipped.

## Context

tnote started as a per-window scratch note for humans, but its fastest-growing
use case is coordinating multiple parallel coding-agent sessions (see
`skills/tnote-agent` and `skills/tnote-manager`). Several items below are
aimed at that use case specifically.

## Near-term

- **`tnote updgrade`** — self update from Github releases
- **`tnote installation integration tests`** — makes sure tnote's latest version
  is installable after release
- **`tnote list --status <blocked|in-progress|done>`** — filter notes by their
  `## Status:` header. Today a manager agent has to `tnote show` every note
  individually to find blockers; status isn't parsed at all currently.
- **`tnote name --template manager|agent`** — scaffold the note skeleton the
  `tnote-agent`/`tnote-manager` skills already hand-write, so the two skills'
  documented formats can't silently drift apart.
- **tmux status-bar indicator** — mark a window's status label when its note
  has changed since last viewed, so you know which windows need attention
  without popping each one open. Directly serves the tool's core pitch
  (reduce context-switch cost).
- **Git worktrees for agent workspaces** — when a manager spawns an agent
  (`tnote-manager` skill §7), the agent's `Workspace` should be a dedicated
  `git worktree` checkout, not the same shared clone the manager works in.
  Keeps parallel agents from colliding on uncommitted changes or branch
  state; the manager itself stays in the main working directory throughout.
- **Land `tnote goto -n <name>`** — note-to-terminal navigation. Prototyped
  and validated on `agent/tnote-research-goto`: jumps the caller's terminal
  to a named note's bound tmux window via `select-window` chained with
  `switch-client` (inside tmux) or `attach-session` (outside), reusing the
  existing reverse name→key lookup (`Notes::link_sources`). Handles unbound
  names, stale/dead bindings, shell-only bindings, and a name bound to
  multiple live windows (all exit nonzero with a specific message) as
  distinct cases. Known unresolved edge case, deliberately shipped
  unguarded: a process that inherits `$TMUX` but was never dispatched by a
  real tmux client (e.g. an agent's detached subprocess, a cron job) makes
  `switch-client`'s client-resolution ambiguous — it could in principle
  redirect an unrelated attached client instead of erroring. Tried gating
  this on `stdout.is_terminal()`, but that would also break the legitimate
  case: tnote's own `:tnote-show`/`:tnote-list` command-line aliases run via
  `run-shell`, which never gives the spawned process a tty either, even
  though it's a real client-dispatched invocation. The two cases aren't
  distinguishable from inside the spawned process with anything tmux
  exposes to `run-shell` commands — needs either a tmux-side signal or
  accepting the risk.

## Mid-term

- **`--json` output** on `list`/`show` — agents currently scrape formatted
  text output; structured output is cheaper and less brittle for them to
  parse, and composes with `jq` for ad hoc queries.
- **Archive retention policy** — auto-purge archived notes after N days,
  rather than only manual `--archive`/`--unarchive`. on clean remove archived
  notes that are older than specified in config --advanced
- **LLM wiki + tnote exploration** - evaludate whether Karpathy's LLM wiki
  idea can combine cleanly with tnote

## Long-term / needs more design

- **Cross-machine note sync** — notes currently live only in the local
  `~/.tnote`.
- **Windows support** — currently Unix-only by design (relies on tmux +
  Unix process semantics). Large scope; revisit if demand shows up.

## Explicitly not planned

- Nothing yet — add items here as ideas get considered and rejected, with a
  one-line reason, so they don't get re-litigated.
