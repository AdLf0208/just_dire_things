#!/usr/bin/env bash
# In-game check for Just Dire Things slice 1: the Simple Coal Generator, on a
# game the launcher built with this flat SDK package (`justdirethings.hmod`).
#
#   horizon-launcher native-build ports/justdirethings --instance <i> --trust-native-code
#   GAME=<that horizon-client> tools/e2e.sh
#
# The package carries the block, its item, the data and the Rust that runs it,
# so there is no `--dev-mod` leg: the game is built against the package. The
# arena is a stone floor at y=100 in the Overworld.
#
# Phases:
#  - control "no fuel": a placed generator makes no FE for 40 ticks and is not burning;
#  - coal in the fuel slot (the container path, automation's way in) starts the burn:
#    one item is consumed, the burn is worth 1,600 x 15 = 24,000 FE at 60 FE/t;
#  - the carried Primal Coal (`coal_t1`, burn_time 4,800, speed 2.0) is worth
#    72,000 FE at 120 FE/t;
#  - control "full buffer": a generator set to its 1,000,000 FE capacity burns no
#    fuel until drained, then resumes;
#  - `/justdirethings set 2000000` reads back 1,000,000 (the capacity clamps);
#  - redstone HIGH: with a stick in the slot and nothing powering it, the machine
#    stores 0 FE for 40 ticks and takes no fuel; a redstone block beside it starts
#    the burn (a stick is 100 fuel ticks = 1,500 FE at 60 FE/t);
#  - redstone LOW: powered, the burn that was in progress ends and nothing new
#    starts (40 ticks change nothing); unpowered again, it burns;
#  - redstone PULSE: no edge, nothing burns; the rising edge buys exactly one
#    stick's burn (1,500 FE), and 60 more ticks with the signal held buy nothing;
#    the next rising edge buys the next stick;
#  - REVIEW FIX: a LAN guest (another player) is refused 40 blocks away by the
#    distance guard for both writes, and the same guest one block away lands
#    both — so the refusal is distance, not a blanket one;
#  - control "unfuelled neighbour": a second generator with no fuel stays at 0.
#
# CONTROL_GAME=<a game without this package> adds the differential leg (ids
# unknown). SELFTEST=1 flips the "no fuel" assertion and SELFTEST=redstone flips
# the "unpowered HIGH makes nothing" assertion; either must fail.
set -euo pipefail
export LC_ALL=C

CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
PORT=$CLONE/ports/justdirethings
CTL=${CTL:-$CLONE/ports/_shared/bin/horizon-0b14f8ac/horizon-ctl}
GAME=${GAME:-}
CONTROL_GAME=${CONTROL_GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-${E2E_TMP:-/tmp/opencode}/justdirethings-e2e.sock}
GSOCK=${GSOCK:-${E2E_TMP:-/tmp/opencode}/justdirethings-e2e-guest.sock}
SELFTEST=${SELFTEST:-0}
KEEP=${KEEP:-0}
WORLD=JustDireThingsE2E
CONTROL_WORLD=JustDireThingsControl

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -n "$GAME" ] || {
    echo "missing GAME: point it at a launcher-built horizon-client that carries justdirethings" >&2
    exit 1
}
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d "${E2E_TMP:-/tmp/opencode}/justdirethings-home.XXXXXX")
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
say() { ctl chat "/$1" >/dev/null; }
fail() { echo "FAIL: $*" >&2; ctl logs --lines 80 >&2 || true; exit 1; }

# info X Y Z: this block's status line, after a fresh reply arrives (the t=
# field makes each reply unique).
info() { # info X Y Z
    local out line before
    before=$(ctl messages 20 | jq -r '.chat[]' | grep -E " at $1,$2,$3:" | head -1)
    say "justdirethings info $1 $2 $3"
    for _ in $(seq 1 25); do
        ctl wait 4 >/dev/null
        out=$(ctl messages 20)
        line=$(jq -r '.chat[]' <<<"$out" | grep -E " at $1,$2,$3:|No Just Dire Things block" | head -1)
        if [ -n "$line" ] && [ "$line" != "$before" ]; then
            echo "$line"
            return
        fi
    done
    echo "NO-INFO $(jq -c '{chat,action_bar}' <<<"$out")"
}
# set_energy X Y Z AMOUNT: the harness control `/justdirethings set`, retried
# until the block reads the value back (chat commands are asynchronous).
set_energy() { # X Y Z AMOUNT
    local want=$4 line
    say "justdirethings set $want $1 $2 $3"
    for _ in $(seq 1 10); do
        line=$(info "$1" "$2" "$3")
        if [ "$(energy_of "$line")" = "$want" ]; then
            echo "  set($1,$2,$3) = $line"
            return
        fi
        say "justdirethings set $want $1 $2 $3"
    done
    fail "could not set $1,$2,$3 to $want FE: $line"
}
# put_fuel X Y Z ITEM COUNT: the container path a hopper uses.
put_fuel() { # put_fuel X Y Z ITEM COUNT
    ctl command "item replace block $1 $2 $3 container.0 with $4 $5" >/dev/null
    ctl wait 4 >/dev/null
}

