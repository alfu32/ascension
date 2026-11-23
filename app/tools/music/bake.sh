#!/usr/bin/env bash
# Re-make the game's music in app/assets/music/ from the scores and the fetched samples.
#
#   PYTHON=<venv>/bin/python SAMPLES=<samples-dir> tools/music/bake.sh <work-dir> [stem ...]   (from app/)
#
# PYTHON    the authoring environment (README.md); default python3
# SAMPLES   the sample folder fetch_samples.py fills (it is checked, and completed, first)
# work-dir  scratch space on disk for the rendered WAV files (about 50 MB per piece while it
#           is being made; each WAV is removed once its Ogg file is written)
# stem ...  only these pieces (e.g. title fusion.theme); default all 29
#
# Steps: scores/build.py writes the General MIDI scores into assets/music/scores/ → render.py
# plays each score with the samples into a WAV → master.py brings it to its loudness (themes and
# the title −16 LUFS, interludes −23 LUFS, true peak ≤ −1.5 dBTP) and encodes Ogg Vorbis, 44.1 kHz
# stereo, quality 4, bit-exact → cues.py writes cues.json from the scores' markers.
# One piece at a time: a render peaks at about 1.3 GB of memory.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
app=$(cd "$here/../.." && pwd)
out=$app/assets/music
python=${PYTHON:-python3}
samples=${SAMPLES:?set SAMPLES to the sample folder (see tools/music/README.md)}
work=${1:?usage: bake.sh <work-dir> [stem ...]}
shift
mkdir -p "$work" "$out/scores"

"$python" "$here/fetch_samples.py" "$samples"
"$python" "$here/scores/build.py" "$out/scores" "$@"

stems=("$@")
if [ ${#stems[@]} -eq 0 ]; then
    mapfile -t stems < <(cd "$out/scores" && ls ./*.mid | sed 's|^\./||; s|\.mid$||')
fi
for stem in "${stems[@]}"; do
    case "$stem" in
        *.interlude) target=-23 ;;
        *) target=-16 ;;
    esac
    echo "== $stem"
    "$python" "$here/render.py" "$out/scores/$stem.mid" "$work/$stem.wav" "$samples" | tail -1
    "$python" "$here/master.py" "$work/$stem.wav" "$out/$stem.ogg" "$target"
    rm -f "$work/$stem.wav"
done

"$python" "$here/cues.py" "$out/scores" "$out/cues.json"
echo "$(ls "$out"/*.ogg | wc -l) pieces, $(du -sh "$out" | cut -f1) in $out"
