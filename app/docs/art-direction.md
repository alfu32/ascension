# Art direction: buildings and units

Owner decisions (2026-11-14): models are **built from parts inside the game**
(procedural, data-driven), in a **stylised, low-detail** style, with **one look
for every faction** to start; regional families come later as data.

## Style

- Clean, chunky shapes with flat-shaded faces. Surface detail (stone courses,
  brick, roof tiles, planks, thatch) comes from the material shaders, not from
  geometry and not from image files.
- Each model reads at gameplay distance (60° tilt, 30–80 tiles across the
  screen) by its **silhouette** first and its colours second.
- Soft contact shading: parts darken slightly where they meet the ground or
  another part (baked into the vertex colour), so models do not look pasted on.
- Natural materials carry most of the surface. **Team colour** covers about
  15–25 % of what is visible on buildings, riders, robed figures, vehicles
  and ships: roof trim or roof, banners and flags, sails, shield faces. Never
  the whole model.
- Foot figures wear a full team-coloured tunic (about 40–60 % of what is
  visible), so armies read by colour from afar. Approved by the owner in
  review round 3 (2026-11-14).

## Scale

- The tile is the unit of length. A standing figure is **1.0 tile** tall,
  about twice "real" scale next to buildings, so units stay readable.
- A building covers 85–95 % of its footprint; the gap shows the ground between
  neighbours. Wall height for an ordinary building is about 1.6–2.2 tiles;
  landmarks (temple, tower, city centre) go higher.
- Mounted figures about 1.4 tiles tall; vehicles, ships and aircraft are sized
  by class, not by real dimensions.

## Telling buildings apart

Each building has one **signature element** that names its job at a glance:

| Building | Signature |
| --- | --- |
| City | a cluster: central hall plus houses, highest roof in the city |
| Farm | crop rows (done) and a small barn |
| Woodcutter's camp | log piles and a saw-horse under an open shed |
| Library | stacked roof or dome over a solid block, few windows |
| Market | open stalls with striped awnings in team colour |
| Granary | round silos |
| Temple | columns and a pediment on a stepped base |
| Barracks | walled yard with a gate and a tall banner |
| Tower | tall and slim, crenellated top |
| Dock | pier reaching over the water, a crane |
| Stable | long low barn with a fenced paddock |
| Mine | headframe over a shaft, spoil heap |
| Smelter / factory | chimneys (smoke later) |
| Automated colony | a glass-domed hub under a tall relay mast ringed with light, a drone on its pad |

Units: silhouette by role — spear (long vertical line), bow (curve), shield
(disc or rectangle), mount (horizontal body), siege (frame and wheels).

## Ages

One family of materials and shapes per era; the same building changes look
when its owner advances.

| Era | Ages | Materials and shapes |
| --- | --- | --- |
| Early | ancient | mud brick, thatch, timber posts, palisades |
| Classical | classical | limestone, columns, terracotta roof tiles |
| Medieval | medieval | rough stone, timber framing, slate, crenellations |
| Early modern | gunpowder, enlightenment | plaster, tile, bastions, brick, sash windows |
| Industrial | industrial | red brick, iron, chimneys, sawtooth roofs |
| Modern | modern, information | concrete, glass, flat roofs, antennas |
| Future | automation and later | white composite, solar panels, domes, light strips |

## Motion

Units move (task 39): figures walk (legs and arms swing, the body bobs), strike,
thrust, throw, draw a bow or whirl a sling as their `attack` knob says, swing a
tool or raise both arms at work, and breathe when idle; beasts trot, wheels
turn, oars sweep and hulls ride the water. Each unit is at its own point of the
cycle, so a group does not move in step. Motion is presentation only.

Task 40 adds firearms: a musket, arquebus or crossbow is raised to the shoulder,
aimed, fired and lowered, a pistol levelled at arm's length; at the shot a flash
grows out of the muzzle for a moment and a cannon's barrel kicks back on its
bed. A trebuchet's arm swings up and over while its counterweight drops,
hanging free, and the sling whips round. Sailing ships heel to the wind and bob
under way, with no oars.

