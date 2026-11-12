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
          'formation','stance','material','mesh','map_style','victory')),
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

-- ── A2. World model: layers, tiers, terrain ─────────────────────────────────
-- ONE map holds everything from the first tick: planetary terrains (land and
-- sea, where cities stand and ground battles happen) set in open space, with
-- small resource planets scattered between them.
-- A layer is a movement medium. Sea and space follow the same rules (ports,
-- vessels, transports, trade lanes) but are different layers, because both
-- exist on the same map at once.
CREATE TABLE layer (
  key  TEXT PRIMARY KEY,               -- ground | fluid | subfluid | air | orbit | space
  note TEXT NOT NULL DEFAULT ''
) STRICT, WITHOUT ROWID;

-- A tier is how far out a player can see and act. Each age belongs to a tier;
-- a player's view and reach widen when THAT player reaches the tier — players
-- get there one by one, and the first to arrive finds space unclaimed.
CREATE TABLE map_scale (
  key   TEXT PRIMARY KEY,              -- planetary | interstellar
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
  map_scale TEXT NOT NULL REFERENCES map_scale(key)   -- tier this age gives its owner
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

-- Identity ladder: nation → continental bloc (forced, after the contemporary
-- ages) → united humanity (forced, on reaching space). `forced = 0` rows would
-- express an optional change of identity (not used at present). A faction is an identity and a
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
  tradeable  INTEGER NOT NULL DEFAULT 1,    -- can be bought/sold at a market
  market_base_price INTEGER NOT NULL DEFAULT 100,
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
  pack_ticks        INTEGER NOT NULL DEFAULT 0
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
  UNIQUE (entity_type_id, slot)
) STRICT;

CREATE TABLE et_weapon_target (        -- which layers a weapon can hit, and how well
  weapon_id INTEGER NOT NULL REFERENCES et_weapon(id) ON DELETE CASCADE,
  layer     TEXT NOT NULL REFERENCES layer(key),
  hit_pct   INTEGER NOT NULL DEFAULT 100,
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
  claim_mode     TEXT NOT NULL DEFAULT 'none' CHECK (claim_mode IN ('none','trader','presence')),
                                       -- trader: a merchant sits on it (rare resources)
                                       -- presence: owned by whoever keeps a unit with claim_tag in range (resource planets)
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
  route_income   INTEGER NOT NULL DEFAULT 0   -- per income period, scaled by route length
) STRICT;

CREATE TABLE et_container (            -- garrison, transport, carrier, hangar
  entity_type_id INTEGER PRIMARY KEY REFERENCES entity_type(id),
  capacity       INTEGER NOT NULL,     -- in pop_cost units
  accept_tag_id  INTEGER REFERENCES tag(id),  -- NULL = anything
  heals          INTEGER NOT NULL DEFAULT 0,
  adds_firepower INTEGER NOT NULL DEFAULT 0,
  launches       INTEGER NOT NULL DEFAULT 0   -- aircraft/drones sortie from here and return
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
  kind    TEXT NOT NULL CHECK (kind IN ('age','track','upgrade','unit_upgrade','government','project','evolution')),
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
              'change_faction','spawn','enable_rule')),
  target_def_id INTEGER REFERENCES def(id),  -- age / unlocked def / new type / ability / faction
  from_def_id   INTEGER REFERENCES def(id),  -- upgrade_type: the old type
  rule_key  TEXT NOT NULL DEFAULT '',        -- enable_rule: 'market_trading','sea_transport','ally_los' …
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

CREATE TABLE entity_visual (           -- most specific row wins (faction, then age)
  id             INTEGER PRIMARY KEY,
  entity_type_id INTEGER NOT NULL REFERENCES entity_type(id),
  faction_id     INTEGER REFERENCES faction(id),
  age_id         INTEGER REFERENCES age(id),
  mesh_id        INTEGER NOT NULL REFERENCES mesh(id),
  material_id    INTEGER REFERENCES material(id),
  scale_pct      INTEGER NOT NULL DEFAULT 100
) STRICT;
CREATE INDEX entity_visual_by_type ON entity_visual(entity_type_id);

-- ── A12. Map styles and match options ───────────────────────────────────────
CREATE TABLE map_style (
  id        INTEGER PRIMARY KEY REFERENCES def(id),
  planets   INTEGER NOT NULL DEFAULT 1,  -- planetary terrains on the map (≥ 1 home terrain)
  fluid_pct INTEGER NOT NULL DEFAULT 30, -- sea share inside a planetary terrain
  space_pct INTEGER NOT NULL DEFAULT 60, -- share of the whole map that is open space
  params    TEXT NOT NULL DEFAULT '{}'  -- JSON: generator knobs
) STRICT;