energy_of() { sed -nE 's/.*: ([0-9]+)\/[0-9]+ FE.*/\1/p' <<<"$1"; }
fe_of()     { sed -nE 's/.*fe ([-0-9]+) of ([-0-9]+).*/\1/p' <<<"$1"; }
worth_of()  { sed -nE 's/.*fe ([-0-9]+) of ([-0-9]+).*/\2/p' <<<"$1"; }
burn_rem_of(){ sed -nE 's/.*burn ([0-9]+)\/([0-9]+).*/\1/p' <<<"$1"; }
mult_of()   { sed -nE 's/.*multiplier ([0-9]+).*/\1/p' <<<"$1"; }
mode_of()   { sed -nE 's/.*mode ([a-z]+).*/\1/p' <<<"$1"; }
burning_of(){ sed -nE 's/.*burning (true|false).*/\1/p' <<<"$1"; }
fuel_of()   { sed -nE 's/.*fuel (.*), t=.*/\1/p' <<<"$1"; }

# set_mode X Y Z MODE: the harness control standing in for upstream's GUI
# button, retried until the block reads the mode back (chat is asynchronous).
# The control refuses a player more than six blocks from the block (the same
# guard `/justdirethings set` has), and the machines this slice drives sit at
# x=12 and x=16 while the arena vantage is at x=0, so walk over first: the
# refusal itself is asserted in the LAN-guest phase, not here.
set_mode() { # X Y Z MODE
    local want=$4 line
    ctl tp "$(awk -v x="$1" 'BEGIN { print x + 0.5 }')" "$2" \
        "$(awk -v z="$3" 'BEGIN { print z + 2.5 }')" >/dev/null
    ctl wait 4 >/dev/null
    say "justdirethings mode $want $1 $2 $3"
    for _ in $(seq 1 10); do
        line=$(info "$1" "$2" "$3")
        if [ "$(mode_of "$line")" = "$want" ]; then
            echo "  mode($1,$2,$3) = $line"
            ctl tp 0.5 100 3.5 >/dev/null
            ctl wait 4 >/dev/null
            return
        fi
        say "justdirethings mode $want $1 $2 $3"
    done
    fail "could not set $1,$2,$3 to mode $want: $line"
}

cleanup() {
    local rc=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "$KEEP" = 1 ]; then
        echo "kept scratch: $SCRATCH" >&2
    else
        rm -rf "$SCRATCH" 2>/dev/null || true
    fi
    echo "EXIT:$rc"
}
trap cleanup EXIT

launch() { # launch [WORLD] [GAME]
    local world=${1:-$WORLD} game=${2:-$GAME}
    rm -f "$SOCK"
    cd "$SCRATCH"
    "$CTL" --socket "$SOCK" launch \
        --game "$game" --world "$world" --seed 20261105 --creative --headless --wait \
        --resources "$RES" >/dev/null
    ctl wait-ready 180
    ctl wait 40
}

# A stone floor at y=99 with clear air above, in the Overworld, and the player
# standing on it at (0.5, 100, 3.5).
arena() {
    ctl tp 0.5 101 3.5 >/dev/null
    ctl wait 20
    ctl command "fill -8 99 -8 24 99 24 minecraft:stone" >/dev/null
    ctl command "fill -8 100 -8 24 110 24 minecraft:air" >/dev/null
    ctl command "kill @e[type=!player]" >/dev/null
    ctl tp 0.5 100 3.5 >/dev/null
    ctl wait 20
    local y feet floor
    y=$(ctl state | jq -r '.player.position[1] // 0')
    feet=$(ctl block 0 100 3 | jq -r '.name // .block // empty')
    floor=$(ctl block 0 99 3 | jq -r '.name // .block // empty')
    echo "  arena: y=$y floor(0,99,3)=$floor feet(0,100,3)=$feet"
    awk -v y="$y" 'BEGIN { exit !(y >= 99.9) }' || fail "the player is not on the arena floor (y=$y)"
    [ "$feet" = "minecraft:air" ] || fail "the arena volume is not clear ($feet at 0,100,3)"
    [ "$floor" = "minecraft:stone" ] || fail "the arena floor is missing ($floor at 0,99,3)"
}

