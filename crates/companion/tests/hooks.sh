#!/usr/bin/env bash
# Checks of the shipped hook files and of `idlefx-companion hooks`, on a
# scratch home and scratch repos. Usage: crates/companion/tests/hooks.sh <case>
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
CONTRIB=$ROOT/crates/companion/contrib
cd "$ROOT"
cargo build --release -q -p idlefx-companion

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

NODE=$(command -v node || ls "$HOME"/.local/share/mise/installs/node/*/bin/node 2>/dev/null | head -1 || true)

# A scratch home, git without the user's config, and only the dog's binary
# (plus the system) on PATH.
export HOME=$TMP/home
mkdir -p "$HOME" "$TMP/bin"
unset XDG_CONFIG_HOME
export GIT_CONFIG_NOSYSTEM=1
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t
ln -s "$ROOT/target/release/idlefx-companion" "$TMP/bin/idlefx-companion"
SYS=/usr/bin:/bin
export PATH=$TMP/bin:$SYS

HOOKS=(post-commit pre-push post-checkout post-merge)
SETTINGS=$HOME/.claude/settings.json
HYPR=$HOME/.config/hypr
EXT=$HOME/.local/share/gnome-shell/extensions/idlefx-companion@idlefx

# A fresh repo with one commit, as the current directory.
repo() {
    git init -q -b main "$TMP/${1:-repo}"
    cd "$TMP/${1:-repo}"
    echo a >a
    git add a
    git commit -q -m init
}

# A stand-in `gnome-extensions` that logs its arguments.
stub_gnome() {
    printf '#!/bin/sh\necho "$*" >>%s/gnome.log\n' "$TMP" >"$TMP/bin/gnome-extensions"
    chmod +x "$TMP/bin/gnome-extensions"
}

# A Claude settings file with other hooks and settings, as Claude writes it.
claude_settings() {
    mkdir -p "$HOME/.claude"
    jq . >"$SETTINGS" <<'EOF'
{"model": "opus", "hooks": {"Stop": [{"hooks": [{"type": "command", "command": "echo mine"}]}]}, "permissions": {"allow": ["Bash(ls)"]}}
EOF
}

# The line of a hooks report for a target.
line() { grep -E "^[^ ]+ $1( |$)" <<<"$2"; }

# The eight Claude events and their pokes.
CLAUDE='[["SessionStart","hello"],["UserPromptSubmit","thinking"],["PreToolUse","working"],["Notification","alert"],["Stop","success"],["StopFailure","failure"],["PreCompact","yawn"],["SessionEnd","bye"]]'

has_claude_hooks() {
    jq -e --argjson m "$CLAUDE" '.hooks as $h | all($m[]; . as [$e, $p]
        | any($h[$e][]?.hooks[]?; .command | test("idlefx-companion poke " + $p + "( |$)")))' "$1" >/dev/null
}

case ${1:?case} in
claude-snippet)
    f=$CONTRIB/claude/settings.json
    jq -e --argjson m "$CLAUDE" '.hooks as $h
        | all($m[]; . as [$e, $p] | $h[$e][0].hooks[0].command | test("^idlefx-companion poke " + $p + "( |$)"))
        and $h.Notification[0].matcher == "permission_prompt"
        and ($h | keys | length) == 8' "$f" >/dev/null
    ;;
hypridle-file)
    f=$CONTRIB/hypridle/idlefx-companion.conf
    grep -qE '^listener *\{' "$f" && grep -qE '^\s*timeout *= *300\s*$' "$f" \
        && grep -qE '^\s*on-timeout *= *idlefx-companion poke sleep( |$)' "$f" \
        && grep -qE '^\s*on-resume *= *idlefx-companion poke wake( |$)' "$f"
    ;;
gnome-parse)
    test -n "$NODE"
    cp "$CONTRIB/gnome/idlefx-companion@idlefx/extension.js" "$TMP/extension.mjs"
    "$NODE" --check "$TMP/extension.mjs"
    ;;
gnome-pokes)
    f=$CONTRIB/gnome/idlefx-companion@idlefx/extension.js
    grep -q 'isLocked' "$f" && grep -q 'add_idle_watch' "$f" && grep -q 'add_user_active_watch' "$f" \
        && grep -q "_poke('sleep')" "$f" && grep -q "_poke('wake')" "$f"
    ;;
git-events)
    printf '#!/bin/sh\necho "$*" >>%s/poke.log\n' "$TMP" >"$TMP/stub"
    mkdir -p "$TMP/stubbin"
    mv "$TMP/stub" "$TMP/stubbin/idlefx-companion"
    chmod +x "$TMP/stubbin/idlefx-companion"
    export PATH=$TMP/stubbin:$SYS
    git init -q --bare "$TMP/remote.git"
    repo
    git remote add origin "$TMP/remote.git"
    for h in "${HOOKS[@]}"; do install -m 755 "$CONTRIB/git/hooks/$h" .git/hooks/$h; done
    step() { rm -f "$TMP/poke.log"; "$@" >/dev/null 2>&1; cat "$TMP/poke.log" 2>/dev/null || true; }
    test "$(step git commit -q --allow-empty -m one)" = "poke commit"
    test "$(step git push -q origin main)" = "poke push"
    test "$(step git checkout -q -b feature)" = "poke curious"
    echo b >b && git add b && git commit -q -m two >/dev/null
    git checkout -q main
    test "$(step git merge -q --no-edit feature)" = "poke curious"
    echo changed >a
    test -z "$(step git checkout -- a)"
    ;;
git-no-binary)
    export PATH=$SYS
    git init -q --bare "$TMP/remote.git"
    repo
    git remote add origin "$TMP/remote.git"
    for h in "${HOOKS[@]}"; do install -m 755 "$CONTRIB/git/hooks/$h" .git/hooks/$h; done
    git commit -q --allow-empty -m one && git push -q origin main 2>/dev/null
    ;;
same-source)
    stub_gnome
    mkdir -p "$HYPR" && echo 'general {' >"$HYPR/hypridle.conf" && echo '}' >>"$HYPR/hypridle.conf"
    repo
    idlefx-companion hooks install claude git hypridle gnome >/dev/null
    for h in "${HOOKS[@]}"; do cmp "$CONTRIB/git/hooks/$h" ".git/hooks/$h"; done
    cmp "$CONTRIB/hypridle/idlefx-companion.conf" "$HYPR/idlefx-companion-hypridle.conf"
    cmp "$CONTRIB/gnome/idlefx-companion@idlefx/extension.js" "$EXT/extension.js"
    cmp "$CONTRIB/gnome/idlefx-companion@idlefx/metadata.json" "$EXT/metadata.json"
    test "$(jq -S .hooks "$SETTINGS")" = "$(jq -S .hooks "$CONTRIB/claude/settings.json")"
    ;;
claude-merge)
    claude_settings
    cp "$SETTINGS" "$TMP/before"
    out=$(idlefx-companion hooks install claude)
    line claude "$out" | grep -q '^✓'
    has_claude_hooks "$SETTINGS"
    test "$(jq -c 'keys_unsorted' "$SETTINGS")" = '["model","hooks","permissions"]'
    test "$(jq -c '.hooks.Stop[0]' "$SETTINGS")" = '{"hooks":[{"type":"command","command":"echo mine"}]}'
    test "$(jq -c '.permissions' "$SETTINGS")" = '{"allow":["Bash(ls)"]}'
    cmp "$TMP/before" "$SETTINGS.bak"
    ;;
idempotent)
    claude_settings
    mkdir -p "$HYPR" && echo 'general {' >"$HYPR/hypridle.conf" && echo '}' >>"$HYPR/hypridle.conf"
    repo
    idlefx-companion hooks install claude git hypridle >/dev/null
    sums() { find "$HOME" "$TMP/repo/.git/hooks" -type f -print0 | sort -z | xargs -0 sha256sum; }
    before=$(sums)
    out=$(idlefx-companion hooks install claude git hypridle)
    test "$before" = "$(sums)"
    for t in claude git hypridle; do line $t "$out" | grep -q 'already installed'; done
    ;;
claude-invalid)
    mkdir -p "$HOME/.claude"
    printf '{ "model": "opus", oops' >"$SETTINGS"
    cp "$SETTINGS" "$TMP/before"
    rc=0
    out=$(idlefx-companion hooks install claude) || rc=$?
    test $rc -eq 1
    line claude "$out" | grep -q '^✗'
    cmp "$TMP/before" "$SETTINGS"
    test ! -e "$SETTINGS.bak"
    ;;
git-append)
    repo
    printf '#!/bin/sh\necho mine\n' >.git/hooks/post-commit
    chmod +x .git/hooks/post-commit
    idlefx-companion hooks install git >/dev/null
    test "$(head -2 .git/hooks/post-commit)" = "$(printf '#!/bin/sh\necho mine')"
    grep -q 'idlefx-companion poke commit' .git/hooks/post-commit
    for h in pre-push post-checkout post-merge; do test -x .git/hooks/$h; done
    ;;
git-template)
    repo
    idlefx-companion hooks install git --template >/dev/null
    dir=$(git config --global init.templateDir)
    for h in "${HOOKS[@]}"; do test -x "${dir/#\~/$HOME}/hooks/$h"; done
    git init -q "$TMP/fresh"
    for h in "${HOOKS[@]}"; do grep -q 'idlefx-companion poke' "$TMP/fresh/.git/hooks/$h"; done
    ;;
hypridle)
    mkdir -p "$HYPR"
    printf 'general {\n    lock_cmd = hyprlock\n}\n' >"$HYPR/hypridle.conf"
    cp "$HYPR/hypridle.conf" "$TMP/before"
    out=$(idlefx-companion hooks install hypridle)
    line hypridle "$out" | grep -q '^✓'
    test "$(grep -c '^source = ' "$HYPR/hypridle.conf")" -eq 1
    grep -qx "source = $HYPR/idlefx-companion-hypridle.conf" "$HYPR/hypridle.conf"
    test "$(head -c "$(wc -c <"$TMP/before")" "$HYPR/hypridle.conf")" = "$(cat "$TMP/before")"
    test -s "$HYPR/idlefx-companion-hypridle.conf"
    ;;
gnome-stub)
    stub_gnome
    out=$(idlefx-companion hooks install gnome)
    line gnome "$out" | grep -q '^✓'
    test -s "$EXT/extension.js" && test -s "$EXT/metadata.json"
    grep -qx 'enable idlefx-companion@idlefx' "$TMP/gnome.log"
    ;;
gnome-missing)
    rc=0
    out=$(idlefx-companion hooks install gnome) || rc=$?
    test $rc -eq 1
    line gnome "$out" | grep -q '^✗.*gnome-extensions'
    test ! -e "$EXT"
    ;;
default-skips)
    mkdir -p "$TMP/plain" && cd "$TMP/plain"
    out=$(idlefx-companion hooks install)
    line claude "$out" | grep -q '^✓'
    line hypridle "$out" | grep -q '^–'
    line gnome "$out" | grep -q '^–'
    ;;
status)
    stub_gnome
    claude_settings
    mkdir -p "$HYPR" && echo 'general {' >"$HYPR/hypridle.conf" && echo '}' >>"$HYPR/hypridle.conf"
    repo
    idlefx-companion hooks install >/dev/null
    out=$(idlefx-companion hooks status)
    for t in claude git hypridle gnome; do line $t "$out" | grep -q 'installed'; done
    idlefx-companion hooks uninstall >/dev/null
    rc=0
    out=$(idlefx-companion hooks status) || rc=$?
    test $rc -eq 1
    for t in claude git hypridle gnome; do line $t "$out" | grep -q 'missing'; done
    ;;
uninstall-restores)
    claude_settings
    mkdir -p "$HYPR"
    printf 'general {\n    lock_cmd = hyprlock\n}' >"$HYPR/hypridle.conf"
    repo
    printf '#!/bin/sh\necho mine' >.git/hooks/post-commit
    chmod +x .git/hooks/post-commit
    cp "$SETTINGS" "$TMP/settings"; cp "$HYPR/hypridle.conf" "$TMP/hypridle"; cp .git/hooks/post-commit "$TMP/post-commit"
    ls .git/hooks >"$TMP/hooks-before"
    idlefx-companion hooks install claude git hypridle >/dev/null
    idlefx-companion hooks uninstall claude git hypridle >/dev/null
    cmp "$TMP/settings" "$SETTINGS" && cmp "$TMP/hypridle" "$HYPR/hypridle.conf" && cmp "$TMP/post-commit" .git/hooks/post-commit
    test "$(ls .git/hooks)" = "$(cat "$TMP/hooks-before")"
    test ! -e "$HYPR/idlefx-companion-hypridle.conf" && test ! -e "$SETTINGS.bak"
    ;;
path-line)
    export PATH=$SYS
    rc=0
    out=$("$ROOT/target/release/idlefx-companion" hooks install claude) || rc=$?
    test $rc -eq 1
    head -1 <<<"$out" | grep -q '^✗.*PATH'
    ;;
hook-timing)
    mkdir -p "$TMP/run"
    export XDG_RUNTIME_DIR=$TMP/run
    cmds=()
    while IFS= read -r c; do cmds+=("$c"); done < <(jq -r '.hooks[][].hooks[].command' "$CONTRIB/claude/settings.json")
    while IFS= read -r c; do cmds+=("$c"); done < <(sed -nE 's/^\s*on-(timeout|resume) *= *//p' "$CONTRIB/hypridle/idlefx-companion.conf")
    for h in "${HOOKS[@]}"; do cmds+=("sh $CONTRIB/git/hooks/$h x y 1 </dev/null"); done
    check() {
        for c in "${cmds[@]}"; do
            s=$(date +%s%N)
            sh -c "$c" >/dev/null 2>&1 || { echo "failed ($1): $c"; return 1; }
            e=$(( ($(date +%s%N) - s) / 1000000 ))
            test $e -lt 200 || { echo "slow ($1, $e ms): $c"; return 1; }
        done
    }
    check "no dog"
    (script -qec "stty rows 16 cols 64; $ROOT/target/release/idlefx-companion --state idle --clock 14:00 --duration 8" /dev/null </dev/null >/dev/null &)
    sleep 1
    check "dog running"
    PATH=$SYS check "no binary"
    ;;
agents-doc)
    f=$ROOT/crates/companion/AGENTS.md
    for e in hello thinking success failure commit working alert yawn bye push curious sleep wake; do
        grep -q "\`$e\`" "$f" || { echo "missing event $e"; exit 1; }
    done
    for c in 'hooks install' 'hooks status' 'hooks uninstall' contrib/claude/settings.json \
        contrib/git/hooks/post-commit contrib/git/hooks/pre-push contrib/git/hooks/post-checkout \
        contrib/git/hooks/post-merge contrib/hypridle/idlefx-companion.conf contrib/gnome/idlefx-companion@idlefx; do
        grep -qF "$c" "$f" || { echo "missing $c"; exit 1; }
    done
    ;;
*)
    echo "unknown case: $1" >&2
    exit 2
    ;;
esac
