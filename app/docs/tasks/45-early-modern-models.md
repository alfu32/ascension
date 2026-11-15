# Task 45 — Models: Early modern: gunpowder and enlightenment

Implementer: Codex (`gpt-6.1-sol`, high). Crates: seed (and `rom-render`
part kit and motion, `rom-app` tests, only as needed). Review round 5.

**Read `model-card-common.md` first**: it holds the method, rules, review
material, tests, the done-when list and how to finish. This card lists only
what to build.

Era family **early modern** (gunpowder, enlightenment): plaster, roof tile,
brick, sash windows, symmetrical façades, low star-shaped earthworks.

## Buildings

- **First looks** (`@enlightenment`): `bld.bastion` (2×2 angled masonry
  bastion with gun embrasures and a flag) and `bld.star_fort` (6×6 low
  star-shaped fort: pointed bastions, glacis, gate, barracks block inside;
  a landmark).
- **Early-modern look of every earlier building**, `@gunpowder` (applies
  through the enlightenment age): city, farm, woodcutter's camp, library,
  market, barracks, tower, dock, granary, lumber mill, mine, university,
  temple, stable, siege workshop, smelter, fort — 17 models.

## Units (`@enlightenment`)

| Model | Silhouette |
| --- | --- |
| `unit.fusilier` | bayonet musket, tricorne, cross belts |
| `unit.light_infantry` | rifle, short jacket, light cap |
| `unit.ranger` | rifle, wide hat, fringed coat (scout) |
| `unit.dragoon` | rider with carbine, helmet with horsehair crest |
| `unit.lancer` | rider with long lance and pennant, czapka-style cap |
| `unit.cannon` | field cannon on a two-wheeled carriage, gunner |
| `unit.frigate` | three-masted square-rigged warship, gun ports |
| `unit.freight_wagon` | large covered freight wagon, four horses |

## Motion fix

Long firearms are fired **two-handed from the shoulder** (left hand under
the barrel), not one-handed from the hip as `unit.musketeer@gunpowder` does
today (owner review pending; fix the figure attack style for all long guns,
including the musketeer and arquebusier, and show before/after). The muzzle
flash should be visible in the `_attack` screenshot.