-- What the generator scatters: forests, mountains, deposits, rare resources, ruins.
CREATE TABLE map_style_feature (
  id           INTEGER PRIMARY KEY,
  map_style_id INTEGER NOT NULL REFERENCES map_style(id),
  feature_def_id INTEGER NOT NULL REFERENCES def(id),  -- terrain or node/pickup entity_type
  region_kind  TEXT NOT NULL DEFAULT 'planet' CHECK (region_kind IN ('planet','space')),
  pattern      TEXT NOT NULL CHECK (pattern IN ('per_player','scattered','centre','edge','contested')),
  chance_pct   INTEGER NOT NULL DEFAULT 100,
  clumps_min   INTEGER NOT NULL DEFAULT 1,
  clumps_max   INTEGER NOT NULL DEFAULT 1,
  size_min     INTEGER NOT NULL DEFAULT 1,
  size_max     INTEGER NOT NULL DEFAULT 1,
  start_min_dist INTEGER NOT NULL DEFAULT 0,  -- tiles from a player start
  start_max_dist INTEGER NOT NULL DEFAULT 0
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
  income_bonus_pct     INTEGER NOT NULL DEFAULT 0, -- applied as an ordinary modifier
  sees_all             INTEGER NOT NULL DEFAULT 0  -- ignores fog of war
) STRICT, WITHOUT ROWID;

-- Order vocabulary. `needs` names the et_* component the executor must have.
CREATE TABLE order_kind (
  key    TEXT PRIMARY KEY,
  needs  TEXT NOT NULL DEFAULT '',     -- '', 'movement','weapon','builder','harvester','healer','trader','container','producer','energy'
  target TEXT NOT NULL CHECK (target IN ('none','point','entity','def','entity_or_point')),
  note   TEXT NOT NULL DEFAULT ''
) STRICT, WITHOUT ROWID;

-- ════════════════════════════════════════════════════════════════════════════
-- PART B — STATE (one database file per saved match)
-- ════════════════════════════════════════════════════════════════════════════

CREATE TABLE match (
  id            INTEGER PRIMARY KEY CHECK (id = 1),
  rules_version TEXT NOT NULL,
  seed          INTEGER NOT NULL,
  map_style_id  INTEGER NOT NULL REFERENCES map_style(id),
  width         INTEGER NOT NULL,      -- tiles, whole map including space
  height        INTEGER NOT NULL,
  speed_pct     INTEGER NOT NULL DEFAULT 100,  -- game speed: sim ticks run per real second = 20 × speed_pct / 100
  tick          INTEGER NOT NULL DEFAULT 0,
  next_entity_id INTEGER NOT NULL DEFAULT 1,
  next_order_id  INTEGER NOT NULL DEFAULT 1
) STRICT;

CREATE TABLE match_setting (
  option_key TEXT PRIMARY KEY REFERENCES match_option(key),
  value      TEXT NOT NULL
) STRICT, WITHOUT ROWID;

-- The whole map — planets and space — is generated before the first tick.
CREATE TABLE map_grid (
  grid TEXT PRIMARY KEY CHECK (grid IN ('terrain','height','territory','variant','region')),
  cell_bytes INTEGER NOT NULL,         -- 1 or 2
  data BLOB NOT NULL                   -- width*height*cell_bytes, row-major
) STRICT, WITHOUT ROWID;

-- Named areas of the map. The 'region' grid stores a region id per tile.
-- Borders, city limits and "first colony" rules are evaluated per region.
CREATE TABLE map_region (
  id        INTEGER PRIMARY KEY,
  kind      TEXT NOT NULL CHECK (kind IN ('planet','space')),
  tier      TEXT NOT NULL REFERENCES map_scale(key),  -- tier a player needs to see / enter it
  name      TEXT NOT NULL,
  x INTEGER NOT NULL, y INTEGER NOT NULL, w INTEGER NOT NULL, h INTEGER NOT NULL,  -- bounding box, tiles
  home      INTEGER NOT NULL DEFAULT 0  -- 1 = starting terrain shared by the players
) STRICT;

