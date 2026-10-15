---
spec: 2026-10-15-dog-companion-hooks-01-spec.md
isa: 20261016-001553_dog-companion-hooks
closed: 2026-10-16
---

# Plan — Dog companion: Claude, git, desktop and wellbeing events

## Problem

The dog reacts to five poke events (`hello`, `thinking`, `success`, `failure`, `commit`), and the only documented triggers are two Claude Code hooks and a git `post-commit`. Most of what happens around it goes unseen: Claude using tools or waiting for a permission, a context compaction, a push, a branch switch, the user leaving the desk or coming back, hours of work without a break. The dog could be a quiet signal for all of these, but it has neither the events nor the reactions, and nothing ships the hook configuration that would send them.

## Vision

You work in Claude Code with the dog in the corner of the screen: it digs while Claude works, stands up with a `?` when Claude needs you, runs off when you push, sleeps while you are away and greets you when you are back, and every 25 minutes it bows to remind you to take a break. One command set it all up and told you it worked.

## Out of Scope

Shell and CI hooks (PROMPT_COMMAND, long-running commands, `gh run watch`), a merge-conflict reaction, a project-level Claude settings file, a global `core.hooksPath`, new reactions for the five existing events, sound, and any change to the dog's routine outside the new events and the break reminder.

## Principles

- The dog only listens: every source pokes it, it never polls or watches anything.
- A hook must never get in your way: it exits 0, fast, whatever happens.
- The installer touches your configuration in the smallest way that can be undone, and says what it did.
- Timing logic is tested in simulated time; the live dog is tested only for what needs it (latency, CPU, the socket).

## Constraints

- A hook never fails or blocks: each installed hook exits 0 within 200 ms when the dog runs, when it does not, and when `idlefx-companion` is not on PATH.
- The installer never loses your configuration: it never overwrites an existing git hook or hypridle line (it appends), writes `settings.json.bak` before changing `~/.claude/settings.json`, keeps that file's key order, and writes nothing when the file is not valid JSON.
- Re-running `hooks install` changes nothing and reports the targets as already installed; `hooks uninstall` leaves every touched file as it was before the install.
- A poke shows its reaction within 100 ms (as in v1); unknown event names are still ignored.
- The five v1 events keep their reactions.
- CPU of the companion process in the default 64x16 window, averaged over 10 s: under 6% while digging (the walking bound), under 3% while alert, under 1% asleep after `sleep`.
- No heap allocation per frame; time-based animation with springs, as the root AGENTS.md says.
- Clippy clean with `-D warnings`; every existing test passes.
- One new dependency: `serde_json` with `preserve_order`; third-party versions already in `Cargo.lock` do not change.

## Goal

The dog reacts to Claude Code (session, prompt, tools, permission waits, compaction, failures, end), to git (commit, push, branch switch, merge), to the desktop (idle or locked → asleep until you return; back → a greeting) and to a pomodoro break reminder. `idlefx-companion hooks install` sets the hooks up for each source and reports success or failure per source; `hooks status` and `hooks uninstall` complete it. The hook files also ship in `crates/companion/contrib/` and are documented in the companion's AGENTS.md.

## Criteria

- [x] ISC-0: Common ground: thirteen events, one source of hook files
  - [x] ISC-0.1: Poke parses and names all thirteen events
  - [x] ISC-0.2: Installed hook files equal the contrib files byte for byte
- [x] ISC-1: Claude Code events
  - [x] ISC-1.1: Working, alert, yawn and bye pokes reach their states within 100 ms
  - [x] ISC-1.2: Repeated working pokes make one change, ending 30 s after the last
  - [x] ISC-1.3: An alert ends at the next event, a click, or 10 min
  - [x] ISC-1.4: Dig and alert poses fit inside the 64x32 canvas
  - [x] ISC-1.5: Dig and alert blend with every pose, at most 2 px per frame
  - [x] ISC-1.6: While digging, the two front paws scrape in opposite phase
  - [x] ISC-1.7: Digging stays under 6% CPU over 10 s in 64x16
  - [x] ISC-1.8: Alert stays under 3% CPU over 10 s in 64x16
  - [x] ISC-1.9: The Claude snippet maps the eight hook events to their pokes
  - [x] ISC-1.10: The dig and alert poses look right to the user