place() { # place X Y Z ID
    ctl command "setblock $1 $2 $3 $4" >/dev/null
    ctl wait 4
    local got
    got=$(ctl block "$1" "$2" "$3" | jq -r '.name // .block // empty')
    echo "  block($1,$2,$3) = $got"
    [ "$got" = "$4" ] || fail "expected $4 at $1,$2,$3, got '$got'"
}

# The negative control: a game of the same build without this package.
control_leg() {
    if [ -z "$CONTROL_GAME" ]; then
        echo "== control: skipped (set CONTROL_GAME=<unmodified client> to run it) =="
        return
    fi
    [ -x "$CONTROL_GAME" ] || fail "missing CONTROL_GAME: $CONTROL_GAME"
    echo "== control: the same build with no justdirethings =="
    launch "$CONTROL_WORLD" "$CONTROL_GAME"
    arena
    local reply name
    reply=$(ctl command "setblock 4 100 0 justdirethings:generatort1")
    echo "  setblock reply: $reply"
    case "$reply" in
        *"Unknown block"*|*"unknown block"*|*"error"*) ;;
        *) fail "the control game knew justdirethings:generatort1" ;;
    esac
    reply=$(ctl command "give @s justdirethings:coal_t1 1")
    echo "  give reply: $reply"
    case "$reply" in
        *"Unknown item"*|*"unknown item"*|*"error"*|*"not found"*) ;;
        *) fail "the control game knew justdirethings:coal_t1" ;;
    esac
    name=$(ctl block 4 100 0 | jq -r '.name // .block // "unknown"')
    [ "$name" = "minecraft:air" ] || fail "the control game holds $name, so the block id was known"
    echo "  without the package: no block, no item (4,100,0 = $name)"
    ctl quit >/dev/null 2>&1 || true
}

control_leg
echo "== launch (Overworld arena at y=100, the package compiled into the game) =="
launch
arena

echo "== place two generators =="
place 0 100 0 justdirethings:generatort1
place 4 100 0 justdirethings:generatort1

echo "== control: no fuel means no FE =="
line=$(info 0 100 0)
echo "  generator: $line"
[ "$(energy_of "$line")" = 0 ] || fail "a generator with no fuel stored FE: $line"
[ "$(burning_of "$line")" = false ] || fail "a generator with no fuel is burning: $line"
ctl wait 40
line=$(info 0 100 0)
echo "  after 40 ticks: $line"
[ "$(energy_of "$line")" = 0 ] || fail "a generator with no fuel made FE: $line"
if [ "$SELFTEST" = 1 ]; then
    echo "  SELFTEST: expecting the empty generator to have made FE (this must fail)"
    [ "$(energy_of "$line")" != 0 ] || fail "SELFTEST flipped: the unfuelled generator made no FE, as it should"
fi
line4=$(info 4 100 0)
echo "  neighbour: $line4"
[ "$(energy_of "$line4")" = 0 ] || fail "an unfuelled generator stored FE: $line4"

echo "== coal starts the burn: 1,600 x 15 = 24,000 FE at 60 FE/t =="
put_fuel 0 100 0 minecraft:coal 5
line=$(info 0 100 0)
echo "  $line"
case "$(fuel_of "$line")" in
    *"minecraft:coal x4"*) ;;
    *) fail "the fuel slot does not hold four coal after one was consumed: $line" ;;
esac
[ "$(burning_of "$line")" = true ] || fail "the generator is not burning coal: $line"
[ "$(mult_of "$line")" = 1 ] || fail "vanilla coal's multiplier is not 1: $line"
[ "$(worth_of "$line")" = 24000 ] || fail "coal should be worth 24,000 FE: $line"
[ "$(energy_of "$line")" -gt 0 ] || fail "the generator made no FE from coal: $line"
[ "$(energy_of "$line")" -le 24000 ] || fail "the generator made more than one coal's FE: $line"

