# idlefx

Idle things that live in a terminal — generative art, a small game, or an idle
companion (not decided yet) — with **stunning, smooth animations**.

## Goals

- **Look great in a terminal.** Truecolor, sub-cell resolution, fluid motion.
  The bar is "people stop and watch it", not "it works".
- **Smooth.** 60fps when things move, no tearing, no flicker, no stutter.
- **Idle-friendly.** Runs all day in a spare pane. When nothing moves, frame
  rate drops (~2fps or sleep). CPU usage at rest must be negligible.
- **One renderer, many ideas.** Build a solid rendering core first; art, game
  and companion are content on top of it.

## Stack

- **Rust** (stable, edition 2024), single static binary.
- **ratatui** — immediate-mode TUI, cell buffer with frame diffing.
- **crossterm** — backend: raw mode, alternate screen, input, synchronized output.
- **tachyonfx** — shader-like effects and transitions on ratatui buffers.
- **Custom half-block framebuffer** for the art layer: `▀` with fg = top pixel,
  bg = bottom pixel, so each cell is 2 truecolor pixels. Braille / octants are
  options for finer, monochrome-ish detail.

Target terminals: Ghostty, Kitty, WezTerm (truecolor, DEC mode 2026, GPU
rendering). Kitty graphics protocol is out of scope unless decided otherwise.

## Rendering rules

- Wrap every frame in synchronized output (begin/end sync update).
- Use the alternate screen; restore the terminal on exit **and** on panic.
- Animation is time-based (`dt` from a monotonic clock), never per-frame steps.
- Simulation runs on a fixed timestep; rendering interpolates between states.
- Motion uses easing or springs, not linear jumps.
- Adaptive frame rate: render only when something changed or an animation
  is active.
- No per-frame heap allocation in the hot path; reuse buffers.

## Ideas backlog

- **Art:** shader-style `f(x, y, t) -> rgb` scenes — plasma, flow fields,
  boids, reaction–diffusion, starfields.
- **Game:** small idle/incremental game with a real simulation loop.
- **Companion:** a sprite with a state machine (idle / sleep / react) that
  reacts to time of day, git activity, CPU load, etc. Could also live in a
  Claude Code pane or status line.

## First milestone

A ~150-line prototype: half-block framebuffer → ratatui buffer → diffed,
synchronized output, rendering a plasma effect at 60fps. Judge smoothness
in the target terminals before building anything else.
