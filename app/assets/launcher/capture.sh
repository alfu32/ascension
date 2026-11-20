#!/bin/bash
# Remakes the launcher's pictures of the game (task 112) with the game's own screenshot mode.
# Run from app/ after `cargo build --release --locked -p rom-app`:  bash assets/launcher/capture.sh
# Every run ends itself (--screenshot, --no-hud), one at a time under a 6 GB cap (Sector and
# larger maps need it); nothing is sent to any window. ImageMagick (`convert`) cuts the
# interesting part of each frame and writes a small JPEG. CROP_ONLY=1 re-cuts existing frames.
#
#   backdrop.jpg            the home planet and its moons: the launcher's backdrop
#   hero.jpg                starships over a city: the home screen's band behind Play
#   maps/<style key>.jpg    one card per map style (the launcher looks pictures up by key)
#   tiles/<name>.jpg        the preset tiles (presets.json names them) and the advanced carousel
set -e
B=${B:-target/release/rise-of-man}
OUT=assets/launcher
RAW=${RAW:-target/launcher-captures}
mkdir -p "$RAW" "$OUT/maps" "$OUT/tiles"

shot() {  # shot <name> <game options...>: one self-ending capture into $RAW/<name>.png
  local name=$1; shift
  [ -n "$CROP_ONLY" ] && return 0
  env -u WAYLAND_DISPLAY XDG_CONFIG_HOME="$PWD/$RAW/config" XDG_DATA_HOME="$PWD/$RAW/data" \
    systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0 \
    "$B" --no-hud --screenshot "$RAW/$name.png" "$@" > "$RAW/$name.log" 2>&1
  grep -h "^screenshot" "$RAW/$name.log"
}
cut() {  # cut <frame> <crop WxH+X+Y> <out.jpg>: the part that tells, 480 x 300
  convert "$RAW/$1.png" -crop "$2" +repage -resize 480x300^ -gravity center -extent 480x300 \
    -strip -quality 80 "$OUT/$3"
}
camera() {  # camera <file> <wheel steps> <x,y>: a script that grants the galactic age and zooms
  printf '0 grant-age age.galactic\n0 grant-tech tech.automated_route_survey\n1 speed 100\n3 cam wheel %s %s\n' \
    "$2" "$3" > "$RAW/$1"
}
mkdir -p "$RAW"
camera wide.txt -24 640,400
camera sector.txt 6 632,368
camera quadrant.txt 5 640,400
camera galaxy.txt 4 632,392
camera orb2.txt -2 800,500
camera orb3.txt -3 800,500
map="--size 1280x800 --universe --zoom-all --ai none"
system="$map --view system,home --graticule off --script docs/tasks/112-system-script.txt --after 6"
stars="$map --view starmap --after 8"

# Map styles: Planet and Solar in their system, the others on the star map, routes known.
shot map.one_world $system --style map.one_world --seed 3
shot map.solar $system --style map.solar --seed 3
shot map.many_worlds $stars --style map.many_worlds --seed 2 --script "$RAW/wide.txt"
shot map.sector $stars --style map.sector --seed 7 --script "$RAW/sector.txt"
shot map.quadrant $stars --style map.quadrant --seed 2 --script "$RAW/quadrant.txt"
shot map.galaxy $stars --style map.galaxy --seed 1 --script "$RAW/galaxy.txt"
cut map.one_world 640x400+320+194 maps/map.one_world.jpg
cut map.solar 960x600+220+40 maps/map.solar.jpg
cut map.many_worlds 640x400+340+270 maps/map.many_worlds.jpg
cut map.sector 1040x650+200+20 maps/map.sector.jpg
cut map.quadrant 960x600+300+120 maps/map.quadrant.jpg
cut map.galaxy 880x550+130+115 maps/map.galaxy.jpg

# The backdrop and Quick match: the home planet from orbit (the window may open larger than
# asked; the crops are taken about the planet).
globe="--size 1600x1000 --universe --zoom-all --style map.many_worlds --seed 5 --ai none --view globe,home --graticule off --after 6"
shot orb3 $globe --script "$RAW/orb3.txt"
shot orb2 $globe --script "$RAW/orb2.txt"
convert "$RAW/orb3.png" -resize 1600x -strip -quality 70 "$OUT/backdrop.jpg"
cut orb2 800x500+542+267 tiles/quick.jpg
cut orb3 1200x750+340+140 tiles/custom.jpg

# The preset tiles: the scene each preset starts with, or what it leads to.
shot classic --size 1280x800 --universe --style map.many_worlds --seed 1 --players 4 --after 4
shot sandbox --size 1280x800 --universe --style map.many_worlds --seed 1 --age 13 --ai none \
  --script assets/scripts/rich-start-13-galactic.txt --after 4
shot orbit --size 1280x800 --universe --zoom-all --players 2 --ai none --seed 1 \
  --script docs/tasks/51-orbit-script.txt --after 6
shot battle --size 1280x800 --universe --zoom-all --players 2 --ai none --seed 1 --sun fixed \
  --script assets/scripts/big-battle.txt --after 9
shot town --size 1280x800 --universe --players 2 --seed 1 --age 5 --ai none --after 4
shot watch --size 1280x800 --universe --style map.one_world --seed 4 --players 3 --watch \
  --ai balanced:moderate,rusher:tough,builder:moderate --speed 2000 --after 25
cut classic 800x500+240+100 tiles/classic.jpg
cut sandbox 800x500+240+60 tiles/sandbox.jpg
cut orbit 1120x700+80+0 tiles/space.jpg
# The home screen's hero band: the same scene, wide.
convert "$RAW/orbit.png" -crop 1280x440+0+40 +repage -resize 1600x -strip -quality 74 "$OUT/hero.jpg"
cut battle 960x600+20+180 tiles/war.jpg
cut town 800x500+240+160 tiles/peaceful.jpg
cut watch 960x600+160+80 tiles/watch.jpg

# The advanced launcher's carousel: the demo, the model gallery and one model.
shot demo --size 1280x800 --universe --style map.one_world --seed 1 --demo --after 12
shot gallery --size 1280x800 --gallery classical --gallery-view game --after 3
shot model --size 1280x800 --model bld.temple@classical --after 3
cut demo 960x600+160+60 tiles/demo.jpg
cut gallery 960x600+160+60 tiles/gallery.jpg
cut model 640x400+640+0 tiles/model.jpg

du -cb "$OUT"/backdrop.jpg "$OUT"/hero.jpg "$OUT"/maps/*.jpg "$OUT"/tiles/*.jpg | tail -1