echo "== the carried Primal Coal is worth 72,000 FE at 120 FE/t =="
put_fuel 4 100 0 justdirethings:coal_t1 5
line4=$(info 4 100 0)
echo "  $line4"
[ "$(burning_of "$line4")" = true ] || fail "the generator is not burning Primal Coal: $line4"
[ "$(mult_of "$line4")" = 2 ] || fail "Primal Coal's multiplier is not 2: $line4"
[ "$(worth_of "$line4")" = 72000 ] || fail "Primal Coal should be worth 72,000 FE: $line4"
[ "$(energy_of "$line4")" -gt 0 ] || fail "the generator made no FE from Primal Coal: $line4"

echo "== control: a full buffer burns nothing until it is drained =="
set_energy 0 100 0 1000000
line=$(info 0 100 0)
fe_before=$(fe_of "$line")
ctl wait 40
line=$(info 0 100 0)
echo "  full buffer after 40 ticks: $line"
[ "$(energy_of "$line")" = 1000000 ] || fail "a full buffer changed: $line"
[ "$(fe_of "$line")" = "$fe_before" ] || fail "a full buffer kept burning ($fe_before -> $(fe_of "$line")): $line"

echo "== drained again, the burn resumes =="
say "justdirethings set 0 0 100 0"
ctl wait 8
line=$(info 0 100 0)
drained=$(energy_of "$line")
echo "  after draining: $line"
[ "$drained" -lt 1000 ] || fail "the buffer did not drain: $line"
ctl wait 40
line=$(info 0 100 0)
echo "  after 40 more ticks: $line"
[ "$(energy_of "$line")" -gt "$drained" ] || fail "the generator did not resume when drained: $line"

echo "== the capacity clamps what is set =="
say "justdirethings set 2000000 0 100 0"
ctl wait 8
line=$(info 0 100 0)
echo "  set 2000000 -> $line"
[ "$(energy_of "$line")" = 1000000 ] || fail "the buffer did not clamp to its capacity: $line"

echo "== redstone HIGH: nothing runs while nothing powers the machine =="
# A stick burns for 100 fuel ticks at 60 FE/t: 25 ticks of output, 1,500 FE.
place 12 100 0 justdirethings:generatort1
line=$(info 12 100 0)
echo "  default mode: $line"
[ "$(mode_of "$line")" = ignored ] || fail "a fresh generator is not IGNORED: $line"
set_mode 12 100 0 high
put_fuel 12 100 0 minecraft:stick 4
line=$(info 12 100 0)
echo "  HIGH, unpowered: $line"
[ "$(burning_of "$line")" = false ] || fail "an unpowered HIGH generator is burning: $line"
ctl wait 40
line=$(info 12 100 0)
echo "  HIGH, unpowered after 40 ticks: $line"
[ "$(energy_of "$line")" = 0 ] || fail "an unpowered HIGH generator made FE: $line"
[ "$(burning_of "$line")" = false ] || fail "an unpowered HIGH generator started a burn: $line"
case "$(fuel_of "$line")" in
    *"minecraft:stick x4"*) ;;
    *) fail "an unpowered HIGH generator took fuel: $line" ;;
esac
if [ "$SELFTEST" = redstone ]; then
    echo "  SELFTEST: expecting the unpowered HIGH generator to have made FE (this must fail)"
    [ "$(energy_of "$line")" != 0 ] || fail "SELFTEST flipped: the unpowered HIGH generator made no FE, as it should"
fi

echo "== redstone HIGH: a neighbour's signal starts it =="
place 13 100 0 minecraft:redstone_block
ctl wait 40
powered_line=$(info 12 100 0)
echo "  HIGH, powered: $powered_line"
[ "$(energy_of "$powered_line")" -gt 0 ] || fail "a powered HIGH generator made no FE: $powered_line"
[ "$(burning_of "$powered_line")" = true ] || fail "a powered HIGH generator is not burning: $powered_line"
[ "$(mult_of "$powered_line")" = 1 ] || fail "a stick's multiplier is not 1: $powered_line"
[ "$(worth_of "$powered_line")" = 1500 ] || fail "a stick should be worth 1,500 FE: $powered_line"

echo "== redstone LOW: powered means stopped, unpowered means running =="
# Let the burn the HIGH phase started run out before reading the frozen buffer.
set_mode 12 100 0 low
ctl wait 80
frozen=$(info 12 100 0)
echo "  LOW, powered, burn ended: $frozen"
[ "$(burning_of "$frozen")" = false ] || fail "the in-progress burn did not end: $frozen"
ctl wait 40
frozen2=$(info 12 100 0)
echo "  LOW, powered, 40 ticks later: $frozen2"
[ "$(energy_of "$frozen2")" = "$(energy_of "$frozen")" ] \
    || fail "a powered LOW generator kept making FE ($(energy_of "$frozen") -> $(energy_of "$frozen2")): $frozen2"
