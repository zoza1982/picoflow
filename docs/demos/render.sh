#!/usr/bin/env bash
# Render every demo tape with VHS, then freeze the final frame for a few extra
# seconds so the closing screen is readable (VHS trims trailing idle frames, so
# the pause has to be added in post). Run from the repo after `cargo build --release`.
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p bin && cp ../../target/release/picoflow bin/picoflow
# end-freeze seconds per demo
declare -A HOLD=( [01-quickstart]=4 [02-run]=6 [03-inspect]=5 )
for t in 01-quickstart 02-run 03-inspect; do
  echo "rendering $t ..."
  vhs "$t.tape"
  ffmpeg -y -loglevel error -i "$t.gif" \
    -vf "tpad=stop_mode=clone:stop_duration=${HOLD[$t]},split[s0][s1];[s0]palettegen=stats_mode=diff[p];[s1][p]paletteuse=dither=bayer" \
    "$t.hold.gif"
  mv "$t.hold.gif" "$t.gif"
done
rm -rf bin
echo "done"
