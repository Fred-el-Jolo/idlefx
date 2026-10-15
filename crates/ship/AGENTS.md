# idlefx-ship

A spaceship game with realistic flight, in the spirit of Star Citizen. Shared
rules (rendering, smoothness) are in the root `AGENTS.md`.

## Idea

- Newtonian 6DOF flight with flight assist, rendered in 3D into the
  half-block framebuffer (with a depth buffer), plus a HUD.
- Steady 60fps while flying.

## Decided in the brainstorm (2026-10-14)

- **Input:** the keyboard is the standard input; a gamepad is optional.
- **Control panel:** if keyboard precision is not enough, an in-terminal
  control panel (sliders, knobs) for fine settings such as throttle.
- **Cameras:** several views, especially for big ships: a cockpit view
  (horizon, roll), a top view (forward/backward, left/right) and a side view
  (pitch, longitudinal tilt).
- **Realism:** realistic piloting feel — mass, inertia, gravity, small ships
  nimble and huge ships slow to react, landing as a patient, precise
  procedure — without extra realism beyond what the feel needs.
- **Travel:** long travel can be accelerated (time compression or a jump
  drive), so distances never become boring.

## Specific needs

- Held keys (thrust, rotation) need key release events: the Kitty keyboard
  protocol, with a key-repeat fallback where the terminal lacks it.
- Mouse aim may use pixel mouse reporting (mode 1016) where supported.
- The 3D rasterizer stays in this crate until another crate needs it.

Status: not started. First step planned: a flight-feel spike (one ship, a
starfield, keyboard only).