- [x] ISC-2: git events
  - [x] ISC-2.1: Push runs to the far edge and back within 1 px, then happy
  - [x] ISC-2.2: Push and curious pokes reach run and curious within 100 ms
  - [x] ISC-2.3: Git hooks poke commit, push, curious, curious; file checkout pokes nothing
  - [x] ISC-2.4: Anti: a commit or push fails without idlefx-companion on PATH
- [x] ISC-3: Desktop events
  - [x] ISC-3.1: After a sleep poke, the dog still sleeps 60 s later at 14:00
  - [x] ISC-3.2: Away, a commit poke gives happy, then sleep within 5 s
  - [x] ISC-3.3: Wake after sleep greets with happy, then the routine resumes
  - [x] ISC-3.4: Sleep and wake pokes reach the running dog within 100 ms
  - [x] ISC-3.5: Asleep after a sleep poke, the dog uses under 1% CPU
  - [x] ISC-3.6: The hypridle listener pokes sleep after 300 s, wake on resume
  - [x] ISC-3.7: The GNOME extension parses as a JavaScript module
  - [x] ISC-3.8: The GNOME extension pokes sleep and wake on lock and idle
  - [x] ISC-3.9: On Hyprland and GNOME, lock sleeps the dog, unlock greets
- [x] ISC-4: Break reminder
  - [x] ISC-4.1: A 10:00 simulation breaks at 25:00-25:01, again 5 min later
  - [x] ISC-4.2: With --break-every 0, a 2 h simulation has no break
  - [x] ISC-4.3: A simulation from 21:50 shows no break after 22:00
  - [x] ISC-4.4: After sleep then wake, the next break comes 25 min after wake
  - [x] ISC-4.5: Petting during a reminder moves the next one 25 min later
- [x] ISC-5: Event list and documentation
  - [x] ISC-5.1: poke --list prints the 13 events, each with its reaction
  - [x] ISC-5.2: The usage text names --break-every and the three hooks commands
  - [x] ISC-5.3: AGENTS.md names every event, hooks command and hook file path
- [x] ISC-6: Hook installer
  - [x] ISC-6.1: install claude adds 8 hooks, keeps keys in order, identical backup
  - [x] ISC-6.2: A second install changes no file and reports already installed
  - [x] ISC-6.3: Invalid settings.json gives a ✗, exit 1, and an unchanged file
  - [x] ISC-6.4: install git appends to post-commit, creates three executable hooks
  - [x] ISC-6.5: install git --template gives a fresh git init the four hooks
  - [x] ISC-6.6: install hypridle appends one source line and writes the listener
  - [x] ISC-6.7: install gnome copies the extension and enables it via gnome-extensions
  - [x] ISC-6.8: install gnome without gnome-extensions prints ✗ with the reason
  - [x] ISC-6.9: With no target, absent hypridle and gnome show – and exit 0
  - [x] ISC-6.10: status: installed and exit 0, then missing and exit 1 after uninstall
  - [x] ISC-6.11: uninstall leaves settings.json, git hooks, hypridle.conf byte-identical
  - [x] ISC-6.12: The PATH line shows ✗ and exit 1 without idlefx-companion on PATH
- [x] ISC-7: Anti: a v1 event changes its reaction
- [x] ISC-8: Anti: an installed hook fails or takes 200 ms or more
- [x] ISC-9: Anti: an unknown event name changes the dog's state
- [x] ISC-10: Anti: a package version from the old Cargo.lock changes
- [x] ISC-11: Anti: clippy warns or a workspace test fails

## Test Strategy