Task 45 fires long guns from the shoulder with both hands: the shooter turns the
left shoulder towards the target, beds the butt in the right shoulder with the
piece beside the cheek and holds the barrel up with the left hand; a rider turns
in the saddle the same way while the legs keep straddling the horse.

Task 46 adds the first machines. A tank's tracks are belts round road wheels that
turn as it moves; its turret swings off and back onto the target before each shot
and the gun recoils; a machine gun fires bursts, its flash showing on every other
keyframe, while its seated gunner keeps both hands on it and shakes with it; a
propeller spins all the time; an aircraft flies, bobbing and banking gently from
side to side while it moves. The renderer draws every unit that is not a
spacecraft on the ground (only ships in orbit get an altitude), so an aircraft's
model carries its own height: the biplane's fuselage is 2.7 tiles up and its
shadow falls on the ground below. A medic bends over the wounded with both hands
when it works. Funnels and chimneys do not smoke yet.

Task 47 (modern and information buildings) adds no motion: buildings stand still (a test
holds every building's rig to a single joint), so radar antennas, gun turrets and searchlights
are fixed in place. Modern buildings wear concrete, precast panels, glass curtain walls (vertical
mullions from the `timber` program in glass blue, with thin steel or precast floor bands as
parts), ribbed steel cladding, flat roofs with team-coloured fascias, and red warning lights
(`mat.signal_red`, self-lit) on masts and chimneys.

Task 48 (modern and information units) adds no new shape or motion knob; it combines the ones
there are. Aircraft carry their own height: propeller aircraft fly 2.7–2.9 tiles up, jets
3.8–4.2, and jets bank harder (`bank` 24 against 18). A missile or rocket leaves with a flash,
the missile itself ahead of the tube and a short smoke trail widening back towards it, all on
the shot's `flash` joint, so they show only at the moment of firing (a burst of 4 for a rocket
salvo; vertical from a cruiser's cells); bombs show under the bay the same way. Rotors and a
search radar's spinning bar are `propeller` parts pitched −90°; drones hover by carrying their
own height and bob and sway with a small `bank`. Modern vehicles are olive drab, information-age
ones a darker green (`mat.nato_green`), warships and jets haze grey (`mat.haze_grey`), sights,
drones and carbines black composite (`mat.composite`); the strategic bomber is anti-flash white.

Task 49 (future buildings, from the automation age) adds one shape, `torus` (a ring of round
tube lying flat: the fusion plant's reactor), and no motion. Future buildings are smooth white
composite (`mat.white_composite`, panelled walls `mat.composite_panel`) with dark teal smart
glass bands (`mat.smart_glass`), round forms (vaulted roofs are lying cylinders, domes, capsule
ends), solar panels on roofs (`mat.solar_panel`) and thin cyan light strips (`mat.light_strip`,
self-lit) along bases, roof edges and walls. Team colour sits on bands, fascias, crowns, domes of
corner towers and banners. Two more self-lit colours: pink grow lights in the automated farm
(`mat.grow_light`) and violet plasma in the fusion reactor, shield pylons and rail-gun muzzles
(`mat.plasma`). Dishes are a shallow dome facing the sky with a deeper one behind, so they read
from both sides.

Task 50 (future units, automation and fusion) adds one material and no shape or motion knob.
Machines wear white composite and graphite with cyan light strips; fusion-age weapons glow
violet (`mat.plasma`). Robots and armoured troops are figures: the worker drone and combat robot
have grey limbs (`skin`, `legs`), a box head with a visor (cyan for the worker, red for the
combat robot) and the team colour on the tunic; the combat robot's rifle arm is a gun held at
arm's length (`attack` 6), the exo-trooper a figure of heavy build with a team-coloured shoulder
yoke and a tall power pack. Hover units carry their own height and float on `bank`: the
hovercraft on its skirt with ducted fans (`torus` rings round `propeller`s), the fusion tank on
glowing lift pads (flat `wheel`s), the hover frigate on two pontoons with glowing undersides.
Energy weapons fire on the `flash` joint like the missiles of task 48: a pulse gunner's and an
interceptor's bolts (plasma cylinders ahead of the muzzle, bursts on alternate keyframes), the
fusion tank's twin plasma bolts, a railgun's bright streak (`mat.light_strip`) beside a flash,
and the laser battery's beam, a thin cylinder of the self-lit red `mat.laser`. Shot effects stay
within about two and a half tiles of the muzzle, because a model's bounds give its selection and
health-bar size.

Task 51 (space age: starfaring to galactic) adds no shape, motion knob or material. Starships
share the look of the task 58 craft: blue-grey steel hulls, dark iron engine bells with the
orange self-lit drives of task 44 (`mat.glow`) and team colour on wings, spines, deck stripes and
bridge bands; weapons fire on the `flash` joint (a muzzle flash across a turret's barrels and a
bolt ahead of each, cyan on the starfaring ships, violet plasma later). They grow with the ages
and each has its own plan seen from above: the survey ship a thin hull under a large dish with a
sensor boom and solar wings (2.5 tiles long), the corvette a narrow dart with one twin turret and
canted fins (2.8), the frigate a hull between two outrigger engine nacelles on team-coloured
pylons with two turrets (3.4), the cruiser a team-coloured hammerhead bow, a tall bridge tower,
radiator wings and three triple turrets (3.9), the battleship a stepped wedge with a spinal
plasma cannon along its bow, four turrets and five engines (5.1). Turrets swing onto the target
(`traverse`); the battleship's cannon recoils. Ground units are future machines and armoured
figures: the marine wears a round helmet with a gold visor and a life-support pack, the beam
trooper a crested helmet and a capacitor ring on the back and fires a thin cyan beam, the shock
marine is a figure of heavy build with a team-coloured breastplate, a tall shield and a plasma
pistol firing bursts; the grav skimmer is a hover bike on two glowing lift discs (`wheel`s) with
a seated rider, the grav tank a wide floating hull on cyan lift rings with a domed turret and a
beam lance. The spaceport (8×8, the largest landmark) keeps a rocket on its pad beside a tall
gantry as its signature, with vaulted hangars, two landing pads (a shuttle on one), a control
tower and an operations block on white paving. Ships in orbit are the same models, drawn two to
five times their size; the system view still draws every ship as the generic hull of task 53.

Task 73 adds two first looks for a later review round, with no new shape or material: the
**automated colony** (`bld.auto_colony@abundance`, 5 × 5) on white paving, a glass-domed
control hub with a team-coloured band, a tall relay mast with two light rings and a tilted dish,
a quad-rotor drone on a lit pad, two storage tanks with team caps, a tilted solar array, a robot
arm over crates and a flag; and an **alternative look for the Automated Research Lab**,
`bld.auto_university@automation~lab`: a white lab hall with sawtooth skylights, a team band and
fume stacks, a particle ring with a glowing detector laid on the ground in front of it, a glass
greenhouse dome and a mast. The alternative is a **review candidate** (`entity_visual.candidate =
'lab'`, `docs/database.md` §10a): the gallery, its manifest and the export show it under that id,
no match draws it, and the approved look stays the default and stays locked. The owner approved
it in review round 12 (2026-11-17): it is now the lab's only look, `bld.auto_lab@automation`
(the building was renamed the Auto Lab the same day), locked in `approved.toml`; the earlier
look was retired.

Task 93 (higher-definition spaceships) adds no shape, motion knob or material. The owner: "there
is never gonna be a whole lot of them, we can afford a higher polygon budget". Every starship and
orbital craft has about four times its earlier near triangles and keeps its far version (the
detail is near-only, `far` 0). Detail that reads at the system view's distance: hull plating in
plates with gaps between them (`mat.composite_panel` tinted blue-grey), dorsal and flank plates,
keels, bridge windows in light strips, masts with yards, radar domes, dishes and whip aerials,
engine bells in cast iron with a band, a glowing throat and a short glowing plume, turrets on
rings with a hood, a mantlet and muzzles, the fore turrets facing ahead and the aft ones astern
(the battleship's aft pair astern and outboard), point-defence mounts, the carrier's lit hangar
mouths (one wide at the bow, two in each flank) and deck lights, running lights (red
`mat.signal_red` to port, `mat.light_strip` to starboard) and a red beacon on the highest mast.
The classes keep their plans and their size order (corvette 3.3 tiles long, frigate 3.9, cruiser
5.0, battleship 7.3 with their shots), and their bounds, which give the landing radius
(`HULL_REACH` in `db/content/generate.py`). The space frigate, cruiser and battleship each have
two looks since task 92 moved them one age earlier: the first one is the earlier generation,
with cyan bolts (`mat.light_strip`; the battleship's spinal mass driver too) and lighter fittings
(the frigate's open radiator vanes, the cruiser's twin aft turret), the later one fires violet
plasma and adds armour and guns (the frigate's nacelle cowls and point defence, the cruiser's
third triple turret and tower wings, the battleship's superfiring mid turrets). The two line
ships of task 92 keep the sea budget: Ships of the Line (`unit.ship_of_the_line@enlightenment`),
a two-decker longer and taller than the frigate with two tiers of ports in team bands and three
masts of three sails, and Dreadnoughts (`unit.dreadnought@modern`), a long haze-grey hull with
five twin turrets (one forward, two on the wings facing out, two aft, one superfiring), a tripod
mast with a spotting top and two funnels with team bands.

Task 95 makes shots visible in flight, as the weapon's `et_weapon.projectile` says (presentation
only; `rom-render/src/projectiles.rs`). Arrows (a shaft with white fletching), sling and siege
stones, shells (brass with a short hot streak behind), rockets and missiles (with a flame and a
smoke trail) and torpedoes are small models of the part kit, about a tile long, turned along
their way; arrows, stones and shells fly an arc whose height grows with the distance, the rest a
line, and anything to or from orbit a straight line drawn two and a half times larger. Bullets
and crossbow bolts are short bright streaks; plasma a violet glowing bolt; lasers a cyan beam
with a white core, a soft halo and a flicker, held for a moment from muzzle to target; rail shots
a pale streak that fades. Each leaves a small effect where it lands: dust for arrows, bolts and
stones, sparks for bullets and rail shots, a flash and smoke for shells, rockets, missiles and
torpedoes, a glow for beams and plasma. Streaks, beams and glows are drawn with additive
blending.

## Budgets

| Model | Near | Far |
| --- | --- | --- |
| Building | ≤ 1,500 triangles | ≤ 300 |
| Unit figure (each figure of a crew) | ≤ 300 | ≤ 80 |
| Vehicle, ship, aircraft | ≤ 800 | ≤ 160 |
| Starship or orbital craft (drawn in space: an `et_view_scale` row) | ≤ 3,200 | ≤ 160 |

Construction: a foundation plate first, then the model rises from the ground
as work progresses.

## How models are reviewed

Every model has a **model id**: `<entity key>@<age>`, for example
`bld.temple@classical` or `unit.hoplite@classical` (`@any` when the look does
not depend on the age). Every part has a name inside its model, so a part is
`bld.temple@classical/roof`. Ids are printed under each model in the gallery
and on the review page; use them in instructions.

1. **Gallery in the game.** `--gallery <age|key|units|all>` lays the models
   out in a grid on plain ground, labelled with their ids (units turned
   three-quarters to the camera); `--model <id>` shows one model from four
   angles; `--parts` labels each part; `--pose <idle|move|attack|work>` sets
   what the units do and `--anim-time <seconds>` stops the motion at that
   moment of every cycle. All work with `--screenshot`.
2. **Review page.** The orchestrator publishes a private page with every
   model: its pictures, part list, parameters and status (draft, needs work,
   approved). Comment per model there or in chat, using the ids.
3. **Directions become data.** A model is a list of named parts in
   `mesh_part` rows (shape, material, size, position, knobs), so most
   directions ("steeper roof on `bld.temple@classical/roof`", "all early roofs
   thatch") are edits to the seed, rebuilt into the gallery in minutes.
   Sketches or marked-up screenshots dropped into `app/docs/art/notes/` are
   read as part of the brief.
4. **Approved models are locked.** An approved model's geometry fingerprint is
   recorded in `app/docs/art/approved.toml`; a test fails if it changes, so
   later work cannot silently alter what was signed off. Changing an approved
   model means updating its fingerprint deliberately, with the owner's say.
