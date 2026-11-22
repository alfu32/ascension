#!/usr/bin/env bash
# Re-make every sound in app/assets/sounds/ from the rules, the recipes and the voice model.
#
#   PYTHON=<venv>/bin/python tools/sounds/bake.sh [work-dir]      (run from app/)
#
# PYTHON   the authoring environment (README.md); default python3
# VOICES=0 keep the committed voice files (and the buildings' spoken acknowledgments) and re-make
#          only the synthesised sounds
# ENCODE_ONLY=1 re-level and re-encode the WAV files already in <work-dir>/wav
#
# Steps: load the rules database (schema, seed, models) → assign.py writes plan.json and
# assignment.json → synth.py and voices.py write WAV files → level.py brings them to the
# loudness targets (app/docs/sound.md) → ffmpeg encodes Ogg Vorbis:
# voices mono 22.05 kHz quality 2, trimmed of leading and trailing silence; ambience stereo
# 44.1 kHz quality 3; everything else mono 44.1 kHz quality 3. Bit-exact flags keep the output
# identical between runs.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
app=$(cd "$here/../.." && pwd)
out=$app/assets/sounds
python=${PYTHON:-python3}
voices=${VOICES:-1}
work=${1:-$(mktemp -d)}
mkdir -p "$work/wav" "$out"
if [ "${ENCODE_ONLY:-0}" != 1 ]; then
rm -f "$work"/wav/*.wav "$work/wav/manifest.tsv" "$work/rules.db"

sqlite3 "$work/rules.db" < "$app/db/schema.sql"
sqlite3 "$work/rules.db" < "$app/db/seed.sql"
cat "$app/db/models/materials.sql" "$app/db/models/shared/"*.sql \
    $(ls "$app"/db/models/*.sql | grep -v '/materials.sql$') | sqlite3 "$work/rules.db"

"$python" "$here/assign.py" "$work/rules.db" "$here"
"$python" "$here/synth.py" "$work/wav" "$here/plan.json"
if [ "$voices" = 1 ]; then
    "$python" "$here/voices.py" "$work/wav" "$here/plan.json"
    rm -f "$out"/*.ogg
else
    find "$out" -name '*.ogg' ! -name 'voice.*' ! -name 'ui.announcer.*' ! -name 'bld.*.select*' -delete
fi
fi

encode() {
    local wav=$1 out=$2 name args
    name=$(basename "$wav" .wav)
    case "$name" in
        voice.* | ui.announcer.*)
            args=(-af "silenceremove=start_periods=1:start_threshold=-50dB:start_silence=0.02,areverse,silenceremove=start_periods=1:start_threshold=-50dB:start_silence=0.05,areverse"
                  -ac 1 -ar 22050 -q:a 2) ;;
        ambient.*) args=(-ac 2 -ar 44100 -q:a 3) ;;
        *) args=(-ac 1 -ar 44100 -q:a 3) ;;
    esac
    # Loops carry their loop points (frames, without the padding level.py adds around them).
    local tags=() loop
    loop=$(awk -F'\t' -v n="$name" '$1 == n { print $2 " " $3 }' "$(dirname "$wav")/loops.tsv")
    if [ -n "$loop" ]; then
        tags=(-metadata "LOOPSTART=${loop% *}" -metadata "LOOPLENGTH=${loop#* }")
    fi
    ffmpeg -nostdin -loglevel error -y -i "$wav" "${args[@]}" -c:a libvorbis \
        -fflags +bitexact -flags:a +bitexact -map_metadata -1 "${tags[@]}" "$out/$name.ogg"
}
export -f encode
rm -rf "$work/level"
"$python" "$here/level.py" "$work/wav" "$work/level" > "$work/level.log"
# Beds (bed.*) are only mixed under the buildings' acknowledgments, never shipped alone.
find "$work/level" -name '*.wav' -print0 | xargs -0 -P 4 -I{} bash -c 'encode "$1" "$2"' _ {} "$out"
cp "$work/wav/manifest.tsv" "$work/manifest.tsv"
echo "$(ls "$out"/*.ogg | wc -l) sounds, $(du -sh "$out" | cut -f1) in $out"
