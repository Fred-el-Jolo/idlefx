# idlefx-ship

A spaceship game with realistic flight, in the spirit of Star Citizen. Shared
rules (rendering, smoothness) are in the root `AGENTS.md`.

## Idea

- Newtonian 6DOF flight with flight assist, rendered in 3D into the
  half-block framebuffer (with a depth buffer), plus a HUD.
- Steady 60fps while flying.

## Specific needs

- Held keys (thrust, rotation) need key release events: the Kitty keyboard
  protocol, with a key-repeat fallback where the terminal lacks it.
- Mouse aim may use pixel mouse reporting (mode 1016) where supported.
- The 3D rasterizer stays in this crate until another crate needs it.

Status: not started. First step planned: a flight-feel spike (one ship, a
starfield, keyboard only).
