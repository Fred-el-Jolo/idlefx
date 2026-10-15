#!/usr/bin/env bash
# Live and headless checks of the dog: events, latency, CPU, the break
# reminder, the event list. Usage: crates/companion/tests/dog.sh <case>
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
BIN=$ROOT/target/release/idlefx-companion
cd "$ROOT"
cargo build --release -q -p idlefx-companion

RUN=$(mktemp -d)
TRACE=$(mktemp)
trap 'rm -rf "$RUN" "$TRACE"' EXIT

# Start the dog in a 64x16 pseudo-terminal, in the background.
dog() {
    (XDG_RUNTIME_DIR=$RUN script -qec "stty rows 16 cols 64; $BIN $*" /dev/null </dev/null >/dev/null &)
    sleep 1
}

poke() { XDG_RUNTIME_DIR=$RUN "$BIN" poke "$1"; }

# A state change to `to` with cause `cause` and a latency under 100 ms.
fast() {
    awk -v to="$1" -v cause="$2" '
        $3 == to && $4 == cause {
            for (i = 5; i <= NF; i++)
                if ($i ~ /^latency_ms=/) { split($i, a, "="); if (a[2] < 100) ok = 1 }
        }
        END { exit !ok }' "$TRACE"
}

# The CPU share printed by --stats is under the limit.
cpu_under() {
    grep -ao 'cpu_pct=[0-9.]*' | tail -1 | awk -F= -v max="$1" '{v = $2} END {exit !(NR > 0 && v < max)}'
}

# Times (HH:MM:SS.mmm) of the break reminders in a simulation trace.
breaks() { awk '$4 == "break" {print $1}' "$TRACE"; }

case ${1:?case} in
claude-pokes)
    dog --state idle --clock 14:00 --duration 7 --trace "$TRACE"
    poke working; sleep 1
    poke alert; sleep 1
    poke yawn; sleep 1.5
    poke bye; sleep 3
    fast working poke:working && fast alert poke:alert && fast stretch poke:yawn && fast lie poke:bye
    ;;
git-pokes)
    dog --state idle --clock 14:00 --at 32 --duration 6 --trace "$TRACE"
    poke push; sleep 3.5
    poke curious; sleep 1.5
    fast run poke:push && fast curious poke:curious
    ;;
desktop-pokes)
    dog --state idle --clock 14:00 --duration 4 --trace "$TRACE"
    poke sleep; sleep 1
    poke wake; sleep 2
    fast sleep poke:sleep && fast happy poke:wake
    ;;
cpu-working)
    XDG_RUNTIME_DIR=$RUN script -qec "stty rows 16 cols 64; $BIN --state working --clock 14:00 --duration 10 --stats" /dev/null | cpu_under 6
    ;;
cpu-alert)
    XDG_RUNTIME_DIR=$RUN script -qec "stty rows 16 cols 64; $BIN --state alert --clock 14:00 --duration 10 --stats" /dev/null | cpu_under 3
    ;;
cpu-away)
    (sleep 0.5; XDG_RUNTIME_DIR=$RUN "$BIN" poke sleep) &
    XDG_RUNTIME_DIR=$RUN script -qec "stty rows 16 cols 64; $BIN --state idle --clock 14:00 --duration 11 --stats --trace $TRACE" /dev/null | cpu_under 1
    grep -q 'sleep poke:sleep' "$TRACE"
    ;;
break-sim)
    "$BIN" simulate --seed 7 --start 10:00 --duration 1800 --trace "$TRACE"
    b=$(breaks)
    first=$(sed -n 1p <<<"$b")
    second=$(sed -n 2p <<<"$b")
    [[ $first > 10:25:00 && $first < 10:25:01 && $second > 10:30:00 && $second < 10:30:01 ]]
    ;;
break-off)
    "$BIN" simulate --seed 7 --start 10:00 --duration 7200 --trace "$TRACE"
    test -n "$(breaks)"
    "$BIN" simulate --seed 7 --start 10:00 --duration 7200 --break-every 0 --trace "$TRACE"
    test -z "$(breaks)"
    ;;
break-night)
    "$BIN" simulate --seed 7 --start 21:50 --duration 3600 --break-every 5 --trace "$TRACE"
    b=$(breaks)
    grep -q '^21:5' <<<"$b"
    ! grep -qv '^21:' <<<"$b"
    ;;
poke-list)
    out=$("$BIN" poke --list)
    test "$(wc -l <<<"$out")" -eq 13
    test "$(awk '{print $1}' <<<"$out" | tr '\n' ' ')" = "hello thinking success failure commit working alert yawn bye push curious sleep wake "
    awk 'NF < 2 {bad = 1} END {exit bad}' <<<"$out"
    ;;
usage)
    out=$("$BIN" --help)
    grep -q -- '--break-every' <<<"$out" && grep -q 'hooks install' <<<"$out" \
        && grep -q 'hooks status' <<<"$out" && grep -q 'hooks uninstall' <<<"$out"
    ;;
unknown-ignored)
    dog --state idle --clock 14:00 --duration 3 --trace "$TRACE"
    poke bogus; sleep 0.5
    poke hello; sleep 2
    ! grep -q 'poke:bogus' "$TRACE" && grep -q 'poke:hello' "$TRACE"
    ;;
lock-versions)
    pairs() { awk '/^name = /{n = $3} /^version = /{print n, $3}' | sort -u; }
    missing=$(comm -23 <(git show d5add4f:Cargo.lock | pairs) <(pairs <Cargo.lock))
    test -z "$missing" || { echo "changed: $missing"; exit 1; }
    ;;
*)
    echo "unknown case: $1" >&2
    exit 2
    ;;
esac