```yaml
- isc: ISC-0.1
  serves: S1+S2+S3+S5
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(cargo test -q -p idlefx-companion brain::tests::poke_names_round_trip 2>&1) && grep -qE \"test result: ok\\. [1-9][0-9]* passed\" <<<\"$out\""
  why: "S1, S2 and S3 each add events to the same Poke list; S5 lists them all"
- isc: ISC-0.2
  serves: S1+S2+S3+S6
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/hooks.sh same-source"
  why: the installer (S6) embeds the hook files of S1-S3 that also ship in contrib; two copies would drift apart
- isc: ISC-1.1
  anchors_to: S1
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/dog.sh claude-pokes"
- isc: ISC-1.2
  anchors_to: S1
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(cargo test -q -p idlefx-companion brain::tests::working_extends 2>&1) && grep -qE \"test result: ok\\. [1-9][0-9]* passed\" <<<\"$out\""
- isc: ISC-1.3
  anchors_to: S1
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(cargo test -q -p idlefx-companion brain::tests::alert_ends 2>&1) && grep -qE \"test result: ok\\. [1-9][0-9]* passed\" <<<\"$out\""
- isc: ISC-1.4
  anchors_to: S1
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(cargo test -q -p idlefx-companion dog::tests::new_poses_fit_canvas 2>&1) && grep -qE \"test result: ok\\. [1-9][0-9]* passed\" <<<\"$out\""
- isc: ISC-1.5
  anchors_to: S1
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(cargo test -q -p idlefx-companion dog::tests::new_poses_blend 2>&1) && grep -qE \"test result: ok\\. [1-9][0-9]* passed\" <<<\"$out\""
- isc: ISC-1.6
  anchors_to: S1
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(cargo test -q -p idlefx-companion dog::tests::dig_paws_alternate 2>&1) && grep -qE \"test result: ok\\. [1-9][0-9]* passed\" <<<\"$out\""
- isc: ISC-1.7
  anchors_to: S1
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/dog.sh cpu-working"
- isc: ISC-1.8
  anchors_to: S1
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/dog.sh cpu-alert"
- isc: ISC-1.9
  anchors_to: S1
  kind: config
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/hooks.sh claude-snippet"
  fails-when: "a hook event is missing, pokes the wrong event, or Notification lacks the permission_prompt matcher"
- isc: ISC-1.10
  anchors_to: S1
  kind: manual
- isc: ISC-2.1
  anchors_to: S2
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(cargo test -q -p idlefx-companion brain::tests::push_runs_and_returns 2>&1) && grep -qE \"test result: ok\\. [1-9][0-9]* passed\" <<<\"$out\""
- isc: ISC-2.2
  anchors_to: S2
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/dog.sh git-pokes"
- isc: ISC-2.3
  anchors_to: S2
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/hooks.sh git-events"
- isc: ISC-2.4
  anchors_to: S2
  kind: regression
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/hooks.sh git-no-binary"
  fails-when: "with the binary missing, the post-commit or pre-push hook exits non-zero and git aborts"
- isc: ISC-3.1
  anchors_to: S3
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(cargo test -q -p idlefx-companion brain::tests::sleep_holds 2>&1) && grep -qE \"test result: ok\\. [1-9][0-9]* passed\" <<<\"$out\""
- isc: ISC-3.2
  anchors_to: S3
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(cargo test -q -p idlefx-companion brain::tests::away_reaction_returns_to_sleep 2>&1) && grep -qE \"test result: ok\\. [1-9][0-9]* passed\" <<<\"$out\""
- isc: ISC-3.3
  anchors_to: S3
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(cargo test -q -p idlefx-companion brain::tests::wake_greets 2>&1) && grep -qE \"test result: ok\\. [1-9][0-9]* passed\" <<<\"$out\""
- isc: ISC-3.4
  anchors_to: S3
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/dog.sh desktop-pokes"
- isc: ISC-3.5
  anchors_to: S3
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/dog.sh cpu-away"
- isc: ISC-3.6
  anchors_to: S3
  kind: config
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/hooks.sh hypridle-file"
  fails-when: "the listener block, its timeout of 300, or one of the two pokes is missing"
- isc: ISC-3.7
  anchors_to: S3
  kind: file
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/hooks.sh gnome-parse"
  fails-when: node --check reports a syntax error in extension.js
- isc: ISC-3.8
  anchors_to: S3
  kind: file
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/hooks.sh gnome-pokes"
  fails-when: "extension.js lacks the lock or idle watch, or never spawns poke sleep / poke wake"
- isc: ISC-3.9
  anchors_to: S3
  kind: manual
- isc: ISC-4.1
  anchors_to: S4
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/dog.sh break-sim"
- isc: ISC-4.2
  anchors_to: S4
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/dog.sh break-off"
- isc: ISC-4.3
  anchors_to: S4
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/dog.sh break-night"
- isc: ISC-4.4
  anchors_to: S4
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(cargo test -q -p idlefx-companion brain::tests::break_after_wake 2>&1) && grep -qE \"test result: ok\\. [1-9][0-9]* passed\" <<<\"$out\""
- isc: ISC-4.5
  anchors_to: S4
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(cargo test -q -p idlefx-companion brain::tests::break_pet_snoozes 2>&1) && grep -qE \"test result: ok\\. [1-9][0-9]* passed\" <<<\"$out\""
- isc: ISC-5.1
  anchors_to: S5
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/dog.sh poke-list"
- isc: ISC-5.2
  anchors_to: S5
  kind: doc
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/dog.sh usage"
  fails-when: "--help lacks --break-every, hooks install, hooks status or hooks uninstall"
- isc: ISC-5.3
  anchors_to: S5
  kind: doc
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/hooks.sh agents-doc"
  fails-when: "one of the 13 events, the 3 hooks commands or a shipped contrib hook file path is missing from crates/companion/AGENTS.md"
- isc: ISC-6.1
  anchors_to: S6
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/hooks.sh claude-merge"
- isc: ISC-6.2
  anchors_to: S6
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/hooks.sh idempotent"
- isc: ISC-6.3
  anchors_to: S6
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/hooks.sh claude-invalid"
- isc: ISC-6.4
  anchors_to: S6
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/hooks.sh git-append"
- isc: ISC-6.5
  anchors_to: S6
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/hooks.sh git-template"
- isc: ISC-6.6
  anchors_to: S6
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/hooks.sh hypridle"
- isc: ISC-6.7
  anchors_to: S6
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/hooks.sh gnome-stub"
- isc: ISC-6.8
  anchors_to: S6
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/hooks.sh gnome-missing"
- isc: ISC-6.9
  anchors_to: S6
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/hooks.sh default-skips"
- isc: ISC-6.10
  anchors_to: S6
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/hooks.sh status"
- isc: ISC-6.11
  anchors_to: S6
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/hooks.sh uninstall-restores"
- isc: ISC-6.12
  anchors_to: S6
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/hooks.sh path-line"
- isc: ISC-7
  anchors_to: Constraints
  kind: regression
  tool: "cd /home/jolo/dev/idlefx && out=$(cargo test -q -p idlefx-companion brain::tests::v1_pokes_keep_reactions 2>&1) && grep -qE \"test result: ok\\. [1-9][0-9]* passed\" <<<\"$out\""
  fails-when: "hello, success or commit no longer give happy, failure no longer sad, or thinking no longer thinking"
- isc: ISC-8
  anchors_to: Constraints
  kind: regression
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/hooks.sh hook-timing"
  fails-when: "a Claude, git or desktop hook command exits non-zero or runs 200 ms or longer, with the dog running, stopped, or the binary off PATH"
- isc: ISC-9
  anchors_to: Constraints
  kind: regression
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/dog.sh unknown-ignored"
  fails-when: the trace shows a state change caused by poke bogus
- isc: ISC-10
  anchors_to: Constraints
  kind: regression
  tool: "cd /home/jolo/dev/idlefx && crates/companion/tests/dog.sh lock-versions"
  fails-when: a name and version pair of Cargo.lock at d5add4f is missing from the current Cargo.lock
- isc: ISC-11
  anchors_to: Constraints
  kind: regression
  tool: "cd /home/jolo/dev/idlefx && cargo clippy -q --workspace --all-targets -- -D warnings && cargo test -q --workspace"
  fails-when: clippy prints a warning or any test of the workspace fails
```

