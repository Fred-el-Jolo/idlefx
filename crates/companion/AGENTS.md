# idlefx-companion

Idle companions that live in a spare terminal pane all day. Shared rules
(rendering, smoothness, idle CPU) are in the root `AGENTS.md`.

## Idea

- A sprite with a state machine (idle / sleep / react) and smooth,
  spring-driven motion.
- Reacts to its surroundings: time of day, git activity, CPU load, etc.
- Could also live in a Claude Code pane or status line.

## Specific rules

- Mostly idle: the frame rate drops to ~2fps or sleeps when nothing moves.
  CPU usage at rest must be negligible.

Status: not started.
