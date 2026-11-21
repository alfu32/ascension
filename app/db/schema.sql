-- Rise of Man — game database schema (SQLite 3.37+, STRICT tables)
--
-- Two halves:
--   PART A  "rules"  — static definitions shipped with the game (what CAN exist).
--   PART B  "state"  — one match / save game (what DOES exist right now).
--
-- The simulation never queries SQL during a tick. At match start the rules are
-- loaded into flat in-memory arrays indexed by def.id; state tables are the
-- save-game / replay-seed format and the authoring/debugging view.
--
-- Conventions
--   * time        : integer simulation ticks. The step is FIXED at 20 ticks per game
--                   second; game speed only changes how many ticks run per real second
--   * distance    : integer milli-tiles (1 tile = 1000). Tile = one terrain cell.
--   * percentages : integer, 100 = 100 %
--   * amounts     : integer resource units (no hidden x10 factor)
--   * every definable thing has ONE row in `def`; subtype tables share its id
--   * booleans    : INTEGER 0/1
--   * no behaviour is keyed on a def's name in code: behaviour comes from
--     component rows, tags and modifiers only

PRAGMA foreign_keys = ON;

-- ════════════════════════════════════════════════════════════════════════════
-- PART A — RULES
-- ════════════════════════════════════════════════════════════════════════════

CREATE TABLE schema_meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
) STRICT;

-- Global tunables (tick rate, respond ranges, flank bonus, refund rate …).
CREATE TABLE game_constant (
  key   TEXT PRIMARY KEY,
  value INTEGER NOT NULL,   -- integers only; express fractions as percentages or milli-units
  unit  TEXT NOT NULL DEFAULT '',
  note  TEXT NOT NULL DEFAULT ''
) STRICT;

-- ── A1. Catalogue supertable ────────────────────────────────────────────────
-- Costs, prerequisites, tags, modifiers, availability and texts all point here,
-- so they work the same for units, buildings, techs, abilities, resources …
CREATE TABLE def (
  id    INTEGER PRIMARY KEY,
  kind  TEXT NOT NULL CHECK (kind IN (
          'age','faction','resource','terrain','entity_type','tech','ability',
          'formation','stance','material','mesh','map_style','victory','brush')),
  key   TEXT NOT NULL UNIQUE,          -- stable id used by data files & saves, e.g. 'unit.citizen'
  name  TEXT NOT NULL,                 -- default (English) display name
  descr TEXT NOT NULL DEFAULT '',
  icon  TEXT NOT NULL DEFAULT '',      -- icon atlas key
  sort  INTEGER NOT NULL DEFAULT 0
) STRICT;
CREATE INDEX def_kind ON def(kind);

CREATE TABLE def_text (                -- localisation overrides
  def_id INTEGER NOT NULL REFERENCES def(id) ON DELETE CASCADE,
  locale TEXT NOT NULL,
  field  TEXT NOT NULL CHECK (field IN ('name','descr')),
  value  TEXT NOT NULL,
  PRIMARY KEY (def_id, locale, field)
) STRICT, WITHOUT ROWID;

-- Tags replace letter-flag masks. Modifiers, weapons, healers, transports and
-- AI target things by tag ('infantry','mounted','mechanical','biological',
-- 'civilian','siege','ship','aircraft','drone','city','wonder', …).
CREATE TABLE tag (
  id       INTEGER PRIMARY KEY,
  key      TEXT NOT NULL UNIQUE,
  category TEXT NOT NULL DEFAULT ''
) STRICT;

CREATE TABLE def_tag (
  def_id INTEGER NOT NULL REFERENCES def(id) ON DELETE CASCADE,
  tag_id INTEGER NOT NULL REFERENCES tag(id),
  PRIMARY KEY (def_id, tag_id)
) STRICT, WITHOUT ROWID;
CREATE INDEX def_tag_by_tag ON def_tag(tag_id);

-- ── A2. World model: layers, tiers, terrain, body kinds ─────────────────────
-- The universe is a star map of star systems joined by routes; each system has
-- a star, habitable planets (cube-sphere surfaces where cities stand and ground
-- battles happen) and resource bodies (small objects claimed by presence).
-- See app/docs/world-model.md.
-- A layer is a movement medium. Sea and space follow the same rules (ports,
-- vessels, transports, trade lanes) but are different layers.
CREATE TABLE layer (
  key  TEXT PRIMARY KEY,               -- ground | fluid | subfluid | air | orbit | space
  note TEXT NOT NULL DEFAULT ''
) STRICT, WITHOUT ROWID;

-- What a body in a star system is: its star, a habitable planet (full surface)
-- or a resource body (no surface play; owned while a starship stays beside it).
CREATE TABLE body_kind (
  key  TEXT PRIMARY KEY,               -- star | habitable | resource
  note TEXT NOT NULL DEFAULT ''
) STRICT, WITHOUT ROWID;

-- A tier is how far out a player can see and act. Each age belongs to a tier;
-- a player's view and reach widen when THAT player reaches the tier — players
-- get there one by one, and the first to arrive finds space unclaimed.
CREATE TABLE map_scale (
  key   TEXT PRIMARY KEY,              -- planetary | interplanetary | interstellar
  name  TEXT NOT NULL,
  rank  INTEGER NOT NULL UNIQUE,       -- 0 = closest
  max_zoom_out INTEGER NOT NULL        -- camera limit, tiles visible across the screen
) STRICT, WITHOUT ROWID;

CREATE TABLE terrain (
  id         INTEGER PRIMARY KEY REFERENCES def(id),
  layer      TEXT NOT NULL REFERENCES layer(key),
  buildable  INTEGER NOT NULL DEFAULT 1,
  blocks_los INTEGER NOT NULL DEFAULT 0,
  defense_pct INTEGER NOT NULL DEFAULT 100,  -- damage taken by units standing here
  material_id INTEGER REFERENCES def(id)
) STRICT;

-- Units sharing a movement class share one pathfinding cost grid.
CREATE TABLE movement_class (
  id   INTEGER PRIMARY KEY,
  key  TEXT NOT NULL UNIQUE,           -- foot, wheeled, tracked, ship, submarine, aircraft, hover, starship …
  flies INTEGER NOT NULL DEFAULT 0     -- ignores terrain collision; uses air/orbit layer
) STRICT;

CREATE TABLE movement_class_terrain (  -- no row = impassable
  movement_class_id INTEGER NOT NULL REFERENCES movement_class(id),
  terrain_id        INTEGER NOT NULL REFERENCES terrain(id),
  speed_pct         INTEGER NOT NULL DEFAULT 100,
  PRIMARY KEY (movement_class_id, terrain_id)
) STRICT, WITHOUT ROWID;

-- ── A3. Timeline ────────────────────────────────────────────────────────────
CREATE TABLE age (
  id        INTEGER PRIMARY KEY REFERENCES def(id),
  ordinal   INTEGER NOT NULL UNIQUE,   -- 0 = first age
  era       TEXT NOT NULL CHECK (era IN ('history','near_future','stellar','galactic')),
  map_scale TEXT NOT NULL REFERENCES map_scale(key),  -- tier this age gives its owner
  view_range INTEGER NOT NULL DEFAULT 0 CHECK (view_range >= 0),  -- largest ground half-width the camera may show, milli-tiles; 0 = whole globe
  free_colonisation INTEGER NOT NULL DEFAULT 0 CHECK (free_colonisation IN (0,1))  -- 1: from this age a city may be founded anywhere not inside another player's territory
) STRICT;

-- ── A4. Factions (nations → races) ──────────────────────────────────────────
CREATE TABLE faction (
  id        INTEGER PRIMARY KEY REFERENCES def(id),
  kind      TEXT NOT NULL CHECK (kind IN ('nation','bloc','race','npc')),
  parent_id INTEGER REFERENCES faction(id),         -- nation → continental bloc → race
  playable  INTEGER NOT NULL DEFAULT 1,
  first_age_id INTEGER REFERENCES age(id),          -- selectable from this age
  last_age_id  INTEGER REFERENCES age(id)           -- nations stop existing after unification
) STRICT;