ctl command "setblock 13 100 0 minecraft:air" >/dev/null
ctl wait 40
unpowered=$(info 12 100 0)
echo "  LOW, unpowered: $unpowered"
[ "$(energy_of "$unpowered")" -gt "$(energy_of "$frozen2")" ] \
    || fail "an unpowered LOW generator stayed stopped: $unpowered"

echo "== redstone PULSE: one burn per rising edge, and a held signal buys nothing =="
place 16 100 0 justdirethings:generatort1
set_mode 16 100 0 pulse
put_fuel 16 100 0 minecraft:stick 4
ctl wait 40
line=$(info 16 100 0)
echo "  PULSE, no edge yet: $line"
[ "$(energy_of "$line")" = 0 ] || fail "an edge-less PULSE generator made FE: $line"
[ "$(burning_of "$line")" = false ] || fail "an edge-less PULSE generator is burning: $line"
case "$(fuel_of "$line")" in
    *"minecraft:stick x4"*) ;;
    *) fail "an edge-less PULSE generator took fuel: $line" ;;
esac
place 17 100 0 minecraft:redstone_block
ctl wait 8
edge=$(info 16 100 0)
echo "  PULSE, rising edge: $edge"
[ "$(burning_of "$edge")" = true ] || fail "the rising edge did not start a burn: $edge"
[ "$(energy_of "$edge")" -gt 0 ] || fail "the rising edge made no FE: $edge"
# The signal stays on: the burn it started pays out to its end and stops there.
ctl wait 60
held=$(info 16 100 0)
echo "  PULSE, signal still on after 60 ticks: $held"
[ "$(burning_of "$held")" = false ] || fail "PULSE kept burning with a held signal: $held"
[ "$(energy_of "$held")" = 1500 ] || fail "one stick is not 1,500 FE in PULSE ($(energy_of "$held")): $held"
case "$(fuel_of "$held")" in
    *"minecraft:stick x3"*) ;;
    *) fail "PULSE did not burn exactly one stick: $held" ;;
esac

echo "== redstone PULSE: the next rising edge buys the next burn =="
ctl command "setblock 17 100 0 minecraft:air" >/dev/null
ctl wait 8
place 17 100 0 minecraft:redstone_block
ctl wait 40
again=$(info 16 100 0)
echo "  PULSE, second edge: $again"
[ "$(energy_of "$again")" -gt 1500 ] || fail "the second edge bought no burn: $again"
case "$(fuel_of "$again")" in
    *"minecraft:stick x2"*) ;;
    *) fail "the second edge did not take a second stick: $again" ;;
esac

echo "== review fix: the harness writes are in reach, and only in reach =="
# The single-player owner is always level 4, and a LAN guest on this pin joins
# with command permission too (measured: it passes the operator gate), so the
# *distance* guard is the one this harness can exercise. It is made
# deterministic: the host teleports the guest to a known spot instead of relying
# on where it spawned. The same guest and the same commands are then run one
# block away, so a blanket refusal cannot pass this phase.
place 8 100 0 justdirethings:generatort1
line=$(info 8 100 0)
echo "  guest's target before: $line"
[ "$(energy_of "$line")" = 0 ] || fail "the guest's target generator is not empty: $line"
PORT_NUMBER=$(ctl lan open 0 commands | jq -r '.port')
[ "$PORT_NUMBER" != "null" ] || fail "the host did not open its world to LAN"
rm -f "$GSOCK"
HORIZON_HOME=$SCRATCH/guest "$CTL" --socket "$GSOCK" launch \
    --game "$GAME" --headless --resources "$RES" --no-pause >/dev/null
sleep 3
gctl() { "$CTL" --socket "$GSOCK" "$@"; }
connect_reply=$(gctl connect "127.0.0.1:$PORT_NUMBER" Guest 2>&1 || true)
echo "  guest connect: $connect_reply"
joined=false
for _ in $(seq 1 90); do
    if ctl messages 12 | jq -r '.chat[]' | grep -qi "Guest joined the game"; then joined=true; break; fi
    sleep 2
done
[ "$joined" = true ] || fail "the guest never joined the host's world"
sleep 3
echo "  guest state: $(gctl state | jq -c '{screen, dimension: .server.dimension, players: .server.players}')"

