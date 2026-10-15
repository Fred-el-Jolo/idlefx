---
status: "acked 2026-10-16 #e4596b17"
effort: E3
---

# Dog companion: Claude, git, desktop and wellbeing events

## Problem

The dog reacts to five poke events (`hello`, `thinking`, `success`, `failure`, `commit`), and the only documented triggers are two Claude Code hooks and a git `post-commit`. Most of what happens around it goes unseen: Claude using tools or waiting for a permission, a context compaction, a push, a branch switch, the user leaving the desk or coming back, hours of work without a break. The dog could be a quiet signal for all of these, but it has neither the events nor the reactions, and nothing ships the hook configuration that would send them.

## Goal

The dog reacts to Claude Code (session, prompt, tools, permission waits, compaction, failures, end), to git (commit, push, branch switch, merge), to the desktop (idle or locked → asleep until you return; back → a greeting) and to a pomodoro break reminder. `idlefx-companion hooks install` sets the hooks up for each source and reports success or failure per source; `hooks status` and `hooks uninstall` complete it. The hook files also ship in `crates/companion/contrib/` and are documented in the companion's AGENTS.md.
Said:
- "can you add propose me several hooks to make the dog interactive (git, claude...)"
- "OK nice, add claude, git Desktop & Wellbeing events" (the rows of the proposed table for those four sources)
- desktop: "Both (Recommended)": a hypridle snippet for Hyprland, and the GNOME Shell extension pokes the dog on screen lock/unlock
- wellbeing: "25 min (pomodoro)": "Every 25 min of work, repeated every 5 min until a lock."
- "Did you plan an installer for those, with a success/failure status ?"
- installer: "install + status + uninstall (Recommended)": install prints ✓/✗/– per target with the reason and exits 1 if any target failed; status reports installed / partial / missing per target, and whether the binary is on PATH and the dog runs; uninstall removes exactly what install added; re-running install is safe
- git scope: "Current repo + --template (Recommended)": the current repo's hooks by default, appending to an existing hook and never overwriting it; `--template` also sets them in a git template directory for new clones
- JSON: "Add serde_json (Recommended)": with `preserve_order`, a `settings.json.bak` backup before any change, invalid JSON → ✗ with nothing written
Assumed:
- new events: `working`, `alert`, `yawn`, `bye` (Claude), `push`, `curious` (git), `sleep`, `wake` (desktop); the break reminder is the dog's own timer, not an event
- Claude Code mapping: SessionStart → `hello`, UserPromptSubmit → `thinking`, PreToolUse → `working`, Notification with matcher `permission_prompt` → `alert`, Stop → `success`, StopFailure → `failure`, PreCompact → `yawn`, SessionEnd → `bye`; PostToolUseFailure is not mapped (a grep with no match is a tool failure, so the dog would be sad all the time), nor `idle_prompt` (it fires a minute after every answer)
- `working`: the dog digs (head low, front paws scraping in turn) until another event, or 30 s after the last `working`; repeated `working` pokes extend it without a new state change
- `alert`: the dog stands tall, ears and tail up, a blinking `?` above its head, until the next event or a click, at most 10 min
- `yawn`: the play-bow stretch (the dog has no mouth to yawn with)
- `bye`: the dog lies down, then carries on with its routine
- git mapping: post-commit → `commit` (as today), pre-push → `push` (runs to the far edge of the window and back, then happy with a `!`), post-checkout on a branch switch and post-merge → `curious`
- the merge-conflict reaction from the proposal is dropped: git runs no hook when a merge or rebase stops on conflicts
- `sleep`: the dog lies down and sleeps until `wake` or a click; other events still get their reaction, then it goes back to sleep; `wake`: happy greeting
- Hyprland: a hypridle listener (5 min idle → `sleep`, resume → `wake`), plus the documented `on_lock_cmd` / `on_unlock_cmd` lines for a manual lock; GNOME: the shipped extension pokes `sleep` on screen lock and after 5 min idle, `wake` on unlock and on activity
- break reminder: 25 min after the dog starts or after the last `wake`, the dog does a play bow with a steaming cup above its head (about 6 s), again every 5 min until `sleep`; petting the dog during a reminder counts as taking the break (the next one comes 25 min later); only between 07:00 and 22:00; `--break-every MIN` changes the 25 (0 turns it off), for the live dog and `simulate`
- every hook calls `idlefx-companion poke <event>` and exits 0 even when the dog or the binary is missing, so it never blocks Claude, a commit or a push
- the installer writes Claude Code's user settings (`~/.claude/settings.json`), not a project's `.claude/settings.json`
- `hooks install` with no target installs what applies on this machine: claude always, git inside a repo, hypridle when `~/.config/hypr/hypridle.conf` exists, gnome when `gnome-extensions` is on PATH; the others show `–` with the reason; a target you name that cannot apply is a ✗
- the hook files are built into the binary, so the installer works from `~/.cargo/bin` without the repo
- `hooks status` exits 0 when every applicable target is installed, 1 otherwise
- every path comes from `$HOME`, so the installer is tested on a scratch home
- this machine has no Hyprland or GNOME session: the live lock/unlock behaviour on each desktop and the look of the new poses are confirmed by you