CREATE TABLE player (
  id         INTEGER PRIMARY KEY,      -- 0 = nature / neutral
  slot       INTEGER NOT NULL UNIQUE,
  name       TEXT NOT NULL,
  controller TEXT NOT NULL CHECK (controller IN ('human','ai','remote','nature')),
  ai_profile TEXT NOT NULL DEFAULT '',
  team       INTEGER NOT NULL DEFAULT 0,
  color      INTEGER NOT NULL DEFAULT 0,
  faction_id INTEGER NOT NULL REFERENCES faction(id),
  age_id     INTEGER NOT NULL REFERENCES age(id),
  capital_entity_id INTEGER,
  status     TEXT NOT NULL DEFAULT 'playing' CHECK (status IN ('playing','defeated','won','resigned'))
) STRICT;

CREATE TABLE player_explored (
  player_id INTEGER PRIMARY KEY REFERENCES player(id),
  data      BLOB NOT NULL              -- 1 bit per tile
) STRICT;

-- A computer player's memory (plans, known enemy positions, army groups),
-- serialised so a loaded game continues exactly as it would have.
CREATE TABLE player_ai_state (
  player_id  INTEGER PRIMARY KEY REFERENCES player(id),
  profile_key TEXT NOT NULL REFERENCES ai_profile(key),
  difficulty_key TEXT NOT NULL REFERENCES ai_difficulty(key),
  rng_state  INTEGER NOT NULL,
  opening_seq INTEGER NOT NULL DEFAULT 0,
  data       BLOB NOT NULL             -- versioned binary snapshot
) STRICT;