## Decisions

- 2026-10-16 00:33: Jev doubted the probes of ISC-1.9, 3.6, 3.7, 3.8, 5.2 and 9: a mutation run showed 1.9 (wrong poke, wrong matcher), 3.6 (timeout 600, wrong resume poke), 3.7 (syntax error) and 3.8 (wake poke removed) each fail; 5.2 greps the four exact strings of --help; ISC-9 is the v1 socket check, kept as a regression guard
- 2026-10-16 00:33: simulate truncated its step count (1800 s gave 215999 steps), so a reminder due at exactly 30:00 never ran; it now rounds, and a 1800 s simulation covers 1800 s
- 2026-10-16 00:33: a break reminder waits only for the called walk and the push run, not for the dog's own wander, so it fires on time; appended git blocks save and restore $? so a failing user hook still fails

## Review

Reviewed 2026-10-16 00:19 · criteria #d4fe4449 · Jev: 1 request, 56 questions
- shared: S1, S2 and S3 all add events to one Poke list that S5 lists (ISC-0.1); the installer of S6 writes the hook files S1-S3 ship in contrib, so they must be one source (ISC-0.2)
- prerequisites: none beyond ISC-0; the test scripts crates/companion/tests/dog.sh and hooks.sh are written first in BUILD, red before the code
- contradictions: none
- drift: none
- gaps: the 100 ms latency of the Constraints is checked for every new event (ISC-1.1, ISC-2.2, ISC-3.4); the paw animation of S1's description (ISC-1.6) and the PATH line of S6's description (ISC-6.12) are not in Accepted-when lines but are stated in their sections
- context: none; every leaf is anchored to a section or to the Constraints
- redundant: none; ISC-0.2 (same bytes) and ISC-1.9 / ISC-3.6 (content of the files) check different things