echo "== the guest 40 blocks away: both writes are refused =="
ctl command "tp Guest 40 100 40" >/dev/null
ctl wait 8
guest_at=$(gctl state | jq -r '.player.position | @csv' 2>/dev/null || true)
echo "  guest position after the host's /tp: $guest_at"
gctl chat "/justdirethings set 123 8 100 0" >/dev/null
sleep 2
gctl messages 20 | jq -r '.chat[]' >> "$SCRATCH/guest-chat.txt" 2>/dev/null || true
cat "$SCRATCH/guest-chat.txt" 2>/dev/null | tail -6
guest_reply=$(gctl messages 20 | jq -r '.chat[]' | tail -5 | tr '\n' ' ')
echo "  guest set reply (40 blocks away): $guest_reply"
case "$guest_reply" in
    *"out of reach"*) ;;
    *) fail "a guest 40 blocks away was not refused for distance: $guest_reply" ;;
esac
line=$(info 8 100 0)
echo "  guest's target after the distant set: $line"
[ "$(energy_of "$line")" = 0 ] || fail "a distant guest changed another generator's buffer: $line"
gctl chat "/justdirethings mode high 8 100 0" >/dev/null
sleep 2
guest_mode_reply=$(gctl messages 20 | jq -r '.chat[]' | tail -6 | tr '\n' ' ')
echo "  guest mode reply (40 blocks away): $guest_mode_reply"
case "$guest_mode_reply" in
    *"out of reach"*) ;;
    *) fail "a guest 40 blocks away was not refused for the mode control: $guest_mode_reply" ;;
esac
line=$(info 8 100 0)
echo "  guest's target mode after the distant mode: $line"
[ "$(mode_of "$line")" = ignored ] || fail "a distant guest changed another generator's mode: $line"

echo "== the same guest one block away: both writes land, so the refusal is distance =="
ctl command "tp Guest 9.5 100 1.5" >/dev/null
ctl wait 8
guest_at=$(gctl state | jq -r '.player.position | @csv' 2>/dev/null || true)
echo "  guest position after the second /tp: $guest_at"
gctl chat "/justdirethings set 123 8 100 0" >/dev/null
sleep 2
guest_reply=$(gctl messages 20 | jq -r '.chat[]' | tail -6 | tr '\n' ' ')
echo "  guest set reply (one block away): $guest_reply"
case "$guest_reply" in
    *"Energy set to 123 FE."*) ;;
    *) fail "an in-reach guest's set did not land, so the phase cannot prove the refusal was distance: $guest_reply" ;;
esac
line=$(info 8 100 0)
echo "  guest's target after the in-reach set: $line"
[ "$(energy_of "$line")" = 123 ] || fail "an in-reach guest's set did not change the buffer: $line"
# The host's own `set` obeys the same reach guard: stand next to the block first.
ctl tp 8.5 100 2.5 >/dev/null
ctl wait 4 >/dev/null
set_energy 8 100 0 0
ctl tp 0.5 100 3.5 >/dev/null
ctl wait 4 >/dev/null
line=$(info 8 100 0)
[ "$(energy_of "$line")" = 0 ] || fail "the harness could not reset the buffer: $line"
gctl chat "/justdirethings mode high 8 100 0" >/dev/null
sleep 2
guest_mode_reply=$(gctl messages 20 | jq -r '.chat[]' | tail -6 | tr '\n' ' ')
echo "  guest mode reply (one block away): $guest_mode_reply"
line=$(info 8 100 0)
echo "  guest's target after the in-reach mode: $line"
[ "$(mode_of "$line")" = high ] || fail "an in-reach guest's mode control did not land: $line"
set_mode 8 100 0 ignored
line=$(info 8 100 0)
[ "$(mode_of "$line")" = ignored ] || fail "the harness could not restore the mode: $line"
gctl quit >/dev/null 2>&1 || true
sleep 1

ctl quit >/dev/null 2>&1 || true
trap - EXIT
sleep 1
rm -rf "$SCRATCH" 2>/dev/null || true
echo "PASS: an unfuelled generator made no FE; coal burned at 60 FE/t for 24,000 FE and Primal Coal at 120 FE/t for 72,000 FE; a full buffer burned nothing until drained; the capacity clamped a larger set; HIGH ran only while powered and LOW only while unpowered; PULSE bought exactly one stick's burn per rising edge and a held signal bought none; and a guest 40 blocks away changed neither the buffer nor the mode while the same guest one block away changed both, so the refusal is distance"
echo "EXIT:0"