CREATE TABLE player_resource (
  player_id   INTEGER NOT NULL REFERENCES player(id),
  resource_id INTEGER NOT NULL REFERENCES resource(id),
  amount      INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (player_id, resource_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE player_tech (
  player_id INTEGER NOT NULL REFERENCES player(id),
  tech_id   INTEGER NOT NULL REFERENCES tech(id),
  times     INTEGER NOT NULL DEFAULT 1,
  done_tick INTEGER NOT NULL,
  PRIMARY KEY (player_id, tech_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE diplomacy (               -- a < b
  player_a INTEGER NOT NULL REFERENCES player(id),
  player_b INTEGER NOT NULL REFERENCES player(id),
  relation TEXT NOT NULL CHECK (relation IN ('war','peace','alliance')),
  met      INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (player_a, player_b),
  CHECK (player_a < player_b)
) STRICT, WITHOUT ROWID;

CREATE TABLE market_price (
  resource_id INTEGER PRIMARY KEY REFERENCES resource(id),
  price       INTEGER NOT NULL
) STRICT;

CREATE TABLE unit_group (              -- control groups and formation groups
  id           INTEGER PRIMARY KEY,
  player_id    INTEGER NOT NULL REFERENCES player(id),
  hotkey       INTEGER,                -- 0-9, NULL = transient formation group
  formation_id INTEGER REFERENCES formation(id),
  facing       INTEGER NOT NULL DEFAULT 0     -- degrees
) STRICT;

CREATE TABLE entity (
  id             INTEGER PRIMARY KEY,
  entity_type_id INTEGER NOT NULL REFERENCES entity_type(id),
  owner_id       INTEGER NOT NULL REFERENCES player(id),
  region_id      INTEGER REFERENCES map_region(id),
  x              INTEGER NOT NULL,     -- milli-tiles
  y              INTEGER NOT NULL,
  z              INTEGER NOT NULL DEFAULT 0,
  facing         INTEGER NOT NULL DEFAULT 0,
  layer          TEXT NOT NULL REFERENCES layer(key),
  hp             INTEGER NOT NULL,
  energy         INTEGER NOT NULL DEFAULT 0,
  build_progress INTEGER NOT NULL DEFAULT -1,  -- ticks done; -1 = complete
  state          TEXT NOT NULL DEFAULT 'idle',
  stance_id      INTEGER REFERENCES stance(id),
  group_id       INTEGER REFERENCES unit_group(id),
  container_id   INTEGER REFERENCES entity(id),  -- garrisoned / carried inside
  city_id        INTEGER REFERENCES entity(id),  -- city whose economy this building belongs to
  work_site_id   INTEGER REFERENCES entity(id),  -- gather site this worker occupies
  rally_x        INTEGER,
  rally_y        INTEGER,
  rally_entity_id INTEGER REFERENCES entity(id),
  node_amount    INTEGER,              -- nodes: what is left
  claimed_by_id  INTEGER REFERENCES entity(id), -- nodes: trader sitting on it
  claim_owner_id INTEGER REFERENCES player(id), -- nodes: current claimant (presence or trader)
  name           TEXT,                 -- cities, heroes
  created_tick   INTEGER NOT NULL DEFAULT 0
) STRICT;
CREATE INDEX entity_owner     ON entity(owner_id, entity_type_id);
CREATE INDEX entity_region    ON entity(region_id);
CREATE INDEX entity_container ON entity(container_id) WHERE container_id IS NOT NULL;
CREATE INDEX entity_city      ON entity(city_id) WHERE city_id IS NOT NULL;

CREATE TABLE entity_cooldown (
  entity_id  INTEGER NOT NULL REFERENCES entity(id) ON DELETE CASCADE,
  ability_id INTEGER NOT NULL REFERENCES ability(id),
  ready_tick INTEGER NOT NULL,
  PRIMARY KEY (entity_id, ability_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE entity_effect (           -- temporary modifiers currently applied
  entity_id   INTEGER NOT NULL REFERENCES entity(id) ON DELETE CASCADE,
  modifier_id INTEGER NOT NULL REFERENCES modifier(id),
  source_entity_id INTEGER,
  end_tick    INTEGER NOT NULL,
  PRIMARY KEY (entity_id, modifier_id)
) STRICT, WITHOUT ROWID;

-- THE ORDER BOOK. Everything a player (or AI) asks for is an order: a unit's
-- command queue and a building's production queue are the same table.
-- Costs are paid when the order is accepted and refunded on cancel.
CREATE TABLE "order" (
  id            INTEGER PRIMARY KEY,
  player_id     INTEGER NOT NULL REFERENCES player(id),
  executor_id   INTEGER NOT NULL REFERENCES entity(id) ON DELETE CASCADE,
  kind          TEXT NOT NULL REFERENCES order_kind(key),
  seq           INTEGER NOT NULL,      -- position in the executor's queue (0 = current)
  status        TEXT NOT NULL DEFAULT 'queued'
                  CHECK (status IN ('queued','active','blocked','done','cancelled','failed')),
  block_reason  TEXT NOT NULL DEFAULT '',  -- 'pop_cap','no_path','no_slot','no_resources' …
  subject_def_id INTEGER REFERENCES def(id),     -- what to train / research / construct / cast
  target_entity_id INTEGER REFERENCES entity(id),
  target_x      INTEGER,
  target_y      INTEGER,
  formation_id  INTEGER REFERENCES formation(id),
  facing        INTEGER,
  progress_ticks INTEGER NOT NULL DEFAULT 0,
  total_ticks   INTEGER NOT NULL DEFAULT 0,
  repeat        INTEGER NOT NULL DEFAULT 0,      -- infinite queue / patrol loop
  issued_tick   INTEGER NOT NULL,
  UNIQUE (executor_id, seq)
) STRICT;
CREATE INDEX order_by_player ON "order"(player_id, status);

CREATE TABLE order_payment (           -- what was actually charged (for refunds)
  order_id    INTEGER NOT NULL REFERENCES "order"(id) ON DELETE CASCADE,
  resource_id INTEGER NOT NULL REFERENCES resource(id),
  amount      INTEGER NOT NULL,
  PRIMARY KEY (order_id, resource_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE trade_route (
  trader_id   INTEGER PRIMARY KEY REFERENCES entity(id) ON DELETE CASCADE,
  from_city_id INTEGER NOT NULL REFERENCES entity(id),
  to_city_id   INTEGER NOT NULL REFERENCES entity(id),
  income      INTEGER NOT NULL
) STRICT;

CREATE TABLE diplomacy_offer (
  id        INTEGER PRIMARY KEY,
  from_player INTEGER NOT NULL REFERENCES player(id),
  to_player   INTEGER NOT NULL REFERENCES player(id),
  proposes  TEXT NOT NULL DEFAULT '' CHECK (proposes IN ('','war','peace','alliance','joint_attack')),
  against_player INTEGER REFERENCES player(id),
  status    TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open','accepted','rejected','cancelled')),
  tick      INTEGER NOT NULL
) STRICT;

CREATE TABLE diplomacy_offer_item (    -- positive = sender gives, negative = sender demands
  offer_id    INTEGER NOT NULL REFERENCES diplomacy_offer(id) ON DELETE CASCADE,
  resource_id INTEGER NOT NULL REFERENCES resource(id),
  amount      INTEGER NOT NULL,
  PRIMARY KEY (offer_id, resource_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE victory_timer (
  victory_id INTEGER NOT NULL REFERENCES victory(id),
  team       INTEGER NOT NULL,
  end_tick   INTEGER NOT NULL,
  PRIMARY KEY (victory_id, team)
) STRICT, WITHOUT ROWID;

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
  OR (d.kind='victory'     AND d.id NOT IN (SELECT id FROM victory));

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