## Out of scope

Shell and CI hooks (PROMPT_COMMAND, long-running commands, `gh run watch`), a merge-conflict reaction, a project-level Claude settings file, a global `core.hooksPath`, new reactions for the five existing events, sound, and any change to the dog's routine outside the new events and the break reminder.

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

## Approaches

1. Pokes over the existing socket, reactions in the brain, hook files built into the binary and installed by `idlefx-companion hooks install` (chosen). Every source already has a hook mechanism (Claude Code hooks, git hooks, hypridle, a GNOME extension); each calls `idlefx-companion poke <event>`. The dog stays a passive listener, costs nothing between events, and the break timer is a few lines in the brain. The installer edits each source's own configuration in the smallest way that can be undone, and says per source whether it worked.
2. The dog watches the sources itself (inotify on `.git` directories, D-Bus for the screen lock, Claude's transcripts). No setup, but the dog would depend on every tool, wake up for unrelated file changes, and need to discover repos. Rejected.
3. Hook files to copy by hand, no installer. Nothing touches your configuration, but setup is four manual steps with no way to tell whether they worked. Rejected by the user ("an installer … with a success/failure status"); the files still ship in `contrib/` for manual setups.

## S1 — Claude Code events

Four new events and their reactions, plus a Claude Code settings snippet that sends all eight Claude events. `working` makes the dog dig: head low, front paws scraping in turn, at the walking frame rate; it lasts until another event or 30 s after the last `working`, and a repeated `working` only extends it. `alert` makes it stand tall with ears and tail up and a blinking `?` above its head, until the next event or a click, at most 10 min. `yawn` is the play-bow stretch, and `bye` lies the dog down. The snippet is `crates/companion/contrib/claude/settings.json`, a `hooks` object you merge into `~/.claude/settings.json`.
Accepted when:
- `poke working`, `poke alert`, `poke yawn` and `poke bye` each move the running dog to digging, alert, stretch and lie, within 100 ms, with cause `poke:<event>` in the trace
- three `poke working` 10 s apart give a single state change, and the dog leaves digging 30 s after the last one
- an alert ends at the next event, at a click, or after 10 min
- the digging and alert poses fit the 64x32 canvas and blend into every other pose without a jump (the v1 continuity test, at most 2 px per frame)
- digging stays under 6% CPU and alert under 3%, over 10 s in the default window
- the snippet maps SessionStart → hello, UserPromptSubmit → thinking, PreToolUse → working, Notification (`permission_prompt`) → alert, Stop → success, StopFailure → failure, PreCompact → yawn, SessionEnd → bye

## S2 — git events

Two new events and four git hook scripts. `push` makes the dog run (faster than its walk) to the far edge of its window and back to where it was, then be happy with a `!`. `curious` is the v1 curious look (head tilted, ears up, 1.5 s). The scripts live in `crates/companion/contrib/git/hooks/`: `post-commit` → `commit`, `pre-push` → `push`, `post-checkout` → `curious` on a branch switch only (not on a file checkout), `post-merge` → `curious`. Each is a POSIX `sh` script, executable, that pokes and exits 0; AGENTS.md explains copying them into a repo's `.git/hooks/` or into a git template directory, and appending the line to a hook that already exists.
Accepted when:
- `poke push` makes the running dog reach the far edge and come back within 1 px of where it started, then go happy with cause `poke:push`
- `poke curious` moves the dog to curious with cause `poke:curious`
- in a scratch repo with the scripts installed and a stand-in `idlefx-companion` on PATH, a commit, a push to a local bare remote, a branch switch and a merge send `commit`, `push`, `curious` and `curious`, and a file checkout sends nothing
- with no `idlefx-companion` on PATH, the commit and the push still succeed

## S3 — Desktop events

Two new events and the desktop side for Hyprland and GNOME. `sleep` puts the dog to sleep and keeps it asleep (no routine) until `wake` or a click; any other event still gets its reaction, then the dog goes back to sleep. `wake` is a happy greeting and ends the away state. Hyprland: `crates/companion/contrib/hypridle/idlefx-companion.conf`, sourced from `hypridle.conf`, a listener that pokes `sleep` after 5 min idle and `wake` on resume; AGENTS.md also gives the `on_lock_cmd` / `on_unlock_cmd` lines for a manual lock. GNOME: the shipped Shell extension pokes `sleep` when the screen locks or after 5 min without input, and `wake` when it unlocks or input returns.
Accepted when:
- after `poke sleep` the running dog sleeps and is still asleep 60 s later at 14:00 (the routine would have woken it)
- after `poke sleep`, `poke commit` gives happy, then the dog is asleep again within 5 s
- `poke wake` after `poke sleep` gives happy with cause `poke:wake`, then the normal routine
- asleep after `sleep`, the dog uses under 1% CPU over 10 s
- the hypridle file holds a listener with `timeout = 300`, `on-timeout` poking `sleep` and `on-resume` poking `wake`
- the GNOME extension parses as JavaScript and pokes `sleep` / `wake` on lock, unlock, idle and activity
- on your Hyprland and your GNOME desktop, locking puts the dog to sleep and unlocking greets you

## S4 — Break reminder

A pomodoro timer inside the dog. A work stretch starts when the dog starts and at every `wake`; `sleep` ends it. After 25 min of a stretch, between 07:00 and 22:00, the dog does a play bow with a small steaming cup above its head for about 6 s (cause `break` in the trace), then goes back to its routine; it reminds you again every 5 min until `sleep`. Petting the dog during a reminder counts as taking the break: the next reminder comes 25 min later. A reminder due while the dog walks or runs waits for its arrival. `--break-every MIN` sets the 25 (0 turns the reminder off), on the live dog and on `simulate`.
Accepted when:
- `simulate --start 10:00 --duration 1800` shows a stretch with cause `break` between 25:00 and 25:01 after start, and another 5 min later
- with `--break-every 0`, a 2 h simulation shows no `break`
- a simulation starting at 21:50 for 1 h shows no `break` after 22:00
- after `sleep` then `wake`, the next reminder comes 25 min after the `wake`
- petting the dog during a reminder moves the next one 25 min later

## S5 — Event list and documentation

The thirteen events are listed everywhere a user looks for them: `poke --list`, the usage text and the companion's AGENTS.md. AGENTS.md gets one table of events with their reactions and sources, the `hooks install / status / uninstall` commands first, then a manual setup paragraph per source (Claude Code, git, Hyprland, GNOME) and the break reminder with its option, replacing the two v1 snippets.
Accepted when:
- `poke --list` prints the 13 events (`hello`, `thinking`, `success`, `failure`, `commit`, `working`, `alert`, `yawn`, `bye`, `push`, `curious`, `sleep`, `wake`), one per line with its reaction
- the usage text names `--break-every` and `hooks install`, `hooks status`, `hooks uninstall`
- AGENTS.md names every event, the three `hooks` commands and the path of every shipped hook file

## S6 — Hook installer

`idlefx-companion hooks install [claude] [git] [hypridle] [gnome] [--template]`, `hooks status` and `hooks uninstall` with the same targets. Each prints one line per target: `✓` done, `✗` failed, `–` skipped, then the target and what happened or why (`✓ claude  ~/.claude/settings.json: 8 hooks added, backup settings.json.bak`), plus a first line saying whether `idlefx-companion` is on PATH (the hooks call it by name; not on PATH is a ✗). Install and uninstall exit 1 when any line is a ✗, else 0.
- claude: adds one hook per event of S1 to `~/.claude/settings.json` (creating it if missing), each tagged by its command `idlefx-companion poke <event>`; an entry already there is not added twice.
- git: the S2 scripts into the current repo's hooks directory (`git rev-parse --git-path hooks`); a hook that already exists gets an `# idlefx-companion` block appended. With `--template`, the same into the git template directory (`init.templateDir`, set to `~/.config/git/template` when unset), for new clones.
- hypridle: writes `~/.config/hypr/idlefx-companion-hypridle.conf` and appends a `source =` line to `hypridle.conf`; says to restart hypridle.
- gnome: copies the extension into `~/.local/share/gnome-shell/extensions/` and enables it with `gnome-extensions enable`; says a log out and back in is needed on Wayland.
- status: per target `installed`, `partial (n/m)` or `missing`, plus whether the dog is running; uninstall removes exactly what install added (our entries, blocks, files and lines) and nothing else.
Accepted when:
- on a scratch home with an existing `settings.json` holding other hooks and settings, `hooks install claude` prints ✓, adds the 8 hooks, keeps every other key in its order, and writes `settings.json.bak` identical to the old file
- a second `hooks install` changes no file and reports already installed
- `hooks install claude` on a `settings.json` that is not valid JSON prints ✗, exits 1 and leaves the file unchanged
- in a scratch repo whose `post-commit` already exists, `hooks install git` keeps the old content, appends the block, and creates the three other hooks executable
- `hooks install git --template` installs the four hooks into the template directory, and a fresh `git init` gets them
- `hooks install hypridle` on a scratch home with a `hypridle.conf` appends one `source =` line and writes the listener file
- `hooks install gnome` with a stand-in `gnome-extensions` copies the extension and calls `enable idlefx-companion@idlefx`; with none on PATH it prints ✗ with the reason
- with no target, a scratch home without hypridle or gnome shows `–` for both and exits 0
- `hooks status` reports `installed` for each installed target and exits 0, and `missing` plus exit 1 after `hooks uninstall`
- after `hooks uninstall`, `settings.json`, the git hooks and `hypridle.conf` are byte-identical to before the install

## Decisions

- 2026-10-15: the dog watching the sources itself rejected (see Approaches).
- 2026-10-15: the merge-conflict reaction dropped: git runs no hook when a merge or rebase stops on conflicts.
- 2026-10-15: PostToolUseFailure and `idle_prompt` left unmapped: both fire far too often to mean anything.
- 2026-10-16: an installer added at the user's request (S6); `serde_json` accepted as the one new dependency to edit `settings.json` safely.

## Open questions