Flags (each needs an answer before the ISA ack):
- R1: serves ISC-1.6 — no 0.81 ≥ 0.70 — Does criterion ISC-1.6 ("While digging, the two front paws scrape in opposite phase", see `criteria`) help solve the spec's stated Problem and reach its Goal, and deliver what section S1 ("Claude Code events") asks for? Answer yes when the Problem needs what it claims; no when it is unrelated to the Problem and the Goal, or only restates a detail no part of the spec asks for. — answer: keep: a frozen crouch with no paw motion would pass every other criterion, and the dog would not look like it is working while Claude runs tools
- R2: serves ISC-1.10 — no 0.74 ≥ 0.70 — Does criterion ISC-1.10 ("The dig and alert poses look right to the user", see `criteria`) help solve the spec's stated Problem and reach its Goal, and deliver what section S1 ("Claude Code events") asks for? Answer yes when the Problem needs what it claims; no when it is unrelated to the Problem and the Goal, or only restates a detail no part of the spec asks for. — answer: keep: fit and blend tests cannot see whether a pose reads as digging or alert; the old sit pose passed every test yet looked like peeing
- R3: serves ISC-10 — no 0.83 ≥ 0.70 — Does criterion ISC-10 ("Anti: a package version from the old Cargo.lock changes", see `criteria`) help solve the spec's stated Problem and reach its Goal? Answer yes when the Problem needs what it claims; no when it is unrelated to the Problem and the Goal, or only restates a detail no part of the spec asks for. — answer: keep: serde_json with preserve_order pulls indexmap and hashbrown, which ratatui already uses; cargo could bump them silently and no test would fail

## Verification

- Goal: yes — 13 events with their reactions, the 25/5 min break reminder, and hooks install/status/uninstall with ✓/✗/– per target; every probe green, and the user saw the poses, the Claude hooks, and Hyprland and GNOME lock/unlock work on their desktop
- Ask 1: met — Claude (8 hooks), git (4 hooks), desktop (hypridle listener, GNOME extension) and the wellbeing break reminder, confirmed live by the user
- Ask 2: met — idlefx-companion hooks install|status|uninstall prints ✓/✗/– per target and exits 1 on any ✗ (ISC-6.1 to 6.12); the user installed with it