-- A player picks a RACE before the match and keeps it. Inside the human race
-- the identity ladder is nation → continental bloc (forced, after the
-- contemporary ages) → united humanity (forced, on reaching space). Other
-- races have their own ladder or none. Races start on separate home worlds,
-- develop equivalent technology, and meet only once they reach space. A faction is an identity and a
-- roster, not a side: several players may share one faction and still be at war.
CREATE TABLE faction_transition (
  from_faction_id INTEGER NOT NULL REFERENCES faction(id),
  to_faction_id   INTEGER NOT NULL REFERENCES faction(id),
  trigger_tech_id INTEGER NOT NULL REFERENCES def(id),
  forced          INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (from_faction_id, to_faction_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE faction_name (            -- leader / city / planet name pools
  faction_id INTEGER NOT NULL REFERENCES faction(id),
  kind       TEXT NOT NULL CHECK (kind IN ('leader','city','planet','ship')),
  seq        INTEGER NOT NULL,
  name       TEXT NOT NULL,
  PRIMARY KEY (faction_id, kind, seq)
) STRICT, WITHOUT ROWID;

-- A race may call a shared definition by its own name: the same age, tech,
-- unit or building is "equivalent" across races and only the label (and the
-- look, through entity_visual) differs. Children inherit from the parent faction.
CREATE TABLE faction_alias (
  def_id     INTEGER NOT NULL REFERENCES def(id) ON DELETE CASCADE,
  faction_id INTEGER NOT NULL REFERENCES faction(id),
  name       TEXT NOT NULL,
  descr      TEXT NOT NULL DEFAULT '',
  PRIMARY KEY (def_id, faction_id)
) STRICT, WITHOUT ROWID;

-- Restriction list. No rows for a def = available to every faction.
-- Rows present = available only to listed factions (children inherit from parent).
CREATE TABLE availability (
  def_id     INTEGER NOT NULL REFERENCES def(id) ON DELETE CASCADE,
  faction_id INTEGER NOT NULL REFERENCES faction(id),
  PRIMARY KEY (def_id, faction_id)
) STRICT, WITHOUT ROWID;

-- ── A5. Resources ───────────────────────────────────────────────────────────
CREATE TABLE resource (
  id         INTEGER PRIMARY KEY REFERENCES def(id),
  kind       TEXT NOT NULL CHECK (kind IN ('base','rare')),
  abbrev     TEXT NOT NULL,
  rate_capped INTEGER NOT NULL DEFAULT 1,   -- income limited by the player's commerce cap
  tradeable  INTEGER NOT NULL DEFAULT 1,    -- can be bought/sold at a market (et_market)
  market_base_price INTEGER NOT NULL DEFAULT 100 CHECK (market_base_price > 0),
                                            -- price of one lot (market.lot_size) at rest, in
                                            -- the market's currency, before the spread
  first_age_id INTEGER REFERENCES age(id)   -- invisible / unusable before this age
) STRICT;

-- A claimed rare resource pays base-resource income; its other bonuses are
-- ordinary rows in `modifier` with source_def_id = the rare resource.
CREATE TABLE resource_yield (
  rare_resource_id INTEGER NOT NULL REFERENCES resource(id),
  base_resource_id INTEGER NOT NULL REFERENCES resource(id),
  amount           INTEGER NOT NULL,        -- per income period
  PRIMARY KEY (rare_resource_id, base_resource_id)
) STRICT, WITHOUT ROWID;

-- ── A6. Combat classes ──────────────────────────────────────────────────────
CREATE TABLE damage_class (id INTEGER PRIMARY KEY, key TEXT NOT NULL UNIQUE) STRICT;
CREATE TABLE armor_class  (id INTEGER PRIMARY KEY, key TEXT NOT NULL UNIQUE) STRICT;

CREATE TABLE damage_matrix (           -- missing row = 100 %
  damage_class_id INTEGER NOT NULL REFERENCES damage_class(id),
  armor_class_id  INTEGER NOT NULL REFERENCES armor_class(id),
  pct             INTEGER NOT NULL,
  PRIMARY KEY (damage_class_id, armor_class_id)
) STRICT, WITHOUT ROWID;

-- ── A7. Entity types and their components ───────────────────────────────────
-- One table for units, buildings, resource nodes, pick-ups and wildlife.
-- What an entity can DO is decided by which et_* component rows it has, so a
-- "flying factory" or a "mobile temple" needs no new code path.
CREATE TABLE entity_type (
  id            INTEGER PRIMARY KEY REFERENCES def(id),
  class         TEXT NOT NULL CHECK (class IN ('unit','building','node','pickup','wildlife')),
  ui_group      TEXT NOT NULL DEFAULT '',     -- economic | military | wonder | foot | mounted | …
  hp            INTEGER NOT NULL DEFAULT 1,
  armor         INTEGER NOT NULL DEFAULT 0,   -- flat reduction per hit (min 1 damage)
  armor_class_id INTEGER REFERENCES armor_class(id),
  los           INTEGER NOT NULL DEFAULT 0,   -- milli-tiles
  pop_cost      INTEGER NOT NULL DEFAULT 0,
  footprint_w   INTEGER NOT NULL DEFAULT 1,   -- tiles (buildings/nodes)
  footprint_h   INTEGER NOT NULL DEFAULT 1,
  radius        INTEGER NOT NULL DEFAULT 250, -- collision radius, milli-tiles (units)
  figures       INTEGER NOT NULL DEFAULT 1,   -- models drawn per unit (3 soldiers = 1 unit)
  build_ticks   INTEGER NOT NULL DEFAULT 0,   -- train / construct time
  ramp_mode     TEXT NOT NULL DEFAULT 'none' CHECK (ramp_mode IN ('none','per_type','per_group')),
  ramp_group    TEXT NOT NULL DEFAULT '',     -- e.g. 'barracks' — shared ramp counter
  limit_scope   TEXT NOT NULL DEFAULT 'none' CHECK (limit_scope IN ('none','per_city','per_player','per_match')),
  limit_count   INTEGER NOT NULL DEFAULT 0,
  upgrades_from_id INTEGER REFERENCES entity_type(id), -- previous step in the upgrade line
  replaces_id   INTEGER REFERENCES entity_type(id),    -- faction-unique substitute for this type
  first_age_id  INTEGER REFERENCES age(id),
  score         INTEGER NOT NULL DEFAULT 0,   -- wonder points / score value
  plunder_pct   INTEGER NOT NULL DEFAULT 0    -- % of cost granted to whoever destroys it
) STRICT;
CREATE INDEX entity_type_class ON entity_type(class);

CREATE TABLE et_movement (             -- "moves"
  entity_type_id    INTEGER PRIMARY KEY REFERENCES entity_type(id),
  movement_class_id INTEGER NOT NULL REFERENCES movement_class(id),
  speed             INTEGER NOT NULL,          -- milli-tiles per second
  turn_rate         INTEGER NOT NULL DEFAULT 360, -- degrees per second
  formation_spacing INTEGER NOT NULL DEFAULT 750,
  packs             INTEGER NOT NULL DEFAULT 0, -- must deploy to fire / pack to move
  pack_ticks        INTEGER NOT NULL DEFAULT 0,
  climb_ticks       INTEGER NOT NULL DEFAULT 0 CHECK (climb_ticks >= 0),
                                       -- orbital craft (task 58): ticks to climb from a planet's
                                       -- surface to its orbit by itself, and to come back down;
                                       -- 0 = cannot. Craft never leave that orbit on their own.
  jump_charge_ticks INTEGER CHECK (jump_charge_ticks >= 0),
                                       -- task 70: ticks a ship charges at its system's edge before
                                       -- a jump along a route (stat `jump_charge`); NULL = the
                                       -- rule default `space.jump_charge_ticks`
  landing_radius    INTEGER CHECK (landing_radius >= 0),
                                       -- task 72: milli-tiles; a landed ship covers the tiles whose
                                       -- centres lie within this radius of the tile it stands on
                                       -- (stat `landing_radius`); NULL = that tile only
  takeoff_ticks     INTEGER NOT NULL DEFAULT 0 CHECK (takeoff_ticks >= 0 AND (takeoff_ticks = 0 OR climb_ticks = 0))
                                       -- aircraft (task 123): ticks to take off from the ground into
                                       -- the air above its planet, and to land; 0 = never lands
                                       -- (always airborne). Never together with climb_ticks.
) STRICT;

CREATE TABLE et_weapon (               -- "attacks" (0..n per type)
  id             INTEGER PRIMARY KEY,
  entity_type_id INTEGER NOT NULL REFERENCES entity_type(id),
  slot           INTEGER NOT NULL DEFAULT 0,
  damage         INTEGER NOT NULL,
  damage_class_id INTEGER NOT NULL REFERENCES damage_class(id),
  range_min      INTEGER NOT NULL DEFAULT 0,
  range_max      INTEGER NOT NULL DEFAULT 0,   -- 0 = melee
  cooldown_ticks INTEGER NOT NULL,
  shots          INTEGER NOT NULL DEFAULT 1,
  projectile_speed INTEGER NOT NULL DEFAULT 0, -- 0 = instant hit
  splash_radius  INTEGER NOT NULL DEFAULT 0,
  splash_pct     INTEGER NOT NULL DEFAULT 0,
  accuracy_pct   INTEGER NOT NULL DEFAULT 100,
  accuracy_falloff INTEGER NOT NULL DEFAULT 0, -- % lost per tile of range
  fires_moving   INTEGER NOT NULL DEFAULT 0,
  consumed       INTEGER NOT NULL DEFAULT 0,   -- attacker is destroyed (missile, fire ship, kamikaze drone)
  orbit_only     INTEGER NOT NULL DEFAULT 0 CHECK (orbit_only IN (0,1)), -- surface range below an orbiting attacker
  -- what the views draw in flight (task 95); presentation only, the simulation never reads it
  projectile     TEXT NOT NULL DEFAULT 'none' CHECK (projectile IN ('none','arrow','bolt','stone',
                   'bullet','shell','rocket','missile','torpedo','beam','plasma','railgun')),
  UNIQUE (entity_type_id, slot)
) STRICT;

CREATE TABLE et_weapon_target (        -- which layers a weapon can hit, and how well
  weapon_id INTEGER NOT NULL REFERENCES et_weapon(id) ON DELETE CASCADE,
  layer     TEXT NOT NULL REFERENCES layer(key), -- 'orbit': entities orbiting a planet; from that
                                       -- planet's surface the range is measured to their subpoint
  hit_pct   INTEGER NOT NULL DEFAULT 100, -- accuracy against this layer, % of the weapon's
  PRIMARY KEY (weapon_id, layer)
) STRICT, WITHOUT ROWID;

CREATE TABLE et_builder (              -- "builds, repairs"
  entity_type_id INTEGER PRIMARY KEY REFERENCES entity_type(id),
  build_pct      INTEGER NOT NULL DEFAULT 100,
  repair_pct     INTEGER NOT NULL DEFAULT 100
) STRICT;

CREATE TABLE et_harvester (            -- "harvests": works at a site with free slots
  entity_type_id INTEGER NOT NULL REFERENCES entity_type(id),
  resource_id    INTEGER NOT NULL REFERENCES resource(id),
  amount         INTEGER NOT NULL,     -- per income period, before site bonuses
  PRIMARY KEY (entity_type_id, resource_id)
) STRICT, WITHOUT ROWID;

-- A gather site (farm, lumber camp, mine, well, university, harvester station).
CREATE TABLE et_gather_site (
  entity_type_id INTEGER PRIMARY KEY REFERENCES entity_type(id),
  resource_id    INTEGER NOT NULL REFERENCES resource(id),
  slots_base     INTEGER NOT NULL DEFAULT 1,
  slots_per_node INTEGER NOT NULL DEFAULT 0,   -- extra slots per node tile in radius
  slots_max      INTEGER NOT NULL DEFAULT 1,
  node_radius    INTEGER NOT NULL DEFAULT 0,
  needs_node_terrain_id INTEGER REFERENCES terrain(id), -- forest, mountain, asteroid field …
  self_income    INTEGER NOT NULL DEFAULT 0    -- income with no worker (automated ages)
) STRICT;

-- A depletable or infinite deposit placed by the map generator.
CREATE TABLE et_node (
  entity_type_id INTEGER PRIMARY KEY REFERENCES entity_type(id),
  resource_id    INTEGER NOT NULL REFERENCES resource(id),
  amount         INTEGER,              -- NULL = infinite
  claim_mode     TEXT NOT NULL DEFAULT 'none' CHECK (claim_mode IN ('none','trader','presence','carry')),
                                       -- trader: a merchant sits on it (rare resources)
                                       -- carry: a tagged unit brings it to an et_keeper
                                       -- presence: owned by whoever keeps a unit with claim_tag in range (resource bodies)
  claim_tag_id   INTEGER REFERENCES tag(id),
  claim_radius   INTEGER NOT NULL DEFAULT 0,
  income         INTEGER NOT NULL DEFAULT 0   -- per income period to the claimant
) STRICT;

CREATE TABLE et_healer (               -- priests, medics, repair drones
  entity_type_id INTEGER PRIMARY KEY REFERENCES entity_type(id),
  target_tag_id  INTEGER NOT NULL REFERENCES tag(id), -- 'biological' or 'mechanical'
  hp_per_second  INTEGER NOT NULL,
  range          INTEGER NOT NULL,
  energy_per_hp  INTEGER NOT NULL DEFAULT 0,
  auto           INTEGER NOT NULL DEFAULT 1,
  can_convert    INTEGER NOT NULL DEFAULT 0   -- may use the 'convert' ability family
) STRICT;

CREATE TABLE et_trader (               -- "trades for gold, claims rare resources"
  entity_type_id INTEGER PRIMARY KEY REFERENCES entity_type(id),
  runs_routes    INTEGER NOT NULL DEFAULT 0,  -- caravan: city ↔ city route
  claims_nodes   INTEGER NOT NULL DEFAULT 0,  -- merchant: sits on a claimable node
  route_resource_id INTEGER REFERENCES resource(id),
  route_income   INTEGER NOT NULL DEFAULT 0   -- per arrival per trade.route_unit of route length
) STRICT;

CREATE TABLE et_keeper (               -- keeps carried nodes (temple ← relics)
  entity_type_id INTEGER PRIMARY KEY REFERENCES entity_type(id),
  capacity       INTEGER NOT NULL       -- nodes kept at once
) STRICT;

CREATE TABLE et_container (            -- garrison, transport, carrier, hangar
  entity_type_id INTEGER PRIMARY KEY REFERENCES entity_type(id),
  capacity       INTEGER NOT NULL,     -- in pop_cost units
  accept_tag_id  INTEGER REFERENCES tag(id),  -- NULL = anything
  heals          INTEGER NOT NULL DEFAULT 0,
  adds_firepower INTEGER NOT NULL DEFAULT 0,
  launches       INTEGER NOT NULL DEFAULT 0   -- aircraft/drones sortie from here and return; a
                                       -- carrier in orbit launches its craft into that orbit
) STRICT;

CREATE TABLE et_producer (             -- "takes production and research orders"
  entity_type_id INTEGER PRIMARY KEY REFERENCES entity_type(id),
  queue_limit    INTEGER NOT NULL DEFAULT 16,
  parallel       INTEGER NOT NULL DEFAULT 1,  -- orders worked on at once
  has_rally      INTEGER NOT NULL DEFAULT 1
) STRICT;

CREATE TABLE et_territory (            -- cities, forts, temples, spaceports
  entity_type_id INTEGER PRIMARY KEY REFERENCES entity_type(id),
  is_city        INTEGER NOT NULL DEFAULT 0,
  border_radius  INTEGER NOT NULL DEFAULT 0,  -- pushes national borders
  economic_radius INTEGER NOT NULL DEFAULT 0, -- city only: where civil buildings may stand
  capturable     INTEGER NOT NULL DEFAULT 0   -- at 0 hp it is captured, not destroyed
) STRICT;

CREATE TABLE et_energy (               -- ability pool
  entity_type_id INTEGER PRIMARY KEY REFERENCES entity_type(id),
  max            INTEGER NOT NULL,
  regen_per_second INTEGER NOT NULL
) STRICT;

CREATE TABLE et_supply (               -- supply wagon / tender / fleet auxiliary
  entity_type_id INTEGER PRIMARY KEY REFERENCES entity_type(id),
  radius         INTEGER NOT NULL,
  stock          INTEGER             -- NULL = unlimited
) STRICT;

CREATE TABLE et_inhibitor (            -- jump inhibitor (task 70): holds enemy ships where it stands
  entity_type_id INTEGER PRIMARY KEY REFERENCES entity_type(id),
  reach          TEXT NOT NULL DEFAULT 'system' CHECK (reach IN ('system')),
                                       -- 'system': every enemy ship in the star system of the
                                       -- planet it stands on may depart only back along the
                                       -- route it arrived by
  charge_pct     INTEGER NOT NULL DEFAULT 0 CHECK (charge_pct >= 0),
                                       -- extra jump charge time of those ships, percent; several
                                       -- inhibitors do not stack (the largest applies)
  arrival_damage INTEGER NOT NULL DEFAULT 0 CHECK (arrival_damage >= 0),
                                       -- task 104: hit points an enemy ship loses, once, when it
                                       -- jumps into that star system; a fixed blow, not through
                                       -- the armour table; several do not stack (the largest)
  arrival_radius INTEGER NOT NULL DEFAULT 0 CHECK (arrival_radius >= 0)
                                       -- task 111: the blow strikes only a ship whose arrival
                                       -- point lies within this distance (milli-tiles of the
                                       -- system's plane) of the planet it stands on; 0 = no blow
) STRICT;

CREATE TABLE et_detector (             -- jump detector (task 118): tells its owner of foreign
                                       -- ships jumping into a star system
  entity_type_id INTEGER PRIMARY KEY REFERENCES entity_type(id),
  scope          TEXT NOT NULL DEFAULT 'system' CHECK (scope IN ('system','radius')),
                                       -- 'system': every arrival in the star system of the
                                       -- planet it stands on; 'radius': arrivals within `radius`
                                       -- of it while it stands still in a system's plane
  radius         INTEGER NOT NULL DEFAULT 0 CHECK (radius >= 0),
                                       -- milli-tiles of the system's plane ('radius' scope)
  enabled        INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0,1))
                                       -- base of the entity stat `detector`: a type whose row
                                       -- says 0 detects only once a modifier sets the stat to 1
) STRICT;

-- Market (task 107): its owner buys and sells lots of the tradeable resources here
-- (orders `buy` and `sell`, subject = the resource) once a technology enables the rule
-- 'market_trading'; every trade is paid in `currency_id`, which is itself not tradeable.
-- Prices are per player; the constants `market.*` and the stat `market_spread` shape them
-- (docs/database.md, "Market trading").
CREATE TABLE et_market (
  entity_type_id INTEGER PRIMARY KEY REFERENCES entity_type(id),
  currency_id    INTEGER NOT NULL REFERENCES resource(id)
) STRICT;

-- Automated colony (task 73): a city (et_territory.is_city) that does not count towards
-- the city limit and keeps a governor. Its owner earns `income` of the resource each income
-- period, plus `site_share_pct` of what the complete gather sites of its area (buildings
-- whose city is the colony) earn by themselves (`et_gather_site.self_income`). A builder
-- without movement (its own et_builder row) builds on sites within its economic radius.
CREATE TABLE et_colony (
  entity_type_id INTEGER PRIMARY KEY REFERENCES entity_type(id),
  resource_id    INTEGER NOT NULL REFERENCES resource(id),
  income         INTEGER NOT NULL DEFAULT 0 CHECK (income >= 0),
  site_share_pct INTEGER NOT NULL DEFAULT 0 CHECK (site_share_pct BETWEEN 0 AND 100)
) STRICT;

-- What a colony's governor keeps standing in its area, by building type: at most `cap` of
-- the type's upgrade line, which is also that line's per-city limit there (task 73).
CREATE TABLE et_colony_site (
  colony_type_id INTEGER NOT NULL REFERENCES et_colony(entity_type_id),
  site_type_id   INTEGER NOT NULL REFERENCES entity_type(id),
  cap            INTEGER NOT NULL CHECK (cap > 0),
  PRIMARY KEY (colony_type_id, site_type_id)
) STRICT, WITHOUT ROWID;

-- Where something may be placed. All rows for a type must pass.
CREATE TABLE placement_rule (
  id             INTEGER PRIMARY KEY,
  entity_type_id INTEGER NOT NULL REFERENCES entity_type(id),
  rule           TEXT NOT NULL CHECK (rule IN (
                   'in_own_territory','in_city_radius','on_terrain','adjacent_terrain',
                   'near_type','min_distance_type','min_distance_enemy_city','on_node','on_layer')),
  ref_def_id     INTEGER REFERENCES def(id),
  layer          TEXT REFERENCES layer(key),
  distance       INTEGER NOT NULL DEFAULT 0
) STRICT;

-- One-time grant when an entity is completed (new farm → food, new market → wealth).
CREATE TABLE completion_bonus (
  entity_type_id INTEGER NOT NULL REFERENCES entity_type(id),
  resource_id    INTEGER NOT NULL REFERENCES resource(id),
  amount         INTEGER NOT NULL,
  PRIMARY KEY (entity_type_id, resource_id)
) STRICT, WITHOUT ROWID;

-- ── A8. The economy: who makes what, for how much, after what ───────────────
-- Every button on a command panel is one row here.
CREATE TABLE production (
  producer_type_id INTEGER NOT NULL REFERENCES entity_type(id),
  product_def_id   INTEGER NOT NULL REFERENCES def(id),  -- entity_type or tech
  mode   TEXT NOT NULL CHECK (mode IN ('train','construct','research','morph')),
  ui_page TEXT NOT NULL DEFAULT '',     -- sub-menu ('economic','military','wonder')
  ui_col INTEGER NOT NULL DEFAULT 0,
  ui_row INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (producer_type_id, product_def_id)
) STRICT, WITHOUT ROWID;
CREATE INDEX production_by_product ON production(product_def_id);

CREATE TABLE cost (
  def_id      INTEGER NOT NULL REFERENCES def(id) ON DELETE CASCADE,
  resource_id INTEGER NOT NULL REFERENCES resource(id),
  amount      INTEGER NOT NULL,
  ramp        INTEGER NOT NULL DEFAULT 0,   -- added per already-owned item (see entity_type.ramp_mode)
  PRIMARY KEY (def_id, resource_id)
) STRICT, WITHOUT ROWID;

-- def is unlocked when, for EVERY alt_group, at least ONE listed requirement is met.
CREATE TABLE prerequisite (
  def_id          INTEGER NOT NULL REFERENCES def(id) ON DELETE CASCADE,
  requires_def_id INTEGER NOT NULL REFERENCES def(id),   -- tech, age or entity_type (must own one)
  alt_group       INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (def_id, requires_def_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE tech (
  id      INTEGER PRIMARY KEY REFERENCES def(id),
  -- 'handicap': never researched; a host grants it to one player at match setup
  -- (a difficulty's income bonus, see ai_difficulty.handicap_tech_id). Its
  -- modifiers apply like any technology's; it has no production row, no
  -- effects and is no prerequisite.
  kind    TEXT NOT NULL CHECK (kind IN ('age','track','upgrade','unit_upgrade','government','project','evolution','handicap')),
  track   TEXT NOT NULL DEFAULT '',        -- military | civic | commerce | science | ''
  level   INTEGER NOT NULL DEFAULT 0,
  research_ticks INTEGER NOT NULL,
  exclusive_group TEXT NOT NULL DEFAULT '', -- only one tech per group (governments, evolutions)
  repeatable INTEGER NOT NULL DEFAULT 0
) STRICT;

-- Non-numeric results of a tech. Numeric results are rows in `modifier`.
CREATE TABLE tech_effect (
  id        INTEGER PRIMARY KEY,
  tech_id   INTEGER NOT NULL REFERENCES tech(id) ON DELETE CASCADE,
  effect    TEXT NOT NULL CHECK (effect IN (
              'set_age','unlock','upgrade_type','grant_ability','reveal_map',
              'change_faction','spawn','enable_rule','ascend')),
              -- 'ascend' (task 76): completing the tech takes its player out of the material
              -- plane (docs/ascension.md); it may be researched only once every other
              -- technology its producer offers is researched.
  target_def_id INTEGER REFERENCES def(id),  -- age / unlocked def / new type / ability / faction
  from_def_id   INTEGER REFERENCES def(id),  -- upgrade_type: the old type
  rule_key  TEXT NOT NULL DEFAULT '',        -- enable_rule: 'market_trading' (task 107: buy and
                                             -- sell at an et_market), 'sea_transport','ally_los' …
                                             -- used (task 70): 'route_sensors', 'route_survey',
                                             -- 'interstellar_trade' (database.md, "Routes and jumps")
  amount    INTEGER NOT NULL DEFAULT 0
) STRICT;
CREATE INDEX tech_effect_by_tech ON tech_effect(tech_id);

-- ── A9. Modifiers — the one mechanism for every bonus in the game ───────────
-- Sources: techs, ages, factions, governments, rare resources, wonders (entity
-- types), auras of units (generals), abilities, terrain, map settings.
CREATE TABLE stat (
  key   TEXT PRIMARY KEY,              -- 'hp','attack','speed','cost.all','cost.food','build_time',
  scope TEXT NOT NULL CHECK (scope IN ('entity','player')),  --  'income.food','pop_cap','commerce_cap' …
  note  TEXT NOT NULL DEFAULT ''
) STRICT, WITHOUT ROWID;

CREATE TABLE modifier (
  id            INTEGER PRIMARY KEY,
  source_def_id INTEGER NOT NULL REFERENCES def(id) ON DELETE CASCADE,
  applies_to    TEXT NOT NULL DEFAULT 'owner' CHECK (applies_to IN ('owner','allies','enemies','everyone')),
  target_tag_id INTEGER REFERENCES tag(id),      -- entity stats: only entities with this tag
  target_def_id INTEGER REFERENCES def(id),      -- … or only this exact def (both NULL = all)
  stat_key      TEXT NOT NULL REFERENCES stat(key),
  op            TEXT NOT NULL CHECK (op IN ('add','pct','mul','set')),
  value         INTEGER NOT NULL,
  aura_radius   INTEGER,                         -- NOT NULL = only near an entity of the source type
  condition     TEXT NOT NULL DEFAULT '' CHECK (condition IN (
                  '','in_own_territory','in_enemy_territory','garrisoned','near_supply','entrenched','on_high_ground')),
  duration_ticks INTEGER,                        -- NOT NULL = temporary (ability effects)
  stack_group   TEXT NOT NULL DEFAULT ''         -- same group: only the strongest applies
) STRICT;
CREATE INDEX modifier_by_source ON modifier(source_def_id);
CREATE INDEX modifier_by_stat   ON modifier(stat_key);
-- Evaluation order for one stat: set → Σ add → × (100 + Σ pct)/100 → Π mul/100.

-- ── A10. Abilities, formations, stances ─────────────────────────────────────
CREATE TABLE ability (
  id           INTEGER PRIMARY KEY REFERENCES def(id),
  target       TEXT NOT NULL CHECK (target IN ('none','entity','point','area')),
  target_relation TEXT NOT NULL DEFAULT 'any' CHECK (target_relation IN ('any','own','ally','enemy','neutral')),
  target_tag_id INTEGER REFERENCES tag(id),
  range        INTEGER NOT NULL DEFAULT 0,
  radius       INTEGER NOT NULL DEFAULT 0,
  energy_cost  INTEGER NOT NULL DEFAULT 0,
  cast_ticks   INTEGER NOT NULL DEFAULT 0,
  cooldown_ticks INTEGER NOT NULL DEFAULT 0,
  effect       TEXT NOT NULL CHECK (effect IN (
                 'apply_modifiers','convert','damage','heal','spawn','reveal','cloak',
                 'disable','teleport','deploy','pack','morph','transfer_resources')),
  effect_def_id INTEGER REFERENCES def(id),   -- spawned / morph-target type
  amount       INTEGER NOT NULL DEFAULT 0,
  breaks_cloak INTEGER NOT NULL DEFAULT 1,
  queueable    INTEGER NOT NULL DEFAULT 0,    -- usable without cancelling current order
  autocast     INTEGER NOT NULL DEFAULT 0
) STRICT;

CREATE TABLE ability_grant (
  entity_type_id INTEGER NOT NULL REFERENCES entity_type(id),
  ability_id     INTEGER NOT NULL REFERENCES ability(id),
  ui_col INTEGER NOT NULL DEFAULT 0,
  ui_row INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (entity_type_id, ability_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE formation (
  id        INTEGER PRIMARY KEY REFERENCES def(id),
  shape     TEXT NOT NULL CHECK (shape IN ('line','column','wedge','square','echelon_left',
                                           'echelon_right','envelop','refused','sparse','mob','sphere')),
  rank_ratio_pct INTEGER NOT NULL DEFAULT 400, -- width : depth
  spacing_pct    INTEGER NOT NULL DEFAULT 100,
  ranged_behind  INTEGER NOT NULL DEFAULT 1,   -- melee front, ranged/support rear
  keeps_slowest  INTEGER NOT NULL DEFAULT 1
) STRICT;

CREATE TABLE stance (
  id       INTEGER PRIMARY KEY REFERENCES def(id),
  family   TEXT NOT NULL CHECK (family IN ('combat','worker','caster','deploy')),
  behavior TEXT NOT NULL            -- aggressive|defensive|stand_ground|hold_fire|raid|raze|gather|build|build_gather|idle|auto|manual
) STRICT;

-- ── A11. Presentation links (procedural — no model files) ───────────────────
CREATE TABLE material (                -- shader program + parameter values
  id      INTEGER PRIMARY KEY REFERENCES def(id),
  program TEXT NOT NULL,               -- 'foliage','glass','brick','tiling','water','metal','noise' …
  params  TEXT NOT NULL DEFAULT '{}'   -- JSON: open-ended shader uniforms
) STRICT;

CREATE TABLE mesh (                    -- procedural generator + parameters
  id        INTEGER PRIMARY KEY REFERENCES def(id),
  generator TEXT NOT NULL,             -- 'terrain_patch','tree','rock','box_building','hull' …
  params    TEXT NOT NULL DEFAULT '{}',
  baked     INTEGER NOT NULL DEFAULT 1 -- generated once before the match, never per frame
) STRICT;

-- A named part of a composed model (mesh.generator = 'composed'): one shape
-- placed in the model's frame (x east, y up, z south; tiles; origin at the
-- footprint centre on the ground).
CREATE TABLE mesh_part (
  id          INTEGER PRIMARY KEY,
  mesh_id     INTEGER NOT NULL REFERENCES mesh(id) ON DELETE CASCADE,
  name        TEXT NOT NULL,              -- reference name, unique in its mesh
  shape       TEXT NOT NULL,              -- renderer shape: 'box','gable_roof', … or 'include'
  material_id INTEGER REFERENCES material(id), -- NULL: the entity_visual's material
  team        INTEGER NOT NULL DEFAULT 0 CHECK (team IN (0,1)), -- wears the owner's colour
  attach      TEXT NOT NULL DEFAULT '',   -- part or socket it moves with ('' = the model)
  params      TEXT NOT NULL DEFAULT '{}', -- flat JSON: position, size, rotation, shape knobs
  UNIQUE (mesh_id, name)
) STRICT;

CREATE TABLE entity_visual (           -- most specific row wins (faction, then age)
  id             INTEGER PRIMARY KEY,
  entity_type_id INTEGER NOT NULL REFERENCES entity_type(id),
  faction_id     INTEGER REFERENCES faction(id),
  age_id         INTEGER REFERENCES age(id),
  mesh_id        INTEGER NOT NULL REFERENCES mesh(id),
  material_id    INTEGER REFERENCES material(id),
  scale_pct      INTEGER NOT NULL DEFAULT 100,
  candidate      TEXT CHECK (candidate IS NULL OR candidate GLOB '[a-z]*')
                                       -- task 73: a review candidate's name: shown by the gallery
                                       -- and its manifest (model id `<entity>@<age>~<name>`),
                                       -- never drawn in a match; NULL = in play
) STRICT;
CREATE INDEX entity_visual_by_type ON entity_visual(entity_type_id);

-- How large a unit is drawn in the space views (presentation only, task 57): above a
-- planet as a percentage of its true size, on a star system's plane as a percentage of
-- the readable reference size there. No row: 100 and 100.
CREATE TABLE et_view_scale (
  entity_type_id INTEGER PRIMARY KEY REFERENCES entity_type(id),
  planet_pct     INTEGER NOT NULL DEFAULT 100 CHECK (planet_pct BETWEEN 10 AND 1000),
  system_pct     INTEGER NOT NULL DEFAULT 100 CHECK (system_pct BETWEEN 10 AND 400)
) STRICT;

-- ── A12. Map styles and match options ───────────────────────────────────────
-- A style is a recipe for a whole universe (and, for tests and skirmishes, a flat map).
-- Universe knobs: which catalogue stars a universe takes (star_region and its numbers), what
-- each system holds; every `_min/_max` pair must satisfy min ≤ max (checked by the rules loader).
-- The launchers list the styles by def.sort.
CREATE TABLE map_style (
  id        INTEGER PRIMARY KEY REFERENCES def(id),
  planets   INTEGER NOT NULL DEFAULT 1,  -- flat map: planetary terrains (≥ 1 home terrain)
  fluid_pct INTEGER NOT NULL DEFAULT 30, -- sea share of a planet surface
  space_pct INTEGER NOT NULL DEFAULT 60, -- flat map: share of the whole map that is open space
  polar_cap_pct INTEGER NOT NULL DEFAULT 20, -- poleward latitude share, 0..=50
  start_latitude_max_mdeg INTEGER NOT NULL DEFAULT 60000, -- absolute latitude, 0..=90000
  polar_terrain_id INTEGER REFERENCES terrain(id), -- required when caps are enabled
  params    TEXT NOT NULL DEFAULT '{}', -- JSON: generator knobs
  systems_min INTEGER NOT NULL DEFAULT 4,  -- task 109: systems in a nearest region, homes included;
  systems_max INTEGER NOT NULL DEFAULT 6,  -- the seed picks the total, never fewer than one per race
  habitable_per_system_min INTEGER NOT NULL DEFAULT 1,      -- habitable planets per system (≥ 1)
  habitable_per_system_max INTEGER NOT NULL DEFAULT 3,
  resource_bodies_per_system_min INTEGER NOT NULL DEFAULT 3, -- moons, asteroids, small planets
  resource_bodies_per_system_max INTEGER NOT NULL DEFAULT 8,
  planet_n_min    INTEGER NOT NULL DEFAULT 128, -- cube face size N of a habitable planet, 64..=512
  planet_n_median INTEGER NOT NULL DEFAULT 192, -- home planets use the median, for fairness
  planet_n_max    INTEGER NOT NULL DEFAULT 256,
  route_extra_pct INTEGER NOT NULL DEFAULT 30,   -- extra routes beyond a spanning tree, % of its edges
  star_map_radius INTEGER NOT NULL DEFAULT 1000, -- widest region of the star catalogue a universe may span: every system within this many map units of its centre star
  star_region     TEXT NOT NULL DEFAULT 'nearest' CHECK (star_region IN ('nearest','single','wedge','all')),
                                       -- task 89, 109: which catalogue stars a universe takes.
                                       -- nearest: a seeded star and its nearest neighbours,
                                       -- systems_min..max in all, within star_map_radius;
                                       -- single: one seeded star whose system holds every home
                                       -- planet; wedge: a run of stars in galactic longitude from
                                       -- a seeded star, holding 1/region_min_den to
                                       -- 1/region_max_den of the catalogue; all: every star.
                                       -- systems_* and star_map_radius apply to nearest only
  region_min_den  INTEGER NOT NULL DEFAULT 1 CHECK (region_min_den >= 1),
                                       -- task 109, wedge only: the region's smallest share of the
                                       -- catalogue's stars is 1/region_min_den (rounded up) ...
  region_max_den  INTEGER NOT NULL DEFAULT 1 CHECK (region_max_den >= 1),
                                       -- ... and its largest 1/region_max_den (rounded down)
  shared_home     INTEGER NOT NULL DEFAULT 0 CHECK (shared_home IN (0,1)),
                                       -- task 109: 1 puts every player on one home planet,
                                       -- whatever the start placement (the Planet style)
  life_n          INTEGER NOT NULL DEFAULT 64 CHECK (life_n BETWEEN 16 AND 512),
                                       -- task 77: cube face size of the dormant surface every
                                       -- resource body gets before play (small bodies grow to it)
  life_terrain_id INTEGER REFERENCES terrain(id), -- the barren ground of a dormant surface; NULL: none
  CHECK (region_max_den <= region_min_den)
) STRICT;

-- What the generator scatters: forests, mountains, deposits, rare resources, ruins.
CREATE TABLE map_style_feature (
  id           INTEGER PRIMARY KEY,
  map_style_id INTEGER NOT NULL REFERENCES map_style(id),
  feature_def_id INTEGER NOT NULL REFERENCES def(id),  -- terrain or node/pickup entity_type
  region_kind  TEXT NOT NULL DEFAULT 'planet' CHECK (region_kind IN ('planet','space','resource_body')),
                                       -- planet: on a planet surface; space: flat map only;
                                       -- resource_body: what a resource body yields (a node with
                                       -- claim_mode 'presence'), weighted by chance_pct;
                                       -- per_player rows: clumps_min such bodies in every home system
  pattern      TEXT NOT NULL CHECK (pattern IN ('per_player','scattered','centre','edge','contested')),
  chance_pct   INTEGER NOT NULL DEFAULT 100,
  coverage_pct INTEGER NOT NULL DEFAULT 0 CHECK (coverage_pct BETWEEN 0 AND 100),
                                       -- sphere planet terrain: share of land tiles covered;
                                       -- 0: derive from clumps and sizes; ignored on flat maps
  clumps_min   INTEGER NOT NULL DEFAULT 1,
  clumps_max   INTEGER NOT NULL DEFAULT 1,
  size_min     INTEGER NOT NULL DEFAULT 1,
  size_max     INTEGER NOT NULL DEFAULT 1,
  start_min_dist INTEGER NOT NULL DEFAULT 0,  -- tiles from a player start
  start_max_dist INTEGER NOT NULL DEFAULT 0
) STRICT;

-- Task 77: the brushes an ascended player paints a planet it brought to life with. A stroke
-- (`paint` order) lays the brush's terrain on every tile within `radius` of the point, sets the
-- tiles' heights by `height_op` (dome toward `height` at the centre, falling to the tile's
-- height at the rim) and puts the brush's node at the centre. Ground terrain without a height
-- op (biomes, deposits) paints land tiles only.
CREATE TABLE life_brush (
  id           INTEGER PRIMARY KEY REFERENCES def(id),        -- def.kind 'brush'
  family       TEXT NOT NULL CHECK (family IN ('land','water','elevation','biome','deposit')),
  terrain_id   INTEGER REFERENCES terrain(id),                -- laid on the disc; NULL keeps it
  node_type_id INTEGER REFERENCES et_node(entity_type_id),    -- a deposit set at the centre
  height_op    TEXT NOT NULL DEFAULT 'keep' CHECK (height_op IN ('keep','set','raise','lower')),
  height       INTEGER NOT NULL DEFAULT 0 CHECK (height BETWEEN 0 AND 255), -- life.sea_height is sea level
  radius       INTEGER NOT NULL CHECK (radius BETWEEN 500 AND 64000),        -- milli-tiles
  CHECK (terrain_id IS NOT NULL OR node_type_id IS NOT NULL OR height_op <> 'keep')
) STRICT;

-- Game-setup options (start age, end age, pop limit, tech cost, rush rule …).
CREATE TABLE match_option (
  key     TEXT PRIMARY KEY,
  name    TEXT NOT NULL,
  default_value TEXT NOT NULL
) STRICT, WITHOUT ROWID;

CREATE TABLE match_option_choice (
  option_key TEXT NOT NULL REFERENCES match_option(key),
  value      TEXT NOT NULL,
  name       TEXT NOT NULL,
  sort       INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (option_key, value)
) STRICT, WITHOUT ROWID;

CREATE TABLE victory (
  id   INTEGER PRIMARY KEY REFERENCES def(id),
  rule TEXT NOT NULL CHECK (rule IN ('conquest','capital','territory','wonder','economic','score','time','tech_race')),
  threshold   INTEGER NOT NULL DEFAULT 0,
  timer_ticks INTEGER NOT NULL DEFAULT 0
) STRICT;

-- ── A13. Computer opponents ─────────────────────────────────────────────────
-- A computer player issues the same orders as a human through the order book.
-- Personality and skill are data; the decision code is shared by all profiles.
CREATE TABLE ai_profile (
  key   TEXT PRIMARY KEY,              -- balanced | rusher | builder | raider | turtle | explorer
  name  TEXT NOT NULL,
  descr TEXT NOT NULL DEFAULT ''
) STRICT, WITHOUT ROWID;

-- Tunable weights read by the decision code. Unknown keys are a data error.
CREATE TABLE ai_weight_key (
  key  TEXT PRIMARY KEY,               -- 'econ.worker_target_pct','mil.attack_ratio_pct','tech.age_priority' …
  default_value INTEGER NOT NULL,
  note TEXT NOT NULL DEFAULT ''
) STRICT, WITHOUT ROWID;

CREATE TABLE ai_profile_weight (
  profile_key TEXT NOT NULL REFERENCES ai_profile(key),
  weight_key  TEXT NOT NULL REFERENCES ai_weight_key(key),
  value       INTEGER NOT NULL,
  PRIMARY KEY (profile_key, weight_key)
) STRICT, WITHOUT ROWID;

-- Opening book: an ordered list of steps followed until it runs out or the
-- player is attacked; after that the general planner takes over.
CREATE TABLE ai_opening_step (
  profile_key TEXT NOT NULL REFERENCES ai_profile(key),
  seq         INTEGER NOT NULL,
  action      TEXT NOT NULL CHECK (action IN ('train','construct','research','assign_workers','scout','found_city')),
  subject_def_id INTEGER REFERENCES def(id),
  count       INTEGER NOT NULL DEFAULT 1,   -- have at least this many (existing + queued)
  blocking    INTEGER NOT NULL DEFAULT 1,   -- wait for resources instead of skipping
  PRIMARY KEY (profile_key, seq)
) STRICT, WITHOUT ROWID;

-- Skill level. Honest limits first (speed, attention); bonuses only at the top.
CREATE TABLE ai_difficulty (
  key   TEXT PRIMARY KEY,
  name  TEXT NOT NULL,
  rank  INTEGER NOT NULL UNIQUE,
  think_interval_ticks INTEGER NOT NULL,  -- how often the planner runs
  orders_per_minute    INTEGER NOT NULL,  -- action budget
  reaction_ticks       INTEGER NOT NULL,  -- delay before responding to a new threat
  mistake_pct          INTEGER NOT NULL DEFAULT 0, -- chance to pick the 2nd-best option
  uses_formations      INTEGER NOT NULL DEFAULT 1,
  uses_abilities       INTEGER NOT NULL DEFAULT 1,
  retreats             INTEGER NOT NULL DEFAULT 1,
  income_bonus_pct     INTEGER NOT NULL DEFAULT 0, -- applied as an ordinary modifier (below)
  sees_all             INTEGER NOT NULL DEFAULT 0, -- ignores fog of war
  -- The 'handicap' technology the host grants a player of this difficulty at
  -- setup; its income.* modifiers must equal income_bonus_pct (validator).
  handicap_tech_id     INTEGER REFERENCES tech(id),
  -- Task 67: how hard the military presses (a learner must be able to win at
  -- the lowest levels). No attack or raid on enemy towns before grace_ticks;
  -- the first wave holds at most first_wave_max units (0 = no limit) and the
  -- limit grows by wave_growth_per_hour units per game hour after the grace;
  -- the army stays within army_vs_enemy_pct of the enemy army it has seen
  -- (0 = no limit); economy_pct of the profile's workers are kept (an honest
  -- limit of attention: fewer hands, a smaller economy).
  grace_ticks          INTEGER NOT NULL DEFAULT 0 CHECK (grace_ticks >= 0),
  first_wave_max       INTEGER NOT NULL DEFAULT 0 CHECK (first_wave_max >= 0),
  wave_growth_per_hour INTEGER NOT NULL DEFAULT 0 CHECK (wave_growth_per_hour >= 0),
  army_vs_enemy_pct    INTEGER NOT NULL DEFAULT 0 CHECK (army_vs_enemy_pct >= 0),
  economy_pct          INTEGER NOT NULL DEFAULT 100 CHECK (economy_pct BETWEEN 1 AND 100)
) STRICT, WITHOUT ROWID;

-- Order vocabulary. `needs` names the et_* component the executor must have.
CREATE TABLE order_kind (
  key    TEXT PRIMARY KEY,
  needs  TEXT NOT NULL DEFAULT '',     -- '', 'movement','weapon','builder','harvester','healer','trader','container','producer','energy','gather_site','colony','market'
  target TEXT NOT NULL CHECK (target IN ('none','point','entity','def','entity_or_point')),
  note   TEXT NOT NULL DEFAULT ''
) STRICT, WITHOUT ROWID;

-- ════════════════════════════════════════════════════════════════════════════
-- PART B — STATE (one database file per saved match)
-- ════════════════════════════════════════════════════════════════════════════
--
-- A save file holds PART B only. Definition ids (`*_id` columns that name a
-- definition) are the ids of the rules database named by `match.rules_version`
-- and `match.rules_fingerprint`; the save does not repeat the rules, and a save
-- from other rules is refused. The REFERENCES to PART A tables document that
-- link; the writer connection does not enforce them.
--
-- Conventions:
--   * a position is four columns `<p>_x, <p>_y, <p>_face, <p>_body` (milli-tiles,
--     cube face, body id; NULL together when optional);
--   * unsigned 64-bit simulation values (ticks, the random generator state, ids)
--     are stored as the same 64 bits read as a signed integer;
--   * grids (terrain, territory winners) are run-length encoded blobs of
--     little-endian LEB128 (value, run) pairs; explored bits are raw u64 words;
--   * `entity.move_remainder` is decimal text (a 128-bit integer).
-- The tick loop never runs SQL; `rom-save` writes and reads these tables
-- between ticks.

CREATE TABLE match (
  id               INTEGER PRIMARY KEY CHECK (id = 1),
  format           INTEGER NOT NULL,         -- layout version of this part of the schema
  rules_version    TEXT NOT NULL,            -- schema_meta 'rules_version' of the rules used
  rules_fingerprint TEXT NOT NULL,           -- hex digest of the loaded rules; must match on load
  seed             INTEGER NOT NULL,
  map_style        TEXT NOT NULL DEFAULT '', -- map_style key, for display and a rebuild
  speed_pct        INTEGER NOT NULL DEFAULT 100,  -- game speed: ticks per real second = 20 × speed_pct / 100
  tick             INTEGER NOT NULL,
  next_entity_id   INTEGER NOT NULL,
  next_order_id    INTEGER NOT NULL,
  rng_state        INTEGER NOT NULL,
  transits_done    INTEGER NOT NULL,         -- completed route legs
  ownable_tiles    INTEGER NOT NULL,         -- ground tiles eligible for the territory victory
  viewer_slots     INTEGER NOT NULL,         -- length of the per-entity viewer list
  elimination      TEXT NOT NULL CHECK (elimination IN ('conquest','capital','sudden_death')),
  outcome_team     INTEGER,                  -- the first completed victory, all three or none
  outcome_rule     TEXT,
  outcome_tick     INTEGER,
  stat_trade_trips INTEGER NOT NULL,
  stat_cities_captured INTEGER NOT NULL,
  stat_orders_done INTEGER NOT NULL,
  stat_orders_failed INTEGER NOT NULL,
  stat_orders_cancelled INTEGER NOT NULL,
  stat_orders_no_path INTEGER NOT NULL,
  stat_orders_unsupported INTEGER NOT NULL,
  stat_orders_no_slot INTEGER NOT NULL,
  stat_orders_target_gone INTEGER NOT NULL,
  CHECK ((outcome_team IS NULL) = (outcome_rule IS NULL) AND (outcome_rule IS NULL) = (outcome_tick IS NULL))
) STRICT;

-- The match setup the caller wants back on load (map size, number of players,
-- options chosen on the setup screen); keys are the caller's.
CREATE TABLE match_setting (
  option_key TEXT PRIMARY KEY,
  value      TEXT NOT NULL
) STRICT, WITHOUT ROWID;

CREATE TABLE match_victory (            -- enabled victory rule families
  rule TEXT PRIMARY KEY
) STRICT, WITHOUT ROWID;

-- ── A13. Star catalogue ─────────────────────────────────────────────────────
-- Real stars at their real places (docs/world-model.md "Real sky"). A match takes a connected
-- region of this table as its star systems. Generated by db/content/generate.py from published
-- catalogue values (docs/database.md "Star catalogue"); positions are never invented.
CREATE TABLE constellation (
  id       INTEGER PRIMARY KEY,
  abbr     TEXT NOT NULL UNIQUE,        -- IAU three-letter abbreviation
  name     TEXT NOT NULL UNIQUE,        -- IAU name, Latin
  genitive TEXT NOT NULL                -- used in Bayer designations: "Alpha Lyrae"
) STRICT;

CREATE TABLE star (
  id               INTEGER PRIMARY KEY,
  hip              INTEGER NOT NULL UNIQUE,  -- Hipparcos catalogue number
  name             TEXT NOT NULL UNIQUE,     -- IAU proper name, or the Bayer or catalogue name
  designation      TEXT NOT NULL,            -- Bayer designation (Latin) or catalogue number
  constellation_id INTEGER NOT NULL REFERENCES constellation(id),
  ra_mdeg          INTEGER NOT NULL CHECK (ra_mdeg BETWEEN 0 AND 359999),    -- right ascension, J2000, milli-degrees
  dec_mdeg         INTEGER NOT NULL CHECK (dec_mdeg BETWEEN -90000 AND 90000), -- declination, milli-degrees
  dist_cly         INTEGER NOT NULL CHECK (dist_cly > 0),                    -- distance from the Sun, 1/100 light-year
  x                INTEGER NOT NULL,         -- map position computed from the three values above
  y                INTEGER NOT NULL,         -- (direction exact, distance compressed), abstract units
  z                INTEGER NOT NULL,
  glon_mdeg        INTEGER NOT NULL CHECK (glon_mdeg BETWEEN 0 AND 359999) -- galactic longitude seen from
                                             -- the Sun, milli-degrees, from ra/dec (task 89: quadrants)
) STRICT;
CREATE INDEX star_by_constellation ON star(constellation_id);

-- The usual stick figure of a constellation: lines between member stars, star_a < star_b.
CREATE TABLE constellation_line (
  constellation_id INTEGER NOT NULL REFERENCES constellation(id),
  star_a           INTEGER NOT NULL REFERENCES star(id),
  star_b           INTEGER NOT NULL REFERENCES star(id),
  PRIMARY KEY (star_a, star_b),
  CHECK (star_a < star_b)
) STRICT, WITHOUT ROWID;

-- The universe (world-model.md): star systems on a star map, the bodies in
-- each system, and the routes between systems. Empty for a one-map match.
CREATE TABLE star_system (
  id   INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  x    INTEGER NOT NULL,                -- star map position, abstract units (equatorial frame, the Sun at 0,0,0)
  y    INTEGER NOT NULL,
  z    INTEGER NOT NULL DEFAULT 0,      -- toward the north celestial pole
  star_id INTEGER REFERENCES star(id),  -- the catalogue star this system is
  constellation TEXT NOT NULL DEFAULT '' -- its constellation, for labels
) STRICT;

-- The stick figure of a constellation inside this match: pairs of systems whose catalogue
-- stars the figure joins (both stars are in the match). Display only.
CREATE TABLE star_figure (
  system_a INTEGER NOT NULL REFERENCES star_system(id),
  system_b INTEGER NOT NULL REFERENCES star_system(id),
  PRIMARY KEY (system_a, system_b),
  CHECK (system_a < system_b)
) STRICT, WITHOUT ROWID;

CREATE TABLE body (
  id               INTEGER PRIMARY KEY,
  system_id        INTEGER NOT NULL REFERENCES star_system(id),
  kind             TEXT NOT NULL REFERENCES body_kind(key),
  name             TEXT NOT NULL,
  orbit_radius     INTEGER NOT NULL CHECK (orbit_radius >= 0),
  orbit_angle_mdeg INTEGER NOT NULL CHECK (orbit_angle_mdeg BETWEEN 0 AND 359999),
  n                INTEGER CHECK (n >= 4),                       -- habitable only: cube face size N
  home_race_id     INTEGER REFERENCES faction(id),               -- habitable only
  node_entity_type_id INTEGER REFERENCES et_node(entity_type_id), -- resource only
  CHECK ((kind = 'habitable') = (n IS NOT NULL)),
  CHECK (home_race_id IS NULL OR kind = 'habitable'),
  CHECK ((kind = 'resource') = (node_entity_type_id IS NOT NULL))
) STRICT;
CREATE INDEX body_by_system ON body(system_id);

-- Task 77: the dormant surface each resource body was given before play (a cube of barren
-- ground, no deposits), and the bodies an ascended player brought to life. A living body is a
-- habitable `body` row with a `surface`; its painted heights are kept here.
CREATE TABLE body_dormant (
  body_id    INTEGER PRIMARY KEY REFERENCES body(id),
  n          INTEGER NOT NULL CHECK (n >= 4),
  terrain_id INTEGER NOT NULL REFERENCES terrain(id)
) STRICT;

CREATE TABLE body_life (
  body_id    INTEGER PRIMARY KEY REFERENCES body(id),
  player_id  INTEGER NOT NULL,            -- player.id of the ascended player that brought it
  since_tick INTEGER NOT NULL,
  strokes    INTEGER NOT NULL,            -- paint strokes applied so far
  heights    BLOB NOT NULL                -- one height 0..255 per tile, RLE like surface.terrain
) STRICT;

CREATE TABLE route (
  system_a INTEGER NOT NULL REFERENCES star_system(id),
  system_b INTEGER NOT NULL REFERENCES star_system(id),
  length   INTEGER NOT NULL CHECK (length > 0),
  PRIMARY KEY (system_a, system_b),
  CHECK (system_a < system_b)
) STRICT, WITHOUT ROWID;

-- Every surface that has been generated: a planet's cube sphere, a system's
-- flat plane, or the one flat map of a match without a universe. The grids are
-- run-length encoded; the path-search state of the surface is in the path_* tables.
CREATE TABLE surface (
  body_id   INTEGER PRIMARY KEY,         -- body.id (0 for a one-map match)
  shape     TEXT NOT NULL CHECK (shape IN ('flat','cube')),
  width     INTEGER NOT NULL,            -- terrain storage, tiles
  height    INTEGER NOT NULL,
  n         INTEGER,                     -- cube: tiles along a face edge
  terrain   BLOB NOT NULL,               -- one terrain definition id per tile, RLE
  territory BLOB NOT NULL,               -- winning emitter entity id per tile (0 = none), RLE
  path_next_request INTEGER NOT NULL,
  path_generation   INTEGER NOT NULL,
  path_searches INTEGER NOT NULL,
  path_nodes    INTEGER NOT NULL,
  path_repaths  INTEGER NOT NULL
) STRICT;

CREATE TABLE surface_touched (          -- tiles newly seen this tick, in order
  body_id INTEGER NOT NULL,
  seq     INTEGER NOT NULL,
  tile    INTEGER NOT NULL,
  PRIMARY KEY (body_id, seq)
) STRICT, WITHOUT ROWID;

CREATE TABLE surface_stain (            -- task 80: where buildings and units were destroyed, oldest first
  body_id   INTEGER NOT NULL,
  seq       INTEGER NOT NULL,
  entity_id INTEGER NOT NULL,            -- the id the destroyed entity had
  type_id   INTEGER NOT NULL,            -- entity_type.id
  owner_id  INTEGER NOT NULL,            -- player.id
  x INTEGER NOT NULL, y INTEGER NOT NULL, face INTEGER NOT NULL,
  radius    INTEGER NOT NULL,            -- milli-tiles
  tick      INTEGER NOT NULL,            -- destroyed at; gone world.stain_days x world.day_ticks later
  PRIMARY KEY (body_id, seq)
) STRICT, WITHOUT ROWID;

CREATE TABLE path_request (             -- queued searches, front first
  body_id INTEGER NOT NULL,
  seq     INTEGER NOT NULL,
  id      INTEGER NOT NULL,
  class   INTEGER NOT NULL,              -- movement class id
  start_face INTEGER NOT NULL, start_x INTEGER NOT NULL, start_y INTEGER NOT NULL,
  goal_face  INTEGER NOT NULL, goal_x  INTEGER NOT NULL, goal_y  INTEGER NOT NULL,
  PRIMARY KEY (body_id, seq)
) STRICT, WITHOUT ROWID;

CREATE TABLE path_waiter (              -- walkers waiting for a request
  body_id INTEGER NOT NULL,
  request_seq INTEGER NOT NULL,
  seq     INTEGER NOT NULL,
  entity_id INTEGER NOT NULL,
  order_id  INTEGER NOT NULL,
  PRIMARY KEY (body_id, request_seq, seq)
) STRICT, WITHOUT ROWID;

CREATE TABLE path_search (              -- the search in progress, if any
  body_id INTEGER PRIMARY KEY,
  request INTEGER NOT NULL,
  target  INTEGER NOT NULL,
  anchor_face INTEGER NOT NULL, anchor_x INTEGER NOT NULL, anchor_y INTEGER NOT NULL,
  best_distance INTEGER NOT NULL,
  best_tile INTEGER NOT NULL,
  expanded  INTEGER NOT NULL,
  generation INTEGER NOT NULL
) STRICT;

CREATE TABLE path_open (                -- open list, ascending
  body_id INTEGER NOT NULL,
  seq     INTEGER NOT NULL,
  f       INTEGER NOT NULL,
  g       INTEGER NOT NULL,
  tile    INTEGER NOT NULL,
  PRIMARY KEY (body_id, seq)
) STRICT, WITHOUT ROWID;

CREATE TABLE path_node (                -- visited tiles of the search
  body_id INTEGER NOT NULL,
  tile    INTEGER NOT NULL,
  g       INTEGER NOT NULL,
  parent  INTEGER NOT NULL,
  h       INTEGER NOT NULL,
  closed  INTEGER NOT NULL CHECK (closed IN (0,1)),
  PRIMARY KEY (body_id, tile)
) STRICT, WITHOUT ROWID;

CREATE TABLE player (
  id         INTEGER PRIMARY KEY,        -- 0 = nature / neutral
  slot       INTEGER NOT NULL UNIQUE,
  name       TEXT NOT NULL,
  controller TEXT NOT NULL CHECK (controller IN ('human','ai','remote','nature')),
  ai_profile TEXT NOT NULL DEFAULT '',
  color      INTEGER NOT NULL DEFAULT 0,
  team       INTEGER NOT NULL DEFAULT 0,
  faction_id INTEGER NOT NULL REFERENCES faction(id),
  age_id     INTEGER NOT NULL REFERENCES age(id),
  home_body_id INTEGER,                  -- home planet; NULL = none recorded
  capital_entity_id INTEGER,             -- first founded city
  status     TEXT NOT NULL DEFAULT 'playing' CHECK (status IN ('playing','defeated','won','ascended')),
  land_owned INTEGER,                    -- land tiles owned (territory victory)
  owned_tiles INTEGER,                   -- tiles owned over all surfaces
  hold_team  INTEGER,                    -- enemy team occupying the capital, and since when
  hold_start INTEGER,
  kills      INTEGER,                    -- enemy kills credited / combat deaths (match statistics)
  losses     INTEGER,
  -- Task 76: the faction the player started with (restored on descent), the tick of its
  -- current ascension (NULL while it is material) and how many times it has ascended.
  origin_faction_id INTEGER REFERENCES faction(id),
  ascended_tick INTEGER,
  ascensions INTEGER NOT NULL DEFAULT 0 CHECK (ascensions >= 0),
  CHECK ((hold_team IS NULL) = (hold_start IS NULL)),
  CHECK ((status = 'ascended') = (ascended_tick IS NOT NULL))
) STRICT;

-- A computer player's memory (plans, known enemy positions, army groups),
-- serialised by the computer-player crate; the save stores the bytes as given.
CREATE TABLE player_ai_state (
  player_id  INTEGER PRIMARY KEY REFERENCES player(id),
  profile_key TEXT NOT NULL,             -- ai_profile.key
  difficulty_key TEXT NOT NULL,          -- ai_difficulty.key
  version    INTEGER NOT NULL,           -- format version of data, the AI crate's own
  rng_state  INTEGER NOT NULL,           -- the computer player's own random generator
  opening_seq INTEGER NOT NULL,          -- index of the current opening-book step
  data       BLOB NOT NULL
) STRICT;

CREATE TABLE player_resource (
  player_id   INTEGER NOT NULL REFERENCES player(id),
  resource_id INTEGER NOT NULL REFERENCES resource(id),
  amount      INTEGER NOT NULL,
  PRIMARY KEY (player_id, resource_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE player_tech (
  player_id INTEGER NOT NULL REFERENCES player(id),
  tech_id   INTEGER NOT NULL REFERENCES tech(id),
  times     INTEGER NOT NULL,
  PRIMARY KEY (player_id, tech_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE income_remainder (         -- fractional income carried between ticks
  player_id   INTEGER NOT NULL REFERENCES player(id),
  resource_id INTEGER NOT NULL REFERENCES resource(id),
  remainder   INTEGER NOT NULL,
  PRIMARY KEY (player_id, resource_id)
) STRICT, WITHOUT ROWID;

-- Market prices away from rest (task 107): `pressure` is in hundredths of a lot, positive
-- after buying, negative after selling; it drifts back to 0. Rows only where it is not 0.
CREATE TABLE market_pressure (
  player_id   INTEGER NOT NULL REFERENCES player(id),
  resource_id INTEGER NOT NULL REFERENCES resource(id),
  pressure    INTEGER NOT NULL,
  PRIMARY KEY (player_id, resource_id)
) STRICT, WITHOUT ROWID;

-- How a match ends (task 117; docs/ascension.md "How a match ends"). The players people play,
-- from the setup: the match ends when every one of them is defeated, or when only they are left.
CREATE TABLE match_human (
  player_id INTEGER PRIMARY KEY REFERENCES player(id)
) STRICT;

-- The end of the match, once reached; at most one row.
CREATE TABLE match_end (
  id     INTEGER PRIMARY KEY CHECK (id = 1),
  tick   INTEGER NOT NULL,
  reason TEXT NOT NULL CHECK (reason IN ('no_human_left','last_standing')),
  team   INTEGER,                        -- the remaining team of 'last_standing'
  CHECK ((reason = 'last_standing') = (team IS NOT NULL))
) STRICT;

-- What each player did, for the match report (task 117): counters updated where the event
-- happens. Kills, losses and ascensions are in `player`.
CREATE TABLE player_stat (
  player_id           INTEGER PRIMARY KEY REFERENCES player(id),
  spent               INTEGER NOT NULL,  -- resources paid for orders when given
  units_trained       INTEGER NOT NULL,
  buildings_completed INTEGER NOT NULL,
  cities_founded      INTEGER NOT NULL,
  cities_captured     INTEGER NOT NULL,
  cities_lost         INTEGER NOT NULL,
  techs_researched    INTEGER NOT NULL,
  peak_land           INTEGER NOT NULL,  -- most tiles owned at the end of a tick
  peak_army           INTEGER NOT NULL,  -- most fighters at the end of a tick
  defeated_tick       INTEGER,           -- when the player was defeated
  defeated_by         INTEGER,           -- the player that took its capital, if any
  CHECK (defeated_by IS NULL OR defeated_tick IS NOT NULL)
) STRICT;

CREATE TABLE player_gathered (          -- resources credited by income and trade
  player_id   INTEGER NOT NULL REFERENCES player(id),
  resource_id INTEGER NOT NULL REFERENCES resource(id),
  amount      INTEGER NOT NULL,
  PRIMARY KEY (player_id, resource_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE player_age_reached (       -- the first tick each age was held
  player_id INTEGER NOT NULL REFERENCES player(id),
  age_id    INTEGER NOT NULL REFERENCES age(id),
  tick      INTEGER NOT NULL,
  PRIMARY KEY (player_id, age_id)
) STRICT, WITHOUT ROWID;

-- One sample per player every `stats.history_period_ticks` (the report's charts).
CREATE TABLE player_history (
  tick      INTEGER NOT NULL,
  player_id INTEGER NOT NULL REFERENCES player(id),
  score     INTEGER NOT NULL,
  land      INTEGER NOT NULL,            -- tiles owned
  army      INTEGER NOT NULL,            -- fighters
  cities    INTEGER NOT NULL,
  stock     INTEGER NOT NULL,            -- sum of the resource stores
  age_id    INTEGER NOT NULL REFERENCES age(id),
  PRIMARY KEY (tick, player_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE victory_timer (            -- territory (and similar) timers per team
  victory_id INTEGER NOT NULL REFERENCES victory(id),
  team       INTEGER NOT NULL,
  end_tick   INTEGER NOT NULL,
  PRIMARY KEY (victory_id, team)
) STRICT, WITHOUT ROWID;

-- Fog of war per surface and player: explored bits (one per tile, 64 per word,
-- little-endian), remembered buildings, and the building index by tile.
CREATE TABLE player_vision (
  body_id   INTEGER NOT NULL,
  player_id INTEGER NOT NULL,
  explored  BLOB NOT NULL,
  PRIMARY KEY (body_id, player_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE vision_memory (
  body_id   INTEGER NOT NULL,
  player_id INTEGER NOT NULL,
  tile      INTEGER NOT NULL,            -- position tile of the building
  entity_id INTEGER NOT NULL,
  entity_type_id INTEGER NOT NULL,
  owner_id  INTEGER NOT NULL,
  pos_x INTEGER NOT NULL, pos_y INTEGER NOT NULL, pos_face INTEGER NOT NULL, pos_body INTEGER NOT NULL,
  PRIMARY KEY (body_id, player_id, tile, entity_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE vision_building (
  body_id   INTEGER NOT NULL,
  tile      INTEGER NOT NULL,
  entity_id INTEGER NOT NULL,
  PRIMARY KEY (body_id, tile, entity_id)
) STRICT, WITHOUT ROWID;

-- What each entity currently reveals; the per-tile viewer counts are rebuilt from these.
CREATE TABLE vision_viewer (
  entity_id INTEGER NOT NULL,
  orbit     INTEGER NOT NULL CHECK (orbit IN (0,1)),  -- 1: the footprint of an orbiting ship below it
  body_id   INTEGER NOT NULL,
  tile      INTEGER NOT NULL,
  los       INTEGER NOT NULL,
  owner_id  INTEGER NOT NULL,
  PRIMARY KEY (entity_id, orbit)
) STRICT, WITHOUT ROWID;

CREATE TABLE entity (
  id             INTEGER PRIMARY KEY,
  location       TEXT NOT NULL DEFAULT 'map' CHECK (location IN ('map','transit')),
  entity_type_id INTEGER NOT NULL REFERENCES entity_type(id),
  owner_id       INTEGER NOT NULL REFERENCES player(id),
  body_id        INTEGER NOT NULL,       -- surface or system-plane position
  face           INTEGER NOT NULL CHECK (face BETWEEN 0 AND 5),
  x              INTEGER NOT NULL,       -- milli-tiles
  y              INTEGER NOT NULL,
  facing         INTEGER NOT NULL,
  hp             INTEGER NOT NULL,
  max_hp         INTEGER NOT NULL,
  los            INTEGER NOT NULL,
  stance_id      INTEGER NOT NULL REFERENCES stance(id),
  energy         INTEGER NOT NULL,
  energy_remainder INTEGER NOT NULL,
  heal_remainder INTEGER NOT NULL,
  attrition_remainder INTEGER NOT NULL,
  neutralised    INTEGER NOT NULL CHECK (neutralised IN (0,1)),
  assimilation_until INTEGER,            -- tick a captured building finishes assimilation
  killer_id      INTEGER,                -- player credited with the kill
  build_progress INTEGER NOT NULL,       -- ticks done; -1 = complete
  move_remainder TEXT NOT NULL,
  gather_slots   INTEGER NOT NULL,
  work_site_id   INTEGER,                -- gather site this worker occupies
  container_id   INTEGER,                -- carried or kept inside
  city_id        INTEGER,                -- city whose economy this building belongs to
  idle_ticks     INTEGER NOT NULL,
  aim_id         INTEGER                 -- enemy a defence keeps firing at (format 4)
) STRICT;
CREATE INDEX entity_owner ON entity(owner_id, entity_type_id);
CREATE INDEX entity_body  ON entity(body_id);

-- A ship in transit between systems (location 'transit'); it stands on no surface.
CREATE TABLE entity_transit (
  entity_id   INTEGER PRIMARY KEY REFERENCES entity(id),
  origin      INTEGER NOT NULL,          -- system ids
  destination INTEGER NOT NULL,
  departed    INTEGER NOT NULL,
  arrives     INTEGER NOT NULL
) STRICT;

-- Routes a player has charted or had revealed (task 70): a voyage along a route the
-- player does not know is slowed by `space.charting_pct` and charts it on arrival.
CREATE TABLE player_route (
  player_id INTEGER NOT NULL REFERENCES player(id),
  system_a  INTEGER NOT NULL,            -- system ids, system_a < system_b
  system_b  INTEGER NOT NULL,
  PRIMARY KEY (player_id, system_a, system_b)
) STRICT, WITHOUT ROWID;

-- The system a ship last arrived from (task 70): under an enemy jump inhibitor it may
-- depart only back along that route.
CREATE TABLE entity_arrival (
  entity_id INTEGER PRIMARY KEY REFERENCES entity(id),
  system_id INTEGER NOT NULL
) STRICT;

-- A ship in orbit retains its system-plane position in entity; this row records
-- its body and moving surface footprint. Shots retain the last fire-time target.
CREATE TABLE entity_orbit (
  entity_id INTEGER PRIMARY KEY REFERENCES entity(id),
  body_id INTEGER NOT NULL REFERENCES body(id),
  face INTEGER NOT NULL CHECK (face BETWEEN 0 AND 5),
  x INTEGER NOT NULL,
  y INTEGER NOT NULL,
  facing INTEGER NOT NULL CHECK (facing BETWEEN 0 AND 359), -- degrees (format 4: any heading)
  move_remainder INTEGER NOT NULL DEFAULT 0,
  last_shot_tick INTEGER,
  last_shot_face INTEGER CHECK (last_shot_face BETWEEN 0 AND 5),
  last_shot_x INTEGER,
  last_shot_y INTEGER,
  CHECK ((last_shot_tick IS NULL AND last_shot_face IS NULL AND last_shot_x IS NULL AND last_shot_y IS NULL)
      OR (last_shot_tick IS NOT NULL AND last_shot_face IS NOT NULL AND last_shot_x IS NOT NULL AND last_shot_y IS NOT NULL))
) STRICT;

CREATE TABLE entity_cooldown (          -- earliest tick each weapon can fire again
  entity_id INTEGER NOT NULL REFERENCES entity(id),
  weapon_id INTEGER NOT NULL,            -- weapon.id of the entity's type
  ready_tick INTEGER NOT NULL,
  PRIMARY KEY (entity_id, weapon_id)
) STRICT, WITHOUT ROWID;

-- A producer's rally point (order `set_rally`): units it trains walk to the
-- point, or to the entity (a harvester to a gather site it can work harvests there).
CREATE TABLE entity_rally (
  entity_id INTEGER PRIMARY KEY REFERENCES entity(id),
  target_entity_id INTEGER,
  x INTEGER, y INTEGER, face INTEGER, body INTEGER,
  CHECK ((target_entity_id IS NULL) = (x IS NOT NULL AND y IS NOT NULL AND face IS NOT NULL AND body IS NOT NULL))
) STRICT;

-- Membership of the simulation's entity sets: 'landed' (ship standing on a
-- planet), 'neutralised' and 'assimilating' (captured buildings), 'idle' and
-- 'dirty' (economy), 'awake' (units to check for separation next tick),
-- 'governor_paused' (an automated colony whose governor its owner paused; task 73).
CREATE TABLE entity_flag (
  entity_id INTEGER NOT NULL,
  flag      TEXT NOT NULL CHECK (flag IN ('landed','neutralised','assimilating','idle','dirty','awake','governor_paused')),
  PRIMARY KEY (entity_id, flag)
) STRICT, WITHOUT ROWID;

-- An aircraft that is not fully in the air (task 123, format 15): ticks it has climbed toward
-- its flight height, 0 = landed, up to its type's `et_movement.takeoff_ticks` (then it is
-- airborne and has no row). Taking off counts up, landing counts down.
CREATE TABLE entity_flight (
  entity_id INTEGER PRIMARY KEY,
  climbed   INTEGER NOT NULL CHECK (climbed >= 0)
) STRICT;

-- A struck-down city held by an attacking team (task 67): the team and the tick
-- its hold began; the city falls to it after `city.capture_hold_ticks`.
CREATE TABLE city_siege (
  city_id INTEGER PRIMARY KEY REFERENCES entity(id),
  team    INTEGER NOT NULL,
  since   INTEGER NOT NULL
) STRICT;

-- Income contributed by an entity: its terms sum to the per-player rates.
CREATE TABLE income_term (
  entity_id   INTEGER NOT NULL,
  seq         INTEGER NOT NULL,
  player_id   INTEGER NOT NULL,
  resource_id INTEGER NOT NULL,
  amount      INTEGER NOT NULL,
  PRIMARY KEY (entity_id, seq)
) STRICT, WITHOUT ROWID;

-- THE ORDER BOOK. Everything a player (or AI) asks for is an order: a unit's
-- command queue and a building's production queue are the same table. Orders in
-- `place` 'queue' belong to an executor's queue (seq 0 = active); orders in
-- 'pending' were submitted and wait for the next tick boundary.
CREATE TABLE "order" (
  ord           INTEGER PRIMARY KEY,     -- row number within this save; children refer to it
  place         TEXT NOT NULL CHECK (place IN ('queue','pending')),
  seq           INTEGER NOT NULL,        -- position in the queue
  append        INTEGER NOT NULL DEFAULT 0 CHECK (append IN (0,1)),  -- pending: queue behind existing
  id            INTEGER NOT NULL,        -- order id
  player_id     INTEGER NOT NULL,
  executor_id   INTEGER NOT NULL,
  kind          TEXT NOT NULL REFERENCES order_kind(key),
  status        TEXT NOT NULL CHECK (status IN ('queued','active','blocked','done','cancelled','failed')),
  reason        TEXT NOT NULL DEFAULT '',  -- 'blocked' or 'failed' reason
  subject_def_id INTEGER,
  target_entity_id INTEGER,
  target_x INTEGER, target_y INTEGER, target_face INTEGER, target_body INTEGER,
  home_x INTEGER, home_y INTEGER, home_face INTEGER, home_body INTEGER,
  progress_ticks INTEGER NOT NULL,
  total_ticks   INTEGER,
  issued_tick   INTEGER NOT NULL,
  build_remainder INTEGER NOT NULL,
  automatic     INTEGER NOT NULL CHECK (automatic IN (0,1)),
  -- a trade route in progress (all NULL otherwise)
  route_trader INTEGER, route_home INTEGER, route_target INTEGER,
  route_length INTEGER, route_income INTEGER, route_resource INTEGER,
  route_to_target INTEGER, route_started INTEGER,
  -- walking state (all NULL otherwise)
  move_origin_x INTEGER, move_origin_y INTEGER, move_origin_face INTEGER, move_origin_body INTEGER,
  move_goal_x INTEGER, move_goal_y INTEGER, move_goal_face INTEGER, move_goal_body INTEGER,
  move_distance INTEGER, move_traveled INTEGER, move_index INTEGER,
  move_request INTEGER, move_repaths INTEGER,
  pace          INTEGER,                 -- formation pace: walking speed cap (milli-tiles/s)
  engaged_id    INTEGER,                 -- guard: the enemy it is fighting off
  UNIQUE (place, executor_id, seq)
) STRICT;
CREATE INDEX order_by_player ON "order"(player_id, status);

CREATE TABLE order_payment (            -- what was actually charged (for refunds)
  ord         INTEGER NOT NULL REFERENCES "order"(ord),
  seq         INTEGER NOT NULL,
  resource_id INTEGER NOT NULL,
  amount      INTEGER NOT NULL,
  PRIMARY KEY (ord, seq)
) STRICT, WITHOUT ROWID;

-- Each player's latest failures of accepted orders (no path, no slot, target gone, ...),
-- oldest first: what World::order_failures reports to the player and its computer brain.
CREATE TABLE order_failure (
  player_id   INTEGER NOT NULL,
  seq         INTEGER NOT NULL,
  tick        INTEGER NOT NULL,
  order_id    INTEGER NOT NULL,
  executor_id INTEGER NOT NULL,
  kind        TEXT NOT NULL,
  subject_def_id INTEGER,
  target_entity_id INTEGER,
  target_x INTEGER, target_y INTEGER, target_face INTEGER, target_body INTEGER,
  reason      TEXT NOT NULL,
  PRIMARY KEY (player_id, seq)
) STRICT, WITHOUT ROWID;

CREATE TABLE order_waypoint (           -- intermediate points of a smoothed path
  ord  INTEGER NOT NULL REFERENCES "order"(ord),
  seq  INTEGER NOT NULL,
  x INTEGER NOT NULL, y INTEGER NOT NULL, face INTEGER NOT NULL, body INTEGER NOT NULL,
  PRIMARY KEY (ord, seq)
) STRICT, WITHOUT ROWID;

-- Weapon hits in flight, and firing decisions not yet rolled.
CREATE TABLE pending_hit (
  seq INTEGER PRIMARY KEY,
  land_tick INTEGER NOT NULL,
  attacker_id INTEGER NOT NULL,
  owner_id INTEGER NOT NULL,
  source_type_id INTEGER NOT NULL,
  weapon_id INTEGER NOT NULL,
  target_id INTEGER NOT NULL,
  damage INTEGER NOT NULL
) STRICT;

CREATE TABLE pending_shot (
  seq INTEGER PRIMARY KEY,
  origin_x INTEGER NOT NULL, origin_y INTEGER NOT NULL, origin_face INTEGER NOT NULL, origin_body INTEGER NOT NULL,
  target_x INTEGER NOT NULL, target_y INTEGER NOT NULL, target_face INTEGER NOT NULL, target_body INTEGER NOT NULL,
  accuracy INTEGER NOT NULL,
  land_tick INTEGER NOT NULL,
  attacker_id INTEGER NOT NULL,
  owner_id INTEGER NOT NULL,
  source_type_id INTEGER NOT NULL,
  weapon_id INTEGER NOT NULL,
  target_id INTEGER NOT NULL,
  damage INTEGER NOT NULL
) STRICT;

-- System space: where each body sits on its system's plane, the neutral marker
-- entity standing there, and who holds a resource body.
CREATE TABLE body_position (
  body_id INTEGER PRIMARY KEY,
  x INTEGER NOT NULL, y INTEGER NOT NULL, face INTEGER NOT NULL, plane_body INTEGER NOT NULL
) STRICT;

CREATE TABLE body_marker (
  body_id   INTEGER PRIMARY KEY,
  entity_id INTEGER NOT NULL
) STRICT;

CREATE TABLE body_claim (
  body_id   INTEGER PRIMARY KEY,
  player_id INTEGER NOT NULL
) STRICT;

-- Borders and cities.
CREATE TABLE emitter (                  -- completed entities pushing borders
  entity_id INTEGER PRIMARY KEY,
  x INTEGER NOT NULL, y INTEGER NOT NULL, face INTEGER NOT NULL, body_id INTEGER NOT NULL,
  radius    INTEGER NOT NULL,            -- milli-tiles
  owner_id  INTEGER NOT NULL,
  completed INTEGER NOT NULL             -- tick; earlier wins ties
) STRICT;

CREATE TABLE city (                     -- city entities, complete or sites
  entity_id INTEGER PRIMARY KEY,
  economic_radius INTEGER                -- milli-tiles; NULL when not computed
) STRICT;

CREATE TABLE city_widener (             -- completed non-city buildings that widen cities
  entity_id INTEGER PRIMARY KEY,
  owner_id  INTEGER NOT NULL,
  x INTEGER NOT NULL, y INTEGER NOT NULL, face INTEGER NOT NULL, body_id INTEGER NOT NULL
) STRICT;

-- ════════════════════════════════════════════════════════════════════════════
-- VIEWS — data lint and convenience
-- ════════════════════════════════════════════════════════════════════════════

-- Must return zero rows: every def needs its subtype row.
CREATE VIEW lint_def_without_subtype AS
SELECT d.id, d.kind, d.key FROM def d WHERE
     (d.kind='age'         AND d.id NOT IN (SELECT id FROM age))
  OR (d.kind='faction'     AND d.id NOT IN (SELECT id FROM faction))
  OR (d.kind='resource'    AND d.id NOT IN (SELECT id FROM resource))
  OR (d.kind='terrain'     AND d.id NOT IN (SELECT id FROM terrain))
  OR (d.kind='entity_type' AND d.id NOT IN (SELECT id FROM entity_type))
  OR (d.kind='tech'        AND d.id NOT IN (SELECT id FROM tech))
  OR (d.kind='ability'     AND d.id NOT IN (SELECT id FROM ability))
  OR (d.kind='formation'   AND d.id NOT IN (SELECT id FROM formation))
  OR (d.kind='stance'      AND d.id NOT IN (SELECT id FROM stance))
  OR (d.kind='material'    AND d.id NOT IN (SELECT id FROM material))
  OR (d.kind='mesh'        AND d.id NOT IN (SELECT id FROM mesh))
  OR (d.kind='map_style'   AND d.id NOT IN (SELECT id FROM map_style))
  OR (d.kind='victory'     AND d.id NOT IN (SELECT id FROM victory))
  OR (d.kind='brush'       AND d.id NOT IN (SELECT id FROM life_brush));

-- Must return zero rows: anything trainable/constructible needs a producer.
CREATE VIEW lint_unproducible AS
SELECT d.key FROM entity_type e JOIN def d ON d.id = e.id
WHERE e.class IN ('unit','building')
  AND e.id NOT IN (SELECT product_def_id FROM production)
  AND e.id NOT IN (SELECT target_def_id FROM tech_effect WHERE effect IN ('upgrade_type','spawn') AND target_def_id IS NOT NULL)
  AND e.id NOT IN (SELECT effect_def_id FROM ability WHERE effect_def_id IS NOT NULL);

-- Command-panel contents per producer.
CREATE VIEW v_production AS
SELECT pd.key AS producer, p.mode, p.ui_page, p.ui_col, p.ui_row, xd.key AS product, xd.name AS product_name
FROM production p
JOIN def pd ON pd.id = p.producer_type_id
JOIN def xd ON xd.id = p.product_def_id
ORDER BY pd.key, p.ui_page, p.ui_row, p.ui_col;

CREATE VIEW v_cost AS
SELECT d.key AS item, rd.key AS resource, c.amount, c.ramp
FROM cost c JOIN def d ON d.id = c.def_id JOIN def rd ON rd.id = c.resource_id;
