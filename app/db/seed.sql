-- Rise of Man — game rules and generated content. Load after schema.sql.
PRAGMA foreign_keys = ON;
BEGIN;
PRAGMA defer_foreign_keys = ON;

INSERT INTO schema_meta VALUES ('schema_version','1'), ('rules_version','0.0.1-seed');

INSERT INTO game_constant(key,value,unit,note) VALUES
 ('sim.ticks_per_second',20,'ticks','fixed simulation step; never changes with game speed'),
 ('space.orbit_reach',6000,'milli-tiles','orbit holding radius'),
 ('space.route_ticks_per_unit',1,'ticks','route time per unit at one tile per second'),
 ('sim.speed_pct.min',25,'%','slowest selectable game speed'),
 ('sim.speed_pct.max',400,'%','fastest selectable game speed'),
 ('economy.income_period_ticks',600,'ticks','gather rates are "per 30 s"'),
 ('economy.cancel_refund_pct',100,'%',''),
 ('economy.tribute_delivered_pct',51,'%','rest is lost in transit'),
 ('combat.flank_bonus_pct',50,'%','per flank level, max 2 levels'),
 ('combat.high_ground_bonus_pct',10,'%',''),
 ('combat.min_damage',1,'hp',''),
 ('combat.attrition_per_period',30,'hp','enemy territory damage per income period'),
 ('combat.attrition_floor_hp',1,'hp','attrition cannot reduce hp below this'),
 ('victory.capital_hold_ticks',2400,'ticks','continuous enemy capital occupation before defeat'),
 ('unit.respond_range',12000,'milli-tiles','aggressive stance acquisition'),
 ('citizen.auto_work_delay_ticks',200,'ticks','idle workers look for work after this'),
 ('city.spacing',20000,'milli-tiles','minimum distance between cities; below a city border radius so a second city fits inside the first one''s borders'),
 ('city.radius_per_building',400,'milli-tiles','economic radius a city gains per completed own building inside it'),
 ('city.radius_from_buildings_max',8000,'milli-tiles','cap on the economic radius gained from buildings'),
 ('city.assimilate_ticks',2400,'ticks','captured city becomes yours after this'),
 ('city.capture_hold_ticks',2400,'ticks','a struck-down city held by attackers this long falls to them even with defenders left'),
 ('player.caravan_limit.base',2,'traders','before caravan_limit modifiers'),
 ('trade.route_unit',10000,'milli-tiles','route income length unit'),
 ('trade.min_route',20000,'milli-tiles','minimum route length; same as city.spacing'),
 ('player.pop_cap.base',50,'pop','before modifiers (stat pop_cap)'),
 ('player.city_limit.base',1,'cities','before modifiers (stat city_limit)'),
 ('player.commerce_cap.base',70,'per period','before modifiers (stat commerce_cap)'),
 ('build.site_start_hp_pct',10,'%','hit points of a building site when placed'),
 ('build.reach',1500,'milli-tiles','how close a builder must be to the footprint to work'),
 ('path.nodes_per_tick',20000,'nodes','path-search nodes expanded per tick across all searches'),
 ('world.day_ticks',6000,'ticks','one orbit of the light that represents the sun around the planet orb (5 game minutes); presentation only'),
 ('world.axial_tilt_mdeg',0,'milli-degrees','latitude the light stays over; 0 means no seasons');

INSERT INTO game_constant(key,value,unit,note) VALUES
 ('space.launch_ticks',20,'ticks','launch duration from a surface producer'),
 ('space.orbit_speed',400,'milli-tiles/second','speed of the orbital ground footprint'),
 ('space.land_ticks',60,'ticks','descent of a carrier from orbit to open ground');

-- Routes and jumps (task 70; docs/database.md "Routes and jumps").
INSERT INTO game_constant(key,value,unit,note) VALUES
 ('space.jump_charge_ticks',160,'ticks','charge at the system edge before a jump, for a ship type without et_movement.jump_charge_ticks'),
 ('space.charting_pct',500,'%','transit time of a voyage along a route its player has not charted; charts the route on arrival'),
 ('trade.route_length_mtiles',1000,'milli-tiles','trade length an interstellar trade route counts per unit of route length');
INSERT INTO stat(key,scope,note) VALUES ('jump_charge','entity','ticks a ship charges before a jump');

INSERT INTO layer VALUES
 ('ground','solid surface'), ('fluid','sea'), ('subfluid','underwater'),
 ('air','atmosphere'), ('orbit','near orbit'), ('space','open space between planets');
INSERT INTO body_kind VALUES
 ('star','centre of a star system'), ('habitable','planet with a full surface'),
 ('resource','moon, asteroid or small planet claimed by starship presence');

INSERT INTO map_scale VALUES
 ('planetary','Planetary',0,60), ('interplanetary','Interplanetary',1,160),
 ('interstellar','Interstellar',2,240);

INSERT INTO tag(key,category) VALUES
 ('biological','body'), ('mechanical','body'),
 ('civilian','role'), ('military','role'), ('infantry','role'), ('mounted','role'), ('vehicle','role'),
 ('siege','role'), ('ship','role'), ('aircraft','role'), ('drone','role'), ('starship','role'),
 ('clergy','role'), ('healer','role'), ('city','structure'), ('temple','structure'), ('wonder','structure'),
 ('production','structure'), ('gather','structure'),
 ('passenger','role'), ('orbital_craft','role');

INSERT INTO stat(key,scope,note) VALUES
 ('hp','entity',''), ('armor','entity',''), ('attack','entity',''), ('range','entity',''),
 ('speed','entity',''), ('los','entity',''), ('heal_rate','entity',''), ('build_time','entity','train/construct ticks'),
 ('cost.all','entity','every resource of the cost'), ('cost.food','entity',''), ('cost.timber','entity',''),
 ('cost.metal','entity',''), ('cost.wealth','entity',''), ('cost.knowledge','entity',''),
 ('border_radius','entity',''), ('economic_radius','entity','city only: radius where civil buildings may stand'), ('garrison_capacity','entity',''), ('attrition_taken','entity',''),
 ('pop_cap','player',''), ('city_limit','player',''), ('commerce_cap','player','max income per resource'),
 ('caravan_limit','player',''), ('income.food','player',''), ('income.timber','player',''),
 ('income.metal','player',''), ('income.wealth','player',''), ('income.knowledge','player',''),
 ('income.oil','player',''), ('income.energy','player',''), ('research_time','player',''),
 ('research_cost','player',''), ('attrition_dealt','player','damage to enemies inside own borders');

INSERT INTO damage_class(key) VALUES ('melee'),('pierce'),('bullet'),('explosive'),('energy'),('bio');
INSERT INTO armor_class(key)  VALUES ('flesh'),('plate'),('vehicle'),('structure'),('hull'),('shield');
WITH v(dk,ak,pct) AS (VALUES
  ('melee','structure',150), ('pierce','structure',20), ('pierce','plate',60), ('bullet','vehicle',40),
  ('explosive','structure',200), ('explosive','flesh',80), ('energy','shield',50), ('bio','flesh',150),
  ('bio','vehicle',25))
INSERT INTO damage_matrix
 SELECT d.id, a.id, v.pct FROM v
 JOIN damage_class d ON d.key=v.dk JOIN armor_class a ON a.key=v.ak;

INSERT INTO movement_class(key,flies) VALUES
 ('foot',0),('wheeled',0),('ship',0),('submarine',0),('aircraft',1),('amphibious',0),('starship',0);

INSERT INTO order_kind(key,needs,target,note) VALUES
 ('stop','','none',''), ('move','movement','entity_or_point','entity target = follow; subject = formation (optional): one order batch walks at its slowest member''s pace when the formation keeps_slowest'),
 ('launch','movement','entity','launch from a completed own producer'),
 ('orbit','movement','entity_or_point','body marker or holding point'),
 ('travel','movement','point','target point body = destination system star body; coordinates ignored'),
 ('attack','weapon','entity',''), ('attack_move','weapon','point','subject = formation (optional), as for move'),
 ('patrol','movement','point','walk between the start and the point and back, fighting like attack_move; subject = formation (optional)'),
 ('guard','weapon','entity','stay beside an own or allied entity and fight off enemies near it'),
 ('explore','movement','none','walk to the nearest ground the player has not seen, until none is left in reach'),
 ('stance','weapon','def','subject = combat stance; applied at once, the queue stays'),
 ('construct','builder','def','subject = building type, target point = site'),
 ('repair','builder','entity',''), ('harvest','harvester','entity','target = gather site'),
 ('heal','healer','entity',''), ('trade','trader','entity','target = city'),
 ('carry','movement','entity','target = carry node; brought to the nearest own keeper'),
 ('claim','trader','entity','target = rare resource node'),
 ('enter','movement','entity','board a carrier: beside it when it has landed, or at an own spaceport below its orbit; orbital craft board one orbiting their planet'),
 ('unload','container','entity_or_point','landed: set passengers down around the carrier (a point: they walk there); in orbit: at the target own spaceport below, or a launching carrier launches its craft into its orbit'),
 ('land','movement','point','a carrier descends from orbit onto open ground of that planet, outside enemy cities; passengers come out and the carrier stays landed; an orbital craft comes down into the air above the point'),
 ('lift','movement','none','a landed carrier returns to orbit; an orbital craft climbs from its planet to that planet''s orbit'),
 ('train','producer','def',''), ('research','producer','def',''), ('morph','','def','upgrade in place'),
 ('ability','energy','entity_or_point','subject = ability'), ('set_rally','producer','entity_or_point','units it trains walk to the point or entity; applied at once, the queue stays'),
 ('raze','','none','pull down an own building at once: an unfinished one refunds the work not done; not a finished city'), ('disband','movement','none','the unit leaves the match; no refund'),
 ('release','gather_site','none','every worker inside a gather building leaves it');

-- ── defs ────────────────────────────────────────────────────────────────────
INSERT INTO def(kind,key,name,sort) VALUES
 ('age','age.ancient','Ancient Age',0), ('age','age.classical','Classical Age',1),
 ('age','age.medieval','Medieval Age',2), ('age','age.gunpowder','Gunpowder Age',3),
 ('age','age.enlightenment','Enlightenment Age',4), ('age','age.industrial','Industrial Age',5),
 ('age','age.modern','Modern Age',6), ('age','age.information','Information Age',7),
 ('age','age.automation','Automation Age',8), ('age','age.fusion','Fusion Age',9),
 ('age','age.starfaring','Starfaring Age',10), ('age','age.expansion','Age of Expansion',11),
 ('age','age.abundance','Age of Abundance',12), ('age','age.galactic','Galactic Empires',13);
INSERT INTO age(id,ordinal,era,map_scale)
 SELECT id, sort,
   CASE WHEN sort<=7 THEN 'history' WHEN sort<=9 THEN 'near_future' WHEN sort<=12 THEN 'stellar'
        ELSE 'galactic' END,
   CASE WHEN sort<=9 THEN 'planetary' WHEN sort=10 THEN 'interplanetary'
        ELSE 'interstellar' END
 FROM def WHERE kind='age';
WITH v(k,r) AS (VALUES ('age.ancient',30000), ('age.classical',36000), ('age.medieval',42000),
  ('age.gunpowder',48000), ('age.enlightenment',55000), ('age.industrial',62000),
  ('age.modern',70000), ('age.information',78000), ('age.automation',110000))
UPDATE age SET view_range = (SELECT v.r FROM v JOIN def d ON d.key = v.k WHERE d.id = age.id)
 WHERE id IN (SELECT d.id FROM v JOIN def d ON d.key = v.k);
-- Colonisation since the 1600s ignored national borders (owner decision, task 52).
UPDATE age SET free_colonisation = 1 WHERE ordinal >= 4;

INSERT INTO def(kind,key,name) VALUES
 ('resource','res.food','Food'), ('resource','res.timber','Timber'), ('resource','res.metal','Metal'),
 ('resource','res.wealth','Wealth'), ('resource','res.knowledge','Knowledge'), ('resource','res.oil','Oil'),
 ('resource','res.energy','Energy');
-- Preserve existing catalogue ids after retiring one rare resource.
INSERT INTO def(id,kind,key,name)
 SELECT MAX(id)+2,'resource','rare.horses','Horses' FROM def;
WITH v(k,kind,ab,cap,tr,age) AS (VALUES
  ('res.food','base','F',1,1,NULL), ('res.timber','base','T',1,1,NULL), ('res.metal','base','M',1,1,'age.classical'),
  ('res.wealth','base','W',1,0,NULL), ('res.knowledge','base','K',0,0,'age.classical'), ('res.oil','base','O',1,1,'age.industrial'),
  ('res.energy','base','E',1,1,'age.fusion'), ('rare.horses','rare','Ho',0,0,NULL)
 )
INSERT INTO resource(id,kind,abbrev,rate_capped,tradeable,first_age_id)
 SELECT d.id, v.kind, v.ab, v.cap, v.tr, (SELECT id FROM def WHERE key=v.age) FROM v JOIN def d ON d.key=v.k;
WITH v(r,b,n) AS (VALUES ('rare.horses','res.food',10),
  ('rare.horses','res.metal',10))
INSERT INTO resource_yield
 SELECT r.id, b.id, v.n FROM v JOIN def r ON r.key=v.r JOIN def b ON b.key=v.b;

INSERT INTO def(kind,key,name) VALUES
 ('terrain','ter.plain','Plain'), ('terrain','ter.forest','Forest'), ('terrain','ter.mountain','Mountain'),
 ('terrain','ter.coast','Shallows'), ('terrain','ter.deep','Deep'),
 ('terrain','ter.space','Open Space'), ('terrain','ter.nebula','Nebula');
WITH v(k,l,b,los) AS (VALUES ('ter.plain','ground',1,0), ('ter.forest','ground',0,1),
  ('ter.mountain','ground',0,1), ('ter.coast','fluid',1,0), ('ter.deep','fluid',0,0),
  ('ter.space','space',0,0), ('ter.nebula','space',0,1))
INSERT INTO terrain(id,layer,buildable,blocks_los)
 SELECT d.id, v.l, v.b, v.los FROM v
 JOIN def d ON d.key=v.k;
WITH v(m,t,pct) AS (VALUES ('foot','ter.plain',100), ('foot','ter.forest',70),
  ('wheeled','ter.plain',100), ('ship','ter.coast',100), ('ship','ter.deep',100),
  ('submarine','ter.deep',100), ('amphibious','ter.plain',100), ('amphibious','ter.coast',80),
  ('starship','ter.space',100), ('starship','ter.nebula',60))
INSERT INTO movement_class_terrain
 SELECT m.id, d.id, v.pct FROM v
 JOIN movement_class m ON m.key=v.m JOIN def d ON d.key=v.t;

INSERT INTO def(kind,key,name) VALUES
 ('faction','race.human','Humanity'), ('faction','race.reptilian','Reptilians'), ('faction','race.avian','Avians'),
 ('faction','npc.nature','Nature'),
 ('faction','nation.ashscale','Ashscale Brood'),
 ('faction','nation.mirefang','Mirefang Brood'),
 ('faction','nation.sunstone','Sunstone Brood'),
 ('faction','nation.highwind','Highwind Flight'),
 ('faction','nation.reedmarsh','Reedmarsh Flight'),
 ('faction','nation.stormcrest','Stormcrest Flight'),
 ('faction','bloc.africa','Africa'), ('faction','bloc.americas','Americas'), ('faction','bloc.asia','Asia'),
 ('faction','bloc.europe','Europe'), ('faction','bloc.oceania','Oceania'),
 ('faction','nation.romans','Romans'),
 ('faction','nation.greeks','Greeks'),
 ('faction','nation.britons','Britons'),
 ('faction','nation.franks','Franks'),
 ('faction','nation.egyptians','Egyptians'),
 ('faction','nation.nubians','Nubians'),
 ('faction','nation.persians','Persians'),
 ('faction','nation.chinese','Chinese'),
 ('faction','nation.japanese','Japanese'),
 ('faction','nation.indians','Indians'),
 ('faction','nation.mongols','Mongols'),
 ('faction','nation.aztecs','Aztecs'),
 ('faction','nation.inca','Inca'),
 ('faction','nation.maori','Maori');
WITH v(k,kind,parent,play,first,last) AS (VALUES
  ('race.human','race',NULL,1,'age.starfaring',NULL),
  ('race.reptilian','race',NULL,1,'age.starfaring',NULL),
  ('race.avian','race',NULL,1,'age.starfaring',NULL),
  ('nation.ashscale','nation','race.reptilian',1,'age.ancient','age.fusion'),
  ('nation.mirefang','nation','race.reptilian',1,'age.ancient','age.fusion'),
  ('nation.sunstone','nation','race.reptilian',1,'age.ancient','age.fusion'),
  ('nation.highwind','nation','race.avian',1,'age.ancient','age.fusion'),
  ('nation.reedmarsh','nation','race.avian',1,'age.ancient','age.fusion'),
  ('nation.stormcrest','nation','race.avian',1,'age.ancient','age.fusion'),
  ('npc.nature','npc',NULL,0,NULL,NULL),
  ('bloc.africa','bloc','race.human',1,'age.automation','age.fusion'),
  ('bloc.americas','bloc','race.human',1,'age.automation','age.fusion'),
  ('bloc.asia','bloc','race.human',1,'age.automation','age.fusion'),
  ('bloc.europe','bloc','race.human',1,'age.automation','age.fusion'),
  ('bloc.oceania','bloc','race.human',1,'age.automation','age.fusion'),
  ('nation.romans','nation','bloc.europe',1,'age.ancient','age.information'),
  ('nation.greeks','nation','bloc.europe',1,'age.ancient','age.information'),
  ('nation.britons','nation','bloc.europe',1,'age.ancient','age.information'),
  ('nation.franks','nation','bloc.europe',1,'age.ancient','age.information'),
  ('nation.egyptians','nation','bloc.africa',1,'age.ancient','age.information'),
  ('nation.nubians','nation','bloc.africa',1,'age.ancient','age.information'),
  ('nation.persians','nation','bloc.asia',1,'age.ancient','age.information'),
  ('nation.chinese','nation','bloc.asia',1,'age.ancient','age.information'),
  ('nation.japanese','nation','bloc.asia',1,'age.ancient','age.information'),
  ('nation.indians','nation','bloc.asia',1,'age.ancient','age.information'),
  ('nation.mongols','nation','bloc.asia',1,'age.ancient','age.information'),
  ('nation.aztecs','nation','bloc.americas',1,'age.ancient','age.information'),
  ('nation.inca','nation','bloc.americas',1,'age.ancient','age.information'),
  ('nation.maori','nation','bloc.oceania',1,'age.ancient','age.information')
 )
INSERT INTO faction(id,kind,parent_id,playable,first_age_id,last_age_id)
 SELECT d.id, v.kind, (SELECT id FROM def WHERE key=v.parent), v.play,
        (SELECT id FROM def WHERE key=v.first), (SELECT id FROM def WHERE key=v.last)
 FROM v JOIN def d ON d.key=v.k;

-- Map nodes are independent of generated units and buildings.
INSERT INTO def(kind,key,name) VALUES
 ('entity_type','node.relics','Relic Site'), ('entity_type','node.ore_planet','Ore Planet');
INSERT INTO entity_type(id,class,footprint_w,footprint_h)
 SELECT id,'node',2,2 FROM def WHERE key IN ('node.relics','node.ore_planet');
INSERT INTO et_node(entity_type_id,resource_id,claim_mode,claim_tag_id,claim_radius,income) VALUES
 ((SELECT id FROM def WHERE key='node.relics'),(SELECT id FROM def WHERE key='res.wealth'),'carry',(SELECT id FROM tag WHERE key='clergy'),0,20),
 ((SELECT id FROM def WHERE key='node.ore_planet'),(SELECT id FROM def WHERE key='res.metal'),'presence',(SELECT id FROM tag WHERE key='starship'),3000,40);

-- Race traits (placeholders): the same roster, tilted by a few modifiers.
INSERT INTO modifier(source_def_id,target_tag_id,stat_key,op,value)
 SELECT d.id, t.id, v.s, v.op, v.n FROM (SELECT 1) JOIN (
   SELECT 'race.reptilian' AS k,'biological' AS t,'hp' AS s,'pct' AS op,15 AS n
   UNION ALL SELECT 'race.reptilian','biological','speed','pct',-5
   UNION ALL SELECT 'race.avian','biological','speed','pct',15
   UNION ALL SELECT 'race.avian','biological','los','pct',25
   UNION ALL SELECT 'race.avian','biological','hp','pct',-10) v
 JOIN def d ON d.key=v.k JOIN tag t ON t.key=v.t;

-- ── abilities, formations, stances ──────────────────────────────────────────
INSERT INTO def(kind,key,name) VALUES ('ability','abl.convert','Convert');
INSERT INTO ability(id,target,target_relation,target_tag_id,range,energy_cost,cast_ticks,cooldown_ticks,effect)
 SELECT d.id,'entity','enemy',(SELECT id FROM tag WHERE key='biological'),5000,100,80,200,'convert'
 FROM def d WHERE d.key='abl.convert';

INSERT INTO def(kind,key,name) VALUES
 ('formation','form.line','Line'), ('formation','form.column','Column'), ('formation','form.wedge','Wedge'),
 ('formation','form.square','Square'), ('formation','form.echelon_left','Echelon Left'),
 ('formation','form.echelon_right','Echelon Right'), ('formation','form.envelop','Envelop'),
 ('formation','form.refused','Refused Flanks'), ('formation','form.sparse','Sparse'), ('formation','form.mob','Mob');
INSERT INTO formation(id,shape) SELECT id, substr(key,6) FROM def WHERE kind='formation';

INSERT INTO def(kind,key,name) VALUES
 ('stance','stance.aggressive','Aggressive'), ('stance','stance.defensive','Defensive'),
 ('stance','stance.stand_ground','Stand Ground'), ('stance','stance.hold_fire','Hold Fire'),
 ('stance','stance.raid','Raid'), ('stance','stance.raze','Raze'),
 ('stance','stance.gather','Gather'), ('stance','stance.build_gather','Build & Gather'),
 ('stance','stance.build','Build'), ('stance','stance.idle','Idle'),
 ('stance','stance.auto_cast','Auto Ability'), ('stance','stance.manual_cast','Manual Ability'),
 ('stance','stance.auto_deploy','Auto Deploy'), ('stance','stance.manual_deploy','Manual Deploy');
INSERT INTO stance(id,family,behavior)
 SELECT id,
  CASE WHEN key IN ('stance.gather','stance.build_gather','stance.build','stance.idle') THEN 'worker'
       WHEN key LIKE '%cast' THEN 'caster' WHEN key LIKE '%deploy' THEN 'deploy' ELSE 'combat' END,
  substr(key,8)
 FROM def WHERE kind='stance';

-- ── map style, options, victory ─────────────────────────────────────────────
INSERT INTO def(kind,key,name) VALUES ('map_style','map.one_world','One World'), ('map_style','map.many_worlds','Many Worlds');
INSERT INTO map_style(id,planets,fluid_pct,space_pct)
 SELECT id, CASE key WHEN 'map.one_world' THEN 1 ELSE 4 END, CASE key WHEN 'map.one_world' THEN 50 ELSE 35 END, CASE key WHEN 'map.one_world' THEN 55 ELSE 70 END
 FROM def WHERE kind='map_style';
-- universe knobs; one world: a quick match, one home system per race and nothing else
WITH v(k,n0,n1,h0,h1,r0,r1,p0,pm,p1,extra,radius) AS (VALUES
  ('map.one_world',   0,0, 1,1, 3,5, 128,192,256, 30,1000),
  ('map.many_worlds', 2,4, 1,3, 3,8, 128,192,256, 30,1000))
UPDATE map_style SET
  neutral_systems_min=v.n0, neutral_systems_max=v.n1,
  habitable_per_system_min=v.h0, habitable_per_system_max=v.h1,
  resource_bodies_per_system_min=v.r0, resource_bodies_per_system_max=v.r1,
  planet_n_min=v.p0, planet_n_median=v.pm, planet_n_max=v.p1,
  route_extra_pct=v.extra, star_map_radius=v.radius
 FROM v JOIN def d ON d.key=v.k WHERE map_style.id=d.id;
WITH v(k,rk,p,c0,c1,s0,s1,d0,d1) AS (VALUES
  ('ter.forest','planet','per_player',1,2,6,10,3,13), ('ter.mountain','planet','per_player',1,2,5,8,6,14),
  ('node.relics','planet','scattered',2,4,1,1,10,0),
  ('node.ore_planet','space','scattered',6,10,1,1,0,0), ('ter.nebula','space','scattered',2,3,6,12,0,0))
INSERT INTO map_style_feature(map_style_id,feature_def_id,region_kind,pattern,clumps_min,clumps_max,size_min,size_max,start_min_dist,start_max_dist)
 SELECT (SELECT id FROM def WHERE key='map.one_world'), d.id, v.rk, v.p, v.c0, v.c1, v.s0, v.s1, v.d0, v.d1 FROM v JOIN def d ON d.key=v.k;
INSERT INTO map_style_feature(map_style_id,feature_def_id,region_kind,pattern,clumps_min,clumps_max,size_min,size_max,start_min_dist,start_max_dist)
 SELECT (SELECT id FROM def WHERE key='map.many_worlds'), feature_def_id, region_kind, pattern, clumps_min, clumps_max, size_min, size_max, start_min_dist, start_max_dist
 FROM map_style_feature WHERE map_style_id=(SELECT id FROM def WHERE key='map.one_world') ORDER BY id;

WITH v(k,pct) AS (VALUES ('ter.forest',12), ('ter.mountain',4))
UPDATE map_style_feature SET coverage_pct=v.pct
 FROM v JOIN def d ON d.key=v.k
 WHERE map_style_feature.feature_def_id=d.id AND region_kind='planet';

INSERT INTO match_option VALUES
 ('start_age','Start Age','age.ancient'), ('end_age','End Age','age.galactic'),
 ('pop_limit','Population Limit','200'), ('game_speed','Game Speed (%)','100'),
 ('start_resources','Starting Resources','standard'), ('tech_pace','Technology Cost & Speed','normal'),
 ('reveal','Reveal Map','normal'), ('rush_rule','Rush Rule','none'),
 ('elimination','Elimination','capital'), ('difficulty','Difficulty','moderate');
INSERT INTO match_option_choice VALUES
 ('game_speed','50','Slow',0), ('game_speed','75','Relaxed',1), ('game_speed','100','Normal',2),
 ('game_speed','150','Fast',3), ('game_speed','200','Very Fast',4),
 ('reveal','normal','Normal',0), ('reveal','explored','Explored',1), ('reveal','all','All Visible',2),
 ('elimination','conquest','Conquest',0), ('elimination','capital','Capital',1), ('elimination','sudden_death','Sudden Death',2);

-- ── computer opponents ──────────────────────────────────────────────────────
INSERT INTO ai_profile VALUES
 ('balanced','Balanced','adapts to what it sees'),
 ('rusher','Rush and Raid','early army, constant pressure on workers and trade'),
 ('builder','Boom and Build','economy and technology first, strong late'),
 ('turtle','Fortress','defends borders, wins by wonders or territory');
INSERT INTO ai_weight_key VALUES
 ('econ.worker_share_pct',55,'share of population kept as workers'),
 ('econ.expand_eagerness',50,'0-100: how soon a new city is founded when the limit allows'),
 ('tech.age_priority',50,'0-100: preference for advancing in age over other research'),
 ('mil.army_share_pct',40,'share of population kept as military'),
 ('mil.attack_ratio_pct',130,'attack when own strength is this % of the known enemy strength'),
 ('mil.retreat_ratio_pct',60,'pull back when the local ratio falls below this'),
 ('mil.raid_share_pct',10,'share of the army sent after workers and trade'),
 ('mil.defense_share_pct',25,'share of the army kept at home'),
 ('scout.units',1,'units kept exploring'),
 ('space.claim_eagerness',60,'0-100: priority of resource planets once in space');
INSERT INTO ai_profile_weight VALUES
 ('rusher','econ.worker_share_pct',40), ('rusher','mil.army_share_pct',55), ('rusher','mil.attack_ratio_pct',100),
 ('rusher','mil.raid_share_pct',30), ('rusher','tech.age_priority',25),
 ('builder','econ.worker_share_pct',65), ('builder','mil.army_share_pct',25), ('builder','tech.age_priority',80),
 ('builder','econ.expand_eagerness',85), ('builder','mil.attack_ratio_pct',170),
 ('turtle','mil.defense_share_pct',70), ('turtle','mil.attack_ratio_pct',250), ('turtle','econ.expand_eagerness',30);
-- grace_ticks, first_wave_max, wave_growth_per_hour, army_vs_enemy_pct, economy_pct
-- (task 67): the lowest levels leave a learner's town alone for the first minutes, come
-- in small waves that grow slowly, keep their army below the one they have seen and keep
-- fewer hands at work. Task 78: easy leaves a town alone for 45 minutes (was 10) and
-- easiest for 48 (was 30): their economies no longer stall, so their armies are ready
-- sooner, and the grace keeps their first waves as late as in 0.0.67, easiest's last.
INSERT INTO ai_difficulty(key,name,rank,think_interval_ticks,orders_per_minute,reaction_ticks,mistake_pct,uses_formations,uses_abilities,retreats,income_bonus_pct,sees_all,grace_ticks,first_wave_max,wave_growth_per_hour,army_vs_enemy_pct,economy_pct) VALUES
 ('easiest','Easiest',0,100,20,600,30,0,0,0,0,0,57600,5,10,40,50),
 ('easy','Easy',1,60,40,300,20,0,0,1,0,0,54000,8,20,80,80),
 ('moderate','Moderate',2,40,80,60,10,1,1,1,0,0,3600,0,0,0,100),
 ('tough','Tough',3,20,160,30,3,1,1,1,0,0,0,0,0,0,100),
 ('tougher','Tougher',4,10,300,10,0,1,1,1,0,0,0,0,0,0,100),
 ('toughest','Toughest',5,10,300,10,0,1,1,1,25,0,0,0,0,0,100);

INSERT INTO def(kind,key,name) VALUES ('victory','win.conquest','Conquest'), ('victory','win.territory','Territory');
INSERT INTO victory(id,rule,threshold,timer_ticks)
 SELECT id, substr(key,5), CASE key WHEN 'win.territory' THEN 70 ELSE 0 END, CASE key WHEN 'win.territory' THEN 6000 ELSE 0 END
 FROM def WHERE kind='victory';

-- ── resource bodies of a star system ────────────────────────────────────────
-- What a planet does not give plentifully. Owned by whoever keeps a starship
-- beside the body; placed by the universe generator through map_style_feature.
INSERT INTO def(kind,key,name) VALUES
 ('entity_type','node.ore_asteroid','Ore Asteroid'), ('entity_type','node.tar_moon','Tar Moon'),
 ('entity_type','node.crystal_asteroid','Crystal Asteroid');
INSERT INTO entity_type(id,class,ui_group,hp,armor,los,pop_cost,footprint_w,footprint_h,build_ticks,first_age_id)
 SELECT id, 'node', '', 1, 0, 0, 0, 2, 2, 0, (SELECT id FROM def WHERE key='age.starfaring')
 FROM def WHERE key IN ('node.ore_asteroid','node.tar_moon','node.crystal_asteroid') ORDER BY id;
WITH v(k,r,inc) AS (VALUES ('node.ore_asteroid','res.metal',40), ('node.tar_moon','res.oil',30),
  ('node.crystal_asteroid','res.energy',30))
INSERT INTO et_node(entity_type_id,resource_id,amount,claim_mode,claim_tag_id,claim_radius,income)
 SELECT d.id, rd.id, NULL, 'presence', (SELECT id FROM tag WHERE key='starship'), 3000, v.inc
 FROM v JOIN def d ON d.key=v.k JOIN def rd ON rd.key=v.r;
-- chance_pct is each kind's weight; per_player rows put clumps_min of that kind in every home system
WITH v(k,p,w,c0,c1) AS (VALUES
  ('node.ore_asteroid','per_player',50,1,1), ('node.tar_moon','scattered',25,1,1),
  ('node.crystal_asteroid','scattered',25,1,1))
INSERT INTO map_style_feature(map_style_id,feature_def_id,region_kind,pattern,chance_pct,clumps_min,clumps_max)
 SELECT s.id, d.id, 'resource_body', v.p, v.w, v.c0, v.c1
 FROM map_style s, v JOIN def d ON d.key=v.k ORDER BY s.id, d.id;


-- ── terrain materials ───────────────────────────────────────────────────────
-- Base colour of each feature terrain for the renderer, until procedural
-- materials replace it. params: {"base_colour":"#rrggbb"}.
INSERT INTO def(kind,key,name) VALUES
 ('material','mat.forest','Forest canopy'), ('material','mat.mountain','Bare rock'),
 ('material','mat.nebula','Nebula');
WITH v(k,prog,col) AS (VALUES ('mat.forest','foliage','#2d5a27'), ('mat.mountain','rock','#8a8178'),
  ('mat.nebula','gas','#4a3a6a'))
INSERT INTO material(id,program,params)
 SELECT d.id, v.prog, '{"base_colour":"' || v.col || '"}' FROM v JOIN def d ON d.key=v.k;
WITH v(t,m) AS (VALUES ('ter.forest','mat.forest'), ('ter.mountain','mat.mountain'), ('ter.nebula','mat.nebula'))
UPDATE terrain SET material_id = (SELECT m.id FROM v JOIN def m ON m.key=v.m JOIN def t ON t.key=v.t WHERE t.id=terrain.id)
 WHERE id IN (SELECT t.id FROM v JOIN def t ON t.key=v.t);

-- A habitable body marker has no resource component.
INSERT INTO def(kind,key,name) VALUES ('entity_type','node.planet_marker','Planet Marker');
INSERT INTO entity_type(id,class,hp,los,pop_cost,footprint_w,footprint_h,build_ticks)
 SELECT id,'node',1,0,0,1,1,0 FROM def WHERE key='node.planet_marker';

-- Append polar definitions so existing definition ids and flat-map hashes stay stable.
INSERT INTO def(kind,key,name) VALUES ('terrain','ter.ice','Ice'), ('material','mat.ice','Ice');
INSERT INTO material(id,program,params)
 SELECT id,'ice','{"base_colour":"#e8eef2"}' FROM def WHERE key='mat.ice';
INSERT INTO terrain(id,layer,buildable,blocks_los,material_id)
 SELECT id,'ground',0,0,(SELECT id FROM def WHERE key='mat.ice') FROM def WHERE key='ter.ice';
INSERT INTO movement_class_terrain(movement_class_id,terrain_id,speed_pct)
 SELECT m.movement_class_id, ice.id, 50 FROM movement_class_terrain m
 JOIN def plain ON plain.id=m.terrain_id AND plain.key='ter.plain'
 JOIN def ice ON ice.key='ter.ice' ORDER BY m.movement_class_id;
UPDATE map_style SET polar_cap_pct=20, start_latitude_max_mdeg=60000,
 polar_terrain_id=(SELECT id FROM def WHERE key='ter.ice');

-- BEGIN GENERATED CONTENT
-- Generated by db/content/generate.py; edit its tables, then regenerate.
INSERT INTO def(kind,key,name) VALUES ('terrain','ter.oil','Oil Deposit');
INSERT INTO terrain(id,layer,buildable,blocks_los) VALUES ((SELECT id FROM def WHERE key='ter.oil'),'ground',1,0);
INSERT INTO movement_class_terrain(movement_class_id,terrain_id,speed_pct) VALUES ((SELECT id FROM movement_class WHERE key='foot'),(SELECT id FROM def WHERE key='ter.oil'),100);
INSERT INTO movement_class_terrain(movement_class_id,terrain_id,speed_pct) VALUES ((SELECT id FROM movement_class WHERE key='wheeled'),(SELECT id FROM def WHERE key='ter.oil'),100);
INSERT INTO movement_class_terrain(movement_class_id,terrain_id,speed_pct) VALUES ((SELECT id FROM movement_class WHERE key='amphibious'),(SELECT id FROM def WHERE key='ter.oil'),100);
INSERT INTO map_style_feature(map_style_id,feature_def_id,region_kind,pattern,coverage_pct,clumps_min,clumps_max,size_min,size_max,start_min_dist,start_max_dist) VALUES ((SELECT id FROM def WHERE key='map.one_world'),(SELECT id FROM def WHERE key='ter.oil'),'planet','per_player',1,1,2,4,6,8,14);
INSERT INTO map_style_feature(map_style_id,feature_def_id,region_kind,pattern,coverage_pct,clumps_min,clumps_max,size_min,size_max,start_min_dist,start_max_dist) VALUES ((SELECT id FROM def WHERE key='map.many_worlds'),(SELECT id FROM def WHERE key='ter.oil'),'planet','per_player',1,1,2,4,6,8,14);
INSERT INTO damage_class(key) VALUES ('kinetic');
INSERT INTO damage_matrix VALUES ((SELECT id FROM damage_class WHERE key='kinetic'),(SELECT id FROM armor_class WHERE key='structure'),300);
INSERT INTO damage_matrix VALUES ((SELECT id FROM damage_class WHERE key='kinetic'),(SELECT id FROM armor_class WHERE key='flesh'),50);
INSERT INTO damage_matrix VALUES ((SELECT id FROM damage_class WHERE key='kinetic'),(SELECT id FROM armor_class WHERE key='plate'),50);
INSERT INTO damage_matrix VALUES ((SELECT id FROM damage_class WHERE key='kinetic'),(SELECT id FROM armor_class WHERE key='vehicle'),40);
INSERT INTO tag(key,category) VALUES ('building','structure');
INSERT INTO tag(key,category) VALUES ('fort','structure');
INSERT INTO tag(key,category) VALUES ('tower','structure');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.city','City');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.farm','Farm');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.woodcutter_camp','Woodcutter''s Camp');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.library','Library');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.market','Market');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.granary','Granary');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.lumber_mill','Lumber Mill');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.mine','Mine');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.university','University');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.temple','Temple');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.smelter','Smelter');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.oil_well','Oil Well');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.refinery','Refinery');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.fusion_plant','Fusion Plant');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.barracks','Barracks');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.tower','Tower');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.dock','Dock');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.stable','Stable');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.siege_workshop','Siege Workshop');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.fort','Fort');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.factory','Factory');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.vehicle_plant','Vehicle Plant');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.airfield','Airfield');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.spaceport','Spaceport');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.bastion','Bastion');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.bunker','Bunker');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.defence_turret','Defence Turret');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.star_fort','Star Fort');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.fortress','Fortress');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.citadel','Citadel');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.auto_farm','Automated Farm');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.auto_woodcutter_camp','Automated Woodcutter Camp');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.auto_mine','Automated Mine');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.auto_oil_well','Automated Oil Well');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.auto_lab','Auto Lab');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.citizen','Citizens');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.worker_drone','Worker Drones');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.spearman','Spearmen');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.hoplite','Hoplites');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.pikeman','Pikemen');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.musketeer','Musketeers');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.fusilier','Fusiliers');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.rifleman','Riflemen');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.infantry','Infantry');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.assault_infantry','Assault Infantry');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.combat_robot','Combat Robots');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.exo_trooper','Exo-Troopers');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.marine','Marines');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.shock_marine','Shock Marines');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.slinger','Slingers');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.archer','Archers');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.crossbowman','Crossbowmen');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.arquebusier','Arquebusiers');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.light_infantry','Light Infantry');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.machine_gunner','Machine Gunners');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.heavy_machine_gunner','Heavy Machine Gunners');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.gun_drone','Gun Drones');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.pulse_gunner','Pulse Gunners');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.beam_trooper','Beam Troopers');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.anti_tank_rifleman','Anti-Tank Riflemen');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.bazooka_team','Bazooka Teams');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.missile_team','Missile Teams');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.railgun_team','Railgun Teams');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.scout','Scouts');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.ranger','Rangers');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.recon_drone','Recon Drones');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.medic','Medics');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.horseman','Horsemen');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.light_cavalry','Light Cavalry');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.hussar','Hussars');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.dragoon','Dragoons');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.armoured_car','Armoured Cars');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.scout_car','Scout Cars');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.hovercraft','Hovercraft');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.grav_skimmer','Grav Skimmers');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.cataphract','Cataphracts');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.knight','Knights');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.cuirassier','Cuirassiers');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.lancer','Lancers');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.light_tank','Light Tanks');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.tank','Tanks');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.main_battle_tank','Main Battle Tanks');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.robot_tank','Robot Tanks');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.fusion_tank','Fusion Tanks');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.grav_tank','Grav Tanks');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.catapult','Catapults');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.trebuchet','Trebuchets');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.bombard','Bombards');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.cannon','Cannons');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.field_gun','Field Guns');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.howitzer','Howitzers');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.rocket_artillery','Rocket Artillery');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.railgun_battery','Railgun Batteries');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.flak_gun','Flak Guns');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.missile_launcher','Missile Launchers');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.laser_battery','Laser Batteries');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.supply_wagon','Supply Wagons');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.supply_truck','Supply Trucks');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.repair_drone','Repair Drones');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.trireme','Triremes');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.war_galley','War Galleys');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.carrack','Carracks');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.frigate','Frigates');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.ironclad','Ironclads');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.destroyer','Destroyers');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.missile_cruiser','Missile Cruisers');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.drone_cruiser','Drone Cruisers');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.hover_frigate','Hover Frigates');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.biplane','Biplanes');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.fighter','Fighters');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.jet_fighter','Jet Fighters');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.drone_fighter','Drone Fighters');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.interceptor','Interceptors');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.bomber','Bombers');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.strategic_bomber','Strategic Bombers');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.drone_bomber','Drone Bombers');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.survey_ship','Survey Ship');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.corvette','Corvettes');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.space_frigate','Space Frigates');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.space_cruiser','Space Cruisers');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.battleship','Battleships');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.scholar','Scholars');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.merchant','Merchants');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.caravan','Caravans');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.freight_wagon','Freight Wagons');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.lorry','Lorries');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.cargo_drone','Cargo Drones');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.priest','Priests');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.transport_ship','Transport Ships');
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.city'),'building','economic',1200,3,(SELECT id FROM armor_class WHERE key='structure'),15500,7,7,150,'per_type','none',0,(SELECT id FROM def WHERE key='age.ancient'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.city'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.city'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.city'),(SELECT id FROM def WHERE key='res.timber'),300,6);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.city'),'construct','economic',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.city'),'construct','economic',0,0);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.city'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.city'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.city'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.city'),(SELECT id FROM tag WHERE key='city'));
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) VALUES ((SELECT id FROM def WHERE key='bld.city'),1,40000,16000,1);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='bld.city'),8,(SELECT id FROM damage_class WHERE key='pierce'),10000,30,'arrow',15000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.city')),'ground');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.city')),'fluid');
INSERT INTO et_container(entity_type_id,capacity,heals,adds_firepower) VALUES ((SELECT id FROM def WHERE key='bld.city'),10,1,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.farm'),'building','economic',400,3,(SELECT id FROM armor_class WHERE key='structure'),14000,4,4,150,'per_type','per_city',5,(SELECT id FROM def WHERE key='age.ancient'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.farm'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.farm'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.farm'),(SELECT id FROM def WHERE key='res.timber'),50,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.farm'),'construct','economic',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.farm'),'construct','economic',1,0);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.farm'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.farm'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.farm'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.farm'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_max,slots_per_node,node_radius,needs_node_terrain_id,self_income) VALUES ((SELECT id FROM def WHERE key='bld.farm'),(SELECT id FROM def WHERE key='res.food'),1,1,0,0,NULL,0);
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.farm'),(SELECT id FROM tag WHERE key='gather'));
INSERT INTO completion_bonus(entity_type_id,resource_id,amount) VALUES ((SELECT id FROM def WHERE key='bld.farm'),(SELECT id FROM def WHERE key='res.food'),20);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.woodcutter_camp'),'building','economic',500,3,(SELECT id FROM armor_class WHERE key='structure'),14000,4,4,150,'per_type','none',0,(SELECT id FROM def WHERE key='age.ancient'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.woodcutter_camp'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.woodcutter_camp'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.woodcutter_camp'),(SELECT id FROM def WHERE key='res.timber'),60,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.woodcutter_camp'),'construct','economic',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.woodcutter_camp'),'construct','economic',2,0);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.woodcutter_camp'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.woodcutter_camp'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.woodcutter_camp'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.woodcutter_camp'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_max,slots_per_node,node_radius,needs_node_terrain_id,self_income) VALUES ((SELECT id FROM def WHERE key='bld.woodcutter_camp'),(SELECT id FROM def WHERE key='res.timber'),0,10,1,7000,(SELECT id FROM def WHERE key='ter.forest'),0);
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.woodcutter_camp'),(SELECT id FROM tag WHERE key='gather'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.library'),'building','economic',800,3,(SELECT id FROM armor_class WHERE key='structure'),14000,4,4,150,'per_type','per_city',1,(SELECT id FROM def WHERE key='age.ancient'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='res.timber'),60,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.library'),'construct','economic',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.library'),'construct','economic',3,0);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.library'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.library'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.library'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.library'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.market'),'building','economic',800,3,(SELECT id FROM armor_class WHERE key='structure'),14000,4,4,150,'per_type','per_city',1,(SELECT id FROM def WHERE key='age.ancient'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.market'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.market'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.market'),(SELECT id FROM def WHERE key='res.timber'),80,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.market'),'construct','economic',4,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.market'),'construct','economic',4,0);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.market'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.market'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.market'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.market'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO completion_bonus(entity_type_id,resource_id,amount) VALUES ((SELECT id FROM def WHERE key='bld.market'),(SELECT id FROM def WHERE key='res.wealth'),20);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.granary'),'building','economic',880,3,(SELECT id FROM armor_class WHERE key='structure'),14000,4,4,175,'per_type','per_city',1,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.granary'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.granary'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.granary'),(SELECT id FROM def WHERE key='res.timber'),80,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.granary'),'construct','economic',0,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.granary'),'construct','economic',0,1);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.granary'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.granary'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.granary'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.granary'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.lumber_mill'),'building','economic',880,3,(SELECT id FROM armor_class WHERE key='structure'),14000,4,4,175,'per_type','per_city',1,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.lumber_mill'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.lumber_mill'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.lumber_mill'),(SELECT id FROM def WHERE key='res.timber'),80,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.lumber_mill'),'construct','economic',1,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.lumber_mill'),'construct','economic',1,1);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.lumber_mill'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.lumber_mill'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.lumber_mill'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.lumber_mill'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.mine'),'building','economic',660,3,(SELECT id FROM armor_class WHERE key='structure'),14000,4,4,175,'per_type','none',0,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.mine'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.mine'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.mine'),(SELECT id FROM def WHERE key='res.timber'),80,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.mine'),'construct','economic',2,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.mine'),'construct','economic',2,1);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.mine'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.mine'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.mine'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.mine'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_max,slots_per_node,node_radius,needs_node_terrain_id,self_income) VALUES ((SELECT id FROM def WHERE key='bld.mine'),(SELECT id FROM def WHERE key='res.metal'),0,10,1,7000,(SELECT id FROM def WHERE key='ter.mountain'),0);
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.mine'),(SELECT id FROM tag WHERE key='gather'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.university'),'building','economic',880,3,(SELECT id FROM armor_class WHERE key='structure'),14000,4,4,175,'per_type','per_city',1,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM def WHERE key='res.timber'),100,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.university'),'construct','economic',3,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.university'),'construct','economic',3,1);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.university'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.university'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.university'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.university'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_max,slots_per_node,node_radius,needs_node_terrain_id,self_income) VALUES ((SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM def WHERE key='res.knowledge'),7,7,0,0,NULL,0);
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM tag WHERE key='gather'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.temple'),'building','economic',880,3,(SELECT id FROM armor_class WHERE key='structure'),14000,4,4,175,'per_type','per_city',1,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.temple'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.temple'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.temple'),(SELECT id FROM def WHERE key='res.timber'),80,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.temple'),'construct','economic',4,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.temple'),'construct','economic',4,1);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.temple'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.temple'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.temple'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.temple'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.temple'),(SELECT id FROM tag WHERE key='temple'));
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) VALUES ((SELECT id FROM def WHERE key='bld.temple'),0,2000,0,0);
INSERT INTO et_keeper(entity_type_id,capacity) VALUES ((SELECT id FROM def WHERE key='bld.temple'),5);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.smelter'),'building','economic',1200,3,(SELECT id FROM armor_class WHERE key='structure'),14000,4,4,200,'per_type','per_city',1,(SELECT id FROM def WHERE key='age.medieval'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.smelter'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.smelter'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.smelter'),(SELECT id FROM def WHERE key='res.timber'),120,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.smelter'),(SELECT id FROM def WHERE key='res.metal'),60,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.smelter'),'construct','economic',0,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.smelter'),'construct','economic',0,2);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.smelter'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.smelter'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.smelter'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.smelter'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.oil_well'),'building','economic',900,3,(SELECT id FROM armor_class WHERE key='structure'),14000,2,2,275,'per_type','none',0,(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.oil_well'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.oil_well'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.oil_well'),(SELECT id FROM def WHERE key='res.timber'),120,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.oil_well'),(SELECT id FROM def WHERE key='res.metal'),60,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.oil_well'),'construct','economic',1,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.oil_well'),'construct','economic',1,2);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.oil_well'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.oil_well'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.oil_well'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.oil_well'),'on_terrain',(SELECT id FROM def WHERE key='ter.oil'));
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_max,slots_per_node,node_radius,needs_node_terrain_id,self_income) VALUES ((SELECT id FROM def WHERE key='bld.oil_well'),(SELECT id FROM def WHERE key='res.oil'),3,3,0,0,NULL,0);
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.oil_well'),(SELECT id FROM tag WHERE key='gather'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.refinery'),'building','economic',1800,3,(SELECT id FROM armor_class WHERE key='structure'),15000,4,4,275,'per_type','per_city',1,(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.refinery'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.refinery'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.refinery'),(SELECT id FROM def WHERE key='res.timber'),160,3);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.refinery'),(SELECT id FROM def WHERE key='res.metal'),80,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.refinery'),'construct','economic',2,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.refinery'),'construct','economic',2,2);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.refinery'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.refinery'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.refinery'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.refinery'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),'building','economic',3420,3,(SELECT id FROM armor_class WHERE key='structure'),17000,4,4,375,'per_type','per_city',1,(SELECT id FROM def WHERE key='age.fusion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),(SELECT id FROM def WHERE key='res.timber'),200,4);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),(SELECT id FROM def WHERE key='res.metal'),100,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.fusion_plant'),'construct','economic',3,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.fusion_plant'),'construct','economic',3,2);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_max,slots_per_node,node_radius,needs_node_terrain_id,self_income) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),(SELECT id FROM def WHERE key='res.energy'),0,0,0,0,NULL,40);
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),(SELECT id FROM tag WHERE key='gather'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),'building','military',1000,3,(SELECT id FROM armor_class WHERE key='structure'),14000,4,4,150,'per_type','none',0,(SELECT id FROM def WHERE key='age.ancient'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='res.timber'),60,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.barracks'),'construct','military',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.barracks'),'construct','military',0,0);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.barracks'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.tower'),'building','military',1100,3,(SELECT id FROM armor_class WHERE key='structure'),15000,2,2,175,'per_type','none',0,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.tower'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.tower'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.tower'),(SELECT id FROM def WHERE key='res.timber'),80,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.tower'),'construct','military',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.tower'),'construct','military',1,0);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.tower'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.tower'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.tower'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.tower'),(SELECT id FROM tag WHERE key='tower'));
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='bld.tower'),10,(SELECT id FROM damage_class WHERE key='pierce'),10000,30,'arrow',15000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.tower')),'ground');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.tower')),'fluid');
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.dock'),'building','military',1000,3,(SELECT id FROM armor_class WHERE key='structure'),14000,4,4,150,'per_type','none',0,(SELECT id FROM def WHERE key='age.ancient'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='res.timber'),100,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.dock'),'construct','military',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.dock'),'construct','military',2,0);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.dock'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.dock'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.dock'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id,distance) VALUES ((SELECT id FROM def WHERE key='bld.dock'),'adjacent_terrain',(SELECT id FROM def WHERE key='ter.coast'),2000);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.stable'),'building','military',1100,3,(SELECT id FROM armor_class WHERE key='structure'),14000,4,4,175,'per_type','none',0,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM def WHERE key='res.timber'),100,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.stable'),'construct','military',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.stable'),'construct','military',3,0);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.stable'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.stable'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.stable'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),'building','military',1320,3,(SELECT id FROM armor_class WHERE key='structure'),14000,4,4,175,'per_type','none',0,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),(SELECT id FROM def WHERE key='res.timber'),120,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.siege_workshop'),'construct','military',4,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.siege_workshop'),'construct','military',4,0);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.fort'),'building','military',2640,3,(SELECT id FROM armor_class WHERE key='structure'),15000,6,6,200,'per_type','none',0,(SELECT id FROM def WHERE key='age.medieval'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.fort'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.fort'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.fort'),(SELECT id FROM def WHERE key='res.timber'),200,4);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.fort'),(SELECT id FROM def WHERE key='res.metal'),100,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.fort'),'construct','military',0,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.fort'),'construct','military',0,1);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.fort'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.fort'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.fort'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.fort'),(SELECT id FROM tag WHERE key='fort'));
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) VALUES ((SELECT id FROM def WHERE key='bld.fort'),0,14000,0,0);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='bld.fort'),12,(SELECT id FROM damage_class WHERE key='pierce'),10000,30,'arrow',15000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fort')),'ground');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fort')),'fluid');
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.factory'),'building','military',2700,3,(SELECT id FROM armor_class WHERE key='structure'),15000,4,4,275,'per_type','none',0,(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='res.timber'),200,4);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='res.metal'),100,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.factory'),'construct','military',1,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.factory'),'construct','military',1,1);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.factory'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.factory'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.factory'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),'building','military',2700,3,(SELECT id FROM armor_class WHERE key='structure'),15000,4,4,275,'per_type','none',0,(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='res.timber'),200,4);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='res.metal'),100,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.vehicle_plant'),'construct','military',2,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.vehicle_plant'),'construct','military',2,1);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),'building','military',2700,3,(SELECT id FROM armor_class WHERE key='structure'),16000,6,6,275,'per_type','none',0,(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='res.timber'),200,4);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='res.metal'),100,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.airfield'),'construct','military',3,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.airfield'),'construct','military',3,1);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.airfield'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),'building','military',5000,3,(SELECT id FROM armor_class WHERE key='structure'),19000,8,8,400,'per_type','none',0,(SELECT id FROM def WHERE key='age.starfaring'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='res.timber'),300,6);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='res.metal'),150,3);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.spaceport'),'construct','military',4,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.spaceport'),'construct','military',4,1);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM tag WHERE key='production'));
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),0,12000,0,0);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,first_age_id,upgrades_from_id) VALUES ((SELECT id FROM def WHERE key='bld.bastion'),'building','military',2100,5,(SELECT id FROM armor_class WHERE key='structure'),15000,2,2,250,(SELECT id FROM def WHERE key='age.enlightenment'),(SELECT id FROM def WHERE key='bld.tower'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.bastion'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.bastion'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.bastion'),(SELECT id FROM tag WHERE key='tower'));
INSERT INTO et_producer(entity_type_id,queue_limit,parallel,has_rally) SELECT (SELECT id FROM def WHERE key='bld.bastion'),queue_limit,parallel,has_rally FROM et_producer WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.tower');
INSERT INTO et_keeper(entity_type_id,capacity) SELECT (SELECT id FROM def WHERE key='bld.bastion'),capacity FROM et_keeper WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.tower');
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) SELECT (SELECT id FROM def WHERE key='bld.bastion'),is_city,border_radius,economic_radius,capturable FROM et_territory WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.tower');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id,layer,distance) SELECT (SELECT id FROM def WHERE key='bld.bastion'),rule,ref_def_id,layer,distance FROM placement_rule WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.tower');
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income) SELECT (SELECT id FROM def WHERE key='bld.bastion'),resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income FROM et_gather_site WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.tower');
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='bld.bastion'),20,(SELECT id FROM damage_class WHERE key='explosive'),10000,30,'shell',24000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bastion')),'ground');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bastion')),'fluid');
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.bastion','Upgrade to Bastion');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.bastion'),'upgrade','',0,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.bastion'),(SELECT id FROM def WHERE key='res.timber'),300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.bastion'),(SELECT id FROM def WHERE key='res.metal'),150,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.bastion'),'upgrade_type',(SELECT id FROM def WHERE key='bld.tower'),(SELECT id FROM def WHERE key='bld.bastion'));
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.bastion'),'construct','military',0,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.bastion'),'construct','military',0,2);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,first_age_id,upgrades_from_id) VALUES ((SELECT id FROM def WHERE key='bld.bunker'),'building','military',2400,5,(SELECT id FROM armor_class WHERE key='structure'),15000,2,2,300,(SELECT id FROM def WHERE key='age.modern'),(SELECT id FROM def WHERE key='bld.bastion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.bunker'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.bunker'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.bunker'),(SELECT id FROM tag WHERE key='tower'));
INSERT INTO et_producer(entity_type_id,queue_limit,parallel,has_rally) SELECT (SELECT id FROM def WHERE key='bld.bunker'),queue_limit,parallel,has_rally FROM et_producer WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bastion');
INSERT INTO et_keeper(entity_type_id,capacity) SELECT (SELECT id FROM def WHERE key='bld.bunker'),capacity FROM et_keeper WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bastion');
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) SELECT (SELECT id FROM def WHERE key='bld.bunker'),is_city,border_radius,economic_radius,capturable FROM et_territory WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bastion');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id,layer,distance) SELECT (SELECT id FROM def WHERE key='bld.bunker'),rule,ref_def_id,layer,distance FROM placement_rule WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bastion');
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income) SELECT (SELECT id FROM def WHERE key='bld.bunker'),resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income FROM et_gather_site WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bastion');
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='bld.bunker'),24,(SELECT id FROM damage_class WHERE key='explosive'),10000,30,'bullet',40000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bunker')),'ground');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bunker')),'fluid');
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.bunker','Upgrade to Bunker');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.bunker'),'upgrade','',0,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.bunker'),(SELECT id FROM def WHERE key='res.timber'),300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.bunker'),(SELECT id FROM def WHERE key='res.metal'),150,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.bunker'),'upgrade_type',(SELECT id FROM def WHERE key='bld.bastion'),(SELECT id FROM def WHERE key='bld.bunker'));
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.bunker'),'construct','military',1,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.bunker'),'construct','military',1,2);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,first_age_id,upgrades_from_id) VALUES ((SELECT id FROM def WHERE key='bld.defence_turret'),'building','military',2700,5,(SELECT id FROM armor_class WHERE key='structure'),16000,2,2,350,(SELECT id FROM def WHERE key='age.automation'),(SELECT id FROM def WHERE key='bld.bunker'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.defence_turret'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.defence_turret'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.defence_turret'),(SELECT id FROM tag WHERE key='tower'));
INSERT INTO et_producer(entity_type_id,queue_limit,parallel,has_rally) SELECT (SELECT id FROM def WHERE key='bld.defence_turret'),queue_limit,parallel,has_rally FROM et_producer WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bunker');
INSERT INTO et_keeper(entity_type_id,capacity) SELECT (SELECT id FROM def WHERE key='bld.defence_turret'),capacity FROM et_keeper WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bunker');
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) SELECT (SELECT id FROM def WHERE key='bld.defence_turret'),is_city,border_radius,economic_radius,capturable FROM et_territory WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bunker');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id,layer,distance) SELECT (SELECT id FROM def WHERE key='bld.defence_turret'),rule,ref_def_id,layer,distance FROM placement_rule WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bunker');
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income) SELECT (SELECT id FROM def WHERE key='bld.defence_turret'),resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income FROM et_gather_site WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bunker');
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile) VALUES ((SELECT id FROM def WHERE key='bld.defence_turret'),28,(SELECT id FROM damage_class WHERE key='energy'),10000,30,'beam');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.defence_turret')),'ground');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.defence_turret')),'fluid');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.defence_turret')),'air');
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.defence_turret','Upgrade to Defence Turret');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.defence_turret'),'upgrade','',0,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.defence_turret'),(SELECT id FROM def WHERE key='res.timber'),300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.defence_turret'),(SELECT id FROM def WHERE key='res.metal'),150,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.defence_turret'),'upgrade_type',(SELECT id FROM def WHERE key='bld.bunker'),(SELECT id FROM def WHERE key='bld.defence_turret'));
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.defence_turret'),'construct','military',2,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.defence_turret'),'construct','military',2,2);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,first_age_id,upgrades_from_id) VALUES ((SELECT id FROM def WHERE key='bld.star_fort'),'building','military',2100,5,(SELECT id FROM armor_class WHERE key='structure'),16000,6,6,250,(SELECT id FROM def WHERE key='age.enlightenment'),(SELECT id FROM def WHERE key='bld.fort'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.star_fort'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.star_fort'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.star_fort'),(SELECT id FROM tag WHERE key='fort'));
INSERT INTO et_producer(entity_type_id,queue_limit,parallel,has_rally) SELECT (SELECT id FROM def WHERE key='bld.star_fort'),queue_limit,parallel,has_rally FROM et_producer WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fort');
INSERT INTO et_keeper(entity_type_id,capacity) SELECT (SELECT id FROM def WHERE key='bld.star_fort'),capacity FROM et_keeper WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fort');
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) SELECT (SELECT id FROM def WHERE key='bld.star_fort'),is_city,border_radius,economic_radius,capturable FROM et_territory WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fort');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id,layer,distance) SELECT (SELECT id FROM def WHERE key='bld.star_fort'),rule,ref_def_id,layer,distance FROM placement_rule WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fort');
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income) SELECT (SELECT id FROM def WHERE key='bld.star_fort'),resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income FROM et_gather_site WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fort');
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='bld.star_fort'),20,(SELECT id FROM damage_class WHERE key='explosive'),10000,30,'shell',24000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.star_fort')),'ground');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.star_fort')),'fluid');
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.star_fort','Upgrade to Star Fort');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.star_fort'),'upgrade','',0,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.star_fort'),(SELECT id FROM def WHERE key='res.timber'),300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.star_fort'),(SELECT id FROM def WHERE key='res.metal'),150,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.star_fort'),'upgrade_type',(SELECT id FROM def WHERE key='bld.fort'),(SELECT id FROM def WHERE key='bld.star_fort'));
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.star_fort'),'construct','military',3,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.star_fort'),'construct','military',3,2);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,first_age_id,upgrades_from_id) VALUES ((SELECT id FROM def WHERE key='bld.fortress'),'building','military',2400,5,(SELECT id FROM armor_class WHERE key='structure'),16000,6,6,300,(SELECT id FROM def WHERE key='age.modern'),(SELECT id FROM def WHERE key='bld.star_fort'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.fortress'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.fortress'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.fortress'),(SELECT id FROM tag WHERE key='fort'));
INSERT INTO et_producer(entity_type_id,queue_limit,parallel,has_rally) SELECT (SELECT id FROM def WHERE key='bld.fortress'),queue_limit,parallel,has_rally FROM et_producer WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.star_fort');
INSERT INTO et_keeper(entity_type_id,capacity) SELECT (SELECT id FROM def WHERE key='bld.fortress'),capacity FROM et_keeper WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.star_fort');
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) SELECT (SELECT id FROM def WHERE key='bld.fortress'),is_city,border_radius,economic_radius,capturable FROM et_territory WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.star_fort');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id,layer,distance) SELECT (SELECT id FROM def WHERE key='bld.fortress'),rule,ref_def_id,layer,distance FROM placement_rule WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.star_fort');
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income) SELECT (SELECT id FROM def WHERE key='bld.fortress'),resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income FROM et_gather_site WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.star_fort');
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='bld.fortress'),24,(SELECT id FROM damage_class WHERE key='explosive'),10000,30,'shell',24000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fortress')),'ground');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fortress')),'fluid');
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.fortress','Upgrade to Fortress');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.fortress'),'upgrade','',0,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.fortress'),(SELECT id FROM def WHERE key='res.timber'),300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.fortress'),(SELECT id FROM def WHERE key='res.metal'),150,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.fortress'),'upgrade_type',(SELECT id FROM def WHERE key='bld.star_fort'),(SELECT id FROM def WHERE key='bld.fortress'));
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.fortress'),'construct','military',4,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.fortress'),'construct','military',4,2);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,first_age_id,upgrades_from_id) VALUES ((SELECT id FROM def WHERE key='bld.citadel'),'building','military',2850,5,(SELECT id FROM armor_class WHERE key='structure'),18000,6,6,375,(SELECT id FROM def WHERE key='age.fusion'),(SELECT id FROM def WHERE key='bld.fortress'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.citadel'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.citadel'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.citadel'),(SELECT id FROM tag WHERE key='fort'));
INSERT INTO et_producer(entity_type_id,queue_limit,parallel,has_rally) SELECT (SELECT id FROM def WHERE key='bld.citadel'),queue_limit,parallel,has_rally FROM et_producer WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fortress');
INSERT INTO et_keeper(entity_type_id,capacity) SELECT (SELECT id FROM def WHERE key='bld.citadel'),capacity FROM et_keeper WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fortress');
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) SELECT (SELECT id FROM def WHERE key='bld.citadel'),is_city,border_radius,economic_radius,capturable FROM et_territory WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fortress');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id,layer,distance) SELECT (SELECT id FROM def WHERE key='bld.citadel'),rule,ref_def_id,layer,distance FROM placement_rule WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fortress');
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income) SELECT (SELECT id FROM def WHERE key='bld.citadel'),resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income FROM et_gather_site WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fortress');
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='bld.citadel'),30,(SELECT id FROM damage_class WHERE key='energy'),10000,30,'plasma',30000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.citadel')),'ground');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.citadel')),'fluid');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.citadel')),'air');
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.citadel','Upgrade to Citadel');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.citadel'),'upgrade','',0,750);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.citadel'),(SELECT id FROM def WHERE key='res.timber'),300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.citadel'),(SELECT id FROM def WHERE key='res.metal'),150,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.citadel'),'upgrade_type',(SELECT id FROM def WHERE key='bld.fortress'),(SELECT id FROM def WHERE key='bld.citadel'));
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.citadel'),'construct','military_2',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.citadel'),'construct','military_2',0,0);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.spearman'),'unit','infantry',100,2,(SELECT id FROM armor_class WHERE key='flesh'),5000,1,150,'per_type',NULL,(SELECT id FROM def WHERE key='age.ancient'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.spearman'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.spearman'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.spearman'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.spearman'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.spearman'),(SELECT id FROM movement_class WHERE key='foot'),1250);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.spearman'),9,(SELECT id FROM damage_class WHERE key='melee'),0,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.spearman')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.spearman'),'train','units',0,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.spearman'),(SELECT id FROM def WHERE key='res.food'),40,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.spearman'),(SELECT id FROM def WHERE key='res.timber'),20,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.hoplite'),'unit','infantry',115,2,(SELECT id FROM armor_class WHERE key='plate'),5000,1,175,'per_type',(SELECT id FROM def WHERE key='unit.spearman'),(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hoplite'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hoplite'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hoplite'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hoplite'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.hoplite'),(SELECT id FROM movement_class WHERE key='foot'),1250);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.hoplite'),10,(SELECT id FROM damage_class WHERE key='melee'),0,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.hoplite')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.hoplite'),'train','units',1,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.hoplite'),(SELECT id FROM def WHERE key='res.food'),46,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.hoplite'),(SELECT id FROM def WHERE key='res.timber'),23,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.hoplite','Upgrade to Hoplites');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.hoplite'),'unit_upgrade','',0,350);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.hoplite'),(SELECT id FROM def WHERE key='res.food'),92,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.hoplite'),(SELECT id FROM def WHERE key='res.timber'),46,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.hoplite'),'upgrade_type',(SELECT id FROM def WHERE key='unit.spearman'),(SELECT id FROM def WHERE key='unit.hoplite'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.pikeman'),'unit','infantry',130,2,(SELECT id FROM armor_class WHERE key='plate'),5000,1,200,'per_type',(SELECT id FROM def WHERE key='unit.hoplite'),(SELECT id FROM def WHERE key='age.medieval'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.pikeman'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.pikeman'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.pikeman'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.pikeman'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.pikeman'),(SELECT id FROM movement_class WHERE key='foot'),1250);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.pikeman'),11,(SELECT id FROM damage_class WHERE key='melee'),0,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.pikeman')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.pikeman'),'train','units',2,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.pikeman'),(SELECT id FROM def WHERE key='res.food'),52,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.pikeman'),(SELECT id FROM def WHERE key='res.metal'),26,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.pikeman','Upgrade to Pikemen');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.pikeman'),'unit_upgrade','',0,400);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.pikeman'),(SELECT id FROM def WHERE key='res.food'),104,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.pikeman'),(SELECT id FROM def WHERE key='res.metal'),52,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.pikeman'),'upgrade_type',(SELECT id FROM def WHERE key='unit.hoplite'),(SELECT id FROM def WHERE key='unit.pikeman'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.musketeer'),'unit','infantry',145,2,(SELECT id FROM armor_class WHERE key='plate'),6000,1,225,'per_type',(SELECT id FROM def WHERE key='unit.pikeman'),(SELECT id FROM def WHERE key='age.gunpowder'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.musketeer'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.musketeer'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.musketeer'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.musketeer'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.musketeer'),(SELECT id FROM movement_class WHERE key='foot'),1250);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.musketeer'),13,(SELECT id FROM damage_class WHERE key='bullet'),5000,30,'bullet',40000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.musketeer')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.musketeer'),'train','units',3,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.musketeer'),(SELECT id FROM def WHERE key='res.food'),58,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.musketeer'),(SELECT id FROM def WHERE key='res.metal'),29,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.musketeer','Upgrade to Musketeers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.musketeer'),'unit_upgrade','',0,450);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.musketeer'),(SELECT id FROM def WHERE key='res.food'),116,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.musketeer'),(SELECT id FROM def WHERE key='res.metal'),58,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.musketeer'),'upgrade_type',(SELECT id FROM def WHERE key='unit.pikeman'),(SELECT id FROM def WHERE key='unit.musketeer'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.fusilier'),'unit','infantry',160,2,(SELECT id FROM armor_class WHERE key='plate'),6000,1,250,'per_type',(SELECT id FROM def WHERE key='unit.musketeer'),(SELECT id FROM def WHERE key='age.enlightenment'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.fusilier'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.fusilier'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.fusilier'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.fusilier'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.fusilier'),(SELECT id FROM movement_class WHERE key='foot'),1250);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.fusilier'),14,(SELECT id FROM damage_class WHERE key='bullet'),5000,30,'bullet',40000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.fusilier')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.fusilier'),'train','units',4,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.fusilier'),(SELECT id FROM def WHERE key='res.food'),64,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.fusilier'),(SELECT id FROM def WHERE key='res.metal'),32,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.fusilier','Upgrade to Fusiliers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.fusilier'),'unit_upgrade','',0,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.fusilier'),(SELECT id FROM def WHERE key='res.food'),128,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.fusilier'),(SELECT id FROM def WHERE key='res.metal'),64,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.fusilier'),'upgrade_type',(SELECT id FROM def WHERE key='unit.musketeer'),(SELECT id FROM def WHERE key='unit.fusilier'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.rifleman'),'unit','infantry',175,2,(SELECT id FROM armor_class WHERE key='plate'),6000,1,275,'per_type',(SELECT id FROM def WHERE key='unit.fusilier'),(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.rifleman'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.rifleman'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.rifleman'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.rifleman'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.rifleman'),(SELECT id FROM movement_class WHERE key='foot'),1250);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.rifleman'),15,(SELECT id FROM damage_class WHERE key='bullet'),5000,30,'bullet',40000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.rifleman')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.rifleman'),'train','units',0,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.rifleman'),(SELECT id FROM def WHERE key='res.food'),70,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.rifleman'),(SELECT id FROM def WHERE key='res.metal'),35,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.rifleman','Upgrade to Riflemen');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.rifleman'),'unit_upgrade','',0,550);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.rifleman'),(SELECT id FROM def WHERE key='res.food'),140,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.rifleman'),(SELECT id FROM def WHERE key='res.metal'),70,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.rifleman'),'upgrade_type',(SELECT id FROM def WHERE key='unit.fusilier'),(SELECT id FROM def WHERE key='unit.rifleman'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.infantry'),'unit','infantry',190,2,(SELECT id FROM armor_class WHERE key='plate'),6000,1,300,'per_type',(SELECT id FROM def WHERE key='unit.rifleman'),(SELECT id FROM def WHERE key='age.modern'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.infantry'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.infantry'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.infantry'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.infantry'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.infantry'),(SELECT id FROM movement_class WHERE key='foot'),1250);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.infantry'),17,(SELECT id FROM damage_class WHERE key='bullet'),5000,30,'bullet',40000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.infantry')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.infantry'),'train','units',1,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.infantry'),(SELECT id FROM def WHERE key='res.food'),76,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.infantry'),(SELECT id FROM def WHERE key='res.metal'),38,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.infantry','Upgrade to Infantry');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.infantry'),'unit_upgrade','',0,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.infantry'),(SELECT id FROM def WHERE key='res.food'),152,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.infantry'),(SELECT id FROM def WHERE key='res.metal'),76,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.infantry'),'upgrade_type',(SELECT id FROM def WHERE key='unit.rifleman'),(SELECT id FROM def WHERE key='unit.infantry'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.assault_infantry'),'unit','infantry',205,2,(SELECT id FROM armor_class WHERE key='plate'),6000,1,325,'per_type',(SELECT id FROM def WHERE key='unit.infantry'),(SELECT id FROM def WHERE key='age.information'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.assault_infantry'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.assault_infantry'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.assault_infantry'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.assault_infantry'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.assault_infantry'),(SELECT id FROM movement_class WHERE key='foot'),1250);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.assault_infantry'),18,(SELECT id FROM damage_class WHERE key='bullet'),5000,30,'bullet',40000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.assault_infantry')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.assault_infantry'),'train','units',2,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.assault_infantry'),(SELECT id FROM def WHERE key='res.food'),82,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.assault_infantry'),(SELECT id FROM def WHERE key='res.metal'),41,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.assault_infantry','Upgrade to Assault Infantry');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.assault_infantry'),'unit_upgrade','',0,650);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.assault_infantry'),(SELECT id FROM def WHERE key='res.food'),164,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.assault_infantry'),(SELECT id FROM def WHERE key='res.metal'),82,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.assault_infantry'),'upgrade_type',(SELECT id FROM def WHERE key='unit.infantry'),(SELECT id FROM def WHERE key='unit.assault_infantry'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.combat_robot'),'unit','infantry',220,2,(SELECT id FROM armor_class WHERE key='plate'),6000,1,350,'per_type',(SELECT id FROM def WHERE key='unit.assault_infantry'),(SELECT id FROM def WHERE key='age.automation'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.combat_robot'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.combat_robot'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.combat_robot'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.combat_robot'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.combat_robot'),(SELECT id FROM movement_class WHERE key='wheeled'),1250);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.combat_robot'),19,(SELECT id FROM damage_class WHERE key='bullet'),5000,30,'bullet',40000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.combat_robot')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.combat_robot'),'train','units',3,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.combat_robot'),(SELECT id FROM def WHERE key='res.food'),88,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.combat_robot'),(SELECT id FROM def WHERE key='res.metal'),44,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.combat_robot','Upgrade to Combat Robots');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.combat_robot'),'unit_upgrade','',0,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.combat_robot'),(SELECT id FROM def WHERE key='res.food'),176,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.combat_robot'),(SELECT id FROM def WHERE key='res.metal'),88,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.combat_robot'),'upgrade_type',(SELECT id FROM def WHERE key='unit.assault_infantry'),(SELECT id FROM def WHERE key='unit.combat_robot'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.exo_trooper'),'unit','infantry',235,2,(SELECT id FROM armor_class WHERE key='plate'),6000,1,375,'per_type',(SELECT id FROM def WHERE key='unit.combat_robot'),(SELECT id FROM def WHERE key='age.fusion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.exo_trooper'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.exo_trooper'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.exo_trooper'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.exo_trooper'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.exo_trooper'),(SELECT id FROM movement_class WHERE key='foot'),1250);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.exo_trooper'),21,(SELECT id FROM damage_class WHERE key='bullet'),5000,30,'bullet',40000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.exo_trooper')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.exo_trooper'),'train','units',4,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.exo_trooper'),(SELECT id FROM def WHERE key='res.food'),94,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.exo_trooper'),(SELECT id FROM def WHERE key='res.metal'),47,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.exo_trooper','Upgrade to Exo-Troopers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.exo_trooper'),'unit_upgrade','',0,750);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.exo_trooper'),(SELECT id FROM def WHERE key='res.food'),188,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.exo_trooper'),(SELECT id FROM def WHERE key='res.metal'),94,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.exo_trooper'),'upgrade_type',(SELECT id FROM def WHERE key='unit.combat_robot'),(SELECT id FROM def WHERE key='unit.exo_trooper'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.marine'),'unit','infantry',250,2,(SELECT id FROM armor_class WHERE key='plate'),6000,1,400,'per_type',(SELECT id FROM def WHERE key='unit.exo_trooper'),(SELECT id FROM def WHERE key='age.starfaring'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.marine'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.marine'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.marine'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.marine'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.marine'),(SELECT id FROM movement_class WHERE key='foot'),1250);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.marine'),22,(SELECT id FROM damage_class WHERE key='energy'),5000,30,'plasma',30000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.marine')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.marine'),'train','units',0,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.marine'),(SELECT id FROM def WHERE key='res.food'),100,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.marine'),(SELECT id FROM def WHERE key='res.metal'),50,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.marine','Upgrade to Marines');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.marine'),'unit_upgrade','',0,800);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.marine'),(SELECT id FROM def WHERE key='res.food'),200,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.marine'),(SELECT id FROM def WHERE key='res.metal'),100,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.marine'),'upgrade_type',(SELECT id FROM def WHERE key='unit.exo_trooper'),(SELECT id FROM def WHERE key='unit.marine'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.shock_marine'),'unit','infantry',280,2,(SELECT id FROM armor_class WHERE key='plate'),6000,1,450,'per_type',(SELECT id FROM def WHERE key='unit.marine'),(SELECT id FROM def WHERE key='age.abundance'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.shock_marine'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.shock_marine'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.shock_marine'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.shock_marine'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.shock_marine'),(SELECT id FROM movement_class WHERE key='foot'),1250);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.shock_marine'),25,(SELECT id FROM damage_class WHERE key='energy'),5000,30,'plasma',30000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.shock_marine')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.shock_marine'),'train','units',1,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.shock_marine'),(SELECT id FROM def WHERE key='res.food'),112,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.shock_marine'),(SELECT id FROM def WHERE key='res.metal'),56,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.shock_marine','Upgrade to Shock Marines');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.shock_marine'),'unit_upgrade','',0,900);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.shock_marine'),(SELECT id FROM def WHERE key='res.food'),224,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.shock_marine'),(SELECT id FROM def WHERE key='res.metal'),112,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.shock_marine'),'upgrade_type',(SELECT id FROM def WHERE key='unit.marine'),(SELECT id FROM def WHERE key='unit.shock_marine'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.slinger'),'unit','ranged',70,0,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,150,'per_type',NULL,(SELECT id FROM def WHERE key='age.ancient'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.slinger'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.slinger'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.slinger'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.slinger'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.slinger'),(SELECT id FROM movement_class WHERE key='foot'),1200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.slinger'),7,(SELECT id FROM damage_class WHERE key='pierce'),5000,30,'stone',10000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.slinger')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.slinger'),'train','units',2,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.slinger'),(SELECT id FROM def WHERE key='res.food'),30,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.slinger'),(SELECT id FROM def WHERE key='res.timber'),30,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.archer'),'unit','ranged',80,0,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,175,'per_type',(SELECT id FROM def WHERE key='unit.slinger'),(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.archer'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.archer'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.archer'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.archer'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.archer'),(SELECT id FROM movement_class WHERE key='foot'),1200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.archer'),8,(SELECT id FROM damage_class WHERE key='pierce'),5000,30,'arrow',15000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.archer')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.archer'),'train','units',3,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.archer'),(SELECT id FROM def WHERE key='res.food'),34,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.archer'),(SELECT id FROM def WHERE key='res.timber'),34,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.archer','Upgrade to Archers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.archer'),'unit_upgrade','',0,350);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.archer'),(SELECT id FROM def WHERE key='res.food'),68,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.archer'),(SELECT id FROM def WHERE key='res.timber'),68,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.archer'),'upgrade_type',(SELECT id FROM def WHERE key='unit.slinger'),(SELECT id FROM def WHERE key='unit.archer'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.crossbowman'),'unit','ranged',91,1,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,200,'per_type',(SELECT id FROM def WHERE key='unit.archer'),(SELECT id FROM def WHERE key='age.medieval'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.crossbowman'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.crossbowman'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.crossbowman'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.crossbowman'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.crossbowman'),(SELECT id FROM movement_class WHERE key='foot'),1200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.crossbowman'),9,(SELECT id FROM damage_class WHERE key='pierce'),5000,30,'bolt',20000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.crossbowman')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.crossbowman'),'train','units',4,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.crossbowman'),(SELECT id FROM def WHERE key='res.food'),39,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.crossbowman'),(SELECT id FROM def WHERE key='res.timber'),39,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.crossbowman','Upgrade to Crossbowmen');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.crossbowman'),'unit_upgrade','',0,400);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.crossbowman'),(SELECT id FROM def WHERE key='res.food'),78,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.crossbowman'),(SELECT id FROM def WHERE key='res.timber'),78,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.crossbowman'),'upgrade_type',(SELECT id FROM def WHERE key='unit.archer'),(SELECT id FROM def WHERE key='unit.crossbowman'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.arquebusier'),'unit','ranged',101,1,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,225,'per_type',(SELECT id FROM def WHERE key='unit.crossbowman'),(SELECT id FROM def WHERE key='age.gunpowder'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.arquebusier'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.arquebusier'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.arquebusier'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.arquebusier'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.arquebusier'),(SELECT id FROM movement_class WHERE key='foot'),1200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.arquebusier'),10,(SELECT id FROM damage_class WHERE key='bullet'),5000,30,'bullet',40000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.arquebusier')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.arquebusier'),'train','units_2',0,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.arquebusier'),(SELECT id FROM def WHERE key='res.food'),43,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.arquebusier'),(SELECT id FROM def WHERE key='res.timber'),43,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.arquebusier','Upgrade to Arquebusiers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.arquebusier'),'unit_upgrade','',0,450);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.arquebusier'),(SELECT id FROM def WHERE key='res.food'),86,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.arquebusier'),(SELECT id FROM def WHERE key='res.timber'),86,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.arquebusier'),'upgrade_type',(SELECT id FROM def WHERE key='unit.crossbowman'),(SELECT id FROM def WHERE key='unit.arquebusier'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.light_infantry'),'unit','ranged',112,2,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,250,'per_type',(SELECT id FROM def WHERE key='unit.arquebusier'),(SELECT id FROM def WHERE key='age.enlightenment'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.light_infantry'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.light_infantry'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.light_infantry'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.light_infantry'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.light_infantry'),(SELECT id FROM movement_class WHERE key='foot'),1200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.light_infantry'),11,(SELECT id FROM damage_class WHERE key='bullet'),5000,30,'bullet',40000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.light_infantry')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.light_infantry'),'train','units_2',1,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.light_infantry'),(SELECT id FROM def WHERE key='res.food'),48,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.light_infantry'),(SELECT id FROM def WHERE key='res.timber'),48,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.light_infantry','Upgrade to Light Infantry');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.light_infantry'),'unit_upgrade','',0,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.light_infantry'),(SELECT id FROM def WHERE key='res.food'),96,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.light_infantry'),(SELECT id FROM def WHERE key='res.timber'),96,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.light_infantry'),'upgrade_type',(SELECT id FROM def WHERE key='unit.arquebusier'),(SELECT id FROM def WHERE key='unit.light_infantry'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.machine_gunner'),'unit','ranged',122,2,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,275,'per_type',(SELECT id FROM def WHERE key='unit.light_infantry'),(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.machine_gunner'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.machine_gunner'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.machine_gunner'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.machine_gunner'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.machine_gunner'),(SELECT id FROM movement_class WHERE key='foot'),1200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.machine_gunner'),12,(SELECT id FROM damage_class WHERE key='bullet'),5000,30,'bullet',40000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.machine_gunner')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.machine_gunner'),'train','units_2',2,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.machine_gunner'),(SELECT id FROM def WHERE key='res.food'),52,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.machine_gunner'),(SELECT id FROM def WHERE key='res.timber'),52,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.machine_gunner','Upgrade to Machine Gunners');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.machine_gunner'),'unit_upgrade','',0,550);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.machine_gunner'),(SELECT id FROM def WHERE key='res.food'),104,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.machine_gunner'),(SELECT id FROM def WHERE key='res.timber'),104,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.machine_gunner'),'upgrade_type',(SELECT id FROM def WHERE key='unit.light_infantry'),(SELECT id FROM def WHERE key='unit.machine_gunner'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.heavy_machine_gunner'),'unit','ranged',133,3,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,300,'per_type',(SELECT id FROM def WHERE key='unit.machine_gunner'),(SELECT id FROM def WHERE key='age.modern'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.heavy_machine_gunner'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.heavy_machine_gunner'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.heavy_machine_gunner'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.heavy_machine_gunner'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.heavy_machine_gunner'),(SELECT id FROM movement_class WHERE key='foot'),1200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.heavy_machine_gunner'),13,(SELECT id FROM damage_class WHERE key='bullet'),5000,30,'bullet',40000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.heavy_machine_gunner')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.heavy_machine_gunner'),'train','units_2',3,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.heavy_machine_gunner'),(SELECT id FROM def WHERE key='res.food'),57,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.heavy_machine_gunner'),(SELECT id FROM def WHERE key='res.timber'),57,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.heavy_machine_gunner','Upgrade to Heavy Machine Gunners');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.heavy_machine_gunner'),'unit_upgrade','',0,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.heavy_machine_gunner'),(SELECT id FROM def WHERE key='res.food'),114,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.heavy_machine_gunner'),(SELECT id FROM def WHERE key='res.timber'),114,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.heavy_machine_gunner'),'upgrade_type',(SELECT id FROM def WHERE key='unit.machine_gunner'),(SELECT id FROM def WHERE key='unit.heavy_machine_gunner'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.gun_drone'),'unit','ranged',143,3,(SELECT id FROM armor_class WHERE key='flesh'),7000,1,325,'per_type',(SELECT id FROM def WHERE key='unit.heavy_machine_gunner'),(SELECT id FROM def WHERE key='age.information'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.gun_drone'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.gun_drone'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.gun_drone'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.gun_drone'),(SELECT id FROM tag WHERE key='drone'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.gun_drone'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.gun_drone'),(SELECT id FROM movement_class WHERE key='wheeled'),1200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.gun_drone'),14,(SELECT id FROM damage_class WHERE key='bullet'),6000,30,'bullet',40000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.gun_drone')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.gun_drone'),'train','units_2',4,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.gun_drone'),(SELECT id FROM def WHERE key='res.food'),61,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.gun_drone'),(SELECT id FROM def WHERE key='res.timber'),61,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.gun_drone','Upgrade to Gun Drones');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.gun_drone'),'unit_upgrade','',0,650);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.gun_drone'),(SELECT id FROM def WHERE key='res.food'),122,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.gun_drone'),(SELECT id FROM def WHERE key='res.timber'),122,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.gun_drone'),'upgrade_type',(SELECT id FROM def WHERE key='unit.heavy_machine_gunner'),(SELECT id FROM def WHERE key='unit.gun_drone'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.pulse_gunner'),'unit','ranged',164,4,(SELECT id FROM armor_class WHERE key='flesh'),7000,1,375,'per_type',(SELECT id FROM def WHERE key='unit.gun_drone'),(SELECT id FROM def WHERE key='age.fusion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.pulse_gunner'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.pulse_gunner'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.pulse_gunner'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.pulse_gunner'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.pulse_gunner'),(SELECT id FROM movement_class WHERE key='foot'),1200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.pulse_gunner'),16,(SELECT id FROM damage_class WHERE key='energy'),6000,30,'plasma',30000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.pulse_gunner')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.pulse_gunner'),'train','units_2',0,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.pulse_gunner'),(SELECT id FROM def WHERE key='res.food'),70,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.pulse_gunner'),(SELECT id FROM def WHERE key='res.timber'),70,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.pulse_gunner','Upgrade to Pulse Gunners');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.pulse_gunner'),'unit_upgrade','',0,750);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.pulse_gunner'),(SELECT id FROM def WHERE key='res.food'),140,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.pulse_gunner'),(SELECT id FROM def WHERE key='res.timber'),140,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.pulse_gunner'),'upgrade_type',(SELECT id FROM def WHERE key='unit.gun_drone'),(SELECT id FROM def WHERE key='unit.pulse_gunner'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.beam_trooper'),'unit','ranged',185,5,(SELECT id FROM armor_class WHERE key='flesh'),7000,1,425,'per_type',(SELECT id FROM def WHERE key='unit.pulse_gunner'),(SELECT id FROM def WHERE key='age.expansion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.beam_trooper'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.beam_trooper'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.beam_trooper'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.beam_trooper'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.beam_trooper'),(SELECT id FROM movement_class WHERE key='foot'),1200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile) VALUES ((SELECT id FROM def WHERE key='unit.beam_trooper'),18,(SELECT id FROM damage_class WHERE key='energy'),6000,30,'beam');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.beam_trooper')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.beam_trooper'),'train','units_2',1,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.beam_trooper'),(SELECT id FROM def WHERE key='res.food'),79,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.beam_trooper'),(SELECT id FROM def WHERE key='res.timber'),79,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.beam_trooper','Upgrade to Beam Troopers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.beam_trooper'),'unit_upgrade','',0,850);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.beam_trooper'),(SELECT id FROM def WHERE key='res.food'),158,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.beam_trooper'),(SELECT id FROM def WHERE key='res.timber'),158,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.beam_trooper'),'upgrade_type',(SELECT id FROM def WHERE key='unit.pulse_gunner'),(SELECT id FROM def WHERE key='unit.beam_trooper'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),'unit','anti_armour',122,2,(SELECT id FROM armor_class WHERE key='flesh'),7000,1,275,'per_type',NULL,(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),(SELECT id FROM movement_class WHERE key='foot'),1100);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),24,(SELECT id FROM damage_class WHERE key='explosive'),6000,30,'bullet',40000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.anti_tank_rifleman')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),'train','units_2',2,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),(SELECT id FROM def WHERE key='res.food'),70,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),(SELECT id FROM def WHERE key='res.metal'),70,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.bazooka_team'),'unit','anti_armour',133,3,(SELECT id FROM armor_class WHERE key='flesh'),7000,1,300,'per_type',(SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),(SELECT id FROM def WHERE key='age.modern'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.bazooka_team'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.bazooka_team'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.bazooka_team'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.bazooka_team'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.bazooka_team'),(SELECT id FROM movement_class WHERE key='foot'),1100);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.bazooka_team'),26,(SELECT id FROM damage_class WHERE key='explosive'),6000,30,'rocket',16000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.bazooka_team')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.bazooka_team'),'train','units_2',3,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.bazooka_team'),(SELECT id FROM def WHERE key='res.food'),76,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.bazooka_team'),(SELECT id FROM def WHERE key='res.metal'),76,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.bazooka_team','Upgrade to Bazooka Teams');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.bazooka_team'),'unit_upgrade','',0,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.bazooka_team'),(SELECT id FROM def WHERE key='res.food'),152,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.bazooka_team'),(SELECT id FROM def WHERE key='res.metal'),152,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.bazooka_team'),'upgrade_type',(SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),(SELECT id FROM def WHERE key='unit.bazooka_team'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_team'),'unit','anti_armour',143,3,(SELECT id FROM armor_class WHERE key='flesh'),7000,1,325,'per_type',(SELECT id FROM def WHERE key='unit.bazooka_team'),(SELECT id FROM def WHERE key='age.information'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_team'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_team'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_team'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_team'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.missile_team'),(SELECT id FROM movement_class WHERE key='foot'),1100);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.missile_team'),28,(SELECT id FROM damage_class WHERE key='explosive'),6000,30,'missile',20000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.missile_team')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.missile_team'),'train','units_2',4,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.missile_team'),(SELECT id FROM def WHERE key='res.food'),82,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.missile_team'),(SELECT id FROM def WHERE key='res.metal'),82,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.missile_team','Upgrade to Missile Teams');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_team'),'unit_upgrade','',0,650);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_team'),(SELECT id FROM def WHERE key='res.food'),164,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_team'),(SELECT id FROM def WHERE key='res.metal'),164,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_team'),'upgrade_type',(SELECT id FROM def WHERE key='unit.bazooka_team'),(SELECT id FROM def WHERE key='unit.missile_team'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.railgun_team'),'unit','anti_armour',164,4,(SELECT id FROM armor_class WHERE key='flesh'),7000,1,375,'per_type',(SELECT id FROM def WHERE key='unit.missile_team'),(SELECT id FROM def WHERE key='age.fusion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.railgun_team'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.railgun_team'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.railgun_team'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.railgun_team'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.railgun_team'),(SELECT id FROM movement_class WHERE key='foot'),1100);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile) VALUES ((SELECT id FROM def WHERE key='unit.railgun_team'),32,(SELECT id FROM damage_class WHERE key='explosive'),6000,30,'railgun');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.railgun_team')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.railgun_team'),'train','units_2',0,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.railgun_team'),(SELECT id FROM def WHERE key='res.food'),94,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.railgun_team'),(SELECT id FROM def WHERE key='res.metal'),94,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.railgun_team','Upgrade to Railgun Teams');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.railgun_team'),'unit_upgrade','',0,750);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.railgun_team'),(SELECT id FROM def WHERE key='res.food'),188,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.railgun_team'),(SELECT id FROM def WHERE key='res.metal'),188,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.railgun_team'),'upgrade_type',(SELECT id FROM def WHERE key='unit.missile_team'),(SELECT id FROM def WHERE key='unit.railgun_team'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.scout'),'unit','scout',70,0,(SELECT id FROM armor_class WHERE key='flesh'),11000,1,150,'per_type',NULL,(SELECT id FROM def WHERE key='age.ancient'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.scout'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.scout'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.scout'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.scout'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.scout'),(SELECT id FROM movement_class WHERE key='foot'),2200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.scout'),7,(SELECT id FROM damage_class WHERE key='pierce'),0,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.scout')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.scout'),'train','units_2',1,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.scout'),(SELECT id FROM def WHERE key='res.food'),30,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.scout'),(SELECT id FROM def WHERE key='res.timber'),30,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.ranger'),'unit','scout',112,2,(SELECT id FROM armor_class WHERE key='flesh'),12000,1,250,'per_type',(SELECT id FROM def WHERE key='unit.scout'),(SELECT id FROM def WHERE key='age.enlightenment'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.ranger'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.ranger'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.ranger'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.ranger'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.ranger'),(SELECT id FROM movement_class WHERE key='foot'),2200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.ranger'),11,(SELECT id FROM damage_class WHERE key='pierce'),0,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.ranger')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.ranger'),'train','units_2',2,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.ranger'),(SELECT id FROM def WHERE key='res.food'),48,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.ranger'),(SELECT id FROM def WHERE key='res.timber'),48,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.ranger','Upgrade to Rangers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.ranger'),'unit_upgrade','',0,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.ranger'),(SELECT id FROM def WHERE key='res.food'),96,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.ranger'),(SELECT id FROM def WHERE key='res.timber'),96,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.ranger'),'upgrade_type',(SELECT id FROM def WHERE key='unit.scout'),(SELECT id FROM def WHERE key='unit.ranger'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.recon_drone'),'unit','scout',143,3,(SELECT id FROM armor_class WHERE key='flesh'),14000,1,325,'per_type',(SELECT id FROM def WHERE key='unit.ranger'),(SELECT id FROM def WHERE key='age.information'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.recon_drone'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.recon_drone'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.recon_drone'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.recon_drone'),(SELECT id FROM tag WHERE key='drone'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.recon_drone'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.recon_drone'),(SELECT id FROM movement_class WHERE key='foot'),2200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.recon_drone'),14,(SELECT id FROM damage_class WHERE key='pierce'),0,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.recon_drone')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.recon_drone'),'train','units_2',3,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.recon_drone'),(SELECT id FROM def WHERE key='res.food'),61,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.recon_drone'),(SELECT id FROM def WHERE key='res.timber'),61,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.recon_drone','Upgrade to Recon Drones');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.recon_drone'),'unit_upgrade','',0,650);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.recon_drone'),(SELECT id FROM def WHERE key='res.food'),122,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.recon_drone'),(SELECT id FROM def WHERE key='res.timber'),122,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.recon_drone'),'upgrade_type',(SELECT id FROM def WHERE key='unit.ranger'),(SELECT id FROM def WHERE key='unit.recon_drone'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.medic'),'unit','medic',60,2,(SELECT id FROM armor_class WHERE key='flesh'),5000,1,275,'per_type',NULL,(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.medic'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.medic'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.medic'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.medic'),(SELECT id FROM movement_class WHERE key='foot'),1400);
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.medic'),(SELECT id FROM tag WHERE key='healer'));
INSERT INTO et_healer(entity_type_id,target_tag_id,hp_per_second,range,energy_per_hp,can_convert) VALUES ((SELECT id FROM def WHERE key='unit.medic'),(SELECT id FROM tag WHERE key='biological'),8,3000,1,0);
INSERT INTO et_energy(entity_type_id,max,regen_per_second) VALUES ((SELECT id FROM def WHERE key='unit.medic'),100,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.medic'),'train','units_2',4,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.medic'),(SELECT id FROM def WHERE key='res.wealth'),70,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.horseman'),'unit','light_mounted',184,0,(SELECT id FROM armor_class WHERE key='plate'),8000,1,175,'per_type',NULL,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.horseman'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.horseman'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.horseman'),(SELECT id FROM tag WHERE key='mounted'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.horseman'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.horseman'),(SELECT id FROM movement_class WHERE key='foot'),2200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.horseman'),13,(SELECT id FROM damage_class WHERE key='melee'),0,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.horseman')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM def WHERE key='unit.horseman'),'train','units',0,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.horseman'),(SELECT id FROM def WHERE key='res.food'),69,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.horseman'),(SELECT id FROM def WHERE key='res.metal'),46,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.light_cavalry'),'unit','light_mounted',208,1,(SELECT id FROM armor_class WHERE key='plate'),8000,1,200,'per_type',(SELECT id FROM def WHERE key='unit.horseman'),(SELECT id FROM def WHERE key='age.medieval'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.light_cavalry'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.light_cavalry'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.light_cavalry'),(SELECT id FROM tag WHERE key='mounted'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.light_cavalry'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.light_cavalry'),(SELECT id FROM movement_class WHERE key='foot'),2200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.light_cavalry'),15,(SELECT id FROM damage_class WHERE key='melee'),0,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.light_cavalry')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM def WHERE key='unit.light_cavalry'),'train','units',1,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.light_cavalry'),(SELECT id FROM def WHERE key='res.food'),78,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.light_cavalry'),(SELECT id FROM def WHERE key='res.metal'),52,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.light_cavalry','Upgrade to Light Cavalry');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.light_cavalry'),'unit_upgrade','',0,400);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.light_cavalry'),(SELECT id FROM def WHERE key='res.food'),156,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.light_cavalry'),(SELECT id FROM def WHERE key='res.metal'),104,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.light_cavalry'),'upgrade_type',(SELECT id FROM def WHERE key='unit.horseman'),(SELECT id FROM def WHERE key='unit.light_cavalry'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.hussar'),'unit','light_mounted',232,1,(SELECT id FROM armor_class WHERE key='plate'),8000,1,225,'per_type',(SELECT id FROM def WHERE key='unit.light_cavalry'),(SELECT id FROM def WHERE key='age.gunpowder'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hussar'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hussar'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hussar'),(SELECT id FROM tag WHERE key='mounted'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hussar'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.hussar'),(SELECT id FROM movement_class WHERE key='foot'),2200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.hussar'),17,(SELECT id FROM damage_class WHERE key='melee'),0,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.hussar')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM def WHERE key='unit.hussar'),'train','units',2,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.hussar'),(SELECT id FROM def WHERE key='res.food'),87,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.hussar'),(SELECT id FROM def WHERE key='res.metal'),58,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.hussar','Upgrade to Hussars');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.hussar'),'unit_upgrade','',0,450);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.hussar'),(SELECT id FROM def WHERE key='res.food'),174,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.hussar'),(SELECT id FROM def WHERE key='res.metal'),116,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.hussar'),'upgrade_type',(SELECT id FROM def WHERE key='unit.light_cavalry'),(SELECT id FROM def WHERE key='unit.hussar'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.dragoon'),'unit','light_mounted',256,2,(SELECT id FROM armor_class WHERE key='plate'),8000,1,250,'per_type',(SELECT id FROM def WHERE key='unit.hussar'),(SELECT id FROM def WHERE key='age.enlightenment'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.dragoon'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.dragoon'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.dragoon'),(SELECT id FROM tag WHERE key='mounted'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.dragoon'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.dragoon'),(SELECT id FROM movement_class WHERE key='foot'),2200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.dragoon'),19,(SELECT id FROM damage_class WHERE key='melee'),0,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.dragoon')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM def WHERE key='unit.dragoon'),'train','units',3,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.dragoon'),(SELECT id FROM def WHERE key='res.food'),96,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.dragoon'),(SELECT id FROM def WHERE key='res.metal'),64,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.dragoon','Upgrade to Dragoons');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.dragoon'),'unit_upgrade','',0,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.dragoon'),(SELECT id FROM def WHERE key='res.food'),192,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.dragoon'),(SELECT id FROM def WHERE key='res.metal'),128,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.dragoon'),'upgrade_type',(SELECT id FROM def WHERE key='unit.hussar'),(SELECT id FROM def WHERE key='unit.dragoon'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.armoured_car'),'unit','light_mounted',280,2,(SELECT id FROM armor_class WHERE key='vehicle'),8000,1,275,'per_type',(SELECT id FROM def WHERE key='unit.dragoon'),(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.armoured_car'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.armoured_car'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.armoured_car'),(SELECT id FROM tag WHERE key='vehicle'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.armoured_car'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.armoured_car'),(SELECT id FROM movement_class WHERE key='wheeled'),1600);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.armoured_car'),21,(SELECT id FROM damage_class WHERE key='explosive'),4000,30,'bullet',40000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.armoured_car')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='unit.armoured_car'),'train','units',0,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.armoured_car'),(SELECT id FROM def WHERE key='res.food'),105,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.armoured_car'),(SELECT id FROM def WHERE key='res.metal'),70,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.armoured_car'),(SELECT id FROM def WHERE key='res.oil'),35,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.armoured_car','Upgrade to Armoured Cars');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.armoured_car'),'unit_upgrade','',0,550);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.armoured_car'),(SELECT id FROM def WHERE key='res.food'),210,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.armoured_car'),(SELECT id FROM def WHERE key='res.metal'),140,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.armoured_car'),(SELECT id FROM def WHERE key='res.oil'),70,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.armoured_car'),'upgrade_type',(SELECT id FROM def WHERE key='unit.dragoon'),(SELECT id FROM def WHERE key='unit.armoured_car'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.scout_car'),'unit','light_mounted',304,3,(SELECT id FROM armor_class WHERE key='vehicle'),8000,1,300,'per_type',(SELECT id FROM def WHERE key='unit.armoured_car'),(SELECT id FROM def WHERE key='age.modern'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.scout_car'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.scout_car'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.scout_car'),(SELECT id FROM tag WHERE key='vehicle'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.scout_car'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.scout_car'),(SELECT id FROM movement_class WHERE key='wheeled'),1600);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.scout_car'),22,(SELECT id FROM damage_class WHERE key='explosive'),4000,30,'bullet',40000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.scout_car')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='unit.scout_car'),'train','units',1,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.scout_car'),(SELECT id FROM def WHERE key='res.food'),114,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.scout_car'),(SELECT id FROM def WHERE key='res.metal'),76,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.scout_car'),(SELECT id FROM def WHERE key='res.oil'),38,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.scout_car','Upgrade to Scout Cars');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.scout_car'),'unit_upgrade','',0,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.scout_car'),(SELECT id FROM def WHERE key='res.food'),228,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.scout_car'),(SELECT id FROM def WHERE key='res.metal'),152,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.scout_car'),(SELECT id FROM def WHERE key='res.oil'),76,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.scout_car'),'upgrade_type',(SELECT id FROM def WHERE key='unit.armoured_car'),(SELECT id FROM def WHERE key='unit.scout_car'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.hovercraft'),'unit','light_mounted',352,4,(SELECT id FROM armor_class WHERE key='vehicle'),8000,1,350,'per_type',(SELECT id FROM def WHERE key='unit.scout_car'),(SELECT id FROM def WHERE key='age.automation'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hovercraft'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hovercraft'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hovercraft'),(SELECT id FROM tag WHERE key='vehicle'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hovercraft'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.hovercraft'),(SELECT id FROM movement_class WHERE key='wheeled'),1600);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.hovercraft'),26,(SELECT id FROM damage_class WHERE key='explosive'),4000,30,'bullet',40000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.hovercraft')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='unit.hovercraft'),'train','units',2,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.hovercraft'),(SELECT id FROM def WHERE key='res.food'),132,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.hovercraft'),(SELECT id FROM def WHERE key='res.metal'),88,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.hovercraft'),(SELECT id FROM def WHERE key='res.oil'),44,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.hovercraft','Upgrade to Hovercraft');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.hovercraft'),'unit_upgrade','',0,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.hovercraft'),(SELECT id FROM def WHERE key='res.food'),264,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.hovercraft'),(SELECT id FROM def WHERE key='res.metal'),176,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.hovercraft'),(SELECT id FROM def WHERE key='res.oil'),88,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.hovercraft'),'upgrade_type',(SELECT id FROM def WHERE key='unit.scout_car'),(SELECT id FROM def WHERE key='unit.hovercraft'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.grav_skimmer'),'unit','light_mounted',400,5,(SELECT id FROM armor_class WHERE key='vehicle'),8000,1,400,'per_type',(SELECT id FROM def WHERE key='unit.hovercraft'),(SELECT id FROM def WHERE key='age.starfaring'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.grav_skimmer'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.grav_skimmer'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.grav_skimmer'),(SELECT id FROM tag WHERE key='vehicle'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.grav_skimmer'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.grav_skimmer'),(SELECT id FROM movement_class WHERE key='wheeled'),1600);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.grav_skimmer'),30,(SELECT id FROM damage_class WHERE key='explosive'),4000,30,'plasma',30000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.grav_skimmer')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='unit.grav_skimmer'),'train','units',3,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.grav_skimmer'),(SELECT id FROM def WHERE key='res.food'),150,3);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.grav_skimmer'),(SELECT id FROM def WHERE key='res.metal'),100,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.grav_skimmer'),(SELECT id FROM def WHERE key='res.oil'),50,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.grav_skimmer','Upgrade to Grav Skimmers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.grav_skimmer'),'unit_upgrade','',0,800);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.grav_skimmer'),(SELECT id FROM def WHERE key='res.food'),300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.grav_skimmer'),(SELECT id FROM def WHERE key='res.metal'),200,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.grav_skimmer'),(SELECT id FROM def WHERE key='res.oil'),100,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.grav_skimmer'),'upgrade_type',(SELECT id FROM def WHERE key='unit.hovercraft'),(SELECT id FROM def WHERE key='unit.grav_skimmer'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.cataphract'),'unit','heavy_mounted',184,0,(SELECT id FROM armor_class WHERE key='plate'),7000,1,175,'per_type',NULL,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cataphract'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cataphract'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cataphract'),(SELECT id FROM tag WHERE key='mounted'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cataphract'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.cataphract'),(SELECT id FROM movement_class WHERE key='foot'),2200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.cataphract'),13,(SELECT id FROM damage_class WHERE key='melee'),0,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.cataphract')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM def WHERE key='unit.cataphract'),'train','units',4,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.cataphract'),(SELECT id FROM def WHERE key='res.food'),69,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.cataphract'),(SELECT id FROM def WHERE key='res.metal'),46,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.knight'),'unit','heavy_mounted',208,1,(SELECT id FROM armor_class WHERE key='plate'),7000,1,200,'per_type',(SELECT id FROM def WHERE key='unit.cataphract'),(SELECT id FROM def WHERE key='age.medieval'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.knight'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.knight'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.knight'),(SELECT id FROM tag WHERE key='mounted'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.knight'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.knight'),(SELECT id FROM movement_class WHERE key='foot'),2200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.knight'),15,(SELECT id FROM damage_class WHERE key='melee'),0,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.knight')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM def WHERE key='unit.knight'),'train','units',0,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.knight'),(SELECT id FROM def WHERE key='res.food'),78,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.knight'),(SELECT id FROM def WHERE key='res.metal'),52,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.knight','Upgrade to Knights');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.knight'),'unit_upgrade','',0,400);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.knight'),(SELECT id FROM def WHERE key='res.food'),156,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.knight'),(SELECT id FROM def WHERE key='res.metal'),104,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.knight'),'upgrade_type',(SELECT id FROM def WHERE key='unit.cataphract'),(SELECT id FROM def WHERE key='unit.knight'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.cuirassier'),'unit','heavy_mounted',232,1,(SELECT id FROM armor_class WHERE key='plate'),7000,1,225,'per_type',(SELECT id FROM def WHERE key='unit.knight'),(SELECT id FROM def WHERE key='age.gunpowder'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cuirassier'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cuirassier'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cuirassier'),(SELECT id FROM tag WHERE key='mounted'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cuirassier'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.cuirassier'),(SELECT id FROM movement_class WHERE key='foot'),2200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.cuirassier'),17,(SELECT id FROM damage_class WHERE key='melee'),0,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.cuirassier')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM def WHERE key='unit.cuirassier'),'train','units',1,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.cuirassier'),(SELECT id FROM def WHERE key='res.food'),87,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.cuirassier'),(SELECT id FROM def WHERE key='res.metal'),58,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.cuirassier','Upgrade to Cuirassiers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.cuirassier'),'unit_upgrade','',0,450);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.cuirassier'),(SELECT id FROM def WHERE key='res.food'),174,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.cuirassier'),(SELECT id FROM def WHERE key='res.metal'),116,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.cuirassier'),'upgrade_type',(SELECT id FROM def WHERE key='unit.knight'),(SELECT id FROM def WHERE key='unit.cuirassier'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.lancer'),'unit','heavy_mounted',256,2,(SELECT id FROM armor_class WHERE key='plate'),7000,1,250,'per_type',(SELECT id FROM def WHERE key='unit.cuirassier'),(SELECT id FROM def WHERE key='age.enlightenment'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.lancer'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.lancer'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.lancer'),(SELECT id FROM tag WHERE key='mounted'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.lancer'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.lancer'),(SELECT id FROM movement_class WHERE key='foot'),2200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.lancer'),19,(SELECT id FROM damage_class WHERE key='melee'),0,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.lancer')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM def WHERE key='unit.lancer'),'train','units',2,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.lancer'),(SELECT id FROM def WHERE key='res.food'),96,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.lancer'),(SELECT id FROM def WHERE key='res.metal'),64,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.lancer','Upgrade to Lancers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.lancer'),'unit_upgrade','',0,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.lancer'),(SELECT id FROM def WHERE key='res.food'),192,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.lancer'),(SELECT id FROM def WHERE key='res.metal'),128,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.lancer'),'upgrade_type',(SELECT id FROM def WHERE key='unit.cuirassier'),(SELECT id FROM def WHERE key='unit.lancer'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.light_tank'),'unit','heavy_mounted',280,2,(SELECT id FROM armor_class WHERE key='vehicle'),7000,1,275,'per_type',(SELECT id FROM def WHERE key='unit.lancer'),(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.light_tank'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.light_tank'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.light_tank'),(SELECT id FROM tag WHERE key='vehicle'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.light_tank'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.light_tank'),(SELECT id FROM movement_class WHERE key='wheeled'),1600);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.light_tank'),21,(SELECT id FROM damage_class WHERE key='explosive'),4000,30,'shell',24000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.light_tank')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='unit.light_tank'),'train','units',4,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.light_tank'),(SELECT id FROM def WHERE key='res.food'),105,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.light_tank'),(SELECT id FROM def WHERE key='res.metal'),70,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.light_tank'),(SELECT id FROM def WHERE key='res.oil'),35,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.light_tank','Upgrade to Light Tanks');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.light_tank'),'unit_upgrade','',0,550);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.light_tank'),(SELECT id FROM def WHERE key='res.food'),210,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.light_tank'),(SELECT id FROM def WHERE key='res.metal'),140,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.light_tank'),(SELECT id FROM def WHERE key='res.oil'),70,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.light_tank'),'upgrade_type',(SELECT id FROM def WHERE key='unit.lancer'),(SELECT id FROM def WHERE key='unit.light_tank'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.tank'),'unit','heavy_mounted',304,3,(SELECT id FROM armor_class WHERE key='vehicle'),7000,1,300,'per_type',(SELECT id FROM def WHERE key='unit.light_tank'),(SELECT id FROM def WHERE key='age.modern'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.tank'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.tank'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.tank'),(SELECT id FROM tag WHERE key='vehicle'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.tank'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.tank'),(SELECT id FROM movement_class WHERE key='wheeled'),1600);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.tank'),22,(SELECT id FROM damage_class WHERE key='explosive'),4000,30,'shell',24000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.tank')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='unit.tank'),'train','units',0,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.tank'),(SELECT id FROM def WHERE key='res.food'),114,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.tank'),(SELECT id FROM def WHERE key='res.metal'),76,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.tank'),(SELECT id FROM def WHERE key='res.oil'),38,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.tank','Upgrade to Tanks');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.tank'),'unit_upgrade','',0,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.tank'),(SELECT id FROM def WHERE key='res.food'),228,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.tank'),(SELECT id FROM def WHERE key='res.metal'),152,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.tank'),(SELECT id FROM def WHERE key='res.oil'),76,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.tank'),'upgrade_type',(SELECT id FROM def WHERE key='unit.light_tank'),(SELECT id FROM def WHERE key='unit.tank'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.main_battle_tank'),'unit','heavy_mounted',328,3,(SELECT id FROM armor_class WHERE key='vehicle'),7000,1,325,'per_type',(SELECT id FROM def WHERE key='unit.tank'),(SELECT id FROM def WHERE key='age.information'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.main_battle_tank'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.main_battle_tank'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.main_battle_tank'),(SELECT id FROM tag WHERE key='vehicle'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.main_battle_tank'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.main_battle_tank'),(SELECT id FROM movement_class WHERE key='wheeled'),1600);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.main_battle_tank'),24,(SELECT id FROM damage_class WHERE key='explosive'),4000,30,'shell',24000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.main_battle_tank')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='unit.main_battle_tank'),'train','units',1,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.main_battle_tank'),(SELECT id FROM def WHERE key='res.food'),123,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.main_battle_tank'),(SELECT id FROM def WHERE key='res.metal'),82,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.main_battle_tank'),(SELECT id FROM def WHERE key='res.oil'),41,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.main_battle_tank','Upgrade to Main Battle Tanks');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.main_battle_tank'),'unit_upgrade','',0,650);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.main_battle_tank'),(SELECT id FROM def WHERE key='res.food'),246,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.main_battle_tank'),(SELECT id FROM def WHERE key='res.metal'),164,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.main_battle_tank'),(SELECT id FROM def WHERE key='res.oil'),82,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.main_battle_tank'),'upgrade_type',(SELECT id FROM def WHERE key='unit.tank'),(SELECT id FROM def WHERE key='unit.main_battle_tank'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.robot_tank'),'unit','heavy_mounted',352,4,(SELECT id FROM armor_class WHERE key='vehicle'),7000,1,350,'per_type',(SELECT id FROM def WHERE key='unit.main_battle_tank'),(SELECT id FROM def WHERE key='age.automation'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.robot_tank'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.robot_tank'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.robot_tank'),(SELECT id FROM tag WHERE key='vehicle'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.robot_tank'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.robot_tank'),(SELECT id FROM movement_class WHERE key='wheeled'),1600);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.robot_tank'),26,(SELECT id FROM damage_class WHERE key='explosive'),4000,30,'shell',24000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.robot_tank')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='unit.robot_tank'),'train','units',2,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.robot_tank'),(SELECT id FROM def WHERE key='res.food'),132,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.robot_tank'),(SELECT id FROM def WHERE key='res.metal'),88,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.robot_tank'),(SELECT id FROM def WHERE key='res.oil'),44,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.robot_tank','Upgrade to Robot Tanks');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.robot_tank'),'unit_upgrade','',0,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.robot_tank'),(SELECT id FROM def WHERE key='res.food'),264,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.robot_tank'),(SELECT id FROM def WHERE key='res.metal'),176,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.robot_tank'),(SELECT id FROM def WHERE key='res.oil'),88,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.robot_tank'),'upgrade_type',(SELECT id FROM def WHERE key='unit.main_battle_tank'),(SELECT id FROM def WHERE key='unit.robot_tank'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.fusion_tank'),'unit','heavy_mounted',376,4,(SELECT id FROM armor_class WHERE key='vehicle'),7000,1,375,'per_type',(SELECT id FROM def WHERE key='unit.robot_tank'),(SELECT id FROM def WHERE key='age.fusion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.fusion_tank'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.fusion_tank'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.fusion_tank'),(SELECT id FROM tag WHERE key='vehicle'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.fusion_tank'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.fusion_tank'),(SELECT id FROM movement_class WHERE key='wheeled'),1600);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.fusion_tank'),28,(SELECT id FROM damage_class WHERE key='explosive'),4000,30,'plasma',30000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.fusion_tank')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='unit.fusion_tank'),'train','units',3,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.fusion_tank'),(SELECT id FROM def WHERE key='res.food'),141,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.fusion_tank'),(SELECT id FROM def WHERE key='res.metal'),94,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.fusion_tank'),(SELECT id FROM def WHERE key='res.oil'),47,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.fusion_tank','Upgrade to Fusion Tanks');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.fusion_tank'),'unit_upgrade','',0,750);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.fusion_tank'),(SELECT id FROM def WHERE key='res.food'),282,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.fusion_tank'),(SELECT id FROM def WHERE key='res.metal'),188,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.fusion_tank'),(SELECT id FROM def WHERE key='res.oil'),94,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.fusion_tank'),'upgrade_type',(SELECT id FROM def WHERE key='unit.robot_tank'),(SELECT id FROM def WHERE key='unit.fusion_tank'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.grav_tank'),'unit','heavy_mounted',424,5,(SELECT id FROM armor_class WHERE key='vehicle'),7000,1,425,'per_type',(SELECT id FROM def WHERE key='unit.fusion_tank'),(SELECT id FROM def WHERE key='age.expansion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.grav_tank'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.grav_tank'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.grav_tank'),(SELECT id FROM tag WHERE key='vehicle'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.grav_tank'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.grav_tank'),(SELECT id FROM movement_class WHERE key='wheeled'),1600);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile) VALUES ((SELECT id FROM def WHERE key='unit.grav_tank'),31,(SELECT id FROM damage_class WHERE key='explosive'),4000,30,'beam');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.grav_tank')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='unit.grav_tank'),'train','units',4,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.grav_tank'),(SELECT id FROM def WHERE key='res.food'),159,3);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.grav_tank'),(SELECT id FROM def WHERE key='res.metal'),106,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.grav_tank'),(SELECT id FROM def WHERE key='res.oil'),53,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.grav_tank','Upgrade to Grav Tanks');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.grav_tank'),'unit_upgrade','',0,850);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.grav_tank'),(SELECT id FROM def WHERE key='res.food'),318,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.grav_tank'),(SELECT id FROM def WHERE key='res.metal'),212,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.grav_tank'),(SELECT id FROM def WHERE key='res.oil'),106,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.grav_tank'),'upgrade_type',(SELECT id FROM def WHERE key='unit.fusion_tank'),(SELECT id FROM def WHERE key='unit.grav_tank'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.catapult'),'unit','siege',138,0,(SELECT id FROM armor_class WHERE key='vehicle'),5000,1,175,'per_type',NULL,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.catapult'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.catapult'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.catapult'),(SELECT id FROM tag WHERE key='siege'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.catapult'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.catapult'),(SELECT id FROM movement_class WHERE key='wheeled'),800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,range_min,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.catapult'),34,(SELECT id FROM damage_class WHERE key='explosive'),10000,30,3000,'stone',10000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.catapult')),'ground');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.catapult')),'fluid');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),(SELECT id FROM def WHERE key='unit.catapult'),'train','units',0,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.catapult'),(SELECT id FROM def WHERE key='res.timber'),69,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.catapult'),(SELECT id FROM def WHERE key='res.metal'),69,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.trebuchet'),'unit','siege',156,1,(SELECT id FROM armor_class WHERE key='vehicle'),5000,1,200,'per_type',(SELECT id FROM def WHERE key='unit.catapult'),(SELECT id FROM def WHERE key='age.medieval'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.trebuchet'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.trebuchet'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.trebuchet'),(SELECT id FROM tag WHERE key='siege'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.trebuchet'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.trebuchet'),(SELECT id FROM movement_class WHERE key='wheeled'),800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,range_min,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.trebuchet'),39,(SELECT id FROM damage_class WHERE key='explosive'),10000,30,3000,'stone',10000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.trebuchet')),'ground');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.trebuchet')),'fluid');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),(SELECT id FROM def WHERE key='unit.trebuchet'),'train','units',1,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.trebuchet'),(SELECT id FROM def WHERE key='res.timber'),78,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.trebuchet'),(SELECT id FROM def WHERE key='res.metal'),78,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.trebuchet','Upgrade to Trebuchets');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.trebuchet'),'unit_upgrade','',0,400);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.trebuchet'),(SELECT id FROM def WHERE key='res.timber'),156,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.trebuchet'),(SELECT id FROM def WHERE key='res.metal'),156,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.trebuchet'),'upgrade_type',(SELECT id FROM def WHERE key='unit.catapult'),(SELECT id FROM def WHERE key='unit.trebuchet'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.bombard'),'unit','siege',174,1,(SELECT id FROM armor_class WHERE key='vehicle'),5000,1,225,'per_type',(SELECT id FROM def WHERE key='unit.trebuchet'),(SELECT id FROM def WHERE key='age.gunpowder'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.bombard'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.bombard'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.bombard'),(SELECT id FROM tag WHERE key='siege'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.bombard'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.bombard'),(SELECT id FROM movement_class WHERE key='wheeled'),800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,range_min,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.bombard'),43,(SELECT id FROM damage_class WHERE key='explosive'),11000,30,3000,'shell',24000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.bombard')),'ground');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.bombard')),'fluid');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),(SELECT id FROM def WHERE key='unit.bombard'),'train','units',2,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.bombard'),(SELECT id FROM def WHERE key='res.timber'),87,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.bombard'),(SELECT id FROM def WHERE key='res.metal'),87,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.bombard','Upgrade to Bombards');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.bombard'),'unit_upgrade','',0,450);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.bombard'),(SELECT id FROM def WHERE key='res.timber'),174,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.bombard'),(SELECT id FROM def WHERE key='res.metal'),174,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.bombard'),'upgrade_type',(SELECT id FROM def WHERE key='unit.trebuchet'),(SELECT id FROM def WHERE key='unit.bombard'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.cannon'),'unit','siege',192,2,(SELECT id FROM armor_class WHERE key='vehicle'),5000,1,250,'per_type',(SELECT id FROM def WHERE key='unit.bombard'),(SELECT id FROM def WHERE key='age.enlightenment'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cannon'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cannon'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cannon'),(SELECT id FROM tag WHERE key='siege'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cannon'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.cannon'),(SELECT id FROM movement_class WHERE key='wheeled'),800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,range_min,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.cannon'),48,(SELECT id FROM damage_class WHERE key='explosive'),11000,30,3000,'shell',24000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.cannon')),'ground');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.cannon')),'fluid');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),(SELECT id FROM def WHERE key='unit.cannon'),'train','units',3,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.cannon'),(SELECT id FROM def WHERE key='res.timber'),96,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.cannon'),(SELECT id FROM def WHERE key='res.metal'),96,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.cannon','Upgrade to Cannons');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.cannon'),'unit_upgrade','',0,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.cannon'),(SELECT id FROM def WHERE key='res.timber'),192,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.cannon'),(SELECT id FROM def WHERE key='res.metal'),192,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.cannon'),'upgrade_type',(SELECT id FROM def WHERE key='unit.bombard'),(SELECT id FROM def WHERE key='unit.cannon'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.field_gun'),'unit','siege',210,2,(SELECT id FROM armor_class WHERE key='vehicle'),5000,1,275,'per_type',(SELECT id FROM def WHERE key='unit.cannon'),(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.field_gun'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.field_gun'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.field_gun'),(SELECT id FROM tag WHERE key='siege'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.field_gun'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.field_gun'),(SELECT id FROM movement_class WHERE key='wheeled'),800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,range_min,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.field_gun'),52,(SELECT id FROM damage_class WHERE key='explosive'),12000,30,3000,'shell',24000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.field_gun')),'ground');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.field_gun')),'fluid');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='unit.field_gun'),'train','units',0,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.field_gun'),(SELECT id FROM def WHERE key='res.timber'),105,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.field_gun'),(SELECT id FROM def WHERE key='res.metal'),105,2);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.field_gun','Upgrade to Field Guns');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.field_gun'),'unit_upgrade','',0,550);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.field_gun'),(SELECT id FROM def WHERE key='res.timber'),210,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.field_gun'),(SELECT id FROM def WHERE key='res.metal'),210,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.field_gun'),'upgrade_type',(SELECT id FROM def WHERE key='unit.cannon'),(SELECT id FROM def WHERE key='unit.field_gun'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.howitzer'),'unit','siege',228,3,(SELECT id FROM armor_class WHERE key='vehicle'),5000,1,300,'per_type',(SELECT id FROM def WHERE key='unit.field_gun'),(SELECT id FROM def WHERE key='age.modern'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.howitzer'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.howitzer'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.howitzer'),(SELECT id FROM tag WHERE key='siege'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.howitzer'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.howitzer'),(SELECT id FROM movement_class WHERE key='wheeled'),800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,range_min,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.howitzer'),57,(SELECT id FROM damage_class WHERE key='explosive'),12000,30,3000,'shell',14000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.howitzer')),'ground');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.howitzer')),'fluid');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='unit.howitzer'),'train','units',1,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.howitzer'),(SELECT id FROM def WHERE key='res.timber'),114,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.howitzer'),(SELECT id FROM def WHERE key='res.metal'),114,2);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.howitzer','Upgrade to Howitzers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.howitzer'),'unit_upgrade','',0,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.howitzer'),(SELECT id FROM def WHERE key='res.timber'),228,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.howitzer'),(SELECT id FROM def WHERE key='res.metal'),228,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.howitzer'),'upgrade_type',(SELECT id FROM def WHERE key='unit.field_gun'),(SELECT id FROM def WHERE key='unit.howitzer'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.rocket_artillery'),'unit','siege',246,3,(SELECT id FROM armor_class WHERE key='vehicle'),5000,1,325,'per_type',(SELECT id FROM def WHERE key='unit.howitzer'),(SELECT id FROM def WHERE key='age.information'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.rocket_artillery'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.rocket_artillery'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.rocket_artillery'),(SELECT id FROM tag WHERE key='siege'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.rocket_artillery'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.rocket_artillery'),(SELECT id FROM movement_class WHERE key='wheeled'),800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,range_min,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.rocket_artillery'),61,(SELECT id FROM damage_class WHERE key='explosive'),13000,30,3000,'rocket',16000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.rocket_artillery')),'ground');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.rocket_artillery')),'fluid');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='unit.rocket_artillery'),'train','units',2,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.rocket_artillery'),(SELECT id FROM def WHERE key='res.timber'),123,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.rocket_artillery'),(SELECT id FROM def WHERE key='res.metal'),123,2);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.rocket_artillery','Upgrade to Rocket Artillery');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.rocket_artillery'),'unit_upgrade','',0,650);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.rocket_artillery'),(SELECT id FROM def WHERE key='res.timber'),246,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.rocket_artillery'),(SELECT id FROM def WHERE key='res.metal'),246,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.rocket_artillery'),'upgrade_type',(SELECT id FROM def WHERE key='unit.howitzer'),(SELECT id FROM def WHERE key='unit.rocket_artillery'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.railgun_battery'),'unit','siege',282,4,(SELECT id FROM armor_class WHERE key='vehicle'),5000,1,375,'per_type',(SELECT id FROM def WHERE key='unit.rocket_artillery'),(SELECT id FROM def WHERE key='age.fusion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.railgun_battery'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.railgun_battery'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.railgun_battery'),(SELECT id FROM tag WHERE key='siege'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.railgun_battery'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.railgun_battery'),(SELECT id FROM movement_class WHERE key='wheeled'),800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,range_min,projectile) VALUES ((SELECT id FROM def WHERE key='unit.railgun_battery'),70,(SELECT id FROM damage_class WHERE key='explosive'),14000,30,3000,'railgun');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.railgun_battery')),'ground');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.railgun_battery')),'fluid');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='unit.railgun_battery'),'train','units',3,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.railgun_battery'),(SELECT id FROM def WHERE key='res.timber'),141,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.railgun_battery'),(SELECT id FROM def WHERE key='res.metal'),141,2);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.railgun_battery','Upgrade to Railgun Batteries');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.railgun_battery'),'unit_upgrade','',0,750);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.railgun_battery'),(SELECT id FROM def WHERE key='res.timber'),282,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.railgun_battery'),(SELECT id FROM def WHERE key='res.metal'),282,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.railgun_battery'),'upgrade_type',(SELECT id FROM def WHERE key='unit.rocket_artillery'),(SELECT id FROM def WHERE key='unit.railgun_battery'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.flak_gun'),'unit','anti_air',133,3,(SELECT id FROM armor_class WHERE key='flesh'),7000,1,300,'per_type',NULL,(SELECT id FROM def WHERE key='age.modern'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.flak_gun'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.flak_gun'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.flak_gun'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.flak_gun'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.flak_gun'),(SELECT id FROM movement_class WHERE key='wheeled'),1100);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.flak_gun'),26,(SELECT id FROM damage_class WHERE key='explosive'),6000,30,'shell',24000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.flak_gun')),'air');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='unit.flak_gun'),'train','units',4,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.flak_gun'),(SELECT id FROM def WHERE key='res.food'),76,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.flak_gun'),(SELECT id FROM def WHERE key='res.metal'),76,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_launcher'),'unit','anti_air',143,3,(SELECT id FROM armor_class WHERE key='flesh'),7000,1,325,'per_type',(SELECT id FROM def WHERE key='unit.flak_gun'),(SELECT id FROM def WHERE key='age.information'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_launcher'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_launcher'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_launcher'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_launcher'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.missile_launcher'),(SELECT id FROM movement_class WHERE key='wheeled'),1100);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.missile_launcher'),28,(SELECT id FROM damage_class WHERE key='explosive'),6000,30,'missile',20000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.missile_launcher')),'air');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='unit.missile_launcher'),'train','units',0,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.missile_launcher'),(SELECT id FROM def WHERE key='res.food'),82,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.missile_launcher'),(SELECT id FROM def WHERE key='res.metal'),82,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.missile_launcher','Upgrade to Missile Launchers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_launcher'),'unit_upgrade','',0,650);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_launcher'),(SELECT id FROM def WHERE key='res.food'),164,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_launcher'),(SELECT id FROM def WHERE key='res.metal'),164,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_launcher'),'upgrade_type',(SELECT id FROM def WHERE key='unit.flak_gun'),(SELECT id FROM def WHERE key='unit.missile_launcher'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.laser_battery'),'unit','anti_air',164,4,(SELECT id FROM armor_class WHERE key='flesh'),7000,1,375,'per_type',(SELECT id FROM def WHERE key='unit.missile_launcher'),(SELECT id FROM def WHERE key='age.fusion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.laser_battery'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.laser_battery'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.laser_battery'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.laser_battery'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.laser_battery'),(SELECT id FROM movement_class WHERE key='wheeled'),1100);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile) VALUES ((SELECT id FROM def WHERE key='unit.laser_battery'),32,(SELECT id FROM damage_class WHERE key='explosive'),6000,30,'beam');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.laser_battery')),'air');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='unit.laser_battery'),'train','units',1,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.laser_battery'),(SELECT id FROM def WHERE key='res.food'),94,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.laser_battery'),(SELECT id FROM def WHERE key='res.metal'),94,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.laser_battery','Upgrade to Laser Batteries');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.laser_battery'),'unit_upgrade','',0,750);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.laser_battery'),(SELECT id FROM def WHERE key='res.food'),188,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.laser_battery'),(SELECT id FROM def WHERE key='res.metal'),188,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.laser_battery'),'upgrade_type',(SELECT id FROM def WHERE key='unit.missile_launcher'),(SELECT id FROM def WHERE key='unit.laser_battery'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.supply_wagon'),'unit','supply',120,1,(SELECT id FROM armor_class WHERE key='flesh'),5000,1,200,'per_type',NULL,(SELECT id FROM def WHERE key='age.medieval'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.supply_wagon'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.supply_wagon'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.supply_wagon'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.supply_wagon'),(SELECT id FROM movement_class WHERE key='wheeled'),1300);
INSERT INTO et_supply(entity_type_id,radius) VALUES ((SELECT id FROM def WHERE key='unit.supply_wagon'),6000);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),(SELECT id FROM def WHERE key='unit.supply_wagon'),'train','units',4,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.supply_wagon'),(SELECT id FROM def WHERE key='res.wealth'),52,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.supply_truck'),'unit','supply',120,2,(SELECT id FROM armor_class WHERE key='flesh'),5000,1,275,'per_type',(SELECT id FROM def WHERE key='unit.supply_wagon'),(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.supply_truck'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.supply_truck'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.supply_truck'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.supply_truck'),(SELECT id FROM movement_class WHERE key='wheeled'),1300);
INSERT INTO et_supply(entity_type_id,radius) VALUES ((SELECT id FROM def WHERE key='unit.supply_truck'),6000);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='unit.supply_truck'),'train','units',2,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.supply_truck'),(SELECT id FROM def WHERE key='res.wealth'),70,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.supply_truck','Upgrade to Supply Trucks');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.supply_truck'),'unit_upgrade','',0,550);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.supply_truck'),(SELECT id FROM def WHERE key='res.wealth'),140,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.supply_truck'),'upgrade_type',(SELECT id FROM def WHERE key='unit.supply_wagon'),(SELECT id FROM def WHERE key='unit.supply_truck'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.repair_drone'),'unit','repair',45,3,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,325,'per_type',NULL,(SELECT id FROM def WHERE key='age.information'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.repair_drone'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.repair_drone'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.repair_drone'),(SELECT id FROM tag WHERE key='drone'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.repair_drone'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.repair_drone'),(SELECT id FROM movement_class WHERE key='aircraft'),2600);
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.repair_drone'),(SELECT id FROM tag WHERE key='healer'));
INSERT INTO et_healer(entity_type_id,target_tag_id,hp_per_second,range,energy_per_hp,can_convert) VALUES ((SELECT id FROM def WHERE key='unit.repair_drone'),(SELECT id FROM tag WHERE key='mechanical'),8,3000,0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='unit.repair_drone'),'train','units',3,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.repair_drone'),(SELECT id FROM def WHERE key='res.wealth'),82,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.trireme'),'unit','frigate',276,0,(SELECT id FROM armor_class WHERE key='hull'),8000,1,175,'per_type',NULL,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.trireme'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.trireme'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.trireme'),(SELECT id FROM tag WHERE key='ship'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.trireme'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.trireme'),(SELECT id FROM movement_class WHERE key='ship'),2400);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.trireme'),20,(SELECT id FROM damage_class WHERE key='pierce'),5000,20,'arrow',15000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.trireme')),'fluid');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.trireme')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='unit.trireme'),'train','units',0,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.trireme'),(SELECT id FROM def WHERE key='res.timber'),64,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.trireme'),(SELECT id FROM def WHERE key='res.metal'),32,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.war_galley'),'unit','frigate',312,1,(SELECT id FROM armor_class WHERE key='hull'),8000,1,200,'per_type',(SELECT id FROM def WHERE key='unit.trireme'),(SELECT id FROM def WHERE key='age.medieval'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.war_galley'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.war_galley'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.war_galley'),(SELECT id FROM tag WHERE key='ship'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.war_galley'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.war_galley'),(SELECT id FROM movement_class WHERE key='ship'),2400);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.war_galley'),23,(SELECT id FROM damage_class WHERE key='pierce'),5000,20,'arrow',15000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.war_galley')),'fluid');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.war_galley')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='unit.war_galley'),'train','units',1,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.war_galley'),(SELECT id FROM def WHERE key='res.timber'),72,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.war_galley'),(SELECT id FROM def WHERE key='res.metal'),36,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.war_galley','Upgrade to War Galleys');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.war_galley'),'unit_upgrade','',0,400);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.war_galley'),(SELECT id FROM def WHERE key='res.timber'),144,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.war_galley'),(SELECT id FROM def WHERE key='res.metal'),72,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.war_galley'),'upgrade_type',(SELECT id FROM def WHERE key='unit.trireme'),(SELECT id FROM def WHERE key='unit.war_galley'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.carrack'),'unit','line_ship',870,1,(SELECT id FROM armor_class WHERE key='hull'),12000,1,225,'per_type',NULL,(SELECT id FROM def WHERE key='age.gunpowder'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.carrack'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.carrack'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.carrack'),(SELECT id FROM tag WHERE key='ship'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.carrack'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.carrack'),(SELECT id FROM movement_class WHERE key='ship'),1400);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,range_min,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.carrack'),52,(SELECT id FROM damage_class WHERE key='explosive'),12000,40,3000,'shell',24000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.carrack')),'fluid');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.carrack')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='unit.carrack'),'train','units',2,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.carrack'),(SELECT id FROM def WHERE key='res.timber'),232,4);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.carrack'),(SELECT id FROM def WHERE key='res.metal'),116,2);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.frigate'),'unit','frigate',384,2,(SELECT id FROM armor_class WHERE key='hull'),9000,1,250,'per_type',(SELECT id FROM def WHERE key='unit.war_galley'),(SELECT id FROM def WHERE key='age.enlightenment'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.frigate'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.frigate'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.frigate'),(SELECT id FROM tag WHERE key='ship'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.frigate'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.frigate'),(SELECT id FROM movement_class WHERE key='ship'),2400);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.frigate'),28,(SELECT id FROM damage_class WHERE key='explosive'),5000,20,'shell',24000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.frigate')),'fluid');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.frigate')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='unit.frigate'),'train','units',3,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.frigate'),(SELECT id FROM def WHERE key='res.timber'),89,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.frigate'),(SELECT id FROM def WHERE key='res.metal'),44,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.frigate','Upgrade to Frigates');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.frigate'),'unit_upgrade','',0,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.frigate'),(SELECT id FROM def WHERE key='res.timber'),178,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.frigate'),(SELECT id FROM def WHERE key='res.metal'),88,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.frigate'),'upgrade_type',(SELECT id FROM def WHERE key='unit.war_galley'),(SELECT id FROM def WHERE key='unit.frigate'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.destroyer'),'unit','destroyer',456,3,(SELECT id FROM armor_class WHERE key='hull'),10000,1,300,'per_type',NULL,(SELECT id FROM def WHERE key='age.modern'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.destroyer'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.destroyer'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.destroyer'),(SELECT id FROM tag WHERE key='ship'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.destroyer'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.destroyer'),(SELECT id FROM movement_class WHERE key='ship'),2400);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.destroyer'),34,(SELECT id FROM damage_class WHERE key='explosive'),5000,20,'shell',24000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.destroyer')),'fluid');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.destroyer')),'ground');
INSERT INTO et_weapon(entity_type_id,slot,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.destroyer'),1,38,(SELECT id FROM damage_class WHERE key='explosive'),6000,20,'bullet',40000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.destroyer') AND slot=1),'air');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='unit.destroyer'),'train','units',1,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.destroyer'),(SELECT id FROM def WHERE key='res.timber'),106,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.destroyer'),(SELECT id FROM def WHERE key='res.metal'),53,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_cruiser'),'unit','strike_ship',738,3,(SELECT id FROM armor_class WHERE key='hull'),15000,1,325,'per_type',NULL,(SELECT id FROM def WHERE key='age.information'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_cruiser'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_cruiser'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_cruiser'),(SELECT id FROM tag WHERE key='ship'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_cruiser'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.missile_cruiser'),(SELECT id FROM movement_class WHERE key='ship'),1800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,range_min,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.missile_cruiser'),82,(SELECT id FROM damage_class WHERE key='explosive'),15000,40,4000,'missile',20000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.missile_cruiser')),'fluid');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.missile_cruiser')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='unit.missile_cruiser'),'train','units',3,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.missile_cruiser'),(SELECT id FROM def WHERE key='res.timber'),246,4);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.missile_cruiser'),(SELECT id FROM def WHERE key='res.metal'),246,4);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_cruiser'),'unit','strike_ship',792,4,(SELECT id FROM armor_class WHERE key='hull'),16000,1,350,'per_type',(SELECT id FROM def WHERE key='unit.missile_cruiser'),(SELECT id FROM def WHERE key='age.automation'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_cruiser'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_cruiser'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_cruiser'),(SELECT id FROM tag WHERE key='ship'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_cruiser'),(SELECT id FROM tag WHERE key='drone'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_cruiser'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.drone_cruiser'),(SELECT id FROM movement_class WHERE key='ship'),1800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,range_min,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.drone_cruiser'),88,(SELECT id FROM damage_class WHERE key='explosive'),16000,40,4000,'missile',20000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.drone_cruiser')),'fluid');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.drone_cruiser')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='unit.drone_cruiser'),'train','units',4,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.drone_cruiser'),(SELECT id FROM def WHERE key='res.timber'),264,5);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.drone_cruiser'),(SELECT id FROM def WHERE key='res.metal'),264,5);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.drone_cruiser','Upgrade to Drone Cruisers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_cruiser'),'unit_upgrade','',0,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_cruiser'),(SELECT id FROM def WHERE key='res.timber'),528,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_cruiser'),(SELECT id FROM def WHERE key='res.metal'),528,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_cruiser'),'upgrade_type',(SELECT id FROM def WHERE key='unit.missile_cruiser'),(SELECT id FROM def WHERE key='unit.drone_cruiser'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.hover_frigate'),'unit','frigate',564,4,(SELECT id FROM armor_class WHERE key='hull'),11000,1,375,'per_type',(SELECT id FROM def WHERE key='unit.frigate'),(SELECT id FROM def WHERE key='age.fusion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hover_frigate'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hover_frigate'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hover_frigate'),(SELECT id FROM tag WHERE key='ship'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hover_frigate'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.hover_frigate'),(SELECT id FROM movement_class WHERE key='ship'),2400);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.hover_frigate'),42,(SELECT id FROM damage_class WHERE key='explosive'),5000,20,'plasma',30000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.hover_frigate')),'fluid');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.hover_frigate')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='unit.hover_frigate'),'train','units',0,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.hover_frigate'),(SELECT id FROM def WHERE key='res.timber'),131,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.hover_frigate'),(SELECT id FROM def WHERE key='res.metal'),65,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.hover_frigate','Upgrade to Hover Frigates');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.hover_frigate'),'unit_upgrade','',0,750);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.hover_frigate'),(SELECT id FROM def WHERE key='res.timber'),262,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.hover_frigate'),(SELECT id FROM def WHERE key='res.metal'),130,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.hover_frigate'),'upgrade_type',(SELECT id FROM def WHERE key='unit.frigate'),(SELECT id FROM def WHERE key='unit.hover_frigate'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.biplane'),'unit','fighter',262,2,(SELECT id FROM armor_class WHERE key='vehicle'),12000,1,275,'per_type',NULL,(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.biplane'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.biplane'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.biplane'),(SELECT id FROM tag WHERE key='aircraft'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.biplane'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.biplane'),(SELECT id FROM movement_class WHERE key='aircraft'),4000);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.biplane'),35,(SELECT id FROM damage_class WHERE key='bullet'),5000,30,'bullet',40000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.biplane')),'air');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='unit.biplane'),'train','units',0,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.biplane'),(SELECT id FROM def WHERE key='res.metal'),140,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.biplane'),(SELECT id FROM def WHERE key='res.oil'),70,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.fighter'),'unit','fighter',285,3,(SELECT id FROM armor_class WHERE key='vehicle'),12000,1,300,'per_type',(SELECT id FROM def WHERE key='unit.biplane'),(SELECT id FROM def WHERE key='age.modern'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.fighter'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.fighter'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.fighter'),(SELECT id FROM tag WHERE key='aircraft'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.fighter'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.fighter'),(SELECT id FROM movement_class WHERE key='aircraft'),4000);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.fighter'),38,(SELECT id FROM damage_class WHERE key='bullet'),5000,30,'bullet',40000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.fighter')),'air');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='unit.fighter'),'train','units',1,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.fighter'),(SELECT id FROM def WHERE key='res.metal'),152,3);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.fighter'),(SELECT id FROM def WHERE key='res.oil'),76,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.fighter','Upgrade to Fighters');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.fighter'),'unit_upgrade','',0,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.fighter'),(SELECT id FROM def WHERE key='res.metal'),304,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.fighter'),(SELECT id FROM def WHERE key='res.oil'),152,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.fighter'),'upgrade_type',(SELECT id FROM def WHERE key='unit.biplane'),(SELECT id FROM def WHERE key='unit.fighter'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.jet_fighter'),'unit','fighter',307,3,(SELECT id FROM armor_class WHERE key='vehicle'),12000,1,325,'per_type',(SELECT id FROM def WHERE key='unit.fighter'),(SELECT id FROM def WHERE key='age.information'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.jet_fighter'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.jet_fighter'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.jet_fighter'),(SELECT id FROM tag WHERE key='aircraft'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.jet_fighter'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.jet_fighter'),(SELECT id FROM movement_class WHERE key='aircraft'),4000);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.jet_fighter'),41,(SELECT id FROM damage_class WHERE key='bullet'),5000,30,'missile',20000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.jet_fighter')),'air');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='unit.jet_fighter'),'train','units',2,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.jet_fighter'),(SELECT id FROM def WHERE key='res.metal'),164,3);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.jet_fighter'),(SELECT id FROM def WHERE key='res.oil'),82,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.jet_fighter','Upgrade to Jet Fighters');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.jet_fighter'),'unit_upgrade','',0,650);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.jet_fighter'),(SELECT id FROM def WHERE key='res.metal'),328,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.jet_fighter'),(SELECT id FROM def WHERE key='res.oil'),164,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.jet_fighter'),'upgrade_type',(SELECT id FROM def WHERE key='unit.fighter'),(SELECT id FROM def WHERE key='unit.jet_fighter'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_fighter'),'unit','fighter',330,4,(SELECT id FROM armor_class WHERE key='vehicle'),13000,1,350,'per_type',(SELECT id FROM def WHERE key='unit.jet_fighter'),(SELECT id FROM def WHERE key='age.automation'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_fighter'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_fighter'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_fighter'),(SELECT id FROM tag WHERE key='aircraft'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_fighter'),(SELECT id FROM tag WHERE key='drone'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_fighter'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.drone_fighter'),(SELECT id FROM movement_class WHERE key='aircraft'),4000);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.drone_fighter'),44,(SELECT id FROM damage_class WHERE key='bullet'),5000,30,'missile',20000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.drone_fighter')),'air');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='unit.drone_fighter'),'train','units',3,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.drone_fighter'),(SELECT id FROM def WHERE key='res.metal'),176,3);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.drone_fighter'),(SELECT id FROM def WHERE key='res.oil'),88,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.drone_fighter','Upgrade to Drone Fighters');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_fighter'),'unit_upgrade','',0,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_fighter'),(SELECT id FROM def WHERE key='res.metal'),352,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_fighter'),(SELECT id FROM def WHERE key='res.oil'),176,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_fighter'),'upgrade_type',(SELECT id FROM def WHERE key='unit.jet_fighter'),(SELECT id FROM def WHERE key='unit.drone_fighter'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.interceptor'),'unit','fighter',352,4,(SELECT id FROM armor_class WHERE key='vehicle'),12000,1,375,'per_type',(SELECT id FROM def WHERE key='unit.drone_fighter'),(SELECT id FROM def WHERE key='age.fusion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.interceptor'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.interceptor'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.interceptor'),(SELECT id FROM tag WHERE key='aircraft'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.interceptor'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.interceptor'),(SELECT id FROM movement_class WHERE key='aircraft'),4000);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.interceptor'),47,(SELECT id FROM damage_class WHERE key='bullet'),5000,30,'plasma',30000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.interceptor')),'air');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='unit.interceptor'),'train','units',4,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.interceptor'),(SELECT id FROM def WHERE key='res.metal'),188,3);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.interceptor'),(SELECT id FROM def WHERE key='res.oil'),94,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.interceptor','Upgrade to Interceptors');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.interceptor'),'unit_upgrade','',0,750);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.interceptor'),(SELECT id FROM def WHERE key='res.metal'),376,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.interceptor'),(SELECT id FROM def WHERE key='res.oil'),188,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.interceptor'),'upgrade_type',(SELECT id FROM def WHERE key='unit.drone_fighter'),(SELECT id FROM def WHERE key='unit.interceptor'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.bomber'),'unit','bomber',285,3,(SELECT id FROM armor_class WHERE key='vehicle'),12000,1,300,'per_type',NULL,(SELECT id FROM def WHERE key='age.modern'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.bomber'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.bomber'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.bomber'),(SELECT id FROM tag WHERE key='aircraft'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.bomber'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.bomber'),(SELECT id FROM movement_class WHERE key='aircraft'),4000);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.bomber'),38,(SELECT id FROM damage_class WHERE key='explosive'),5000,30,'shell',24000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.bomber')),'ground');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.bomber')),'fluid');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='unit.bomber'),'train','units',0,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.bomber'),(SELECT id FROM def WHERE key='res.metal'),152,3);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.bomber'),(SELECT id FROM def WHERE key='res.oil'),76,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.strategic_bomber'),'unit','bomber',307,3,(SELECT id FROM armor_class WHERE key='vehicle'),12000,1,325,'per_type',(SELECT id FROM def WHERE key='unit.bomber'),(SELECT id FROM def WHERE key='age.information'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.strategic_bomber'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.strategic_bomber'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.strategic_bomber'),(SELECT id FROM tag WHERE key='aircraft'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.strategic_bomber'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.strategic_bomber'),(SELECT id FROM movement_class WHERE key='aircraft'),4000);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.strategic_bomber'),41,(SELECT id FROM damage_class WHERE key='explosive'),5000,30,'shell',24000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.strategic_bomber')),'ground');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.strategic_bomber')),'fluid');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='unit.strategic_bomber'),'train','units',1,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.strategic_bomber'),(SELECT id FROM def WHERE key='res.metal'),164,3);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.strategic_bomber'),(SELECT id FROM def WHERE key='res.oil'),82,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.strategic_bomber','Upgrade to Strategic Bombers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.strategic_bomber'),'unit_upgrade','',0,650);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.strategic_bomber'),(SELECT id FROM def WHERE key='res.metal'),328,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.strategic_bomber'),(SELECT id FROM def WHERE key='res.oil'),164,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.strategic_bomber'),'upgrade_type',(SELECT id FROM def WHERE key='unit.bomber'),(SELECT id FROM def WHERE key='unit.strategic_bomber'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_bomber'),'unit','bomber',330,4,(SELECT id FROM armor_class WHERE key='vehicle'),13000,1,350,'per_type',(SELECT id FROM def WHERE key='unit.strategic_bomber'),(SELECT id FROM def WHERE key='age.automation'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_bomber'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_bomber'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_bomber'),(SELECT id FROM tag WHERE key='aircraft'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_bomber'),(SELECT id FROM tag WHERE key='drone'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_bomber'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.drone_bomber'),(SELECT id FROM movement_class WHERE key='aircraft'),4000);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.drone_bomber'),44,(SELECT id FROM damage_class WHERE key='explosive'),5000,30,'shell',24000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.drone_bomber')),'ground');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.drone_bomber')),'fluid');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='unit.drone_bomber'),'train','units',2,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.drone_bomber'),(SELECT id FROM def WHERE key='res.metal'),176,3);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.drone_bomber'),(SELECT id FROM def WHERE key='res.oil'),88,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.drone_bomber','Upgrade to Drone Bombers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_bomber'),'unit_upgrade','',0,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_bomber'),(SELECT id FROM def WHERE key='res.metal'),352,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_bomber'),(SELECT id FROM def WHERE key='res.oil'),176,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_bomber'),'upgrade_type',(SELECT id FROM def WHERE key='unit.strategic_bomber'),(SELECT id FROM def WHERE key='unit.drone_bomber'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.survey_ship'),'unit','survey',1000,5,(SELECT id FROM armor_class WHERE key='shield'),27000,2,400,'per_type',NULL,(SELECT id FROM def WHERE key='age.starfaring'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.survey_ship'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.survey_ship'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.survey_ship'),(SELECT id FROM tag WHERE key='starship'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.survey_ship'),(SELECT id FROM movement_class WHERE key='starship'),3000);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile) VALUES ((SELECT id FROM def WHERE key='unit.survey_ship'),75,(SELECT id FROM damage_class WHERE key='energy'),8000,30,'beam');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.survey_ship')),'space');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.survey_ship')),'orbit');
INSERT INTO et_container(entity_type_id,capacity,accept_tag_id) VALUES ((SELECT id FROM def WHERE key='unit.survey_ship'),1,(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='unit.survey_ship'),'train','units',0,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.survey_ship'),(SELECT id FROM def WHERE key='res.metal'),500,10);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.survey_ship'),(SELECT id FROM def WHERE key='res.energy'),250,5);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.corvette'),'unit','corvette',800,3,(SELECT id FROM armor_class WHERE key='shield'),19000,1,400,'per_type',NULL,(SELECT id FROM def WHERE key='age.starfaring'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.corvette'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.corvette'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.corvette'),(SELECT id FROM tag WHERE key='starship'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.corvette'),(SELECT id FROM movement_class WHERE key='starship'),3600);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile) VALUES ((SELECT id FROM def WHERE key='unit.corvette'),45,(SELECT id FROM damage_class WHERE key='kinetic'),6000,15,'railgun');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.corvette')),'space');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.corvette')),'orbit');
INSERT INTO et_container(entity_type_id,capacity,accept_tag_id) VALUES ((SELECT id FROM def WHERE key='unit.corvette'),1,(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='unit.corvette'),'train','units',1,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.corvette'),(SELECT id FROM def WHERE key='res.metal'),300,6);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.corvette'),(SELECT id FROM def WHERE key='res.energy'),150,3);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.space_frigate'),'unit','space_frigate',1300,5,(SELECT id FROM armor_class WHERE key='shield'),22000,2,400,'per_type',NULL,(SELECT id FROM def WHERE key='age.starfaring'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.space_frigate'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.space_frigate'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.space_frigate'),(SELECT id FROM tag WHERE key='starship'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.space_frigate'),(SELECT id FROM movement_class WHERE key='starship'),3200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile) VALUES ((SELECT id FROM def WHERE key='unit.space_frigate'),40,(SELECT id FROM damage_class WHERE key='energy'),7000,10,'beam');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.space_frigate')),'space');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.space_frigate')),'orbit');
INSERT INTO et_container(entity_type_id,capacity,accept_tag_id) VALUES ((SELECT id FROM def WHERE key='unit.space_frigate'),2,(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='unit.space_frigate'),'train','units',2,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.space_frigate'),(SELECT id FROM def WHERE key='res.metal'),450,9);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.space_frigate'),(SELECT id FROM def WHERE key='res.energy'),200,4);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.space_cruiser'),'unit','space_cruiser',2000,6,(SELECT id FROM armor_class WHERE key='shield'),25000,2,425,'per_type',NULL,(SELECT id FROM def WHERE key='age.expansion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.space_cruiser'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.space_cruiser'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.space_cruiser'),(SELECT id FROM tag WHERE key='starship'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.space_cruiser'),(SELECT id FROM movement_class WHERE key='starship'),2600);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile) VALUES ((SELECT id FROM def WHERE key='unit.space_cruiser'),120,(SELECT id FROM damage_class WHERE key='kinetic'),10000,40,'railgun');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.space_cruiser')),'space');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.space_cruiser')),'orbit');
INSERT INTO et_container(entity_type_id,capacity,accept_tag_id) VALUES ((SELECT id FROM def WHERE key='unit.space_cruiser'),2,(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='unit.space_cruiser'),'train','units',3,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.space_cruiser'),(SELECT id FROM def WHERE key='res.metal'),700,14);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.space_cruiser'),(SELECT id FROM def WHERE key='res.energy'),300,6);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.battleship'),'unit','battleship',3600,8,(SELECT id FROM armor_class WHERE key='shield'),28000,3,450,'per_type',NULL,(SELECT id FROM def WHERE key='age.abundance'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.battleship'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.battleship'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.battleship'),(SELECT id FROM tag WHERE key='starship'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.battleship'),(SELECT id FROM movement_class WHERE key='starship'),2000);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.battleship'),260,(SELECT id FROM damage_class WHERE key='kinetic'),13000,60,'plasma',30000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.battleship')),'space');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.battleship')),'orbit');
INSERT INTO et_container(entity_type_id,capacity,accept_tag_id) VALUES ((SELECT id FROM def WHERE key='unit.battleship'),2,(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='unit.battleship'),'train','units',4,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.battleship'),(SELECT id FROM def WHERE key='res.metal'),1200,24);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.battleship'),(SELECT id FROM def WHERE key='res.energy'),500,10);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),'unit','civilian',40,0,(SELECT id FROM armor_class WHERE key='flesh'),4000,1,150,'per_type',NULL,(SELECT id FROM def WHERE key='age.ancient'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM tag WHERE key='civilian'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM movement_class WHERE key='foot'),1300);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),2,(SELECT id FROM damage_class WHERE key='melee'),0,40);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.citizen')),'ground');
INSERT INTO et_builder(entity_type_id) VALUES ((SELECT id FROM def WHERE key='unit.citizen'));
INSERT INTO et_harvester(entity_type_id,resource_id,amount) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='res.food'),10);
INSERT INTO et_harvester(entity_type_id,resource_id,amount) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='res.timber'),10);
INSERT INTO et_harvester(entity_type_id,resource_id,amount) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='res.metal'),10);
INSERT INTO et_harvester(entity_type_id,resource_id,amount) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='res.oil'),10);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.city'),(SELECT id FROM def WHERE key='unit.citizen'),'train','units',0,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='res.food'),20,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),'unit','civilian',40,4,(SELECT id FROM armor_class WHERE key='flesh'),4000,1,350,'per_type',(SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='age.automation'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM tag WHERE key='civilian'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM tag WHERE key='drone'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM movement_class WHERE key='foot'),1300);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),4,(SELECT id FROM damage_class WHERE key='melee'),0,40);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.worker_drone')),'ground');
INSERT INTO et_builder(entity_type_id) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'));
INSERT INTO et_harvester(entity_type_id,resource_id,amount) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='res.food'),10);
INSERT INTO et_harvester(entity_type_id,resource_id,amount) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='res.timber'),10);
INSERT INTO et_harvester(entity_type_id,resource_id,amount) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='res.metal'),10);
INSERT INTO et_harvester(entity_type_id,resource_id,amount) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='res.oil'),10);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.city'),(SELECT id FROM def WHERE key='unit.worker_drone'),'train','units',1,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='res.food'),44,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.worker_drone','Upgrade to Worker Drones');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.worker_drone'),'unit_upgrade','',0,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.worker_drone'),(SELECT id FROM def WHERE key='res.food'),88,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.worker_drone'),'upgrade_type',(SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='unit.worker_drone'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.scholar'),'unit','scholar',50,0,(SELECT id FROM armor_class WHERE key='flesh'),5000,1,175,'per_type',NULL,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.scholar'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.scholar'),(SELECT id FROM tag WHERE key='civilian'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.scholar'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.scholar'),(SELECT id FROM movement_class WHERE key='foot'),1300);
INSERT INTO et_harvester(entity_type_id,resource_id,amount) VALUES ((SELECT id FROM def WHERE key='unit.scholar'),(SELECT id FROM def WHERE key='res.knowledge'),10);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM def WHERE key='unit.scholar'),'train','units',0,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.scholar'),(SELECT id FROM def WHERE key='res.wealth'),46,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.merchant'),'unit','merchant',50,0,(SELECT id FROM armor_class WHERE key='flesh'),5000,1,150,'per_type',NULL,(SELECT id FROM def WHERE key='age.ancient'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.merchant'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.merchant'),(SELECT id FROM tag WHERE key='civilian'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.merchant'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.merchant'),(SELECT id FROM movement_class WHERE key='foot'),1500);
INSERT INTO et_trader(entity_type_id,runs_routes,claims_nodes,route_resource_id,route_income) VALUES ((SELECT id FROM def WHERE key='unit.merchant'),1,0,(SELECT id FROM def WHERE key='res.wealth'),3);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.market'),(SELECT id FROM def WHERE key='unit.merchant'),'train','units',0,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.merchant'),(SELECT id FROM def WHERE key='res.wealth'),40,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.caravan'),'unit','caravan',90,0,(SELECT id FROM armor_class WHERE key='vehicle'),5000,1,175,'per_type',NULL,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.caravan'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.caravan'),(SELECT id FROM tag WHERE key='civilian'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.caravan'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.caravan'),(SELECT id FROM movement_class WHERE key='wheeled'),1300);
INSERT INTO et_trader(entity_type_id,runs_routes,claims_nodes,route_resource_id,route_income) VALUES ((SELECT id FROM def WHERE key='unit.caravan'),1,0,(SELECT id FROM def WHERE key='res.wealth'),6);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.market'),(SELECT id FROM def WHERE key='unit.caravan'),'train','units',1,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.caravan'),(SELECT id FROM def WHERE key='res.food'),46,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.caravan'),(SELECT id FROM def WHERE key='res.timber'),69,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.freight_wagon'),'unit','caravan',130,2,(SELECT id FROM armor_class WHERE key='vehicle'),5000,1,250,'per_type',(SELECT id FROM def WHERE key='unit.caravan'),(SELECT id FROM def WHERE key='age.enlightenment'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.freight_wagon'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.freight_wagon'),(SELECT id FROM tag WHERE key='civilian'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.freight_wagon'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.freight_wagon'),(SELECT id FROM movement_class WHERE key='wheeled'),1600);
INSERT INTO et_trader(entity_type_id,runs_routes,claims_nodes,route_resource_id,route_income) VALUES ((SELECT id FROM def WHERE key='unit.freight_wagon'),1,0,(SELECT id FROM def WHERE key='res.wealth'),9);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.market'),(SELECT id FROM def WHERE key='unit.freight_wagon'),'train','units',2,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.freight_wagon'),(SELECT id FROM def WHERE key='res.food'),64,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.freight_wagon'),(SELECT id FROM def WHERE key='res.timber'),96,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.freight_wagon','Upgrade to Freight Wagons');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.freight_wagon'),'unit_upgrade','',0,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.freight_wagon'),(SELECT id FROM def WHERE key='res.food'),128,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.freight_wagon'),(SELECT id FROM def WHERE key='res.timber'),192,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.freight_wagon'),'upgrade_type',(SELECT id FROM def WHERE key='unit.caravan'),(SELECT id FROM def WHERE key='unit.freight_wagon'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.lorry'),'unit','caravan',170,3,(SELECT id FROM armor_class WHERE key='vehicle'),5000,1,300,'per_type',(SELECT id FROM def WHERE key='unit.freight_wagon'),(SELECT id FROM def WHERE key='age.modern'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.lorry'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.lorry'),(SELECT id FROM tag WHERE key='civilian'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.lorry'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.lorry'),(SELECT id FROM movement_class WHERE key='wheeled'),1900);
INSERT INTO et_trader(entity_type_id,runs_routes,claims_nodes,route_resource_id,route_income) VALUES ((SELECT id FROM def WHERE key='unit.lorry'),1,0,(SELECT id FROM def WHERE key='res.wealth'),12);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.market'),(SELECT id FROM def WHERE key='unit.lorry'),'train','units',3,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.lorry'),(SELECT id FROM def WHERE key='res.food'),76,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.lorry'),(SELECT id FROM def WHERE key='res.metal'),114,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.lorry'),(SELECT id FROM def WHERE key='res.oil'),38,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.lorry','Upgrade to Lorries');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.lorry'),'unit_upgrade','',0,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.lorry'),(SELECT id FROM def WHERE key='res.food'),152,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.lorry'),(SELECT id FROM def WHERE key='res.metal'),228,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.lorry'),(SELECT id FROM def WHERE key='res.oil'),76,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.lorry'),'upgrade_type',(SELECT id FROM def WHERE key='unit.freight_wagon'),(SELECT id FROM def WHERE key='unit.lorry'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.cargo_drone'),'unit','caravan',210,4,(SELECT id FROM armor_class WHERE key='vehicle'),5000,1,350,'per_type',(SELECT id FROM def WHERE key='unit.lorry'),(SELECT id FROM def WHERE key='age.automation'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cargo_drone'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cargo_drone'),(SELECT id FROM tag WHERE key='civilian'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cargo_drone'),(SELECT id FROM tag WHERE key='drone'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cargo_drone'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.cargo_drone'),(SELECT id FROM movement_class WHERE key='wheeled'),2200);
INSERT INTO et_trader(entity_type_id,runs_routes,claims_nodes,route_resource_id,route_income) VALUES ((SELECT id FROM def WHERE key='unit.cargo_drone'),1,0,(SELECT id FROM def WHERE key='res.wealth'),15);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.market'),(SELECT id FROM def WHERE key='unit.cargo_drone'),'train','units',4,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.cargo_drone'),(SELECT id FROM def WHERE key='res.food'),88,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.cargo_drone'),(SELECT id FROM def WHERE key='res.metal'),132,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.cargo_drone'),(SELECT id FROM def WHERE key='res.oil'),44,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.cargo_drone','Upgrade to Cargo Drones');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.cargo_drone'),'unit_upgrade','',0,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.cargo_drone'),(SELECT id FROM def WHERE key='res.food'),176,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.cargo_drone'),(SELECT id FROM def WHERE key='res.metal'),264,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.cargo_drone'),(SELECT id FROM def WHERE key='res.oil'),88,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.cargo_drone'),'upgrade_type',(SELECT id FROM def WHERE key='unit.lorry'),(SELECT id FROM def WHERE key='unit.cargo_drone'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.priest'),'unit','priest',35,0,(SELECT id FROM armor_class WHERE key='flesh'),5000,1,175,'per_type',NULL,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.priest'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.priest'),(SELECT id FROM tag WHERE key='civilian'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.priest'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.priest'),(SELECT id FROM movement_class WHERE key='foot'),1100);
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.priest'),(SELECT id FROM tag WHERE key='clergy'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.priest'),(SELECT id FROM tag WHERE key='healer'));
INSERT INTO et_healer(entity_type_id,target_tag_id,hp_per_second,range,energy_per_hp,can_convert) VALUES ((SELECT id FROM def WHERE key='unit.priest'),(SELECT id FROM tag WHERE key='biological'),4,5000,0,1);
INSERT INTO et_energy(entity_type_id,max,regen_per_second) VALUES ((SELECT id FROM def WHERE key='unit.priest'),100,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.temple'),(SELECT id FROM def WHERE key='unit.priest'),'train','units',0,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.priest'),(SELECT id FROM def WHERE key='res.wealth'),46,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.transport_ship'),'unit','transport',1500,5,(SELECT id FROM armor_class WHERE key='shield'),22000,2,400,'per_type',NULL,(SELECT id FROM def WHERE key='age.starfaring'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.transport_ship'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.transport_ship'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.transport_ship'),(SELECT id FROM tag WHERE key='starship'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.transport_ship'),(SELECT id FROM movement_class WHERE key='starship'),2600);
INSERT INTO et_container(entity_type_id,capacity,accept_tag_id) VALUES ((SELECT id FROM def WHERE key='unit.transport_ship'),20,(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='unit.transport_ship'),'train','units',0,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.transport_ship'),(SELECT id FROM def WHERE key='res.metal'),600,12);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.transport_ship'),(SELECT id FROM def WHERE key='res.energy'),200,4);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.military.1','Militia');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.military.1'),'track','military',1,400);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.1'),(SELECT id FROM def WHERE key='res.food'),200,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.1'),(SELECT id FROM def WHERE key='res.timber'),200,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.military.1'),'pop_cap','add',25);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.military.2','Drill');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.military.2'),'track','military',2,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.2'),(SELECT id FROM def WHERE key='res.food'),300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.2'),(SELECT id FROM def WHERE key='res.knowledge'),300,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.military.2'),'pop_cap','add',25);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.military.3','Standing Army');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.military.3'),'track','military',3,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.3'),(SELECT id FROM def WHERE key='res.food'),400,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.3'),(SELECT id FROM def WHERE key='res.knowledge'),400,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.military.3'),'pop_cap','add',25);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.military.4','Conscription');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.military.4'),'track','military',4,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.4'),(SELECT id FROM def WHERE key='res.food'),500,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.4'),(SELECT id FROM def WHERE key='res.knowledge'),500,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.military.4'),'pop_cap','add',25);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.military.5','Professional Army');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.military.5'),'track','military',5,800);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.5'),(SELECT id FROM def WHERE key='res.food'),600,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.5'),(SELECT id FROM def WHERE key='res.knowledge'),600,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.military.5'),'pop_cap','add',25);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.military.6','Combined Arms');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.military.6'),'track','military',6,900);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.6'),(SELECT id FROM def WHERE key='res.food'),700,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.6'),(SELECT id FROM def WHERE key='res.knowledge'),700,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.military.6'),'pop_cap','add',25);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.military.7','Mechanised Warfare');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.military.7'),'track','military',7,1000);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.7'),(SELECT id FROM def WHERE key='res.food'),800,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.7'),(SELECT id FROM def WHERE key='res.knowledge'),800,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.military.7'),'pop_cap','add',25);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.military.8','Network Warfare');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.military.8'),'track','military',8,1100);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.8'),(SELECT id FROM def WHERE key='res.food'),900,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.8'),(SELECT id FROM def WHERE key='res.knowledge'),900,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.military.8'),'pop_cap','add',25);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.military.9','Autonomous Command');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.military.9'),'track','military',9,1200);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.9'),(SELECT id FROM def WHERE key='res.food'),1000,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.9'),(SELECT id FROM def WHERE key='res.knowledge'),1000,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.military.9'),'pop_cap','add',25);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.military.10','Orbital Doctrine');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.military.10'),'track','military',10,1300);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.10'),(SELECT id FROM def WHERE key='res.food'),1100,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.10'),(SELECT id FROM def WHERE key='res.knowledge'),1100,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.military.10'),'pop_cap','add',25);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.military.11','Fleet Doctrine');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.military.11'),'track','military',11,1400);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.11'),(SELECT id FROM def WHERE key='res.food'),1200,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.11'),(SELECT id FROM def WHERE key='res.knowledge'),1200,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.military.11'),'pop_cap','add',25);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.military.12','Grand Strategy');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.military.12'),'track','military',12,1500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.12'),(SELECT id FROM def WHERE key='res.food'),1300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.12'),(SELECT id FROM def WHERE key='res.knowledge'),1300,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.military.12'),'pop_cap','add',25);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.military.13','Total Defence');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.military.13'),'track','military',13,1600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.13'),(SELECT id FROM def WHERE key='res.food'),1400,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.military.13'),(SELECT id FROM def WHERE key='res.knowledge'),1400,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.military.13'),'pop_cap','add',25);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.civic.1','Chiefdom');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.civic.1'),'track','civic',1,400);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.1'),(SELECT id FROM def WHERE key='res.food'),200,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.1'),(SELECT id FROM def WHERE key='res.timber'),200,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.civic.1'),'city_limit','add',1);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.1'),'border_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.1'),'economic_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.civic.2','City State');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.civic.2'),'track','civic',2,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.2'),(SELECT id FROM def WHERE key='res.food'),300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.2'),(SELECT id FROM def WHERE key='res.knowledge'),300,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.civic.2'),'city_limit','add',1);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.2'),'border_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.2'),'economic_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.civic.3','Kingdom');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.civic.3'),'track','civic',3,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.3'),(SELECT id FROM def WHERE key='res.food'),400,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.3'),(SELECT id FROM def WHERE key='res.knowledge'),400,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.civic.3'),'city_limit','add',1);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.3'),'border_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.3'),'economic_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.civic.4','Charter');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.civic.4'),'track','civic',4,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.4'),(SELECT id FROM def WHERE key='res.food'),500,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.4'),(SELECT id FROM def WHERE key='res.knowledge'),500,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.civic.4'),'city_limit','add',1);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.4'),'border_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.4'),'economic_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.civic.5','Constitution');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.civic.5'),'track','civic',5,800);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.5'),(SELECT id FROM def WHERE key='res.food'),600,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.5'),(SELECT id FROM def WHERE key='res.knowledge'),600,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.civic.5'),'city_limit','add',1);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.5'),'border_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.5'),'economic_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.civic.6','Nation State');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.civic.6'),'track','civic',6,900);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.6'),(SELECT id FROM def WHERE key='res.food'),700,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.6'),(SELECT id FROM def WHERE key='res.knowledge'),700,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.civic.6'),'city_limit','add',1);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.6'),'border_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.6'),'economic_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.civic.7','Federation');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.civic.7'),'track','civic',7,1000);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.7'),(SELECT id FROM def WHERE key='res.food'),800,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.7'),(SELECT id FROM def WHERE key='res.knowledge'),800,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.civic.7'),'city_limit','add',1);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.7'),'border_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.7'),'economic_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.civic.8','Information Society');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.civic.8'),'track','civic',8,1100);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.8'),(SELECT id FROM def WHERE key='res.food'),900,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.8'),(SELECT id FROM def WHERE key='res.knowledge'),900,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.civic.8'),'city_limit','add',1);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.8'),'border_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.8'),'economic_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.civic.9','Planned Society');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.civic.9'),'track','civic',9,1200);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.9'),(SELECT id FROM def WHERE key='res.food'),1000,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.9'),(SELECT id FROM def WHERE key='res.knowledge'),1000,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.civic.9'),'city_limit','add',1);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.9'),'border_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.9'),'economic_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.civic.10','Continental Union');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.civic.10'),'track','civic',10,1300);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.10'),(SELECT id FROM def WHERE key='res.food'),1100,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.10'),(SELECT id FROM def WHERE key='res.knowledge'),1100,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.civic.10'),'city_limit','add',1);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.10'),'border_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.10'),'economic_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.civic.11','Interplanetary Law');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.civic.11'),'track','civic',11,1400);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.11'),(SELECT id FROM def WHERE key='res.food'),1200,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.11'),(SELECT id FROM def WHERE key='res.knowledge'),1200,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.civic.11'),'city_limit','add',1);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.11'),'border_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.11'),'economic_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.civic.12','Commonwealth');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.civic.12'),'track','civic',12,1500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.12'),(SELECT id FROM def WHERE key='res.food'),1300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.12'),(SELECT id FROM def WHERE key='res.knowledge'),1300,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.civic.12'),'city_limit','add',1);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.12'),'border_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.12'),'economic_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.civic.13','Galactic Charter');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.civic.13'),'track','civic',13,1600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.13'),(SELECT id FROM def WHERE key='res.food'),1400,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.civic.13'),(SELECT id FROM def WHERE key='res.knowledge'),1400,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.civic.13'),'city_limit','add',1);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.13'),'border_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.civic.13'),'economic_radius','mul',106,(SELECT id FROM tag WHERE key='city'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.commerce.1','Barter');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.commerce.1'),'track','commerce',1,400);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.1'),(SELECT id FROM def WHERE key='res.food'),200,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.1'),(SELECT id FROM def WHERE key='res.timber'),200,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.1'),'commerce_cap','add',40);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.1'),'caravan_limit','add',1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.commerce.2','Coinage');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.commerce.2'),'track','commerce',2,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.2'),(SELECT id FROM def WHERE key='res.food'),300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.2'),(SELECT id FROM def WHERE key='res.knowledge'),300,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.2'),'commerce_cap','add',40);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.2'),'caravan_limit','add',1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.commerce.3','Trade Routes');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.commerce.3'),'track','commerce',3,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.3'),(SELECT id FROM def WHERE key='res.food'),400,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.3'),(SELECT id FROM def WHERE key='res.knowledge'),400,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.3'),'commerce_cap','add',40);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.3'),'caravan_limit','add',1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.commerce.4','Banking');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.commerce.4'),'track','commerce',4,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.4'),(SELECT id FROM def WHERE key='res.food'),500,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.4'),(SELECT id FROM def WHERE key='res.knowledge'),500,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.4'),'commerce_cap','add',40);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.4'),'caravan_limit','add',1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.commerce.5','Joint Stock');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.commerce.5'),'track','commerce',5,800);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.5'),(SELECT id FROM def WHERE key='res.food'),600,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.5'),(SELECT id FROM def WHERE key='res.knowledge'),600,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.5'),'commerce_cap','add',40);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.5'),'caravan_limit','add',1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.commerce.6','Industry');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.commerce.6'),'track','commerce',6,900);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.6'),(SELECT id FROM def WHERE key='res.food'),700,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.6'),(SELECT id FROM def WHERE key='res.knowledge'),700,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.6'),'commerce_cap','add',40);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.6'),'caravan_limit','add',1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.commerce.7','Mass Production');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.commerce.7'),'track','commerce',7,1000);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.7'),(SELECT id FROM def WHERE key='res.food'),800,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.7'),(SELECT id FROM def WHERE key='res.knowledge'),800,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.7'),'commerce_cap','add',40);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.7'),'caravan_limit','add',1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.commerce.8','Globalisation');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.commerce.8'),'track','commerce',8,1100);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.8'),(SELECT id FROM def WHERE key='res.food'),900,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.8'),(SELECT id FROM def WHERE key='res.knowledge'),900,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.8'),'commerce_cap','add',40);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.8'),'caravan_limit','add',1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.commerce.9','Automated Logistics');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.commerce.9'),'track','commerce',9,1200);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.9'),(SELECT id FROM def WHERE key='res.food'),1000,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.9'),(SELECT id FROM def WHERE key='res.knowledge'),1000,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.9'),'commerce_cap','add',40);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.9'),'caravan_limit','add',1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.commerce.10','Fusion Economy');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.commerce.10'),'track','commerce',10,1300);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.10'),(SELECT id FROM def WHERE key='res.food'),1100,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.10'),(SELECT id FROM def WHERE key='res.knowledge'),1100,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.10'),'commerce_cap','add',40);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.10'),'caravan_limit','add',1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.commerce.11','Trade Lanes');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.commerce.11'),'track','commerce',11,1400);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.11'),(SELECT id FROM def WHERE key='res.food'),1200,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.11'),(SELECT id FROM def WHERE key='res.knowledge'),1200,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.11'),'commerce_cap','add',40);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.11'),'caravan_limit','add',1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.commerce.12','Post-scarcity Markets');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.commerce.12'),'track','commerce',12,1500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.12'),(SELECT id FROM def WHERE key='res.food'),1300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.12'),(SELECT id FROM def WHERE key='res.knowledge'),1300,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.12'),'commerce_cap','add',40);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.12'),'caravan_limit','add',1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.commerce.13','Galactic Exchange');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.commerce.13'),'track','commerce',13,1600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.13'),(SELECT id FROM def WHERE key='res.food'),1400,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.commerce.13'),(SELECT id FROM def WHERE key='res.knowledge'),1400,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.13'),'commerce_cap','add',40);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.commerce.13'),'caravan_limit','add',1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.science.1','Writing');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.science.1'),'track','science',1,400);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.1'),(SELECT id FROM def WHERE key='res.food'),200,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.1'),(SELECT id FROM def WHERE key='res.timber'),200,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.science.1'),'research_time','pct',-4);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.science.2','Mathematics');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.science.2'),'track','science',2,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.2'),(SELECT id FROM def WHERE key='res.food'),300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.2'),(SELECT id FROM def WHERE key='res.knowledge'),300,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.science.2'),'research_time','pct',-4);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.science.3','Astronomy');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.science.3'),'track','science',3,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.3'),(SELECT id FROM def WHERE key='res.food'),400,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.3'),(SELECT id FROM def WHERE key='res.knowledge'),400,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.science.3'),'research_time','pct',-4);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.science.4','Optics');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.science.4'),'track','science',4,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.4'),(SELECT id FROM def WHERE key='res.food'),500,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.4'),(SELECT id FROM def WHERE key='res.knowledge'),500,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.science.4'),'research_time','pct',-4);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.science.5','Natural Philosophy');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.science.5'),'track','science',5,800);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.5'),(SELECT id FROM def WHERE key='res.food'),600,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.5'),(SELECT id FROM def WHERE key='res.knowledge'),600,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.science.5'),'research_time','pct',-4);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.science.6','Electricity');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.science.6'),'track','science',6,900);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.6'),(SELECT id FROM def WHERE key='res.food'),700,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.6'),(SELECT id FROM def WHERE key='res.knowledge'),700,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.science.6'),'research_time','pct',-4);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.science.7','Electronics');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.science.7'),'track','science',7,1000);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.7'),(SELECT id FROM def WHERE key='res.food'),800,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.7'),(SELECT id FROM def WHERE key='res.knowledge'),800,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.science.7'),'research_time','pct',-4);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.science.8','Computing');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.science.8'),'track','science',8,1100);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.8'),(SELECT id FROM def WHERE key='res.food'),900,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.8'),(SELECT id FROM def WHERE key='res.knowledge'),900,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.science.8'),'research_time','pct',-4);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.science.9','Machine Learning');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.science.9'),'track','science',9,1200);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.9'),(SELECT id FROM def WHERE key='res.food'),1000,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.9'),(SELECT id FROM def WHERE key='res.knowledge'),1000,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.science.9'),'research_time','pct',-4);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.science.10','Fusion Power');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.science.10'),'track','science',10,1300);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.10'),(SELECT id FROM def WHERE key='res.food'),1100,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.10'),(SELECT id FROM def WHERE key='res.knowledge'),1100,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.science.10'),'research_time','pct',-4);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.science.11','Spacetime Theory');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.science.11'),'track','science',11,1400);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.11'),(SELECT id FROM def WHERE key='res.food'),1200,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.11'),(SELECT id FROM def WHERE key='res.knowledge'),1200,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.science.11'),'research_time','pct',-4);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.science.12','Matter Synthesis');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.science.12'),'track','science',12,1500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.12'),(SELECT id FROM def WHERE key='res.food'),1300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.12'),(SELECT id FROM def WHERE key='res.knowledge'),1300,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.science.12'),'research_time','pct',-4);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.science.13','Unified Theory');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.science.13'),'track','science',13,1600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.13'),(SELECT id FROM def WHERE key='res.food'),1400,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.science.13'),(SELECT id FROM def WHERE key='res.knowledge'),1400,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.science.13'),'research_time','pct',-4);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.military.2'),'los','add',1000,(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.science.2'),'range','add',1000,(SELECT id FROM tag WHERE key='tower'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.science.2'),'los','add',1500,(SELECT id FROM tag WHERE key='tower'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.science.2'),'range','add',1000,(SELECT id FROM tag WHERE key='city'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.science.2'),'los','add',1500,(SELECT id FROM tag WHERE key='city'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.science.3'),'los','add',2000,(SELECT id FROM tag WHERE key='ship'));
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.science.4'),'los','pct',15);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.science.7'),'los','pct',20,(SELECT id FROM tag WHERE key='building'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.science.7'),'los','pct',20,(SELECT id FROM tag WHERE key='aircraft'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.military.8'),'los','pct',15,(SELECT id FROM tag WHERE key='military'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.military.10'),'los','pct',25,(SELECT id FROM tag WHERE key='starship'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.age.classical','Advance to Classical');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.age.classical'),'age','',1,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.age.classical'),(SELECT id FROM def WHERE key='res.food'),250,0);
INSERT INTO tech_effect(tech_id,effect,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.age.classical'),'set_age',(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.age.medieval','Advance to Medieval');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.age.medieval'),'age','',2,800);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.age.medieval'),(SELECT id FROM def WHERE key='res.food'),500,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.age.medieval'),(SELECT id FROM def WHERE key='res.knowledge'),500,0);
INSERT INTO tech_effect(tech_id,effect,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.age.medieval'),'set_age',(SELECT id FROM def WHERE key='age.medieval'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.age.gunpowder','Advance to Gunpowder');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.age.gunpowder'),'age','',3,1000);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.age.gunpowder'),(SELECT id FROM def WHERE key='res.food'),750,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.age.gunpowder'),(SELECT id FROM def WHERE key='res.knowledge'),750,0);
INSERT INTO tech_effect(tech_id,effect,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.age.gunpowder'),'set_age',(SELECT id FROM def WHERE key='age.gunpowder'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.age.enlightenment','Advance to Enlightenment');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.age.enlightenment'),'age','',4,1200);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.age.enlightenment'),(SELECT id FROM def WHERE key='res.food'),1000,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.age.enlightenment'),(SELECT id FROM def WHERE key='res.knowledge'),1000,0);
INSERT INTO tech_effect(tech_id,effect,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.age.enlightenment'),'set_age',(SELECT id FROM def WHERE key='age.enlightenment'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.age.industrial','Advance to Industrial');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.age.industrial'),'age','',5,1400);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.age.industrial'),(SELECT id FROM def WHERE key='res.food'),1250,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.age.industrial'),(SELECT id FROM def WHERE key='res.knowledge'),1250,0);
INSERT INTO tech_effect(tech_id,effect,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.age.industrial'),'set_age',(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.age.modern','Advance to Modern');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.age.modern'),'age','',6,1600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.age.modern'),(SELECT id FROM def WHERE key='res.food'),1500,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.age.modern'),(SELECT id FROM def WHERE key='res.knowledge'),1500,0);
INSERT INTO tech_effect(tech_id,effect,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.age.modern'),'set_age',(SELECT id FROM def WHERE key='age.modern'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.age.information','Advance to Information');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.age.information'),'age','',7,1800);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.age.information'),(SELECT id FROM def WHERE key='res.food'),1750,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.age.information'),(SELECT id FROM def WHERE key='res.knowledge'),1750,0);
INSERT INTO tech_effect(tech_id,effect,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.age.information'),'set_age',(SELECT id FROM def WHERE key='age.information'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.age.automation','Advance to Automation');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.age.automation'),'age','',8,2000);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.age.automation'),(SELECT id FROM def WHERE key='res.food'),2000,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.age.automation'),(SELECT id FROM def WHERE key='res.knowledge'),2000,0);
INSERT INTO tech_effect(tech_id,effect,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.age.automation'),'set_age',(SELECT id FROM def WHERE key='age.automation'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.age.fusion','Advance to Fusion');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.age.fusion'),'age','',9,2200);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.age.fusion'),(SELECT id FROM def WHERE key='res.food'),2250,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.age.fusion'),(SELECT id FROM def WHERE key='res.knowledge'),2250,0);
INSERT INTO tech_effect(tech_id,effect,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.age.fusion'),'set_age',(SELECT id FROM def WHERE key='age.fusion'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.age.starfaring','Advance to Starfaring');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.age.starfaring'),'age','',10,2400);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.age.starfaring'),(SELECT id FROM def WHERE key='res.food'),2500,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.age.starfaring'),(SELECT id FROM def WHERE key='res.knowledge'),2500,0);
INSERT INTO tech_effect(tech_id,effect,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.age.starfaring'),'set_age',(SELECT id FROM def WHERE key='age.starfaring'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.age.expansion','Advance to Expansion');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.age.expansion'),'age','',11,2600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.age.expansion'),(SELECT id FROM def WHERE key='res.food'),2750,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.age.expansion'),(SELECT id FROM def WHERE key='res.knowledge'),2750,0);
INSERT INTO tech_effect(tech_id,effect,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.age.expansion'),'set_age',(SELECT id FROM def WHERE key='age.expansion'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.age.abundance','Advance to Abundance');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.age.abundance'),'age','',12,2800);
INSERT INTO tech_effect(tech_id,effect,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.age.abundance'),'set_age',(SELECT id FROM def WHERE key='age.abundance'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.age.galactic','Advance to Galactic');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.age.galactic'),'age','',13,3000);
INSERT INTO tech_effect(tech_id,effect,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.age.galactic'),'set_age',(SELECT id FROM def WHERE key='age.galactic'));
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.age.abundance'),'cost.all','mul',0);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.agriculture','Agriculture');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.agriculture'),'upgrade','',0,350);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.agriculture'),(SELECT id FROM def WHERE key='res.wealth'),200,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.agriculture'),(SELECT id FROM def WHERE key='res.knowledge'),100,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.agriculture'),'income.food','pct',10);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.crop_rotation','Crop Rotation');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.crop_rotation'),'upgrade','',0,450);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.crop_rotation'),(SELECT id FROM def WHERE key='res.wealth'),400,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.crop_rotation'),(SELECT id FROM def WHERE key='res.knowledge'),200,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.crop_rotation'),'income.food','pct',15);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.food_processing','Food Processing');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.food_processing'),'upgrade','',0,550);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.food_processing'),(SELECT id FROM def WHERE key='res.wealth'),600,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.food_processing'),(SELECT id FROM def WHERE key='res.knowledge'),300,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.food_processing'),'income.food','pct',20);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.carpentry','Carpentry');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.carpentry'),'upgrade','',0,350);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.carpentry'),(SELECT id FROM def WHERE key='res.wealth'),200,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.carpentry'),(SELECT id FROM def WHERE key='res.knowledge'),100,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.carpentry'),'income.timber','pct',10);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.carpentry'),'build_time','pct',-5,(SELECT id FROM tag WHERE key='building'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.architecture','Architecture');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.architecture'),'upgrade','',0,450);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.architecture'),(SELECT id FROM def WHERE key='res.wealth'),400,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.architecture'),(SELECT id FROM def WHERE key='res.knowledge'),200,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.architecture'),'income.timber','pct',15);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.architecture'),'build_time','pct',-10,(SELECT id FROM tag WHERE key='building'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.engineering','Engineering');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.engineering'),'upgrade','',0,550);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.engineering'),(SELECT id FROM def WHERE key='res.wealth'),600,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.engineering'),(SELECT id FROM def WHERE key='res.knowledge'),300,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.engineering'),'income.timber','pct',20);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.engineering'),'build_time','pct',-15,(SELECT id FROM tag WHERE key='building'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.metallurgy','Metallurgy');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.metallurgy'),'upgrade','',0,400);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.metallurgy'),(SELECT id FROM def WHERE key='res.wealth'),300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.metallurgy'),(SELECT id FROM def WHERE key='res.knowledge'),150,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.metallurgy'),'income.metal','pct',10);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.steel','Steel');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.steel'),'upgrade','',0,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.steel'),(SELECT id FROM def WHERE key='res.wealth'),500,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.steel'),(SELECT id FROM def WHERE key='res.knowledge'),250,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.steel'),'income.metal','pct',15);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.alloys','Alloys');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.alloys'),'upgrade','',0,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.alloys'),(SELECT id FROM def WHERE key='res.wealth'),700,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.alloys'),(SELECT id FROM def WHERE key='res.knowledge'),350,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.alloys'),'income.metal','pct',20);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.cracking','Cracking');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.cracking'),'upgrade','',0,550);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.cracking'),(SELECT id FROM def WHERE key='res.wealth'),600,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.cracking'),(SELECT id FROM def WHERE key='res.knowledge'),300,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.cracking'),'income.oil','pct',15);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.petrochemistry','Petrochemistry');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.petrochemistry'),'upgrade','',0,650);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.petrochemistry'),(SELECT id FROM def WHERE key='res.wealth'),800,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.petrochemistry'),(SELECT id FROM def WHERE key='res.knowledge'),400,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.petrochemistry'),'income.oil','pct',25);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.plasma_containment','Plasma Containment');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.plasma_containment'),'upgrade','',0,750);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.plasma_containment'),(SELECT id FROM def WHERE key='res.wealth'),1000,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.plasma_containment'),(SELECT id FROM def WHERE key='res.knowledge'),500,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.plasma_containment'),'income.energy','pct',15);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.superconducting_grid','Superconducting Grid');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.superconducting_grid'),'upgrade','',0,800);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.superconducting_grid'),(SELECT id FROM def WHERE key='res.wealth'),1100,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.superconducting_grid'),(SELECT id FROM def WHERE key='res.knowledge'),550,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.superconducting_grid'),'income.energy','pct',15);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.stellar_collectors','Stellar Collectors');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.stellar_collectors'),'upgrade','',0,850);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.stellar_collectors'),(SELECT id FROM def WHERE key='res.wealth'),1200,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.stellar_collectors'),(SELECT id FROM def WHERE key='res.knowledge'),600,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.stellar_collectors'),'income.energy','pct',20);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.literacy','Literacy');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.literacy'),'upgrade','',0,400);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.literacy'),(SELECT id FROM def WHERE key='res.wealth'),300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.literacy'),(SELECT id FROM def WHERE key='res.knowledge'),150,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.literacy'),'income.knowledge','pct',20);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.printing','Printing');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.printing'),'upgrade','',0,450);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.printing'),(SELECT id FROM def WHERE key='res.wealth'),400,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.printing'),(SELECT id FROM def WHERE key='res.knowledge'),200,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.printing'),'income.knowledge','pct',20);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.scientific_method','Scientific Method');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.scientific_method'),'upgrade','',0,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.scientific_method'),(SELECT id FROM def WHERE key='res.wealth'),500,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.scientific_method'),(SELECT id FROM def WHERE key='res.knowledge'),250,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.scientific_method'),'income.knowledge','pct',20);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.research_institutes','Research Institutes');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.research_institutes'),'upgrade','',0,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.research_institutes'),(SELECT id FROM def WHERE key='res.wealth'),700,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.research_institutes'),(SELECT id FROM def WHERE key='res.knowledge'),350,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.research_institutes'),'income.knowledge','pct',20);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.supercomputers','Supercomputers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.supercomputers'),'upgrade','',0,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.supercomputers'),(SELECT id FROM def WHERE key='res.wealth'),900,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.supercomputers'),(SELECT id FROM def WHERE key='res.knowledge'),450,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.supercomputers'),'income.knowledge','pct',20);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.religion','Religion');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.religion'),'upgrade','',0,350);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.religion'),(SELECT id FROM def WHERE key='res.wealth'),200,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.religion'),(SELECT id FROM def WHERE key='res.knowledge'),100,0);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.religion'),'border_radius','add',2000,(SELECT id FROM tag WHERE key='temple'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.religion'),'hp','pct',25,(SELECT id FROM tag WHERE key='city'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.theology','Theology');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.theology'),'upgrade','',0,450);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.theology'),(SELECT id FROM def WHERE key='res.wealth'),400,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.theology'),(SELECT id FROM def WHERE key='res.knowledge'),200,0);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.theology'),'border_radius','add',2000,(SELECT id FROM tag WHERE key='temple'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.theology'),'hp','pct',25,(SELECT id FROM tag WHERE key='city'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.philosophy','Philosophy');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.philosophy'),'upgrade','',0,550);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.philosophy'),(SELECT id FROM def WHERE key='res.wealth'),600,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.philosophy'),(SELECT id FROM def WHERE key='res.knowledge'),300,0);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.philosophy'),'border_radius','add',2000,(SELECT id FROM tag WHERE key='temple'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.philosophy'),'hp','pct',25,(SELECT id FROM tag WHERE key='city'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.militia_training','Militia Training');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.militia_training'),'upgrade','',0,350);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.militia_training'),(SELECT id FROM def WHERE key='res.wealth'),200,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.militia_training'),(SELECT id FROM def WHERE key='res.knowledge'),100,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.militia_training'),'attrition_dealt','pct',25);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.allegiance','Allegiance');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.allegiance'),'upgrade','',0,400);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.allegiance'),(SELECT id FROM def WHERE key='res.wealth'),300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.allegiance'),(SELECT id FROM def WHERE key='res.knowledge'),150,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.allegiance'),'attrition_dealt','pct',25);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.patriotism','Patriotism');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.patriotism'),'upgrade','',0,550);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.patriotism'),(SELECT id FROM def WHERE key='res.wealth'),600,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.patriotism'),(SELECT id FROM def WHERE key='res.knowledge'),300,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.patriotism'),'attrition_dealt','pct',25);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.nationalism','Nationalism');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.nationalism'),'upgrade','',0,650);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.nationalism'),(SELECT id FROM def WHERE key='res.wealth'),800,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.nationalism'),(SELECT id FROM def WHERE key='res.knowledge'),400,0);
INSERT INTO modifier(source_def_id,stat_key,op,value) VALUES ((SELECT id FROM def WHERE key='tech.nationalism'),'attrition_dealt','pct',25);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.fortification','Fortification');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.fortification'),'upgrade','',0,400);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.fortification'),(SELECT id FROM def WHERE key='res.wealth'),300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.fortification'),(SELECT id FROM def WHERE key='res.knowledge'),150,0);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.fortification'),'border_radius','add',3000,(SELECT id FROM tag WHERE key='fort'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.fortification'),'range','add',1000,(SELECT id FROM tag WHERE key='fort'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.fortification'),'los','add',1500,(SELECT id FROM tag WHERE key='fort'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.bombardment','Bombardment');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.bombardment'),'upgrade','',0,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.bombardment'),(SELECT id FROM def WHERE key='res.wealth'),500,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.bombardment'),(SELECT id FROM def WHERE key='res.knowledge'),250,0);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.bombardment'),'border_radius','add',3000,(SELECT id FROM tag WHERE key='fort'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.bombardment'),'range','add',1000,(SELECT id FROM tag WHERE key='fort'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.bombardment'),'los','add',1500,(SELECT id FROM tag WHERE key='fort'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.strategic_reserves','Strategic Reserves');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.strategic_reserves'),'upgrade','',0,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.strategic_reserves'),(SELECT id FROM def WHERE key='res.wealth'),700,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.strategic_reserves'),(SELECT id FROM def WHERE key='res.knowledge'),350,0);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.strategic_reserves'),'border_radius','add',3000,(SELECT id FROM tag WHERE key='fort'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.strategic_reserves'),'range','add',1000,(SELECT id FROM tag WHERE key='fort'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.strategic_reserves'),'los','add',1500,(SELECT id FROM tag WHERE key='fort'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,limit_scope,limit_count,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_farm'),'building','economic',720,3,(SELECT id FROM armor_class WHERE key='structure'),17000,4,4,350,'per_city',5,(SELECT id FROM def WHERE key='bld.farm'),(SELECT id FROM def WHERE key='age.automation'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_farm'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_farm'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_farm'),(SELECT id FROM tag WHERE key='gather'));
INSERT INTO et_producer(entity_type_id,queue_limit,parallel,has_rally) SELECT (SELECT id FROM def WHERE key='bld.auto_farm'),queue_limit,parallel,has_rally FROM et_producer WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.farm');
INSERT INTO et_keeper(entity_type_id,capacity) SELECT (SELECT id FROM def WHERE key='bld.auto_farm'),capacity FROM et_keeper WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.farm');
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) SELECT (SELECT id FROM def WHERE key='bld.auto_farm'),is_city,border_radius,economic_radius,capturable FROM et_territory WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.farm');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id,layer,distance) SELECT (SELECT id FROM def WHERE key='bld.auto_farm'),rule,ref_def_id,layer,distance FROM placement_rule WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.farm');
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income) SELECT (SELECT id FROM def WHERE key='bld.auto_farm'),resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income FROM et_gather_site WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.farm');
UPDATE et_gather_site SET slots_base=0,slots_max=0,slots_per_node=0,self_income=10 WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.auto_farm');
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.auto_farm','Upgrade to Automated Farm');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_farm'),'upgrade','',0,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_farm'),(SELECT id FROM def WHERE key='res.metal'),400,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_farm'),(SELECT id FROM def WHERE key='res.knowledge'),400,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_farm'),'upgrade_type',(SELECT id FROM def WHERE key='bld.farm'),(SELECT id FROM def WHERE key='bld.auto_farm'));
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.auto_farm'),'construct','economic',4,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.auto_farm'),'construct','economic',4,2);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,limit_scope,limit_count,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_woodcutter_camp'),'building','economic',900,3,(SELECT id FROM armor_class WHERE key='structure'),17000,4,4,350,'none',0,(SELECT id FROM def WHERE key='bld.woodcutter_camp'),(SELECT id FROM def WHERE key='age.automation'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_woodcutter_camp'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_woodcutter_camp'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_woodcutter_camp'),(SELECT id FROM tag WHERE key='gather'));
INSERT INTO et_producer(entity_type_id,queue_limit,parallel,has_rally) SELECT (SELECT id FROM def WHERE key='bld.auto_woodcutter_camp'),queue_limit,parallel,has_rally FROM et_producer WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.woodcutter_camp');
INSERT INTO et_keeper(entity_type_id,capacity) SELECT (SELECT id FROM def WHERE key='bld.auto_woodcutter_camp'),capacity FROM et_keeper WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.woodcutter_camp');
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) SELECT (SELECT id FROM def WHERE key='bld.auto_woodcutter_camp'),is_city,border_radius,economic_radius,capturable FROM et_territory WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.woodcutter_camp');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id,layer,distance) SELECT (SELECT id FROM def WHERE key='bld.auto_woodcutter_camp'),rule,ref_def_id,layer,distance FROM placement_rule WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.woodcutter_camp');
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income) SELECT (SELECT id FROM def WHERE key='bld.auto_woodcutter_camp'),resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income FROM et_gather_site WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.woodcutter_camp');
UPDATE et_gather_site SET slots_base=0,slots_max=0,slots_per_node=0,self_income=100 WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.auto_woodcutter_camp');
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.auto_woodcutter_camp','Upgrade to Automated Woodcutter Camp');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_woodcutter_camp'),'upgrade','',0,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_woodcutter_camp'),(SELECT id FROM def WHERE key='res.metal'),400,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_woodcutter_camp'),(SELECT id FROM def WHERE key='res.knowledge'),400,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_woodcutter_camp'),'upgrade_type',(SELECT id FROM def WHERE key='bld.woodcutter_camp'),(SELECT id FROM def WHERE key='bld.auto_woodcutter_camp'));
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.auto_woodcutter_camp'),'construct','economic_2',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.auto_woodcutter_camp'),'construct','economic_2',0,0);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,limit_scope,limit_count,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_mine'),'building','economic',1080,3,(SELECT id FROM armor_class WHERE key='structure'),17000,4,4,350,'none',0,(SELECT id FROM def WHERE key='bld.mine'),(SELECT id FROM def WHERE key='age.automation'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_mine'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_mine'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_mine'),(SELECT id FROM tag WHERE key='gather'));
INSERT INTO et_producer(entity_type_id,queue_limit,parallel,has_rally) SELECT (SELECT id FROM def WHERE key='bld.auto_mine'),queue_limit,parallel,has_rally FROM et_producer WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.mine');
INSERT INTO et_keeper(entity_type_id,capacity) SELECT (SELECT id FROM def WHERE key='bld.auto_mine'),capacity FROM et_keeper WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.mine');
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) SELECT (SELECT id FROM def WHERE key='bld.auto_mine'),is_city,border_radius,economic_radius,capturable FROM et_territory WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.mine');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id,layer,distance) SELECT (SELECT id FROM def WHERE key='bld.auto_mine'),rule,ref_def_id,layer,distance FROM placement_rule WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.mine');
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income) SELECT (SELECT id FROM def WHERE key='bld.auto_mine'),resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income FROM et_gather_site WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.mine');
UPDATE et_gather_site SET slots_base=0,slots_max=0,slots_per_node=0,self_income=100 WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.auto_mine');
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.auto_mine','Upgrade to Automated Mine');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_mine'),'upgrade','',0,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_mine'),(SELECT id FROM def WHERE key='res.metal'),400,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_mine'),(SELECT id FROM def WHERE key='res.knowledge'),400,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_mine'),'upgrade_type',(SELECT id FROM def WHERE key='bld.mine'),(SELECT id FROM def WHERE key='bld.auto_mine'));
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.auto_mine'),'construct','economic_2',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.auto_mine'),'construct','economic_2',1,0);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,limit_scope,limit_count,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_oil_well'),'building','economic',1080,3,(SELECT id FROM armor_class WHERE key='structure'),16000,2,2,350,'none',0,(SELECT id FROM def WHERE key='bld.oil_well'),(SELECT id FROM def WHERE key='age.automation'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_oil_well'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_oil_well'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_oil_well'),(SELECT id FROM tag WHERE key='gather'));
INSERT INTO et_producer(entity_type_id,queue_limit,parallel,has_rally) SELECT (SELECT id FROM def WHERE key='bld.auto_oil_well'),queue_limit,parallel,has_rally FROM et_producer WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.oil_well');
INSERT INTO et_keeper(entity_type_id,capacity) SELECT (SELECT id FROM def WHERE key='bld.auto_oil_well'),capacity FROM et_keeper WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.oil_well');
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) SELECT (SELECT id FROM def WHERE key='bld.auto_oil_well'),is_city,border_radius,economic_radius,capturable FROM et_territory WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.oil_well');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id,layer,distance) SELECT (SELECT id FROM def WHERE key='bld.auto_oil_well'),rule,ref_def_id,layer,distance FROM placement_rule WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.oil_well');
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income) SELECT (SELECT id FROM def WHERE key='bld.auto_oil_well'),resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income FROM et_gather_site WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.oil_well');
UPDATE et_gather_site SET slots_base=0,slots_max=0,slots_per_node=0,self_income=30 WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.auto_oil_well');
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.auto_oil_well','Upgrade to Automated Oil Well');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_oil_well'),'upgrade','',0,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_oil_well'),(SELECT id FROM def WHERE key='res.metal'),400,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_oil_well'),(SELECT id FROM def WHERE key='res.knowledge'),400,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_oil_well'),'upgrade_type',(SELECT id FROM def WHERE key='bld.oil_well'),(SELECT id FROM def WHERE key='bld.auto_oil_well'));
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.auto_oil_well'),'construct','economic_2',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.auto_oil_well'),'construct','economic_2',2,0);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,limit_scope,limit_count,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_lab'),'building','economic',1440,3,(SELECT id FROM armor_class WHERE key='structure'),17000,4,4,350,'per_city',1,(SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM def WHERE key='age.automation'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_lab'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_lab'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_lab'),(SELECT id FROM tag WHERE key='gather'));
INSERT INTO et_producer(entity_type_id,queue_limit,parallel,has_rally) SELECT (SELECT id FROM def WHERE key='bld.auto_lab'),queue_limit,parallel,has_rally FROM et_producer WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.university');
INSERT INTO et_keeper(entity_type_id,capacity) SELECT (SELECT id FROM def WHERE key='bld.auto_lab'),capacity FROM et_keeper WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.university');
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) SELECT (SELECT id FROM def WHERE key='bld.auto_lab'),is_city,border_radius,economic_radius,capturable FROM et_territory WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.university');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id,layer,distance) SELECT (SELECT id FROM def WHERE key='bld.auto_lab'),rule,ref_def_id,layer,distance FROM placement_rule WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.university');
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income) SELECT (SELECT id FROM def WHERE key='bld.auto_lab'),resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income FROM et_gather_site WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.university');
UPDATE et_gather_site SET slots_base=0,slots_max=0,slots_per_node=0,self_income=70 WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.auto_lab');
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.auto_lab','Upgrade to Auto Lab');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_lab'),'upgrade','',0,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_lab'),(SELECT id FROM def WHERE key='res.metal'),400,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_lab'),(SELECT id FROM def WHERE key='res.knowledge'),400,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_lab'),'upgrade_type',(SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM def WHERE key='bld.auto_lab'));
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.auto_lab'),'construct','economic_2',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.auto_lab'),'construct','economic_2',3,0);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.continental_union','Continental Union');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.continental_union'),'project','',0,3000);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.continental_union'),(SELECT id FROM def WHERE key='res.knowledge'),1000,0);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.unification','Unification');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.unification'),'project','',0,4000);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.unification'),(SELECT id FROM def WHERE key='res.knowledge'),1000,0);
INSERT INTO tech_effect(tech_id,effect,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.unification'),'change_faction',(SELECT id FROM def WHERE key='race.human'));
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.orbital_fighter','Orbital Fighters');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.orbital_bomber','Orbital Bombers');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.carrier','Carriers');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.ground_battery','Ground Battery');
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.orbital_fighter'),'unit','orbital_fighter',420,4,(SELECT id FROM armor_class WHERE key='vehicle'),16000,1,400,'per_type',(SELECT id FROM def WHERE key='age.starfaring'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.orbital_fighter'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.orbital_fighter'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.orbital_fighter'),(SELECT id FROM tag WHERE key='aircraft'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.orbital_fighter'),(SELECT id FROM tag WHERE key='orbital_craft'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed,climb_ticks) VALUES ((SELECT id FROM def WHERE key='unit.orbital_fighter'),(SELECT id FROM movement_class WHERE key='aircraft'),5000,100);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.orbital_fighter'),40,(SELECT id FROM damage_class WHERE key='explosive'),5000,20,'missile',20000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.orbital_fighter')),'air');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.orbital_fighter')),'orbit');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.orbital_fighter')),'space');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='unit.orbital_fighter'),'train','units',3,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.orbital_fighter'),(SELECT id FROM def WHERE key='res.metal'),220,4);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.orbital_fighter'),(SELECT id FROM def WHERE key='res.energy'),80,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.orbital_bomber'),'unit','orbital_bomber',520,4,(SELECT id FROM armor_class WHERE key='vehicle'),16000,1,450,'per_type',(SELECT id FROM def WHERE key='age.starfaring'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.orbital_bomber'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.orbital_bomber'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.orbital_bomber'),(SELECT id FROM tag WHERE key='aircraft'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.orbital_bomber'),(SELECT id FROM tag WHERE key='orbital_craft'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed,climb_ticks) VALUES ((SELECT id FROM def WHERE key='unit.orbital_bomber'),(SELECT id FROM movement_class WHERE key='aircraft'),4000,120);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.orbital_bomber'),30,(SELECT id FROM damage_class WHERE key='explosive'),4000,40,'missile',20000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.orbital_bomber')),'orbit');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.orbital_bomber')),'space');
INSERT INTO et_weapon(entity_type_id,slot,damage,damage_class_id,range_max,cooldown_ticks,orbit_only,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.orbital_bomber'),1,90,(SELECT id FROM damage_class WHERE key='explosive'),8000,60,1,'shell',24000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.orbital_bomber') AND slot=1),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='unit.orbital_bomber'),'train','units',4,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.orbital_bomber'),(SELECT id FROM def WHERE key='res.metal'),260,5);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.orbital_bomber'),(SELECT id FROM def WHERE key='res.energy'),100,2);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.carrier'),'unit','carrier',2000,6,(SELECT id FROM armor_class WHERE key='shield'),28000,2,500,'per_type',(SELECT id FROM def WHERE key='age.starfaring'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.carrier'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.carrier'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.carrier'),(SELECT id FROM tag WHERE key='starship'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.carrier'),(SELECT id FROM movement_class WHERE key='starship'),2400);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile) VALUES ((SELECT id FROM def WHERE key='unit.carrier'),25,(SELECT id FROM damage_class WHERE key='energy'),6000,30,'beam');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.carrier')),'space');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.carrier')),'orbit');
INSERT INTO et_container(entity_type_id,capacity,accept_tag_id,launches) VALUES ((SELECT id FROM def WHERE key='unit.carrier'),10,(SELECT id FROM tag WHERE key='orbital_craft'),1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='unit.carrier'),'train','units',1,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.carrier'),(SELECT id FROM def WHERE key='res.metal'),700,14);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.carrier'),(SELECT id FROM def WHERE key='res.energy'),300,6);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.ground_battery'),'building','military',2400,5,(SELECT id FROM armor_class WHERE key='structure'),21000,3,3,400,'per_type',(SELECT id FROM def WHERE key='age.starfaring'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.ground_battery'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.ground_battery'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.ground_battery'),(SELECT id FROM def WHERE key='res.metal'),400,8);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.ground_battery'),(SELECT id FROM def WHERE key='res.energy'),200,4);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.ground_battery'),'construct','military_2',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.ground_battery'),'construct','military_2',1,0);
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.ground_battery'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.ground_battery'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='bld.ground_battery'),150,(SELECT id FROM damage_class WHERE key='explosive'),14000,30,'missile',20000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.ground_battery')),'orbit');
INSERT INTO tag(key,category) VALUES ('orbit_bound','movement');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.space_freighter','Space Freighters');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.satellite','Satellites');
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.space_freighter'),'unit','space_freighter',500,3,(SELECT id FROM armor_class WHERE key='shield'),22000,2,450,'per_type',(SELECT id FROM def WHERE key='age.starfaring'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.space_freighter'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.space_freighter'),(SELECT id FROM tag WHERE key='civilian'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.space_freighter'),(SELECT id FROM movement_class WHERE key='starship'),2800);
INSERT INTO et_trader(entity_type_id,runs_routes,claims_nodes,route_resource_id,route_income) VALUES ((SELECT id FROM def WHERE key='unit.space_freighter'),1,0,(SELECT id FROM def WHERE key='res.wealth'),8);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='unit.space_freighter'),'train','units',2,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.space_freighter'),(SELECT id FROM def WHERE key='res.metal'),260,5);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.space_freighter'),(SELECT id FROM def WHERE key='res.energy'),60,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.satellite'),'unit','satellite',150,0,(SELECT id FROM armor_class WHERE key='shield'),96000,1,300,'per_type',(SELECT id FROM def WHERE key='age.starfaring'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.satellite'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.satellite'),(SELECT id FROM tag WHERE key='orbit_bound'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.satellite'),(SELECT id FROM movement_class WHERE key='starship'),1500);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='unit.satellite'),'train','units',3,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.satellite'),(SELECT id FROM def WHERE key='res.metal'),150,3);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.satellite'),(SELECT id FROM def WHERE key='res.energy'),100,2);
INSERT INTO et_view_scale(entity_type_id,planet_pct,system_pct) VALUES ((SELECT id FROM def WHERE key='unit.space_freighter'),300,35);
INSERT INTO et_view_scale(entity_type_id,planet_pct,system_pct) VALUES ((SELECT id FROM def WHERE key='unit.satellite'),220,25);
UPDATE et_movement SET jump_charge_ticks=120 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.survey_ship');
UPDATE et_movement SET jump_charge_ticks=120 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.corvette');
UPDATE et_movement SET jump_charge_ticks=160 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.space_frigate');
UPDATE et_movement SET jump_charge_ticks=200 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.space_cruiser');
UPDATE et_movement SET jump_charge_ticks=240 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.battleship');
UPDATE et_movement SET jump_charge_ticks=160 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.transport_ship');
UPDATE et_movement SET jump_charge_ticks=240 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.carrier');
UPDATE et_movement SET jump_charge_ticks=160 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.space_freighter');
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.jump_inhibitor','Jump Inhibitor');
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.jump_inhibitor'),'building','military',3000,5,(SELECT id FROM armor_class WHERE key='structure'),16500,3,3,400,'per_type',(SELECT id FROM def WHERE key='age.starfaring'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.jump_inhibitor'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.jump_inhibitor'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.jump_inhibitor'),(SELECT id FROM def WHERE key='res.metal'),500,10);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.jump_inhibitor'),(SELECT id FROM def WHERE key='res.energy'),300,6);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.jump_inhibitor'),'construct','military_2',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.jump_inhibitor'),'construct','military_2',2,0);
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.jump_inhibitor'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.jump_inhibitor'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO et_inhibitor(entity_type_id,reach,charge_pct) VALUES ((SELECT id FROM def WHERE key='bld.jump_inhibitor'),'system',50);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.enhanced_sensors','Enhanced Sensors');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.enhanced_sensors'),'upgrade','',0,850);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.enhanced_sensors'),(SELECT id FROM def WHERE key='res.wealth'),1200,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.enhanced_sensors'),(SELECT id FROM def WHERE key='res.knowledge'),600,0);
INSERT INTO tech_effect(tech_id,effect,rule_key) VALUES ((SELECT id FROM def WHERE key='tech.enhanced_sensors'),'enable_rule','route_sensors');
INSERT INTO def(kind,key,name) VALUES ('tech','tech.interstellar_trade','Interstellar Trade');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.interstellar_trade'),'upgrade','',0,900);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.interstellar_trade'),(SELECT id FROM def WHERE key='res.wealth'),1300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.interstellar_trade'),(SELECT id FROM def WHERE key='res.knowledge'),650,0);
INSERT INTO tech_effect(tech_id,effect,rule_key) VALUES ((SELECT id FROM def WHERE key='tech.interstellar_trade'),'enable_rule','interstellar_trade');
INSERT INTO def(kind,key,name) VALUES ('tech','tech.automated_route_survey','Automated Route Survey');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.automated_route_survey'),'upgrade','',0,950);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.automated_route_survey'),(SELECT id FROM def WHERE key='res.wealth'),1400,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.automated_route_survey'),(SELECT id FROM def WHERE key='res.knowledge'),700,0);
INSERT INTO tech_effect(tech_id,effect,rule_key) VALUES ((SELECT id FROM def WHERE key='tech.automated_route_survey'),'enable_rule','route_survey');
INSERT INTO game_constant(key,value,unit,note) VALUES ('space.landing_land_pct',20,'%','least share of the tiles under a landed ship that must be ground; the rest may be water');
INSERT INTO stat(key,scope,note) VALUES ('landing_radius','entity','milli-tiles of ground a landed ship covers around its tile');
INSERT INTO tag(key,category) VALUES ('no_landing','terrain');
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='ter.mountain'),(SELECT id FROM tag WHERE key='no_landing'));
INSERT INTO et_view_scale(entity_type_id,planet_pct,system_pct) VALUES ((SELECT id FROM def WHERE key='unit.survey_ship'),200,25);
INSERT INTO et_view_scale(entity_type_id,planet_pct,system_pct) VALUES ((SELECT id FROM def WHERE key='unit.corvette'),210,30);
INSERT INTO et_view_scale(entity_type_id,planet_pct,system_pct) VALUES ((SELECT id FROM def WHERE key='unit.space_frigate'),230,35);
UPDATE et_view_scale SET planet_pct=230,system_pct=35 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.space_freighter');
INSERT INTO et_view_scale(entity_type_id,planet_pct,system_pct) VALUES ((SELECT id FROM def WHERE key='unit.transport_ship'),240,40);
INSERT INTO et_view_scale(entity_type_id,planet_pct,system_pct) VALUES ((SELECT id FROM def WHERE key='unit.space_cruiser'),260,40);
INSERT INTO et_view_scale(entity_type_id,planet_pct,system_pct) VALUES ((SELECT id FROM def WHERE key='unit.carrier'),280,45);
INSERT INTO et_view_scale(entity_type_id,planet_pct,system_pct) VALUES ((SELECT id FROM def WHERE key='unit.battleship'),300,50);
INSERT INTO et_view_scale(entity_type_id,planet_pct,system_pct) VALUES ((SELECT id FROM def WHERE key='unit.orbital_fighter'),300,25);
INSERT INTO et_view_scale(entity_type_id,planet_pct,system_pct) VALUES ((SELECT id FROM def WHERE key='unit.orbital_bomber'),320,25);
UPDATE et_movement SET landing_radius=3100 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.survey_ship');
UPDATE et_movement SET landing_radius=3900 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.corvette');
UPDATE et_movement SET landing_radius=4900 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.space_frigate');
UPDATE et_movement SET landing_radius=4600 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.space_freighter');
UPDATE et_movement SET landing_radius=4600 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.transport_ship');
UPDATE et_movement SET landing_radius=7800 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.space_cruiser');
UPDATE et_movement SET landing_radius=6600 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.carrier');
UPDATE et_movement SET landing_radius=14600 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.battleship');
INSERT INTO order_kind(key,needs,target,note) VALUES ('pause_governor','colony','none','an automated colony''s governor stops placing and paying for sites; applied at once, the queue stays'), ('resume_governor','colony','none','the governor of an automated colony works again; applied at once');
INSERT INTO def(kind,key,name) VALUES ('tech','tech.auto_colonies','Automated Colonies');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.auto_colonies'),'upgrade','',0,900);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.auto_colonies'),(SELECT id FROM def WHERE key='res.wealth'),1300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.auto_colonies'),(SELECT id FROM def WHERE key='res.knowledge'),650,0);
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.auto_colony','Automated Colony');
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_colony'),'building','economic',3000,4,(SELECT id FROM armor_class WHERE key='structure'),17500,5,5,450,'per_type',(SELECT id FROM def WHERE key='age.abundance'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_colony'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_colony'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_colony'),(SELECT id FROM tag WHERE key='city'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.auto_colony'),(SELECT id FROM def WHERE key='res.metal'),400,8);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.auto_colony'),(SELECT id FROM def WHERE key='res.energy'),200,4);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.auto_colony'),'construct','economic_2',4,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.auto_colony'),'construct','economic_2',4,0);
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) VALUES ((SELECT id FROM def WHERE key='bld.auto_colony'),1,24000,16000,1);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_colony'));
INSERT INTO et_builder(entity_type_id,build_pct,repair_pct) VALUES ((SELECT id FROM def WHERE key='bld.auto_colony'),200,100);
INSERT INTO et_colony(entity_type_id,resource_id,income,site_share_pct) VALUES ((SELECT id FROM def WHERE key='bld.auto_colony'),(SELECT id FROM def WHERE key='res.wealth'),30,10);
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.auto_colony'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_colony'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO et_colony_site(colony_type_id,site_type_id,cap) VALUES ((SELECT id FROM def WHERE key='bld.auto_colony'),(SELECT id FROM def WHERE key='bld.auto_farm'),5);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.auto_colony'),(SELECT id FROM def WHERE key='bld.auto_farm'),'construct','economic',0,0);
INSERT INTO et_colony_site(colony_type_id,site_type_id,cap) VALUES ((SELECT id FROM def WHERE key='bld.auto_colony'),(SELECT id FROM def WHERE key='bld.auto_woodcutter_camp'),5);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.auto_colony'),(SELECT id FROM def WHERE key='bld.auto_woodcutter_camp'),'construct','economic',1,0);
INSERT INTO et_colony_site(colony_type_id,site_type_id,cap) VALUES ((SELECT id FROM def WHERE key='bld.auto_colony'),(SELECT id FROM def WHERE key='bld.auto_lab'),5);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.auto_colony'),(SELECT id FROM def WHERE key='bld.auto_lab'),'construct','economic',2,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.auto_farm'),(SELECT id FROM def WHERE key='res.metal'),200,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.auto_farm'),(SELECT id FROM def WHERE key='res.timber'),100,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.auto_woodcutter_camp'),(SELECT id FROM def WHERE key='res.metal'),200,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.auto_woodcutter_camp'),(SELECT id FROM def WHERE key='res.timber'),100,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.auto_lab'),(SELECT id FROM def WHERE key='res.metal'),300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.auto_lab'),(SELECT id FROM def WHERE key='res.knowledge'),150,0);
INSERT INTO et_weapon(entity_type_id,slot,damage,damage_class_id,range_max,cooldown_ticks,orbit_only,splash_radius,splash_pct,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.corvette'),1,60,(SELECT id FROM damage_class WHERE key='explosive'),10000,60,1,2000,50,'missile',20000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.corvette') AND slot=1),'ground');
INSERT INTO et_weapon(entity_type_id,slot,damage,damage_class_id,range_max,cooldown_ticks,orbit_only,splash_radius,splash_pct,projectile) VALUES ((SELECT id FROM def WHERE key='unit.space_frigate'),1,2000,(SELECT id FROM damage_class WHERE key='kinetic'),12000,100,1,12000,100,'beam');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.space_frigate') AND slot=1),'ground');
INSERT INTO et_weapon(entity_type_id,slot,damage,damage_class_id,range_max,cooldown_ticks,orbit_only,splash_radius,splash_pct,projectile) VALUES ((SELECT id FROM def WHERE key='unit.space_cruiser'),1,2600,(SELECT id FROM damage_class WHERE key='kinetic'),13000,100,1,14000,100,'railgun');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.space_cruiser') AND slot=1),'ground');
INSERT INTO et_weapon(entity_type_id,slot,damage,damage_class_id,range_max,cooldown_ticks,orbit_only,splash_radius,splash_pct,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.battleship'),1,3200,(SELECT id FROM damage_class WHERE key='kinetic'),14000,80,1,16000,100,'plasma',30000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.battleship') AND slot=1),'ground');
UPDATE et_weapon SET damage=300,damage_class_id=(SELECT id FROM damage_class WHERE key='kinetic'),range_max=8000,cooldown_ticks=80,splash_radius=4000,splash_pct=100 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.orbital_bomber') AND orbit_only=1;
UPDATE entity_type SET hp=9000,los=30000,armor_class_id=(SELECT id FROM armor_class WHERE key='hull') WHERE id=(SELECT id FROM def WHERE key='bld.ground_battery');
UPDATE et_weapon SET damage=520,range_max=20000,cooldown_ticks=40 WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.ground_battery');
INSERT INTO game_constant(key,value,unit,note) VALUES ('world.stain_days',6,'days','a stain left where a building or unit was destroyed lasts this many world.day_ticks; presentation only');
INSERT INTO game_constant(key,value,unit,note) VALUES ('world.stain_cap',2000,'stains','most stains a surface keeps; past it the oldest goes');
INSERT INTO def(kind,key,name) VALUES ('tech','tech.ascension','Ascension');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.ascension'),'project','',0,7500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.ascension'),(SELECT id FROM def WHERE key='res.knowledge'),10000,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.ascension'),(SELECT id FROM def WHERE key='res.wealth'),10000,0);
INSERT INTO tech_effect(tech_id,effect) VALUES ((SELECT id FROM def WHERE key='tech.ascension'),'ascend');
INSERT INTO order_kind(key,needs,target,note) VALUES ('descend','','point','an ascended player takes a city on open land of a habitable planet; no executor');
INSERT INTO body_kind(key,note) VALUES ('black_hole','the centre of the celestial home (task 76): no surface, no routes');
INSERT INTO body_kind(key,note) VALUES ('celestial','the mythical planet of the celestial home where ascended players live; no surface');
INSERT INTO game_constant(key,value,unit,note) VALUES ('ascension.duration_pct',250,'%','the ascension lasts this share of the research time of the age advance that brought the player its current age');
INSERT INTO game_constant(key,value,unit,note) VALUES ('ascension.descent_research_pct',90,'%','on descent, of each resource the library research costs, this share of the sum of those costs');
INSERT INTO game_constant(key,value,unit,note) VALUES ('ascension.descent_other_pct',15,'%','on descent, of each other base resource, this share of the sum of all library research costs');
INSERT INTO game_constant(key,value,unit,note) VALUES ('ascension.descent_citizens',5,'units','builders that come with the city on descent');
INSERT INTO game_constant(key,value,unit,note) VALUES ('ascension.ai_descend_ticks',12000,'ticks','an ascended computer player descends this long after it ascended');
INSERT INTO game_constant(key,value,unit,note) VALUES ('celestial.dir_x',-54657,'millionths','unit vector to the galactic centre, equatorial frame (x toward RA 0h)');
INSERT INTO game_constant(key,value,unit,note) VALUES ('celestial.dir_y',-872844,'millionths','y toward RA 6h');
INSERT INTO game_constant(key,value,unit,note) VALUES ('celestial.dir_z',-484929,'millionths','z toward the north celestial pole');
INSERT INTO game_constant(key,value,unit,note) VALUES ('celestial.distance_pct',150,'%','the celestial home lies this share of the match region''s radius from its middle');
INSERT INTO game_constant(key,value,unit,note) VALUES ('celestial.planet_orbit',6000,'system units','orbit radius of the mythical planet about the black hole');
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.ship_of_the_line','Ships of the Line');
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.ship_of_the_line'),'unit','line_ship',960,2,(SELECT id FROM armor_class WHERE key='hull'),12000,1,250,'per_type',(SELECT id FROM def WHERE key='unit.carrack'),(SELECT id FROM def WHERE key='age.enlightenment'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.ship_of_the_line'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.ship_of_the_line'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.ship_of_the_line'),(SELECT id FROM tag WHERE key='ship'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.ship_of_the_line'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.ship_of_the_line'),(SELECT id FROM movement_class WHERE key='ship'),1400);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,range_min,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.ship_of_the_line'),57,(SELECT id FROM damage_class WHERE key='explosive'),12000,40,3000,'shell',24000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.ship_of_the_line')),'fluid');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.ship_of_the_line')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='unit.ship_of_the_line'),'train','units',4,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.ship_of_the_line'),(SELECT id FROM def WHERE key='res.timber'),256,5);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.ship_of_the_line'),(SELECT id FROM def WHERE key='res.metal'),128,2);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.ship_of_the_line','Upgrade to Ships of the Line');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.ship_of_the_line'),'unit_upgrade','',0,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.ship_of_the_line'),(SELECT id FROM def WHERE key='res.timber'),512,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.ship_of_the_line'),(SELECT id FROM def WHERE key='res.metal'),256,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.ship_of_the_line'),'upgrade_type',(SELECT id FROM def WHERE key='unit.carrack'),(SELECT id FROM def WHERE key='unit.ship_of_the_line'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.ironclad'),'unit','line_ship',1050,2,(SELECT id FROM armor_class WHERE key='hull'),13000,1,275,'per_type',(SELECT id FROM def WHERE key='unit.ship_of_the_line'),(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.ironclad'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.ironclad'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.ironclad'),(SELECT id FROM tag WHERE key='ship'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.ironclad'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.ironclad'),(SELECT id FROM movement_class WHERE key='ship'),1400);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,range_min,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.ironclad'),63,(SELECT id FROM damage_class WHERE key='explosive'),13000,40,3000,'shell',24000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.ironclad')),'fluid');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.ironclad')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='unit.ironclad'),'train','units',0,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.ironclad'),(SELECT id FROM def WHERE key='res.timber'),280,5);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.ironclad'),(SELECT id FROM def WHERE key='res.metal'),140,2);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.ironclad','Upgrade to Ironclads');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.ironclad'),'unit_upgrade','',0,550);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.ironclad'),(SELECT id FROM def WHERE key='res.timber'),560,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.ironclad'),(SELECT id FROM def WHERE key='res.metal'),280,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.ironclad'),'upgrade_type',(SELECT id FROM def WHERE key='unit.ship_of_the_line'),(SELECT id FROM def WHERE key='unit.ironclad'));
INSERT INTO def(kind,key,name) VALUES ('entity_type','unit.dreadnought','Dreadnoughts');
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.dreadnought'),'unit','line_ship',1140,3,(SELECT id FROM armor_class WHERE key='hull'),14000,1,300,'per_type',(SELECT id FROM def WHERE key='unit.ironclad'),(SELECT id FROM def WHERE key='age.modern'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.dreadnought'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.dreadnought'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.dreadnought'),(SELECT id FROM tag WHERE key='ship'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.dreadnought'),(SELECT id FROM tag WHERE key='passenger'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.dreadnought'),(SELECT id FROM movement_class WHERE key='ship'),1400);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks,range_min,projectile,projectile_speed) VALUES ((SELECT id FROM def WHERE key='unit.dreadnought'),68,(SELECT id FROM damage_class WHERE key='explosive'),14000,40,3000,'shell',24000);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.dreadnought')),'fluid');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.dreadnought')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='unit.dreadnought'),'train','units',2,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.dreadnought'),(SELECT id FROM def WHERE key='res.timber'),304,6);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.dreadnought'),(SELECT id FROM def WHERE key='res.metal'),152,3);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.dreadnought','Upgrade to Dreadnoughts');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.dreadnought'),'unit_upgrade','',0,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.dreadnought'),(SELECT id FROM def WHERE key='res.timber'),608,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.dreadnought'),(SELECT id FROM def WHERE key='res.metal'),304,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.dreadnought'),'upgrade_type',(SELECT id FROM def WHERE key='unit.ironclad'),(SELECT id FROM def WHERE key='unit.dreadnought'));
UPDATE entity_type SET upgrades_from_id=(SELECT id FROM def WHERE key='bld.stable') WHERE id=(SELECT id FROM def WHERE key='bld.vehicle_plant');
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.vehicle_plant','Upgrade to Vehicle Plant');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.vehicle_plant'),'upgrade','',0,274);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.vehicle_plant'),(SELECT id FROM def WHERE key='res.timber'),200,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.vehicle_plant'),(SELECT id FROM def WHERE key='res.metal'),100,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.vehicle_plant'),'upgrade_type',(SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM def WHERE key='bld.vehicle_plant'));
UPDATE entity_type SET upgrades_from_id=(SELECT id FROM def WHERE key='bld.siege_workshop') WHERE id=(SELECT id FROM def WHERE key='bld.factory');
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.factory','Upgrade to Factory');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.factory'),'upgrade','',0,274);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.factory'),(SELECT id FROM def WHERE key='res.timber'),200,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.factory'),(SELECT id FROM def WHERE key='res.metal'),100,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.factory'),'upgrade_type',(SELECT id FROM def WHERE key='bld.siege_workshop'),(SELECT id FROM def WHERE key='bld.factory'));
INSERT INTO tag(key,category) VALUES ('line_ship','role');
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.carrack'),(SELECT id FROM tag WHERE key='line_ship'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.ship_of_the_line'),(SELECT id FROM tag WHERE key='line_ship'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.ironclad'),(SELECT id FROM tag WHERE key='line_ship'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.dreadnought'),(SELECT id FROM tag WHERE key='line_ship'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.fire_control','Fire Control');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.fire_control'),'upgrade','',0,650);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.fire_control'),(SELECT id FROM def WHERE key='res.wealth'),800,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.fire_control'),(SELECT id FROM def WHERE key='res.knowledge'),400,0);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.fire_control'),'range','add',2000,(SELECT id FROM tag WHERE key='line_ship'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.armour_belts','Armour Belts');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.armour_belts'),'upgrade','',0,750);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.armour_belts'),(SELECT id FROM def WHERE key='res.wealth'),1000,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.armour_belts'),(SELECT id FROM def WHERE key='res.knowledge'),500,0);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.armour_belts'),'hp','pct',25,(SELECT id FROM tag WHERE key='line_ship'));
INSERT INTO ai_weight_key VALUES ('sea.navy_share_pct',20,'share of the military target kept at sea while an enemy can be reached by water'), ('sea.min_difficulty_rank',1,'difficulties of a lower rank stay ashore');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.military.1'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.civic.1'),'research','research',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.commerce.1'),'research','research',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.science.1'),'research','research',4,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.age.classical'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.hoplite'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.archer'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.military.2'),'research','research',1,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.civic.2'),'research','research',2,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.commerce.2'),'research','research',3,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.science.2'),'research','research',4,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.age.medieval'),'research','research',0,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.granary'),(SELECT id FROM def WHERE key='tech.agriculture'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.lumber_mill'),(SELECT id FROM def WHERE key='tech.carpentry'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.temple'),(SELECT id FROM def WHERE key='tech.religion'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.tower'),(SELECT id FROM def WHERE key='tech.militia_training'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.pikeman'),'research','research',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.crossbowman'),'research','research',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM def WHERE key='tech.up.light_cavalry'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM def WHERE key='tech.up.knight'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),(SELECT id FROM def WHERE key='tech.up.trebuchet'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='tech.up.war_galley'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.military.3'),'research','research',1,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.civic.3'),'research','research',2,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.commerce.3'),'research','research',3,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.science.3'),'research','research',4,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.age.gunpowder'),'research','research',0,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.smelter'),(SELECT id FROM def WHERE key='tech.metallurgy'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM def WHERE key='tech.literacy'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.tower'),(SELECT id FROM def WHERE key='tech.allegiance'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.fort'),(SELECT id FROM def WHERE key='tech.fortification'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.musketeer'),'research','research',4,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.arquebusier'),'research','research',0,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM def WHERE key='tech.up.hussar'),'research','research',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM def WHERE key='tech.up.cuirassier'),'research','research',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),(SELECT id FROM def WHERE key='tech.up.bombard'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.military.4'),'research','research_2',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.civic.4'),'research','research_2',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.commerce.4'),'research','research_2',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.science.4'),'research','research_2',4,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.age.enlightenment'),'research','research_2',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.granary'),(SELECT id FROM def WHERE key='tech.crop_rotation'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.lumber_mill'),(SELECT id FROM def WHERE key='tech.architecture'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM def WHERE key='tech.printing'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.temple'),(SELECT id FROM def WHERE key='tech.theology'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.tower'),(SELECT id FROM def WHERE key='tech.up.bastion'),'research','research',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.fort'),(SELECT id FROM def WHERE key='tech.up.star_fort'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.fusilier'),'research','research',1,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.light_infantry'),'research','research',2,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.ranger'),'research','research',3,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM def WHERE key='tech.up.dragoon'),'research','research',4,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM def WHERE key='tech.up.lancer'),'research','research',0,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),(SELECT id FROM def WHERE key='tech.up.cannon'),'research','research',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='tech.up.frigate'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='tech.up.ship_of_the_line'),'research','research',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.market'),(SELECT id FROM def WHERE key='tech.up.freight_wagon'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.military.5'),'research','research_2',1,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.civic.5'),'research','research_2',2,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.commerce.5'),'research','research_2',3,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.science.5'),'research','research_2',4,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.age.industrial'),'research','research_2',0,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.smelter'),(SELECT id FROM def WHERE key='tech.steel'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM def WHERE key='tech.scientific_method'),'research','research',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.fort'),(SELECT id FROM def WHERE key='tech.bombardment'),'research','research',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.rifleman'),'research','research',4,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.machine_gunner'),'research','research',0,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='tech.up.armoured_car'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='tech.up.light_tank'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='tech.up.field_gun'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='tech.up.supply_truck'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='tech.up.ironclad'),'research','research',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.military.6'),'research','research_2',1,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.civic.6'),'research','research_2',2,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.commerce.6'),'research','research_2',3,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.science.6'),'research','research_2',4,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.age.modern'),'research','research_2',0,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.granary'),(SELECT id FROM def WHERE key='tech.food_processing'),'research','research',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.lumber_mill'),(SELECT id FROM def WHERE key='tech.engineering'),'research','research',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.refinery'),(SELECT id FROM def WHERE key='tech.cracking'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.temple'),(SELECT id FROM def WHERE key='tech.philosophy'),'research','research',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.tower'),(SELECT id FROM def WHERE key='tech.patriotism'),'research','research',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM def WHERE key='tech.up.vehicle_plant'),'research','research',1,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),(SELECT id FROM def WHERE key='tech.up.factory'),'research','research',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.bastion'),(SELECT id FROM def WHERE key='tech.up.bunker'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.star_fort'),(SELECT id FROM def WHERE key='tech.up.fortress'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.infantry'),'research','research',1,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.heavy_machine_gunner'),'research','research',2,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.bazooka_team'),'research','research',3,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='tech.up.scout_car'),'research','research',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='tech.up.tank'),'research','research',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='tech.up.howitzer'),'research','research',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='tech.up.dreadnought'),'research','research',4,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='tech.up.fighter'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.market'),(SELECT id FROM def WHERE key='tech.up.lorry'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.military.7'),'research','research_3',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.civic.7'),'research','research_3',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.commerce.7'),'research','research_3',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.science.7'),'research','research_3',4,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.age.information'),'research','research_3',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.smelter'),(SELECT id FROM def WHERE key='tech.alloys'),'research','research',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM def WHERE key='tech.research_institutes'),'research','research',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.fort'),(SELECT id FROM def WHERE key='tech.strategic_reserves'),'research','research',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.assault_infantry'),'research','research',4,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.gun_drone'),'research','research_2',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.missile_team'),'research','research_2',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.recon_drone'),'research','research_2',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='tech.up.main_battle_tank'),'research','research',4,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='tech.up.rocket_artillery'),'research','research',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='tech.up.missile_launcher'),'research','research',4,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='tech.up.jet_fighter'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='tech.up.strategic_bomber'),'research','research',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.military.8'),'research','research_3',1,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.civic.8'),'research','research_3',2,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.commerce.8'),'research','research_3',3,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.science.8'),'research','research_3',4,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.age.automation'),'research','research_3',0,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.refinery'),(SELECT id FROM def WHERE key='tech.petrochemistry'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.tower'),(SELECT id FROM def WHERE key='tech.nationalism'),'research','research',4,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='tech.fire_control'),'research','research',0,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.bunker'),(SELECT id FROM def WHERE key='tech.up.defence_turret'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.combat_robot'),'research','research_2',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='tech.up.hovercraft'),'research','research',0,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='tech.up.robot_tank'),'research','research',1,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='tech.up.drone_cruiser'),'research','research',1,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='tech.up.drone_fighter'),'research','research',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='tech.up.drone_bomber'),'research','research',4,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.city'),(SELECT id FROM def WHERE key='tech.up.worker_drone'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.market'),(SELECT id FROM def WHERE key='tech.up.cargo_drone'),'research','research',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.military.9'),'research','research_3',1,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.civic.9'),'research','research_3',2,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.commerce.9'),'research','research_3',3,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.science.9'),'research','research_3',4,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.age.fusion'),'research','research_3',0,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM def WHERE key='tech.supercomputers'),'research','research',4,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.farm'),(SELECT id FROM def WHERE key='tech.up.auto_farm'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.woodcutter_camp'),(SELECT id FROM def WHERE key='tech.up.auto_woodcutter_camp'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.mine'),(SELECT id FROM def WHERE key='tech.up.auto_mine'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.oil_well'),(SELECT id FROM def WHERE key='tech.up.auto_oil_well'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM def WHERE key='tech.up.auto_lab'),'research','research',0,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.continental_union'),'research','projects',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.fortress'),(SELECT id FROM def WHERE key='tech.up.citadel'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.exo_trooper'),'research','research_2',4,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.pulse_gunner'),'research','research_2',0,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.railgun_team'),'research','research_2',1,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='tech.up.fusion_tank'),'research','research',2,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='tech.up.railgun_battery'),'research','research',0,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='tech.up.laser_battery'),'research','research',1,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='tech.up.hover_frigate'),'research','research',2,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='tech.up.interceptor'),'research','research',0,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.military.10'),'research','research_4',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.civic.10'),'research','research_4',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.commerce.10'),'research','research_4',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.science.10'),'research','research_4',4,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.age.starfaring'),'research','research_4',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),(SELECT id FROM def WHERE key='tech.plasma_containment'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='tech.armour_belts'),'research','research',3,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.marine'),'research','research_2',2,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='tech.up.grav_skimmer'),'research','research',3,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.military.11'),'research','research_4',1,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.civic.11'),'research','research_4',2,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.commerce.11'),'research','research_4',3,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.science.11'),'research','research_4',4,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.age.expansion'),'research','research_4',0,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),(SELECT id FROM def WHERE key='tech.superconducting_grid'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.unification'),'research','projects',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.beam_trooper'),'research','research_2',3,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='tech.up.grav_tank'),'research','research',4,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.military.12'),'research','research_4',1,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.civic.12'),'research','research_4',2,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.commerce.12'),'research','research_4',3,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.science.12'),'research','research_4',4,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.age.abundance'),'research','research_4',0,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),(SELECT id FROM def WHERE key='tech.stellar_collectors'),'research','research',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='tech.enhanced_sensors'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.shock_marine'),'research','research_2',4,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.military.13'),'research','research_5',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.civic.13'),'research','research_5',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.commerce.13'),'research','research_5',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.science.13'),'research','research_5',4,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.age.galactic'),'research','research_5',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='tech.interstellar_trade'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.city'),(SELECT id FROM def WHERE key='tech.auto_colonies'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='tech.automated_route_survey'),'research','research',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.ascension'),'research','projects',2,0);
INSERT OR IGNORE INTO production SELECT (SELECT id FROM def WHERE key='bld.bastion'),product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id=(SELECT id FROM def WHERE key='bld.tower');
INSERT OR IGNORE INTO production SELECT (SELECT id FROM def WHERE key='bld.bunker'),product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id=(SELECT id FROM def WHERE key='bld.tower');
INSERT OR IGNORE INTO production SELECT (SELECT id FROM def WHERE key='bld.defence_turret'),product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id=(SELECT id FROM def WHERE key='bld.tower');
INSERT OR IGNORE INTO production SELECT (SELECT id FROM def WHERE key='bld.bunker'),product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id=(SELECT id FROM def WHERE key='bld.bastion');
INSERT OR IGNORE INTO production SELECT (SELECT id FROM def WHERE key='bld.defence_turret'),product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id=(SELECT id FROM def WHERE key='bld.bastion');
INSERT OR IGNORE INTO production SELECT (SELECT id FROM def WHERE key='bld.defence_turret'),product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id=(SELECT id FROM def WHERE key='bld.bunker');
INSERT OR IGNORE INTO production SELECT (SELECT id FROM def WHERE key='bld.star_fort'),product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id=(SELECT id FROM def WHERE key='bld.fort');
INSERT OR IGNORE INTO production SELECT (SELECT id FROM def WHERE key='bld.fortress'),product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id=(SELECT id FROM def WHERE key='bld.fort');
INSERT OR IGNORE INTO production SELECT (SELECT id FROM def WHERE key='bld.citadel'),product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id=(SELECT id FROM def WHERE key='bld.fort');
INSERT OR IGNORE INTO production SELECT (SELECT id FROM def WHERE key='bld.fortress'),product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id=(SELECT id FROM def WHERE key='bld.star_fort');
INSERT OR IGNORE INTO production SELECT (SELECT id FROM def WHERE key='bld.citadel'),product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id=(SELECT id FROM def WHERE key='bld.star_fort');
INSERT OR IGNORE INTO production SELECT (SELECT id FROM def WHERE key='bld.citadel'),product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id=(SELECT id FROM def WHERE key='bld.fortress');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='unit.horseman'),'train','units',0,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='unit.light_cavalry'),'train','units',1,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='unit.hussar'),'train','units',2,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='unit.dragoon'),'train','units',3,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='unit.cataphract'),'train','units',4,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='unit.knight'),'train','units_2',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='unit.cuirassier'),'train','units_2',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='unit.lancer'),'train','units_2',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='tech.up.light_cavalry'),'research','research',0,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='tech.up.knight'),'research','research',1,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='tech.up.hussar'),'research','research',2,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='tech.up.cuirassier'),'research','research',3,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='tech.up.dragoon'),'research','research',4,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='tech.up.lancer'),'research','research_2',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='unit.catapult'),'train','units',4,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='unit.trebuchet'),'train','units',0,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='unit.bombard'),'train','units',1,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='unit.cannon'),'train','units',2,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='unit.supply_wagon'),'train','units',3,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='tech.up.trebuchet'),'research','research',2,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='tech.up.bombard'),'research','research',3,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='tech.up.cannon'),'research','research',4,1);
INSERT INTO production SELECT (SELECT id FROM def WHERE key='bld.auto_farm'),product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id=(SELECT id FROM def WHERE key='bld.farm');
INSERT INTO production SELECT (SELECT id FROM def WHERE key='bld.auto_woodcutter_camp'),product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id=(SELECT id FROM def WHERE key='bld.woodcutter_camp');
INSERT INTO production SELECT (SELECT id FROM def WHERE key='bld.auto_mine'),product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id=(SELECT id FROM def WHERE key='bld.mine');
INSERT INTO production SELECT (SELECT id FROM def WHERE key='bld.auto_oil_well'),product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id=(SELECT id FROM def WHERE key='bld.oil_well');
INSERT INTO production SELECT (SELECT id FROM def WHERE key='bld.auto_lab'),product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id=(SELECT id FROM def WHERE key='bld.university');
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.city'),(SELECT id FROM def WHERE key='age.ancient'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.farm'),(SELECT id FROM def WHERE key='age.ancient'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.woodcutter_camp'),(SELECT id FROM def WHERE key='age.ancient'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='age.ancient'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.market'),(SELECT id FROM def WHERE key='age.ancient'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.market'),(SELECT id FROM def WHERE key='tech.commerce.1'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.granary'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.granary'),(SELECT id FROM def WHERE key='tech.science.1'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.lumber_mill'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.lumber_mill'),(SELECT id FROM def WHERE key='tech.science.1'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.mine'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM def WHERE key='tech.science.1'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.temple'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.temple'),(SELECT id FROM def WHERE key='tech.civic.1'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.smelter'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.smelter'),(SELECT id FROM def WHERE key='tech.science.2'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.oil_well'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.refinery'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.refinery'),(SELECT id FROM def WHERE key='tech.science.5'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),(SELECT id FROM def WHERE key='tech.science.9'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='age.ancient'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.military.1'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.tower'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.tower'),(SELECT id FROM def WHERE key='tech.military.1'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='age.ancient'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='tech.commerce.1'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM def WHERE key='tech.military.1'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),(SELECT id FROM def WHERE key='tech.military.2'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.fort'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.fort'),(SELECT id FROM def WHERE key='tech.civic.2'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='tech.military.5'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='tech.military.5'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='tech.military.6'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.bastion'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.bastion'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.bastion'),(SELECT id FROM def WHERE key='tech.up.bastion'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.bunker'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.bunker'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.bunker'),(SELECT id FROM def WHERE key='tech.up.bastion'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.bunker'),(SELECT id FROM def WHERE key='tech.up.bunker'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.defence_turret'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.defence_turret'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.defence_turret'),(SELECT id FROM def WHERE key='tech.up.bunker'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.defence_turret'),(SELECT id FROM def WHERE key='tech.up.defence_turret'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.star_fort'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.star_fort'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.star_fort'),(SELECT id FROM def WHERE key='tech.up.star_fort'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.fortress'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.fortress'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.fortress'),(SELECT id FROM def WHERE key='tech.up.star_fort'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.fortress'),(SELECT id FROM def WHERE key='tech.up.fortress'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.citadel'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.citadel'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.citadel'),(SELECT id FROM def WHERE key='tech.up.fortress'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.citadel'),(SELECT id FROM def WHERE key='tech.up.citadel'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.spearman'),(SELECT id FROM def WHERE key='age.ancient'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.hoplite'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.hoplite'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.hoplite'),(SELECT id FROM def WHERE key='tech.up.hoplite'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.pikeman'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.pikeman'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.pikeman'),(SELECT id FROM def WHERE key='tech.up.hoplite'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.pikeman'),(SELECT id FROM def WHERE key='tech.up.pikeman'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.musketeer'),(SELECT id FROM def WHERE key='age.gunpowder'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.musketeer'),(SELECT id FROM def WHERE key='age.gunpowder'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.musketeer'),(SELECT id FROM def WHERE key='tech.up.pikeman'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.musketeer'),(SELECT id FROM def WHERE key='tech.up.musketeer'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.fusilier'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.fusilier'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.fusilier'),(SELECT id FROM def WHERE key='tech.up.musketeer'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.fusilier'),(SELECT id FROM def WHERE key='tech.up.fusilier'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.rifleman'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.rifleman'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.rifleman'),(SELECT id FROM def WHERE key='tech.up.fusilier'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.rifleman'),(SELECT id FROM def WHERE key='tech.up.rifleman'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.infantry'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.infantry'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.infantry'),(SELECT id FROM def WHERE key='tech.up.rifleman'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.infantry'),(SELECT id FROM def WHERE key='tech.up.infantry'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.assault_infantry'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.assault_infantry'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.assault_infantry'),(SELECT id FROM def WHERE key='tech.up.infantry'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.assault_infantry'),(SELECT id FROM def WHERE key='tech.up.assault_infantry'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.combat_robot'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.combat_robot'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.combat_robot'),(SELECT id FROM def WHERE key='tech.up.assault_infantry'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.combat_robot'),(SELECT id FROM def WHERE key='tech.up.combat_robot'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.exo_trooper'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.exo_trooper'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.exo_trooper'),(SELECT id FROM def WHERE key='tech.up.combat_robot'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.exo_trooper'),(SELECT id FROM def WHERE key='tech.up.exo_trooper'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.marine'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.marine'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.marine'),(SELECT id FROM def WHERE key='tech.up.exo_trooper'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.marine'),(SELECT id FROM def WHERE key='tech.up.marine'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.shock_marine'),(SELECT id FROM def WHERE key='age.abundance'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.shock_marine'),(SELECT id FROM def WHERE key='age.abundance'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.shock_marine'),(SELECT id FROM def WHERE key='tech.up.marine'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.shock_marine'),(SELECT id FROM def WHERE key='tech.up.shock_marine'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.slinger'),(SELECT id FROM def WHERE key='age.ancient'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.archer'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.archer'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.archer'),(SELECT id FROM def WHERE key='tech.up.archer'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.crossbowman'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.crossbowman'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.crossbowman'),(SELECT id FROM def WHERE key='tech.up.archer'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.crossbowman'),(SELECT id FROM def WHERE key='tech.up.crossbowman'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.arquebusier'),(SELECT id FROM def WHERE key='age.gunpowder'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.arquebusier'),(SELECT id FROM def WHERE key='age.gunpowder'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.arquebusier'),(SELECT id FROM def WHERE key='tech.up.crossbowman'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.arquebusier'),(SELECT id FROM def WHERE key='tech.up.arquebusier'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.light_infantry'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.light_infantry'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.light_infantry'),(SELECT id FROM def WHERE key='tech.up.arquebusier'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.light_infantry'),(SELECT id FROM def WHERE key='tech.up.light_infantry'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.machine_gunner'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.machine_gunner'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.machine_gunner'),(SELECT id FROM def WHERE key='tech.up.light_infantry'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.machine_gunner'),(SELECT id FROM def WHERE key='tech.up.machine_gunner'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.heavy_machine_gunner'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.heavy_machine_gunner'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.heavy_machine_gunner'),(SELECT id FROM def WHERE key='tech.up.machine_gunner'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.heavy_machine_gunner'),(SELECT id FROM def WHERE key='tech.up.heavy_machine_gunner'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.gun_drone'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.gun_drone'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.gun_drone'),(SELECT id FROM def WHERE key='tech.up.heavy_machine_gunner'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.gun_drone'),(SELECT id FROM def WHERE key='tech.up.gun_drone'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.pulse_gunner'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.pulse_gunner'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.pulse_gunner'),(SELECT id FROM def WHERE key='tech.up.gun_drone'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.pulse_gunner'),(SELECT id FROM def WHERE key='tech.up.pulse_gunner'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.beam_trooper'),(SELECT id FROM def WHERE key='age.expansion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.beam_trooper'),(SELECT id FROM def WHERE key='age.expansion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.beam_trooper'),(SELECT id FROM def WHERE key='tech.up.pulse_gunner'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.beam_trooper'),(SELECT id FROM def WHERE key='tech.up.beam_trooper'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.bazooka_team'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.bazooka_team'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.bazooka_team'),(SELECT id FROM def WHERE key='tech.up.bazooka_team'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.missile_team'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_team'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_team'),(SELECT id FROM def WHERE key='tech.up.bazooka_team'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.missile_team'),(SELECT id FROM def WHERE key='tech.up.missile_team'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.railgun_team'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.railgun_team'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.railgun_team'),(SELECT id FROM def WHERE key='tech.up.missile_team'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.railgun_team'),(SELECT id FROM def WHERE key='tech.up.railgun_team'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.scout'),(SELECT id FROM def WHERE key='age.ancient'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.ranger'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.ranger'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.ranger'),(SELECT id FROM def WHERE key='tech.up.ranger'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.recon_drone'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.recon_drone'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.recon_drone'),(SELECT id FROM def WHERE key='tech.up.ranger'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.recon_drone'),(SELECT id FROM def WHERE key='tech.up.recon_drone'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.medic'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.horseman'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.light_cavalry'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.light_cavalry'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.light_cavalry'),(SELECT id FROM def WHERE key='tech.up.light_cavalry'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.hussar'),(SELECT id FROM def WHERE key='age.gunpowder'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.hussar'),(SELECT id FROM def WHERE key='age.gunpowder'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.hussar'),(SELECT id FROM def WHERE key='tech.up.light_cavalry'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.hussar'),(SELECT id FROM def WHERE key='tech.up.hussar'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.dragoon'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.dragoon'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.dragoon'),(SELECT id FROM def WHERE key='tech.up.hussar'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.dragoon'),(SELECT id FROM def WHERE key='tech.up.dragoon'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.armoured_car'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.armoured_car'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.armoured_car'),(SELECT id FROM def WHERE key='tech.up.dragoon'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.armoured_car'),(SELECT id FROM def WHERE key='tech.up.armoured_car'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.scout_car'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.scout_car'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.scout_car'),(SELECT id FROM def WHERE key='tech.up.armoured_car'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.scout_car'),(SELECT id FROM def WHERE key='tech.up.scout_car'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.hovercraft'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.hovercraft'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.hovercraft'),(SELECT id FROM def WHERE key='tech.up.scout_car'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.hovercraft'),(SELECT id FROM def WHERE key='tech.up.hovercraft'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.grav_skimmer'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.grav_skimmer'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.grav_skimmer'),(SELECT id FROM def WHERE key='tech.up.hovercraft'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.grav_skimmer'),(SELECT id FROM def WHERE key='tech.up.grav_skimmer'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.cataphract'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.knight'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.knight'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.knight'),(SELECT id FROM def WHERE key='tech.up.knight'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.cuirassier'),(SELECT id FROM def WHERE key='age.gunpowder'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.cuirassier'),(SELECT id FROM def WHERE key='age.gunpowder'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.cuirassier'),(SELECT id FROM def WHERE key='tech.up.knight'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.cuirassier'),(SELECT id FROM def WHERE key='tech.up.cuirassier'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.lancer'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.lancer'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.lancer'),(SELECT id FROM def WHERE key='tech.up.cuirassier'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.lancer'),(SELECT id FROM def WHERE key='tech.up.lancer'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.light_tank'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.light_tank'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.light_tank'),(SELECT id FROM def WHERE key='tech.up.lancer'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.light_tank'),(SELECT id FROM def WHERE key='tech.up.light_tank'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.tank'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.tank'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.tank'),(SELECT id FROM def WHERE key='tech.up.light_tank'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.tank'),(SELECT id FROM def WHERE key='tech.up.tank'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.main_battle_tank'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.main_battle_tank'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.main_battle_tank'),(SELECT id FROM def WHERE key='tech.up.tank'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.main_battle_tank'),(SELECT id FROM def WHERE key='tech.up.main_battle_tank'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.robot_tank'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.robot_tank'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.robot_tank'),(SELECT id FROM def WHERE key='tech.up.main_battle_tank'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.robot_tank'),(SELECT id FROM def WHERE key='tech.up.robot_tank'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.fusion_tank'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.fusion_tank'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.fusion_tank'),(SELECT id FROM def WHERE key='tech.up.robot_tank'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.fusion_tank'),(SELECT id FROM def WHERE key='tech.up.fusion_tank'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.grav_tank'),(SELECT id FROM def WHERE key='age.expansion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.grav_tank'),(SELECT id FROM def WHERE key='age.expansion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.grav_tank'),(SELECT id FROM def WHERE key='tech.up.fusion_tank'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.grav_tank'),(SELECT id FROM def WHERE key='tech.up.grav_tank'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.catapult'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.trebuchet'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.trebuchet'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.trebuchet'),(SELECT id FROM def WHERE key='tech.up.trebuchet'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.bombard'),(SELECT id FROM def WHERE key='age.gunpowder'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.bombard'),(SELECT id FROM def WHERE key='age.gunpowder'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.bombard'),(SELECT id FROM def WHERE key='tech.up.trebuchet'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.bombard'),(SELECT id FROM def WHERE key='tech.up.bombard'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.cannon'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.cannon'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.cannon'),(SELECT id FROM def WHERE key='tech.up.bombard'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.cannon'),(SELECT id FROM def WHERE key='tech.up.cannon'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.field_gun'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.field_gun'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.field_gun'),(SELECT id FROM def WHERE key='tech.up.cannon'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.field_gun'),(SELECT id FROM def WHERE key='tech.up.field_gun'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.howitzer'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.howitzer'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.howitzer'),(SELECT id FROM def WHERE key='tech.up.field_gun'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.howitzer'),(SELECT id FROM def WHERE key='tech.up.howitzer'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.rocket_artillery'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.rocket_artillery'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.rocket_artillery'),(SELECT id FROM def WHERE key='tech.up.howitzer'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.rocket_artillery'),(SELECT id FROM def WHERE key='tech.up.rocket_artillery'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.railgun_battery'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.railgun_battery'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.railgun_battery'),(SELECT id FROM def WHERE key='tech.up.rocket_artillery'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.railgun_battery'),(SELECT id FROM def WHERE key='tech.up.railgun_battery'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.flak_gun'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.missile_launcher'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_launcher'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.missile_launcher'),(SELECT id FROM def WHERE key='tech.up.missile_launcher'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.laser_battery'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.laser_battery'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.laser_battery'),(SELECT id FROM def WHERE key='tech.up.missile_launcher'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.laser_battery'),(SELECT id FROM def WHERE key='tech.up.laser_battery'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.supply_wagon'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.supply_truck'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.supply_truck'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.supply_truck'),(SELECT id FROM def WHERE key='tech.up.supply_truck'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.repair_drone'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.trireme'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.war_galley'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.war_galley'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.war_galley'),(SELECT id FROM def WHERE key='tech.up.war_galley'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.carrack'),(SELECT id FROM def WHERE key='age.gunpowder'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.frigate'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.frigate'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.frigate'),(SELECT id FROM def WHERE key='tech.up.war_galley'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.frigate'),(SELECT id FROM def WHERE key='tech.up.frigate'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.ship_of_the_line'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.ship_of_the_line'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.ship_of_the_line'),(SELECT id FROM def WHERE key='tech.up.ship_of_the_line'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.ironclad'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.ironclad'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.ironclad'),(SELECT id FROM def WHERE key='tech.up.ship_of_the_line'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.ironclad'),(SELECT id FROM def WHERE key='tech.up.ironclad'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.destroyer'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.dreadnought'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.dreadnought'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.dreadnought'),(SELECT id FROM def WHERE key='tech.up.ironclad'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.dreadnought'),(SELECT id FROM def WHERE key='tech.up.dreadnought'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.missile_cruiser'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.drone_cruiser'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_cruiser'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.drone_cruiser'),(SELECT id FROM def WHERE key='tech.up.drone_cruiser'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.hover_frigate'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.hover_frigate'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.hover_frigate'),(SELECT id FROM def WHERE key='tech.up.frigate'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.hover_frigate'),(SELECT id FROM def WHERE key='tech.up.hover_frigate'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.biplane'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.fighter'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.fighter'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.fighter'),(SELECT id FROM def WHERE key='tech.up.fighter'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.jet_fighter'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.jet_fighter'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.jet_fighter'),(SELECT id FROM def WHERE key='tech.up.fighter'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.jet_fighter'),(SELECT id FROM def WHERE key='tech.up.jet_fighter'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.drone_fighter'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_fighter'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_fighter'),(SELECT id FROM def WHERE key='tech.up.jet_fighter'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.drone_fighter'),(SELECT id FROM def WHERE key='tech.up.drone_fighter'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.interceptor'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.interceptor'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.interceptor'),(SELECT id FROM def WHERE key='tech.up.drone_fighter'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.interceptor'),(SELECT id FROM def WHERE key='tech.up.interceptor'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.bomber'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.strategic_bomber'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.strategic_bomber'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.strategic_bomber'),(SELECT id FROM def WHERE key='tech.up.strategic_bomber'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.drone_bomber'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_bomber'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_bomber'),(SELECT id FROM def WHERE key='tech.up.strategic_bomber'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.drone_bomber'),(SELECT id FROM def WHERE key='tech.up.drone_bomber'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.survey_ship'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.corvette'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.space_frigate'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.space_cruiser'),(SELECT id FROM def WHERE key='age.expansion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.battleship'),(SELECT id FROM def WHERE key='age.abundance'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='age.ancient'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.worker_drone'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='tech.up.worker_drone'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.scholar'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.merchant'),(SELECT id FROM def WHERE key='age.ancient'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.caravan'),(SELECT id FROM def WHERE key='tech.commerce.2'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.caravan'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.freight_wagon'),(SELECT id FROM def WHERE key='tech.commerce.2'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.freight_wagon'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.freight_wagon'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.freight_wagon'),(SELECT id FROM def WHERE key='tech.up.freight_wagon'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.lorry'),(SELECT id FROM def WHERE key='tech.commerce.2'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.lorry'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.lorry'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.lorry'),(SELECT id FROM def WHERE key='tech.up.freight_wagon'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.lorry'),(SELECT id FROM def WHERE key='tech.up.lorry'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.cargo_drone'),(SELECT id FROM def WHERE key='tech.commerce.2'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.cargo_drone'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.cargo_drone'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.cargo_drone'),(SELECT id FROM def WHERE key='tech.up.lorry'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.cargo_drone'),(SELECT id FROM def WHERE key='tech.up.cargo_drone'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.priest'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.transport_ship'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.1'),(SELECT id FROM def WHERE key='age.ancient'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.2'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.2'),(SELECT id FROM def WHERE key='tech.military.1'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.3'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.3'),(SELECT id FROM def WHERE key='tech.military.2'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.4'),(SELECT id FROM def WHERE key='age.gunpowder'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.4'),(SELECT id FROM def WHERE key='tech.military.3'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.5'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.5'),(SELECT id FROM def WHERE key='tech.military.4'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.6'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.6'),(SELECT id FROM def WHERE key='tech.military.5'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.7'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.7'),(SELECT id FROM def WHERE key='tech.military.6'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.8'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.8'),(SELECT id FROM def WHERE key='tech.military.7'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.9'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.9'),(SELECT id FROM def WHERE key='tech.military.8'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.10'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.10'),(SELECT id FROM def WHERE key='tech.military.9'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.11'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.11'),(SELECT id FROM def WHERE key='tech.military.10'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.12'),(SELECT id FROM def WHERE key='age.expansion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.12'),(SELECT id FROM def WHERE key='tech.military.11'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.13'),(SELECT id FROM def WHERE key='age.abundance'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.military.13'),(SELECT id FROM def WHERE key='tech.military.12'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.1'),(SELECT id FROM def WHERE key='age.ancient'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.2'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.2'),(SELECT id FROM def WHERE key='tech.civic.1'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.3'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.3'),(SELECT id FROM def WHERE key='tech.civic.2'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.4'),(SELECT id FROM def WHERE key='age.gunpowder'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.4'),(SELECT id FROM def WHERE key='tech.civic.3'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.5'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.5'),(SELECT id FROM def WHERE key='tech.civic.4'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.6'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.6'),(SELECT id FROM def WHERE key='tech.civic.5'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.7'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.7'),(SELECT id FROM def WHERE key='tech.civic.6'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.8'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.8'),(SELECT id FROM def WHERE key='tech.civic.7'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.9'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.9'),(SELECT id FROM def WHERE key='tech.civic.8'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.10'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.10'),(SELECT id FROM def WHERE key='tech.civic.9'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.11'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.11'),(SELECT id FROM def WHERE key='tech.civic.10'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.12'),(SELECT id FROM def WHERE key='age.expansion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.12'),(SELECT id FROM def WHERE key='tech.civic.11'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.13'),(SELECT id FROM def WHERE key='age.abundance'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.civic.13'),(SELECT id FROM def WHERE key='tech.civic.12'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.1'),(SELECT id FROM def WHERE key='age.ancient'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.2'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.2'),(SELECT id FROM def WHERE key='tech.commerce.1'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.3'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.3'),(SELECT id FROM def WHERE key='tech.commerce.2'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.4'),(SELECT id FROM def WHERE key='age.gunpowder'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.4'),(SELECT id FROM def WHERE key='tech.commerce.3'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.5'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.5'),(SELECT id FROM def WHERE key='tech.commerce.4'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.6'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.6'),(SELECT id FROM def WHERE key='tech.commerce.5'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.7'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.7'),(SELECT id FROM def WHERE key='tech.commerce.6'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.8'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.8'),(SELECT id FROM def WHERE key='tech.commerce.7'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.9'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.9'),(SELECT id FROM def WHERE key='tech.commerce.8'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.10'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.10'),(SELECT id FROM def WHERE key='tech.commerce.9'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.11'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.11'),(SELECT id FROM def WHERE key='tech.commerce.10'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.12'),(SELECT id FROM def WHERE key='age.expansion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.12'),(SELECT id FROM def WHERE key='tech.commerce.11'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.13'),(SELECT id FROM def WHERE key='age.abundance'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.commerce.13'),(SELECT id FROM def WHERE key='tech.commerce.12'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.1'),(SELECT id FROM def WHERE key='age.ancient'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.2'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.2'),(SELECT id FROM def WHERE key='tech.science.1'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.3'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.3'),(SELECT id FROM def WHERE key='tech.science.2'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.4'),(SELECT id FROM def WHERE key='age.gunpowder'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.4'),(SELECT id FROM def WHERE key='tech.science.3'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.5'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.5'),(SELECT id FROM def WHERE key='tech.science.4'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.6'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.6'),(SELECT id FROM def WHERE key='tech.science.5'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.7'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.7'),(SELECT id FROM def WHERE key='tech.science.6'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.8'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.8'),(SELECT id FROM def WHERE key='tech.science.7'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.9'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.9'),(SELECT id FROM def WHERE key='tech.science.8'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.10'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.10'),(SELECT id FROM def WHERE key='tech.science.9'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.11'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.11'),(SELECT id FROM def WHERE key='tech.science.10'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.12'),(SELECT id FROM def WHERE key='age.expansion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.12'),(SELECT id FROM def WHERE key='tech.science.11'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.13'),(SELECT id FROM def WHERE key='age.abundance'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.science.13'),(SELECT id FROM def WHERE key='tech.science.12'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.classical'),(SELECT id FROM def WHERE key='age.ancient'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.classical'),(SELECT id FROM def WHERE key='tech.military.1'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.classical'),(SELECT id FROM def WHERE key='tech.civic.1'),2);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.classical'),(SELECT id FROM def WHERE key='tech.commerce.1'),3);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.classical'),(SELECT id FROM def WHERE key='tech.science.1'),4);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.medieval'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.medieval'),(SELECT id FROM def WHERE key='tech.military.2'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.medieval'),(SELECT id FROM def WHERE key='tech.civic.2'),2);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.medieval'),(SELECT id FROM def WHERE key='tech.commerce.2'),3);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.medieval'),(SELECT id FROM def WHERE key='tech.science.2'),4);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.gunpowder'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.gunpowder'),(SELECT id FROM def WHERE key='tech.military.3'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.gunpowder'),(SELECT id FROM def WHERE key='tech.civic.3'),2);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.gunpowder'),(SELECT id FROM def WHERE key='tech.commerce.3'),3);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.gunpowder'),(SELECT id FROM def WHERE key='tech.science.3'),4);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.enlightenment'),(SELECT id FROM def WHERE key='age.gunpowder'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.enlightenment'),(SELECT id FROM def WHERE key='tech.military.4'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.enlightenment'),(SELECT id FROM def WHERE key='tech.civic.4'),2);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.enlightenment'),(SELECT id FROM def WHERE key='tech.commerce.4'),3);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.enlightenment'),(SELECT id FROM def WHERE key='tech.science.4'),4);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.industrial'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.industrial'),(SELECT id FROM def WHERE key='tech.military.5'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.industrial'),(SELECT id FROM def WHERE key='tech.civic.5'),2);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.industrial'),(SELECT id FROM def WHERE key='tech.commerce.5'),3);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.industrial'),(SELECT id FROM def WHERE key='tech.science.5'),4);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.modern'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.modern'),(SELECT id FROM def WHERE key='tech.military.6'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.modern'),(SELECT id FROM def WHERE key='tech.civic.6'),2);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.modern'),(SELECT id FROM def WHERE key='tech.commerce.6'),3);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.modern'),(SELECT id FROM def WHERE key='tech.science.6'),4);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.information'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.information'),(SELECT id FROM def WHERE key='tech.military.7'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.information'),(SELECT id FROM def WHERE key='tech.civic.7'),2);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.information'),(SELECT id FROM def WHERE key='tech.commerce.7'),3);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.information'),(SELECT id FROM def WHERE key='tech.science.7'),4);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.automation'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.automation'),(SELECT id FROM def WHERE key='tech.military.8'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.automation'),(SELECT id FROM def WHERE key='tech.civic.8'),2);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.automation'),(SELECT id FROM def WHERE key='tech.commerce.8'),3);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.automation'),(SELECT id FROM def WHERE key='tech.science.8'),4);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.fusion'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.fusion'),(SELECT id FROM def WHERE key='tech.military.9'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.fusion'),(SELECT id FROM def WHERE key='tech.civic.9'),2);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.fusion'),(SELECT id FROM def WHERE key='tech.commerce.9'),3);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.fusion'),(SELECT id FROM def WHERE key='tech.science.9'),4);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.starfaring'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.starfaring'),(SELECT id FROM def WHERE key='tech.military.10'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.starfaring'),(SELECT id FROM def WHERE key='tech.civic.10'),2);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.starfaring'),(SELECT id FROM def WHERE key='tech.commerce.10'),3);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.starfaring'),(SELECT id FROM def WHERE key='tech.science.10'),4);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.expansion'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.expansion'),(SELECT id FROM def WHERE key='tech.military.11'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.expansion'),(SELECT id FROM def WHERE key='tech.civic.11'),2);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.expansion'),(SELECT id FROM def WHERE key='tech.commerce.11'),3);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.expansion'),(SELECT id FROM def WHERE key='tech.science.11'),4);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.abundance'),(SELECT id FROM def WHERE key='age.expansion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.abundance'),(SELECT id FROM def WHERE key='tech.military.12'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.abundance'),(SELECT id FROM def WHERE key='tech.civic.12'),2);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.abundance'),(SELECT id FROM def WHERE key='tech.commerce.12'),3);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.abundance'),(SELECT id FROM def WHERE key='tech.science.12'),4);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.galactic'),(SELECT id FROM def WHERE key='age.abundance'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.galactic'),(SELECT id FROM def WHERE key='tech.military.13'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.galactic'),(SELECT id FROM def WHERE key='tech.civic.13'),2);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.galactic'),(SELECT id FROM def WHERE key='tech.commerce.13'),3);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.age.galactic'),(SELECT id FROM def WHERE key='tech.science.13'),4);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.agriculture'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.crop_rotation'),(SELECT id FROM def WHERE key='age.gunpowder'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.crop_rotation'),(SELECT id FROM def WHERE key='tech.agriculture'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.food_processing'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.food_processing'),(SELECT id FROM def WHERE key='tech.crop_rotation'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.carpentry'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.architecture'),(SELECT id FROM def WHERE key='age.gunpowder'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.architecture'),(SELECT id FROM def WHERE key='tech.carpentry'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.engineering'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.engineering'),(SELECT id FROM def WHERE key='tech.architecture'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.metallurgy'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.steel'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.steel'),(SELECT id FROM def WHERE key='tech.metallurgy'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.alloys'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.alloys'),(SELECT id FROM def WHERE key='tech.steel'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.cracking'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.petrochemistry'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.petrochemistry'),(SELECT id FROM def WHERE key='tech.cracking'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.plasma_containment'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.superconducting_grid'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.superconducting_grid'),(SELECT id FROM def WHERE key='tech.plasma_containment'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.stellar_collectors'),(SELECT id FROM def WHERE key='age.expansion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.stellar_collectors'),(SELECT id FROM def WHERE key='tech.superconducting_grid'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.literacy'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.printing'),(SELECT id FROM def WHERE key='age.gunpowder'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.printing'),(SELECT id FROM def WHERE key='tech.literacy'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.scientific_method'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.scientific_method'),(SELECT id FROM def WHERE key='tech.printing'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.research_institutes'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.research_institutes'),(SELECT id FROM def WHERE key='tech.scientific_method'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.supercomputers'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.supercomputers'),(SELECT id FROM def WHERE key='tech.research_institutes'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.religion'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.theology'),(SELECT id FROM def WHERE key='age.gunpowder'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.theology'),(SELECT id FROM def WHERE key='tech.religion'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.philosophy'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.philosophy'),(SELECT id FROM def WHERE key='tech.theology'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.militia_training'),(SELECT id FROM def WHERE key='age.classical'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.allegiance'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.allegiance'),(SELECT id FROM def WHERE key='tech.militia_training'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.patriotism'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.patriotism'),(SELECT id FROM def WHERE key='tech.allegiance'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.nationalism'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.nationalism'),(SELECT id FROM def WHERE key='tech.patriotism'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.fortification'),(SELECT id FROM def WHERE key='age.medieval'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.bombardment'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.bombardment'),(SELECT id FROM def WHERE key='tech.fortification'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.strategic_reserves'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.strategic_reserves'),(SELECT id FROM def WHERE key='tech.bombardment'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_farm'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.auto_farm'),(SELECT id FROM def WHERE key='tech.up.auto_farm'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.auto_farm'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_woodcutter_camp'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.auto_woodcutter_camp'),(SELECT id FROM def WHERE key='tech.up.auto_woodcutter_camp'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.auto_woodcutter_camp'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_mine'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.auto_mine'),(SELECT id FROM def WHERE key='tech.up.auto_mine'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.auto_mine'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_oil_well'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.auto_oil_well'),(SELECT id FROM def WHERE key='tech.up.auto_oil_well'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.auto_oil_well'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_lab'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.auto_lab'),(SELECT id FROM def WHERE key='tech.up.auto_lab'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.auto_lab'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.continental_union'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.unification'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.orbital_fighter'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.orbital_bomber'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.carrier'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.ground_battery'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.space_freighter'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.satellite'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.jump_inhibitor'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.enhanced_sensors'),(SELECT id FROM def WHERE key='age.expansion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.interstellar_trade'),(SELECT id FROM def WHERE key='age.abundance'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.automated_route_survey'),(SELECT id FROM def WHERE key='age.galactic'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.auto_colonies'),(SELECT id FROM def WHERE key='age.abundance'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.auto_colony'),(SELECT id FROM def WHERE key='age.abundance'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.auto_colony'),(SELECT id FROM def WHERE key='tech.auto_colonies'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.auto_farm'),(SELECT id FROM def WHERE key='tech.auto_colonies'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.auto_woodcutter_camp'),(SELECT id FROM def WHERE key='tech.auto_colonies'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.auto_lab'),(SELECT id FROM def WHERE key='tech.auto_colonies'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.ascension'),(SELECT id FROM def WHERE key='age.galactic'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.vehicle_plant'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='tech.up.vehicle_plant'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.factory'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='tech.up.factory'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.fire_control'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.armour_belts'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO constellation(id,abbr,name,genitive) VALUES (1,'And','Andromeda','Andromedae');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (2,'Ant','Antlia','Antliae');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (3,'Aps','Apus','Apodis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (4,'Aql','Aquila','Aquilae');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (5,'Aqr','Aquarius','Aquarii');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (6,'Ara','Ara','Arae');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (7,'Ari','Aries','Arietis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (8,'Aur','Auriga','Aurigae');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (9,'Boo','Bootes','Bootis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (10,'CMa','Canis Major','Canis Majoris');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (11,'CMi','Canis Minor','Canis Minoris');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (12,'CVn','Canes Venatici','Canum Venaticorum');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (13,'Cae','Caelum','Caeli');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (14,'Cam','Camelopardalis','Camelopardalis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (15,'Cap','Capricornus','Capricorni');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (16,'Car','Carina','Carinae');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (17,'Cas','Cassiopeia','Cassiopeiae');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (18,'Cen','Centaurus','Centauri');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (19,'Cep','Cepheus','Cephei');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (20,'Cet','Cetus','Ceti');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (21,'Cha','Chamaeleon','Chamaeleontis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (22,'Cir','Circinus','Circini');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (23,'Cnc','Cancer','Cancri');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (24,'Col','Columba','Columbae');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (25,'Com','Coma Berenices','Comae Berenices');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (26,'CrA','Corona Australis','Coronae Australis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (27,'CrB','Corona Borealis','Coronae Borealis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (28,'Crt','Crater','Crateris');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (29,'Cru','Crux','Crucis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (30,'Crv','Corvus','Corvi');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (31,'Cyg','Cygnus','Cygni');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (32,'Del','Delphinus','Delphini');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (33,'Dor','Dorado','Doradus');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (34,'Dra','Draco','Draconis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (35,'Equ','Equuleus','Equulei');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (36,'Eri','Eridanus','Eridani');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (37,'For','Fornax','Fornacis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (38,'Gem','Gemini','Geminorum');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (39,'Gru','Grus','Gruis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (40,'Her','Hercules','Herculis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (41,'Hor','Horologium','Horologii');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (42,'Hya','Hydra','Hydrae');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (43,'Hyi','Hydrus','Hydri');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (44,'Ind','Indus','Indi');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (45,'LMi','Leo Minor','Leonis Minoris');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (46,'Lac','Lacerta','Lacertae');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (47,'Leo','Leo','Leonis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (48,'Lep','Lepus','Leporis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (49,'Lib','Libra','Librae');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (50,'Lup','Lupus','Lupi');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (51,'Lyn','Lynx','Lyncis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (52,'Lyr','Lyra','Lyrae');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (53,'Mon','Monoceros','Monocerotis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (54,'Mus','Musca','Muscae');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (55,'Nor','Norma','Normae');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (56,'Oct','Octans','Octantis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (57,'Oph','Ophiuchus','Ophiuchi');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (58,'Ori','Orion','Orionis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (59,'Pav','Pavo','Pavonis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (60,'Peg','Pegasus','Pegasi');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (61,'Per','Perseus','Persei');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (62,'Phe','Phoenix','Phoenicis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (63,'Pic','Pictor','Pictoris');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (64,'PsA','Piscis Austrinus','Piscis Austrini');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (65,'Psc','Pisces','Piscium');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (66,'Pup','Puppis','Puppis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (67,'Pyx','Pyxis','Pyxidis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (68,'Ret','Reticulum','Reticuli');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (69,'Scl','Sculptor','Sculptoris');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (70,'Sco','Scorpius','Scorpii');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (71,'Sct','Scutum','Scuti');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (72,'Ser','Serpens','Serpentis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (73,'Sex','Sextans','Sextantis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (74,'Sge','Sagitta','Sagittae');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (75,'Sgr','Sagittarius','Sagittarii');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (76,'Tau','Taurus','Tauri');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (77,'Tel','Telescopium','Telescopii');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (78,'TrA','Triangulum Australe','Trianguli Australis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (79,'Tri','Triangulum','Trianguli');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (80,'Tuc','Tucana','Tucanae');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (81,'UMa','Ursa Major','Ursae Majoris');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (82,'UMi','Ursa Minor','Ursae Minoris');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (83,'Vel','Vela','Velorum');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (84,'Vir','Virgo','Virginis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (85,'Vol','Volans','Volantis');
INSERT INTO constellation(id,abbr,name,genitive) VALUES (86,'Vul','Vulpecula','Vulpeculae');
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (1,677,'Alpheratz','Alpha Andromedae',1,2097,29090,9701,860,31,479,111733);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (2,746,'Caph','Beta Cassiopeiae',17,2295,59150,5474,379,15,635,117529);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (3,765,'Epsilon Phoenicis','Epsilon Phoenicis',62,2353,-45747,14419,837,34,-860,324336);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (4,1067,'Algenib','Gamma Pegasi',60,3309,15184,39154,1906,110,518,109435);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (5,1562,'Iota Ceti','Iota Ceti',20,4857,-8824,27454,1631,139,-254,98981);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (6,1599,'Zeta Tucanae','Zeta Tucanae',80,5018,-64875,2801,224,20,-479,308321);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (7,2021,'Beta Hydri','Beta Hydri',43,6438,-77254,2433,108,12,-481,304772);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (8,2072,'Kappa Phoenicis','Kappa Phoenicis',62,6551,-43680,7766,633,73,-609,318414);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (9,2081,'Ankaa','Alpha Phoenicis',62,6571,-42306,8472,676,78,-620,319998);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (10,2484,'Beta1 Tucanae','Beta1 Tucanae',80,7886,-62958,13505,523,72,-1035,306779);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (11,2599,'Kappa Cassiopeiae','Kappa Cassiopeiae',17,8250,62932,446790,3010,436,5952,120836);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (12,2912,'Pi Andromedae','Pi Andromedae',1,9220,33719,59845,2008,326,1358,119470);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (13,2920,'Fulu','Zeta Cassiopeiae',17,9243,53897,59301,1416,230,1968,120776);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (14,3031,'Epsilon Andromedae','Epsilon Andromedae',1,9639,29312,16382,1100,187,627,119566);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (15,3092,'Delta Andromedae','Delta Andromedae',1,9832,30861,10552,869,151,527,119870);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (16,3179,'Schedar','Alpha Cassiopeiae',17,10127,56537,22824,820,146,1260,121417);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (17,3405,'Eta Phoenicis','Eta Phoenicis',62,10838,-57463,24634,829,159,-1323,305082);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (18,3419,'Diphda','Beta Ceti',20,10897,-17987,9632,917,176,-303,111330);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (19,3693,'Zeta Andromedae','Zeta Andromedae',1,11835,24267,18919,1227,257,565,121738);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (20,3786,'Delta Piscium','Delta Piscium',65,12171,7585,31122,1709,369,233,121735);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (21,3821,'Achird','Eta Cassiopeiae',17,12276,57815,1942,229,50,373,122620);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (22,4427,'Gamma Cassiopeiae','Gamma Cassiopeiae',17,14177,60717,54908,1111,281,2044,123577);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (23,4436,'Mu Andromedae','Mu Andromedae',1,14188,38499,12974,864,218,709,124074);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (24,4463,'Eta Andromedae','Eta Andromedae',1,14302,23418,22969,1348,344,602,124647);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (25,4577,'Alpha Sculptoris','Alpha Sculptoris',69,14651,-29357,77656,2350,614,-1366,268045);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (26,4906,'Epsilon Piscium','Epsilon Piscium',65,15736,7890,18180,1286,362,185,127889);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (27,5348,'Wurren','Zeta Phoenicis',62,17096,-55246,29868,942,290,-1420,297833);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (28,5364,'Eta Ceti','Eta Ceti',20,17147,-10182,12392,1047,323,-197,137161);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (29,5434,'Phi Andromedae','Phi Andromedae',1,17376,47242,71683,1735,543,1966,126113);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (30,5447,'Mirach','Beta Andromedae',1,17433,35621,19743,1090,342,818,127108);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (31,5542,'Theta Cassiopeiae','Theta Cassiopeiae',17,17776,55150,13356,629,202,948,125764);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (32,6537,'Theta Ceti','Theta Ceti',20,21006,-8183,11380,986,379,-152,146595);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (33,6686,'Ruchbah','Delta Cassiopeiae',17,21454,60235,9941,461,181,866,127190);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (34,6867,'Gamma Phoenicis','Gamma Phoenicis',62,22091,-43318,23364,1030,418,-1049,280527);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (35,7083,'Delta Phoenicis','Delta Phoenicis',62,22813,-49073,14212,720,303,-901,286264);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (36,7097,'Alpherg','Eta Piscium',65,22871,15346,34958,1661,701,495,137010);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (37,7513,'Titawin','Upsilon Andromedae',1,24199,41405,4400,454,204,439,132001);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (38,7588,'Achernar','Alpha Eridani',36,24429,-57237,13944,582,264,-993,290841);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (39,7607,'Nembus','HR 464',1,24498,48628,17716,801,365,999,130815);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (40,7884,'Nu Piscium','Nu Piscium',65,25358,5488,36320,1714,812,182,145119);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (41,8068,'Phi Persei','Phi Persei',61,25915,50689,71841,1527,742,2074,131325);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (42,8102,'Tau Ceti','Tau Ceti',20,26017,-15937,1191,298,146,-95,173101);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (43,8198,'Torcular','Omicron Piscium',65,26348,9158,27948,1479,733,266,144608);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (44,8645,'Baten Kaitos','Zeta Ceti',20,27865,-10335,23498,1333,705,-275,165883);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (45,8796,'Mothallah','Alpha Trianguli',79,28270,29579,6333,610,328,393,138643);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (46,8837,'Psi Phoenicis','Psi Phoenicis',62,28411,-46303,34188,1124,608,-1337,274351);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (47,8886,'Segin','Epsilon Cassiopeiae',17,28599,63670,41181,790,431,1819,129845);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (48,8903,'Sheratan','Beta Arietis',7,28660,20808,5866,628,343,272,142246);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (49,9007,'Chi Eridani','Chi Eridani',36,28989,-51609,5822,414,230,-598,280862);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (50,9236,'Alpha Hydri','Alpha Hydri',43,29692,-61570,7179,350,200,-745,289446);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (51,9347,'Upsilon Ceti','Upsilon Ceti',20,30001,-21078,29278,1383,798,-615,195473);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (52,9487,'Alrescha','Alpha Piscium',65,30512,2764,15058,1056,622,59,155352);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (53,9640,'Almach','Gamma1 Andromedae',1,30975,42330,39296,1256,754,1335,136965);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (54,9884,'Hamal','Alpha Arietis',7,31793,23462,6581,633,392,323,144579);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (55,10064,'Beta Trianguli','Beta Trianguli',79,32386,34987,12686,779,494,646,140549);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (56,10324,'Xi1 Ceti','Xi1 Ceti',20,33250,8847,38326,1618,1061,301,154551);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (57,10602,'Phi Eridani','Phi Eridani',36,34127,-51512,15370,639,433,-970,275352);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (58,10670,'Gamma Trianguli','Gamma Trianguli',79,34329,33847,11231,727,496,590,142651);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (59,10826,'Omicron Ceti','Omicron Ceti',20,34837,-2978,29895,1417,986,-90,167757);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (60,11001,'Delta Hydri','Delta Hydri',43,35437,-68659,13968,350,249,-1101,291250);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (61,11407,'Kappa Eridani','Kappa Eridani',36,36746,-47704,50803,1215,907,-1667,267125);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (62,11484,'Xi2 Ceti','Xi2 Ceti',20,37040,8460,19311,1097,828,204,159692);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (63,11767,'Polaris','Alpha Ursae Minoris',82,37955,89264,43257,21,16,2080,123281);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (64,12387,'Delta Ceti','Delta Ceti',20,39871,329,64971,1956,1634,15,170765);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (65,12394,'Epsilon Hydri','Epsilon Hydri',43,39897,-68267,15184,350,293,-1145,288957);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (66,12486,'Iota Eridani','Iota Eridani',36,40167,-39855,15065,720,608,-787,249296);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (67,12706,'Kaffaljidhma','Gamma Ceti',20,40825,3236,7961,674,582,50,168920);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (68,12770,'Pi Ceti','Pi Ceti',20,41031,-13859,39296,1452,1263,-475,191810);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (69,12777,'Theta Persei','Theta Persei',61,41050,49228,3629,297,258,456,141166);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (70,12828,'Mu Ceti','Mu Ceti',20,41236,10114,8406,679,595,161,163256);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (71,12843,'Tau1 Eridani','Tau1 Eridani',36,41276,-18573,4638,485,426,-217,200836);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (72,13147,'Beta Fornacis','Beta Fornacis',37,42273,-32406,17266,821,746,-704,231789);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (73,13209,'Bharani','HR 838',7,42496,27261,16565,844,773,590,152978);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (74,13268,'Miram','Eta Persei',61,42674,55895,87913,1222,1127,2455,139145);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (75,13531,'Tau Persei','Tau Persei',61,43564,52762,25421,699,665,1269,141024);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (76,13701,'Azha','Eta Eridani',36,44107,-8898,13652,829,803,-181,187157);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (77,13847,'Acamar','Theta1 Eridani',36,44565,-40305,16122,690,679,-821,247857);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (78,14135,'Menkar','Alpha Ceti',20,45570,4090,24916,1102,1124,113,173317);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (79,14146,'Tau3 Eridani','Tau3 Eridani',36,45598,-23624,8863,604,616,-377,213539);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (80,14328,'Gamma Persei','Gamma Persei',61,46199,53506,24322,642,669,1254,142068);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (81,14354,'Rho Persei','Rho Persei',61,46294,38840,30769,944,988,1100,149599);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (82,14576,'Algol','Beta Persei',61,47042,40956,8992,488,524,622,148977);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (83,14632,'Iota Persei','Iota Persei',61,47267,49613,3438,258,279,447,144598);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (84,14668,'Misam','Kappa Persei',61,47374,44858,11274,510,554,749,147123);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (85,14838,'Botein','Delta Arietis',7,47907,19727,16970,822,910,440,162602);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (86,14879,'Dalim','Alpha Fornacis',37,48019,-28988,4643,399,443,-330,224732);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (87,15474,'Tau4 Eridani','Tau4 Eridani',36,49879,-21758,30453,1044,1239,-647,212090);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (88,15863,'Mirfak','Alpha Persei',61,51081,49861,50645,911,1129,1720,146569);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (89,15900,'Omicron Tauri','Omicron Tauri',76,51203,9029,29095,1056,1313,268,174118);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (90,16083,'Xi Tauri','Xi Tauri',76,51792,9733,20907,881,1120,244,174016);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (91,16335,'Sigma Persei','Sigma Persei',61,52644,47995,35960,770,1009,1409,148481);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (92,16537,'Epsilon Eridani','Epsilon Eridani',36,53233,-9458,1049,191,256,-53,195845);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (93,16611,'Tau5 Eridani','Tau5 Eridani',36,53447,-21633,29331,948,1279,-631,213521);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (94,16826,'Psi Persei','Psi Persei',61,54122,48193,58346,944,1305,1800,149173);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (95,17358,'Delta Persei','Delta Persei',61,55731,47788,51607,859,1261,1683,150283);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (96,17378,'Rana','Delta Eridani',36,55812,-9763,2949,301,443,-92,198087);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (97,17440,'Beta Reticuli','Beta Reticuli',68,56050,-64807,9739,235,348,-893,279236);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (98,17448,'Atik','Omicron Persei',61,56080,32288,112081,1579,2349,1788,160364);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (99,17499,'Electra','HR 1142',76,56219,24113,40466,1021,1526,822,166181);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (100,17529,'Nu Persei','Nu Persei',61,56298,42579,55658,964,1445,1596,153832);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (101,17531,'Taygeta','HR 1145',76,56302,24467,40923,1022,1532,838,165985);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (102,17573,'Maia','HR 1149',76,56457,24368,38326,985,1486,808,166171);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (103,17593,'Pi Eridani','Pi Eridani',36,56536,-12102,48106,1183,1789,-460,201555);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (104,17608,'Merope','HR 1156',76,56582,23948,38014,981,1487,791,166571);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (105,17651,'Tau6 Eridani','Tau6 Eridani',36,56712,-23250,5749,382,582,-299,217352);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (106,17678,'Gamma Hydri','Gamma Hydri',43,56810,-74239,21401,218,333,-1408,289129);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (107,17702,'Alcyone','Eta Tauri',76,56871,24105,40316,1002,1535,820,166669);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (108,17847,'Atlas','HR 1178',76,57291,24053,38236,965,1502,797,167015);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (109,18246,'Zeta Persei','Zeta Persei',61,58533,31884,75151,1215,1985,1448,162289);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (110,18532,'Epsilon Persei','Epsilon Persei',61,59463,40010,63827,983,1667,1624,157354);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (111,18543,'Zaurak','Gamma Eridani',36,59507,-13509,20334,704,1195,-333,205163);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (112,18614,'Menkib','Xi Persei',61,59741,35791,124487,1442,2472,2063,160373);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (113,18724,'Lambda Tauri','Lambda Tauri',76,60170,12490,48391,1068,1863,476,178373);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (114,18907,'Nu Tauri','Nu Tauri',76,60789,5989,11694,525,939,113,184680);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (115,19167,'Lambda Persei','Lambda Persei',61,61646,50351,42194,622,1153,1582,151626);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (116,19587,'Beid','Omicron1 Eridani',36,62966,-6838,12170,498,976,-131,199325);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (117,19747,'Alpha Horologii','Alpha Horologii',41,63500,-42294,11501,354,710,-722,246879);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (118,19780,'Alpha Reticuli','Alpha Reticuli',68,63606,-62474,16162,261,526,-1127,274314);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (119,19812,'Mu Persei','Mu Persei',61,63724,48409,90098,882,1787,2245,153940);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (120,19849,'Keid','Omicron2 Eridani',36,63818,-7653,1626,176,359,-54,200753);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (121,19860,'Mu Tauri','Mu Tauri',76,63884,8892,45553,928,1893,330,184208);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (122,19893,'Gamma Doradus','Gamma Doradus',33,64007,-51487,6674,223,457,-639,259835);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (123,19921,'Epsilon Reticuli','Epsilon Reticuli',68,64121,-59302,5949,172,354,-663,270204);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (124,20042,'Upsilon4 Eridani','Upsilon4 Eridani',36,64474,-33798,17794,478,1000,-742,234675);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (125,20205,'Prima Hyadum','Gamma Tauri',76,64948,15628,16154,518,1109,342,179084);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (126,20455,'Secunda Hyadum','Delta1 Tauri',76,65734,17543,15561,489,1084,376,178016);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (127,20535,'Beemim','HR 1393',36,66009,-34017,29624,580,1303,-963,235173);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (128,20635,'Kappa1 Tauri','Kappa1 Tauri',76,66342,22294,15392,461,1051,471,174565);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (129,20648,'Delta3 Tauri','Delta3 Tauri',76,66372,17928,14852,465,1062,375,178118);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (130,20711,'Upsilon Tauri','Upsilon Tauri',76,66577,22814,15377,454,1049,481,174302);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (131,20885,'Theta1 Tauri','Theta1 Tauri',76,67144,15962,15436,464,1101,342,180257);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (132,20889,'Ain','Epsilon Tauri',76,67154,19180,14665,444,1054,398,177597);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (133,20894,'Chamukuy','Theta2 Tauri',76,67166,15871,15037,458,1087,335,180348);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (134,21281,'Alpha Doradus','Alpha Doradus',33,68499,-55045,16864,273,692,-1064,263833);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (135,21393,'Theemin','Upsilon2 Eridani',36,68888,-30562,21387,454,1175,-744,231013);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (136,21421,'Aldebaran','Alpha Tauri',76,68980,16509,6664,281,731,232,180973);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (137,21444,'Nu Eridani','Nu Eridani',36,69080,-3352,67527,926,2423,-152,199312);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (138,21594,'Sceptrum','HR 1481',36,69545,-14304,10985,355,952,-259,211320);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (139,21770,'Alpha Caeli','Alpha Caeli',13,70140,-41864,6577,205,568,-541,246157);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (140,21881,'Tau Tauri','Tau Tauri',76,70561,22957,39824,612,1733,778,176645);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (141,22109,'Mu Eridani','Mu Eridani',36,71376,-3255,52185,728,2161,-130,200531);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (142,22449,'Tabit','Pi3 Orionis',58,72460,6961,2632,153,486,62,191453);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (143,22509,'Pi2 Orionis','Pi2 Orionis',58,72653,8900,22447,441,1413,232,189823);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (144,22549,'Pi4 Orionis','Pi4 Orionis',58,72802,5605,105212,954,3084,317,192891);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (145,22701,'Omega Eridani','Omega Eridani',36,73224,-5453,23498,440,1461,-146,203749);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (146,22797,'Pi5 Orionis','Pi5 Orionis',58,73563,2441,134221,1036,3511,156,196267);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (147,22957,'Omicron2 Orionis','Omicron2 Orionis',58,74093,13514,18595,363,1275,319,186627);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (148,23015,'Hassaleh','Iota Aurigae',8,74248,33166,49343,505,1790,1215,170590);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (149,23123,'Pi6 Orionis','Pi6 Orionis',58,74637,1714,94538,814,2964,92,197559);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (150,23453,'Saclateni','Zeta Aurigae',8,75620,41076,78592,525,2047,1842,165019);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (151,23522,'Beta Camelopardalis','Beta Camelopardalis',14,75855,60442,87208,356,1413,2569,149569);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (152,23685,'Epsilon Leporis','Epsilon Leporis',48,76365,-22371,21331,318,1313,-556,223247);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (153,23767,'Haedus','Eta Aurigae',8,76629,41234,24340,271,1141,1028,165355);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (154,23875,'Cursa','Beta Eridani',36,76962,-5086,8936,212,917,-84,205338);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (155,23972,'Lambda Eridani','Lambda Eridani',36,77287,-8754,81133,620,2746,-434,209141);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (156,24244,'Iota Leporis','Iota Leporis',48,78075,-11869,23181,308,1458,-313,212673);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (157,24305,'Mu Leporis','Mu Leporis',48,78233,-16205,18595,267,1282,-381,217250);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (158,24327,'Kappa Leporis','Kappa Leporis',48,78308,-12941,72803,533,2575,-604,213880);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (159,24331,'Rho Orionis','Rho Orionis',58,78323,2861,34995,378,1830,93,198517);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (160,24436,'Rigel','Beta Orionis',58,78634,-8202,86285,573,2850,-419,209242);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (161,24608,'Capella','Alpha Aurigae',8,79172,45998,4280,85,446,471,162589);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (162,24674,'Tau Orionis','Tau Orionis',58,79402,-6844,49418,406,2170,-265,208277);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (163,24845,'Lambda Leporis','Lambda Leporis',48,79894,-13177,85158,499,2797,-665,214827);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (164,25281,'Eta Orionis','Eta Orionis',58,81119,-2397,97652,482,3085,-131,204866);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (165,25336,'Bellatrix','Gamma Orionis',58,81283,6350,25244,239,1561,176,196928);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (166,25428,'Elnath','Beta Tauri',76,81573,28607,13389,149,1005,554,177995);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (167,25606,'Nihal','Beta Leporis',48,82061,-20759,16035,164,1173,-449,223561);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (168,25859,'Epsilon Columbae','Epsilon Columbae',24,82803,-35471,26324,166,1311,-942,239891);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (169,25930,'Mintaka','Delta Orionis',58,83002,-299,69248,321,2612,-14,203856);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (170,25985,'Arneb','Alpha Leporis',48,83183,-17822,221875,532,4453,-1442,220947);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (171,26069,'Beta Doradus','Beta Doradus',33,83406,-62490,100666,168,1456,-2814,271734);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (172,26176,'Phi1 Orionis','Phi1 Orionis',58,83705,9490,108719,357,3233,544,195402);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (173,26207,'Meissa','Lambda Orionis',58,83784,9934,109817,353,3245,572,195052);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (174,26241,'Hatysa','Iota Orionis',58,83858,-5910,232969,514,4773,-497,209522);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (175,26311,'Alnilam','Epsilon Orionis',58,84053,-1202,197671,461,4421,-93,205212);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (176,26366,'Phi2 Orionis','Phi2 Orionis',58,84227,9291,11749,108,1064,175,195845);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (177,26451,'Tianguan','Zeta Tauri',76,84411,21143,44496,192,1958,761,185686);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (178,26634,'Phact','Alpha Columbae',24,84912,-34074,26134,119,1334,-906,238809);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (179,26727,'Alnitak','Zeta Orionis',58,85190,-1943,73624,227,2702,-92,206453);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (180,27072,'Gamma Leporis','Gamma Leporis',48,86116,-22448,2912,34,498,-206,226796);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (181,27100,'Delta Doradus','Delta Doradus',33,86193,-65736,14961,33,502,-1115,275475);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (182,27288,'Zeta Leporis','Zeta Leporis',48,86739,-14822,7047,46,810,-215,219400);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (183,27321,'Beta Pictoris','Beta Pictoris',63,86821,-51067,6341,28,500,-619,258365);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (184,27366,'Saiph','Kappa Orionis',58,86939,-9670,64714,134,2504,-427,214515);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (185,27628,'Wazn','Beta Columbae',24,87740,-35768,8718,30,757,-546,241342);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (186,27654,'Delta Leporis','Delta Leporis',48,87830,-20879,11372,38,996,-380,225835);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (187,27673,'Nu Aurigae','Nu Aurigae',8,87873,39148,23034,44,1176,958,171841);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (188,27913,'Chi1 Orionis','Chi1 Orionis',58,88596,20276,2826,12,499,184,188459);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (189,27989,'Betelgeuse','Alpha Orionis',58,88793,7407,49795,47,2212,288,199788);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (190,28103,'Eta Leporis','Eta Leporis',48,89101,-14168,4853,11,675,-171,219756);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (191,28199,'Gamma Columbae','Gamma Columbae',24,89384,-35283,86975,26,2407,-1703,241226);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (192,28328,'Eta Columbae','Eta Columbae',24,89787,-42815,47201,6,1594,-1477,249457);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (193,28358,'Delta Aurigae','Delta Aurigae',8,89882,54285,12603,1,655,911,158952);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (194,28360,'Menkalinan','Beta Aurigae',8,89882,44947,8111,1,637,636,167457);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (195,28380,'Mahasim','Theta Aurigae',8,89930,37213,16556,1,1025,778,174338);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (196,28404,'Pi Aurigae','Pi Aurigae',8,89984,45937,75151,1,1906,1970,166602);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (197,28614,'Mu Orionis','Mu Orionis',58,90596,9647,15494,-13,1227,209,198684);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (198,29038,'Nu Orionis','Nu Orionis',58,91893,14768,51607,-73,2195,579,194807);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (199,29426,'Xi Orionis','Xi Orionis',58,92985,14209,60737,-124,2386,605,195808);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (200,29651,'Gamma Monocerotis','Gamma Monocerotis',53,93714,-6275,49795,-144,2213,-244,214405);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (201,29655,'Propus','Eta Geminorum',38,93719,22507,38462,-118,1808,751,188853);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (202,29696,'Kappa Aurigae','Kappa Aurigae',8,93845,29498,17697,-78,1155,655,182727);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (203,29807,'Kappa Columbae','Kappa Columbae',24,94138,-35141,18252,-80,1102,-778,242348);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (204,30122,'Furud','Zeta Canis Majoris',10,95078,-30063,36240,-146,1641,-954,237522);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (205,30277,'Delta Columbae','Delta Columbae',24,95528,-33436,23397,-123,1271,-843,241026);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (206,30324,'Mirzam','Beta Canis Majoris',10,95675,-17956,49268,-209,2101,-684,226060);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (207,30343,'Tejat','Mu Geminorum',38,95740,22514,23165,-141,1399,583,189726);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (208,30419,'Epsilon Monocerotis','Epsilon Monocerotis',53,95942,4593,12229,-114,1096,89,205675);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (209,30438,'Canopus','Alpha Carinae',16,95988,-52696,30915,-111,1060,-1399,261213);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (210,30788,'Lambda Canis Majoris','Lambda Canis Majoris',10,97043,-32580,42358,-213,1721,-1108,240642);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (211,30883,'Nu Geminorum','Nu Geminorum',38,97241,20212,54450,-276,2172,806,192421);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (212,31125,'Xi1 Canis Majoris','Xi1 Canis Majoris',10,97964,-23418,138202,-473,3378,-1477,232111);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (213,31592,'Nu2 Canis Majoris','Nu2 Canis Majoris',10,99171,-19256,6442,-121,748,-265,228689);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (214,31681,'Alhena','Gamma Geminorum',38,99428,16399,10930,-164,989,295,196774);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (215,31685,'Nu Puppis','Nu Puppis',66,99440,-43196,37148,-230,1386,-1319,251936);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (216,31700,'Nu3 Canis Majoris','Nu3 Canis Majoris',10,99473,-18237,42139,-321,1923,-642,227870);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (217,32246,'Mebsuta','Epsilon Geminorum',38,100983,25131,84496,-501,2583,1234,189536);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (218,32349,'Sirius','Alpha Canis Majoris',10,101287,-16716,860,-55,275,-84,227230);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (219,32362,'Alzirr','Xi Geminorum',38,101322,12896,5870,-147,732,171,200739);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (220,32607,'Alpha Pictoris','Alpha Pictoris',63,102048,-61941,9655,-96,452,-867,271918);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (221,32759,'Kappa Canis Majoris','Kappa Canis Majoris',10,102460,-32508,65890,-467,2114,-1379,242355);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (222,32768,'Tau Puppis','Tau Puppis',66,102484,-50615,18201,-185,836,-1043,260164);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (223,33018,'Theta Geminorum','Theta Geminorum',38,103197,33961,18908,-260,1110,768,182106);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (224,33160,'Theta Canis Majoris','Theta Canis Majoris',10,103547,-12039,26072,-370,1535,-337,223983);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (225,33347,'Iota Canis Majoris','Iota Canis Majoris',10,104034,-17054,250890,-1161,4646,-1469,228703);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (226,33579,'Adhara','Epsilon Canis Majoris',10,104656,-28972,40516,-446,1704,-975,239831);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (227,33856,'Nganurganity','Sigma Canis Majoris',10,105430,-27935,112081,-787,2851,-1568,239169);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (228,34045,'Muliphein','Gamma Canis Majoris',10,105940,-15633,44195,-556,1947,-567,228252);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (229,34088,'Mekbuda','Zeta Geminorum',38,106027,20570,137619,-959,3338,1303,195747);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (230,34444,'Wezen','Delta Canis Majoris',10,107098,-26393,160668,-1056,3432,-1782,238418);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (231,34481,'Gamma2 Volantis','Gamma2 Volantis',85,107187,-70499,13980,-117,377,-1115,281556);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (232,34693,'Tau Geminorum','Tau Geminorum',38,107785,30245,32102,-473,1474,902,187219);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (233,34769,'Delta Monocerotis','Delta Monocerotis',53,107966,-493,38417,-605,1864,-17,215705);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (234,35037,'Omega Canis Majoris','Omega Canis Majoris',10,108703,-26773,91105,-864,2552,-1360,239408);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (235,35228,'Delta Volantis','Delta Volantis',85,109208,-67957,73791,-335,963,-2518,279086);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (236,35264,'Pi Puppis','Pi Puppis',66,109286,-37097,80732,-749,2139,-1714,249011);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (237,35350,'Lambda Geminorum','Lambda Geminorum',38,109523,16540,10088,-322,907,286,200917);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (238,35550,'Wasat','Delta Geminorum',38,110031,21982,6047,-247,677,291,195986);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (239,35904,'Aludra','Eta Canis Majoris',10,111024,-29303,198876,-1395,3630,-2183,242617);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (240,36046,'Iota Geminorum','Iota Geminorum',38,111432,27798,12035,-355,903,512,190847);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (241,36188,'Gomeisa','Beta Canis Minoris',11,111788,8289,16170,-467,1168,183,209521);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (242,36284,'Gamma Canis Minoris','Gamma Canis Minoris',11,112041,8926,31820,-661,1633,277,209048);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (243,36366,'Rho Geminorum','Rho Geminorum',38,112278,31785,5886,-247,603,404,187133);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (244,36377,'Sigma Puppis','Sigma Puppis',66,112308,-43301,19368,-384,937,-954,255736);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (245,36850,'Castor','Alpha Geminorum',38,113649,31888,5087,-243,555,377,187441);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (246,36962,'Upsilon Geminorum','Upsilon Geminorum',38,113981,26896,27089,-597,1341,745,192605);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (247,37279,'Procyon','Alpha Canis Minoris',11,114825,5225,1146,-142,306,31,213702);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (248,37447,'Alpha Monocerotis','Alpha Monocerotis',53,115312,-9551,14778,-513,1084,-202,227187);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (249,37504,'Zeta Volantis','Zeta Volantis',85,115455,-72606,14101,-153,321,-1133,284571);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (250,37629,'Sigma Geminorum','Sigma Geminorum',38,115828,28884,12506,-427,881,540,191189);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (251,37740,'Kappa Geminorum','Kappa Geminorum',38,116112,24398,14138,-477,972,491,195848);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (252,37826,'Pollux','Beta Geminorum',38,116329,28026,3378,-228,460,273,192230);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (253,38070,'Omicron Puppis','Omicron Puppis',66,117022,-25937,141807,-1539,3017,-1647,242246);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (254,38170,'Azmidi','Xi Puppis',66,117324,-24860,119910,-1442,2791,-1456,241456);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (255,38827,'Chi Carinae','Chi Carinae',16,119195,-52982,45489,-626,1121,-1703,266677);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (256,39429,'Naos','Zeta Puppis',66,120896,-40003,108358,-1295,2164,-2116,255976);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (257,39757,'Tureis','Rho Puppis',66,121886,-24304,6354,-384,617,-328,243148);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (258,39794,'Epsilon Volantis','Epsilon Volantis',85,121982,-68617,56234,-458,733,-2208,281624);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (259,39863,'Zeta Monocerotis','Zeta Monocerotis',53,122149,-2984,105895,-1729,2751,-169,224723);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (260,39953,'Gamma2 Velorum','Gamma2 Velorum',83,122383,-47337,111697,-1213,1913,-2458,262803);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (261,40526,'Tarf','Beta Cancri',23,124129,9186,30340,-965,1423,278,214253);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (262,40702,'Alpha Chamaeleontis','Alpha Chamaeleontis',21,124631,-76920,6380,-103,149,-778,289860);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (263,40888,'Theta Chamaeleontis','Theta Chamaeleontis',21,125161,-77484,15531,-156,221,-1217,290458);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (264,41037,'Avior','Epsilon Carinae',16,125628,-59509,60511,-727,1015,-2120,274285);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (265,41075,'Alsciaukat','HR 3275',51,125709,43188,38236,-832,1158,1338,177390);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (266,41312,'Beta Volantis','Beta Volantis',85,126434,-66137,10754,-249,338,-948,280262);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (267,41704,'Muscida','Omicron Ursae Majoris',81,127566,60718,17911,-399,519,1167,155977);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (268,42313,'Delta Hydrae','Delta Hydrae',42,129414,5704,16035,-800,973,126,220255);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (269,42402,'Minchir','Sigma Hydrae',42,129689,3341,37275,-1231,1483,113,222714);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (270,42515,'Beta Pyxidis','Beta Pyxidis',67,130026,-35308,41602,-1070,1275,-1179,256237);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (271,42799,'Eta Hydrae','Eta Hydrae',42,130806,3399,58661,-1580,1830,144,223251);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (272,42828,'Alpha Pyxidis','Alpha Pyxidis',67,130898,-33186,87913,-1625,1876,-1623,254993);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (273,42911,'Asellus Australis','Delta Cancri',23,131171,18154,13057,-715,817,356,208026);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (274,42913,'Alsephina','Delta Velorum',83,131176,-54709,8055,-341,390,-733,272079);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (275,43103,'Iota Cancri','Iota Cancri',23,131674,28760,33112,-1061,1192,876,195883);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (276,43109,'Ashlesha','Epsilon Hydrae',42,131694,6419,12927,-752,844,127,220718);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (277,43234,'Rho Hydrae','Rho Hydrae',42,132108,5838,35413,-1255,1389,191,221522);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (278,43409,'Gamma Pyxidis','Gamma Pyxidis',67,132633,-27710,20735,-863,938,-670,251571);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (279,43813,'Zeta Hydrae','Zeta Hydrae',42,133848,5946,16717,-891,927,134,222342);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (280,44066,'Acubens','Alpha Cancri',23,134622,11858,18831,-943,956,282,216510);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (281,44127,'Talitha','Iota Ursae Majoris',81,134802,48042,4732,-324,326,512,171512);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (282,44382,'Alpha Volantis','Alpha Volantis',85,135612,-66396,12492,-320,313,-1024,282713);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (283,44471,'Alkaphrah','Kappa Ursae Majoris',81,135906,47157,35841,-925,896,1388,172627);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (284,44816,'Suhail','Lambda Velorum',83,136999,-43433,54450,-1239,1156,-1604,265936);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (285,45238,'Miaplacidus','Beta Carinae',16,138300,-69717,11317,-275,245,-998,285978);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (286,45336,'Theta Hydrae','Theta Hydrae',42,138591,2314,11349,-798,704,43,228773);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (287,45556,'Aspidiske','Iota Carinae',16,139273,-59275,76563,-1071,922,-2379,278459);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (288,45860,'Alpha Lyncis','Alpha Lyncis',51,140264,34393,20309,-904,752,805,190235);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (289,45941,'Markeb','Kappa Velorum',83,140528,-55011,57220,-1059,872,-1960,275883);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (290,46146,'Kappa Leonis','Kappa Leonis',47,141164,26182,20133,-992,798,626,201763);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (291,46390,'Alphard','Alpha Hydrae',42,141897,-8659,18030,-1045,819,-202,241489);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (292,46651,'Psi Velorum','Psi Velorum',83,142675,-40467,6137,-474,361,-508,266777);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (293,46750,'Alterf','Lambda Leonis',47,142930,22968,32912,-1333,1007,708,206693);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (294,46853,'Theta Ursae Majoris','Theta Ursae Majoris',81,143214,51677,4396,-329,246,520,165463);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (295,47431,'Ukdah','Iota Hydrae',42,144964,-1143,26324,-1328,931,-32,236490);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (296,47508,'Subra','Omicron Leonis',47,145288,9892,13031,-924,640,196,224601);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (297,47908,'Epsilon Leonis','Epsilon Leonis',47,146463,23774,24671,-1198,794,633,206817);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (298,48002,'Upsilon Carinae','Upsilon Carinae',16,146776,-65072,143681,-1336,875,-3437,285039);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (299,48319,'Upsilon Ursae Majoris','Upsilon Ursae Majoris',81,147747,59039,11624,-469,296,925,154310);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (300,48356,'Zhang','Upsilon1 Hydrae',42,147870,-14847,26388,-1330,835,-416,251236);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (301,48455,'Rasalas','Mu Leonis',47,148191,26007,12411,-851,528,488,204045);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (302,48774,'Phi Velorum','Phi Velorum',83,149216,-54568,159101,-1987,1184,-3250,279347);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (303,49583,'Eta Leonis','Eta Leonis',47,151833,16763,126909,-3007,1610,1027,219530);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (304,49641,'Alpha Sextantis','Alpha Sextantis',73,151985,-372,28337,-1486,791,-11,241110);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (305,49669,'Regulus','Alpha Leonis',47,152093,11967,7930,-770,408,185,226428);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (306,49841,'Lambda Hydrae','Lambda Hydrae',42,152647,-12354,11255,-920,476,-227,253005);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (307,50099,'Omega Carinae','Omega Carinae',16,153434,-70038,34188,-565,282,-1738,290164);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (308,50335,'Adhafera','Zeta Leonis',47,154173,23417,27408,-1367,662,658,210228);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (309,50372,'Tania Borealis','Lambda Ursae Majoris',81,154274,42914,13750,-774,373,798,175872);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (310,50583,'Algieba','Gamma1 Leonis',47,154993,19841,13010,-972,454,387,216552);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (311,50801,'Tania Australis','Mu Ursae Majoris',81,155582,41500,23034,-1035,470,1006,177902);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (312,51069,'Mu Hydrae','Mu Hydrae',42,156523,-16836,23414,-1343,583,-443,259987);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (313,51172,'Alpha Antliae','Alpha Antliae',2,156788,-31068,36606,-1506,646,-987,269877);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (314,51233,'Beta Leonis Minoris','Beta Leonis Minoris',45,156971,36707,15392,-915,389,742,186287);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (315,51624,'Rho Leonis','Rho Leonis',47,158203,9307,543594,-6756,2702,1192,234887);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (316,51839,'Gamma Chamaeleontis','Gamma Chamaeleontis',21,158867,-78608,41761,-376,146,-2003,296282);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (317,52419,'Theta Carinae','Theta Carinae',16,160739,-64394,45553,-871,304,-1925,289598);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (318,52633,'Delta2 Chamaeleontis','Delta2 Chamaeleontis',21,161446,-80540,35071,-292,98,-1847,297735);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (319,52727,'Mu Velorum','Mu Velorum',83,161692,-49420,11715,-668,221,-822,283027);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (320,52943,'Nu Hydrae','Nu Hydrae',42,162406,-16194,14374,-1097,348,-334,265053);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (321,53229,'Praecipua','HR 4247',45,163328,34215,9487,-772,231,548,190010);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (322,53740,'Alkes','Alpha Crateris',28,164944,-18299,15918,-1157,311,-396,269053);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (323,53910,'Merak','Beta Ursae Majoris',81,165460,56382,7974,-479,124,744,149166);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (324,54061,'Dubhe','Alpha Ursae Majoris',81,165932,61751,12289,-509,128,977,142848);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (325,54539,'Psi Ursae Majoris','Psi Ursae Majoris',81,167416,44498,14451,-837,187,843,165804);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (326,54682,'Beta Crateris','Beta Crateris',28,167915,-22826,34010,-1662,356,-715,274778);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (327,54872,'Zosma','Delta Leonis',47,168527,20524,5843,-702,142,268,224228);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (328,54879,'Chertan','Theta Leonis',47,168560,15430,16506,-1214,246,342,235371);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (329,55084,'Phi Leonis','Phi Leonis',47,169165,-3652,18417,-1330,255,-86,262662);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (330,55219,'Alula Borealis','Nu Ursae Majoris',81,169620,33094,39921,-1647,302,1091,190727);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (331,55282,'Delta Crateris','Delta Crateris',28,169835,-14779,18574,-1297,233,-348,272068);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (332,55425,'Pi Centauri','Pi Centauri',18,170252,-54491,35763,-1083,186,-1539,289956);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (333,55434,'Sigma Leonis','Sigma Leonis',47,170284,6029,22008,-1454,249,156,253360);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (334,55642,'Iota Leonis','Iota Leonis',47,170981,10530,7722,-853,135,161,247599);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (335,55705,'Gamma Crateris','Gamma Crateris',28,171221,-17684,8232,-854,132,-276,275408);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (336,56211,'Giausar','Lambda Draconis',34,172851,69331,33315,-639,80,1708,132981);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (337,56343,'Xi Hydrae','Xi Hydrae',42,173250,-31858,12963,-960,114,-601,284075);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (338,56561,'Lambda Centauri','Lambda Centauri',18,173945,-63020,41976,-924,98,-1826,294473);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (339,56647,'Upsilon Leonis','Upsilon Leonis',47,174237,-824,18150,-1340,135,-19,267264);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (340,57363,'Lambda Muscae','Lambda Muscae',54,176402,-66729,12716,-445,28,-1036,296484);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (341,57380,'Nu Virginis','Nu Virginis',84,176465,6529,29383,-1700,105,195,262867);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (342,57399,'Taiyangshou','Chi Ursae Majoris',81,176513,47779,18365,-909,55,1004,150319);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (343,57632,'Denebola','Beta Leonis',47,177265,14572,3588,-579,28,151,250641);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (344,57757,'Zavijava','Beta Virginis',84,177674,1765,3565,-596,24,18,270525);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (345,57936,'Beta Hydrae','Beta Hydrae',42,178227,-33908,30974,-1460,45,-982,289272);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (346,58001,'Phecda','Gamma Ursae Majoris',81,178458,53695,8318,-540,15,735,140839);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (347,58758,'Theta1 Crucis','Theta1 Crucis',29,180756,-63313,23498,-688,-9,-1370,297528);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (348,58948,'Omicron Virginis','Omicron Virginis',84,181302,8733,16324,-1263,-29,194,270061);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (349,59072,'Eta Crucis','Eta Crucis',29,181720,-64614,6443,-344,-10,-725,298178);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (350,59196,'Delta Centauri','Delta Centauri',18,182090,-50722,41496,-1289,-47,-1577,295997);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (351,59199,'Alchiba','Alpha Corvi',30,182103,-24729,4872,-634,-23,-292,290659);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (352,59316,'Epsilon Corvi','Epsilon Corvi',30,182531,-22620,31789,-1644,-73,-686,290591);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (353,59449,'Rho Centauri','Rho Centauri',18,182913,-52368,37881,-1187,-60,-1541,296784);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (354,59747,'Imai','Delta Crucis',29,183786,-58749,34514,-962,-64,-1588,298229);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (355,59774,'Megrez','Delta Ursae Majoris',81,183857,57033,8051,-487,-33,753,132565);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (356,59803,'Gienah','Gamma Corvi',30,183952,-17542,15363,-1179,-81,-374,290986);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (357,59929,'Epsilon Muscae','Epsilon Muscae',54,184393,-67961,30144,-650,-50,-1609,299752);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (358,60000,'Beta Chamaeleontis','Beta Chamaeleontis',21,184587,-79312,29840,-319,-26,-1697,301337);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (359,60009,'Zeta Crucis','Zeta Crucis',29,184609,-64003,35763,-826,-67,-1700,299325);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (360,60129,'Eta Virginis','Eta Virginis',84,184976,-667,26538,-1623,-141,-19,286399);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (361,60260,'Ginan','Epsilon Crucis',29,185340,-60401,22985,-746,-70,-1318,299223);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (362,60718,'Acrux','Alpha1 Crucis',29,186650,-63099,32197,-806,-94,-1600,300127);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (363,60742,'Gamma Comae Berenices','Gamma Comae Berenices',25,186734,28268,16726,-1131,-134,612,199647);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (364,60823,'Sigma Centauri','Sigma Centauri',18,187010,-50231,41181,-1288,-158,-1560,299104);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (365,60965,'Algorab','Delta Corvi',30,187466,-16515,8686,-886,-116,-265,295473);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (366,61084,'Gacrux','Gamma Crucis',29,187791,-57113,8856,-506,-69,-790,300169);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (367,61174,'Eta Corvi','Eta Corvi',30,188018,-16196,5963,-734,-103,-215,296180);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (368,61199,'Gamma Muscae','Gamma Muscae',54,188117,-72133,32486,-547,-78,-1715,301459);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (369,61281,'Kappa Draconis','Kappa Draconis',34,188371,69788,49046,-757,-111,2078,125215);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (370,61317,'Chara','Beta Canum Venaticorum',12,188436,41357,2753,-390,-58,347,136131);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (371,61359,'Kraz','Beta Corvi',30,188597,-23397,14567,-1095,-166,-479,297875);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (372,61585,'Alpha Muscae','Alpha Muscae',54,189296,-69136,31543,-624,-102,-1660,301656);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (373,61622,'Tau Centauri','Tau Centauri',18,189426,-48541,13125,-748,-124,-859,300587);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (374,61932,'Gamma Centauri','Gamma Centauri',18,190379,-48960,13015,-737,-135,-860,301255);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (375,61941,'Porrima','Gamma Virginis',84,190415,-1449,3811,-607,-112,-16,297835);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (376,62322,'Beta Muscae','Beta Muscae',54,191570,-68108,34153,-675,-138,-1715,302449);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (377,62434,'Mimosa','Beta Crucis',29,191930,-59689,27853,-824,-174,-1441,302462);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (378,62956,'Alioth','Epsilon Ursae Majoris',81,193507,55960,8255,-495,-119,753,122180);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (379,63003,'Mu1 Crucis','Mu1 Crucis',29,193648,-57178,41443,-1072,-260,-1711,303362);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (380,63090,'Minelauva','Delta Virginis',84,193901,3397,19839,-1365,-338,83,305515);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (381,63125,'Cor Caroli','Alpha2 Canum Venaticorum',12,194007,38318,11480,-816,-203,664,118303);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (382,63608,'Vindemiatrix','Epsilon Virginis',84,195544,10959,10960,-990,-275,199,312326);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (383,63613,'Delta Muscae','Delta Muscae',54,195568,-71549,9090,-291,-81,-904,303799);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (384,64004,'Xi2 Centauri','Xi2 Centauri',18,196728,-49906,46727,-1333,-401,-1654,305487);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (385,64238,'Theta Virginis','Theta Virginis',84,197487,-5539,31574,-1687,-531,-172,311418);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (386,64394,'Beta Comae Berenices','Beta Comae Berenices',25,197968,27878,2978,-459,-149,255,43504);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (387,64962,'Gamma Hydrae','Gamma Hydrae',42,199730,-23172,13384,-1001,-359,-455,311099);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (388,65109,'Iota Centauri','Iota Centauri',18,200149,-36712,5878,-577,-212,-458,309419);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (389,65378,'Mizar','Zeta Ursae Majoris',81,200981,54925,8581,-497,-191,758,113110);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (390,65474,'Spica','Alpha Virginis',84,201298,-11161,24974,-1445,-563,-306,316113);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (391,65477,'Alcor','HR 5062',81,201306,54988,8172,-483,-188,740,112769);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (392,66249,'Heze','Zeta Virginis',84,203673,-596,7408,-788,-346,-9,325246);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (393,66657,'Epsilon Centauri','Epsilon Centauri',18,204972,-53466,42747,-1116,-520,-1661,310192);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (394,67275,'Tau Bootis','Tau Bootis',9,206816,17457,5094,-608,-307,214,358940);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (395,67301,'Alkaid','Eta Ursae Majoris',81,206885,49313,10394,-593,-301,773,100695);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (396,67459,'Upsilon Bootis','Upsilon Bootis',9,207369,15798,26345,-1387,-718,442,355812);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (397,67464,'Nu Centauri','Nu Centauri',18,207376,-41688,43662,-1386,-718,-1390,314414);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (398,67472,'Mu Centauri','Mu Centauri',18,207404,-42474,50567,-1472,-763,-1518,314238);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (399,67927,'Muphrid','Eta Bootis',9,208671,18398,3717,-508,-278,192,5291);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (400,68002,'Zeta Centauri','Zeta Centauri',18,208885,-47288,38192,-1161,-640,-1436,314069);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (401,68245,'Phi Centauri','Phi Centauri',18,209568,-42101,52521,-1479,-839,-1536,315977);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (402,68282,'Upsilon1 Centauri','Upsilon1 Centauri',18,209670,-44804,42747,-1275,-726,-1457,315287);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (403,68520,'Tau Virginis','Tau Virginis',84,210412,1545,22494,-1293,-759,40,339220);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (404,68702,'Hadar','Beta Centauri',18,210956,-60373,39201,-839,-503,-1721,311767);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (405,68756,'Thuban','Alpha Draconis',34,211097,64376,30312,-645,-389,1570,110524);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (406,68862,'Chi Centauri','Chi Centauri',18,211512,-41180,51042,-1450,-889,-1488,317731);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (407,68895,'Pi Hydrae','Pi Hydrae',42,211593,-26682,10098,-765,-470,-451,323002);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (408,68933,'Menkent','Theta Centauri',18,211671,-36370,5882,-526,-324,-455,319454);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (409,69427,'Kang','Kappa Virginis',84,213224,-10274,25481,-1314,-861,-285,333516);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (410,69673,'Arcturus','Alpha Bootis',9,213915,19182,3672,-475,-319,199,15050);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (411,69701,'Syrma','Iota Virginis',84,214004,-6001,7253,-702,-474,-89,337742);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (412,69732,'Xuange','Lambda Bootis',9,214096,46088,9902,-572,-387,717,86980);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (413,69996,'Iota Lupi','Iota Lupi',50,214851,-46058,33799,-1047,-729,-1324,318476);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (414,70090,'Psi Centauri','Psi Centauri',18,215139,-37885,25885,-1038,-731,-988,321721);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (415,70497,'Theta Bootis','Theta Bootis',9,216299,51851,4739,-343,-252,541,93832);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (416,70576,'Tau2 Lupi','Tau2 Lupi',50,216545,-45379,31914,-1008,-747,-1272,319866);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (417,70638,'Delta Octantis','Delta Octantis',56,216730,-83668,29895,-153,-114,-1718,305678);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (418,70890,'Proxima Centauri','GJ 551',18,217429,-62679,423,-75,-57,-183,313940);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (419,71053,'Rho Bootis','Rho Bootis',9,217957,30371,16012,-861,-671,640,47296);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (420,71075,'Seginus','Gamma Bootis',9,218019,38308,8679,-576,-450,577,67263);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (421,71121,'Sigma Lupi','Sigma Lupi',50,218154,-50457,57523,-1201,-943,-1850,318930);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (422,71284,'Sigma Bootis','Sigma Bootis',9,218670,29745,5164,-487,-390,357,45627);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (423,71352,'Eta Centauri','Eta Centauri',18,218877,-42158,30568,-1009,-813,-1173,322775);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (424,71536,'Rho Lupi','Rho Lupi',50,219472,-49426,31604,-893,-735,-1350,320134);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (425,71681,'Alpha2 Centauri','Alpha2 Centauri',18,219896,-60838,409,-76,-63,-177,315730);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (426,71683,'Alpha Centauri','Alpha1 Centauri',18,219902,-60834,432,-78,-65,-181,315734);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (427,71795,'Zeta Bootis','Zeta Bootis',9,220287,13728,17573,-982,-833,315,10804);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (428,71860,'Alpha Lupi','Alpha Lupi',50,220482,-47388,46461,-1110,-947,-1586,321611);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (429,71908,'Alpha Circini','Alpha Circini',22,220627,-64975,5404,-236,-202,-666,314335);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (430,71957,'Mu Virginis','Mu Virginis',84,220765,-5658,5959,-582,-502,-76,346552);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (431,72105,'Izar','Epsilon Bootis',9,221247,27074,20258,-953,-836,648,39385);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (432,72370,'Alpha Apodis','Alpha Apodis',3,221965,-79045,44679,-299,-269,-2075,308493);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (433,72607,'Kochab','Beta Ursae Minoris',82,222676,74156,13093,-230,-212,1101,112648);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (434,72622,'Zubenelgenubi','Alpha2 Librae',49,222720,-16042,7580,-615,-568,-241,340327);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (435,72683,'Omicron Lupi','Omicron Lupi',50,222910,-43575,40416,-1067,-992,-1386,324900);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (436,73273,'Beta Lupi','Beta Lupi',50,224633,-43134,38281,-1016,-1003,-1338,326253);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (437,73334,'Kappa Centauri','Kappa Centauri',18,224790,-42104,38326,-1031,-1023,-1313,326872);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (438,73555,'Nekkar','Beta Bootis',9,225487,40391,22525,-801,-815,973,67620);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (439,73714,'Brachium','Sigma Librae',49,226018,-25282,28838,-1066,-1105,-725,337223);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (440,74117,'Lambda Lupi','Lambda Lupi',50,227211,-45280,77656,-1332,-1439,-1980,326802);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (441,74376,'Kappa1 Lupi','Kappa1 Lupi',50,227984,-48738,18000,-592,-657,-1009,325457);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (442,74395,'Zeta Lupi','Zeta Lupi',50,228071,-52099,11732,-445,-495,-855,323759);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (443,74666,'Delta Bootis','Delta Bootis',9,228876,33315,12179,-607,-695,606,53113);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (444,74785,'Zubeneschamali','Beta Librae',49,229252,-9383,18511,-876,-1017,-222,352023);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (445,74824,'Beta Circini','Beta Circini',22,229379,-58801,9965,-337,-392,-854,320891);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (446,74911,'Mu Lupi','Mu Lupi',50,229633,-47875,33555,-796,-936,-1359,326857);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (447,74946,'Gamma Trianguli Australis','Gamma Trianguli Australis',78,229727,-68680,18385,-319,-376,-1263,315712);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (448,75097,'Pherkad','Gamma Ursae Minoris',82,230182,71834,48680,-440,-528,2096,108463);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (449,75141,'Delta Lupi','Delta Lupi',50,230343,-40648,88389,-1440,-1737,-1937,331322);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (450,75177,'Phi1 Lupi','Phi1 Lupi',50,230452,-36261,27501,-851,-1031,-981,333961);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (451,75264,'Epsilon Lupi','Epsilon Lupi',50,230670,-44690,51202,-1020,-1244,-1591,329228);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (452,75411,'Alkalurops','Mu1 Bootis',9,231123,37377,11313,-531,-658,646,60394);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (453,75458,'Edasich','Iota Draconis',34,231232,58966,10120,-325,-404,862,93971);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (454,75695,'Nusakan','Beta Coronae Borealis',27,231957,29106,11181,-569,-728,514,45588);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (455,76127,'Theta Coronae Borealis','Theta Coronae Borealis',27,233232,31359,37532,-990,-1325,1008,49685);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (456,76267,'Alphecca','Alpha Coronae Borealis',27,233672,26715,7505,-458,-623,389,41871);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (457,76276,'Delta Serpentis','Delta Serpentis',72,233701,10539,22808,-879,-1197,276,17594);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (458,76297,'Gamma Lupi','Gamma Lupi',50,233785,-41167,42085,-912,-1246,-1350,333194);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (459,76333,'Zubenelhakrabi','Gamma Librae',49,233882,-14790,16316,-728,-998,-326,351516);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (460,76440,'Epsilon Trianguli Australis','Epsilon Trianguli Australis',78,234180,-66317,20170,-334,-463,-1301,318490);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (461,76470,'Upsilon Librae','Upsilon Librae',49,234256,-28135,22370,-770,-1070,-705,341885);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (462,76552,'Omega Lupi','Omega Lupi',50,234513,-42567,21860,-632,-887,-1000,332783);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (463,76600,'Tau Librae','Tau Librae',49,234664,-29778,36688,-962,-1356,-951,341065);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (464,76952,'Gamma Coronae Borealis','Gamma Coronae Borealis',27,235686,26296,14606,-611,-895,535,41742);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (465,77055,'Zeta Ursae Minoris','Zeta Ursae Minoris',82,236015,77794,36896,-227,-337,1877,112680);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (466,77070,'Unukalhai','Alpha Serpentis',72,236067,6426,7396,-477,-709,96,14204);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (467,77233,'Beta Serpentis','Beta Serpentis',72,236547,15422,15509,-662,-1002,331,25981);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (468,77257,'Lambda Serpentis','Lambda Serpentis',72,236611,7353,3954,-343,-521,80,15697);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (469,77450,'Gudja','Kappa Serpentis',72,237185,18142,38192,-1006,-1561,609,30099);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (470,77516,'Mu Serpentis','Mu Serpentis',72,237405,-3430,16961,-700,-1095,-78,4594);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (471,77622,'Epsilon Serpentis','Epsilon Serpentis',72,237704,4478,7044,-447,-707,66,13153);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (472,77634,'Chi Lupi','Chi Lupi',50,237740,-33627,19519,-621,-984,-774,340571);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (473,77853,'Theta Librae','Theta Librae',49,238456,-16729,16847,-650,-1059,-374,353546);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (474,77952,'Beta Trianguli Australis','Beta Trianguli Australis',78,238786,-63431,4037,-147,-243,-568,321845);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (475,78072,'Gamma Serpentis','Gamma Serpentis',72,239113,15662,3670,-299,-501,164,27730);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (476,78104,'Iklil','Rho Scorpii',70,239221,-29214,47201,-970,-1629,-1060,344629);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (477,78159,'Epsilon Coronae Borealis','Epsilon Coronae Borealis',27,239397,26878,22142,-676,-1142,673,43654);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (478,78265,'Fang','Pi Scorpii',70,239713,-26114,58556,-1096,-1876,-1065,347215);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (479,78384,'Eta Lupi','Eta Lupi',50,240031,-38397,44195,-823,-1427,-1306,338774);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (480,78401,'Dschubba','Delta Scorpii',70,240083,-22622,49120,-1020,-1773,-853,350097);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (481,78527,'Theta Draconis','Theta Draconis',34,240472,58565,6861,-213,-376,707,90190);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (482,78820,'Acrab','Beta1 Scorpii',70,241359,-19805,40416,-907,-1660,-681,353194);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (483,78918,'Theta Lupi','Theta Lupi',50,241648,-36802,41443,-774,-1435,-1220,340835);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (484,78933,'Omega1 Scorpii','Omega1 Scorpii',70,241702,-20669,47132,-963,-1789,-766,352751);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (485,78990,'Omega2 Scorpii','Omega2 Scorpii',70,241851,-20869,29069,-752,-1405,-607,352697);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (486,79101,'Phi Herculis','Phi Herculis',40,242192,44935,20398,-472,-894,1009,70848);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (487,79374,'Jabbah','Nu Scorpii',70,242999,-19461,47406,-932,-1829,-725,354609);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (488,79593,'Yed Prior','Delta Ophiuchi',57,243586,-3694,17112,-581,-1169,-84,8844);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (489,79664,'Delta Trianguli Australis','Delta Trianguli Australis',78,243859,-63686,60737,-481,-981,-2209,323360);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (490,79882,'Yed Posterior','Epsilon Ophiuchi',57,244580,-4693,10645,-441,-929,-84,8566);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (491,79992,'Tau Herculis','Tau Herculis',40,244935,46313,30740,-513,-1097,1268,72485);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (492,80000,'Gamma2 Normae','Gamma2 Normae',55,244960,-50156,12876,-308,-659,-871,333301);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (493,80112,'Alniyat','Sigma Scorpii',70,245297,-25593,69692,-995,-2163,-1140,351313);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (494,80170,'Gamma Herculis','Gamma Herculis',40,245480,19153,19265,-544,-1193,455,35259);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (495,80331,'Athebyne','Eta Draconis',34,245998,61514,9208,-186,-418,843,92584);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (496,80343,'Psi Ophiuchi','Psi Ophiuchi',57,246026,-20037,19948,-539,-1212,-484,356170);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (497,80569,'Chi Ophiuchi','Chi Ophiuchi',57,246756,-18456,52521,-858,-1997,-726,357933);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (498,80582,'Epsilon Normae','Epsilon Normae',55,246796,-47555,53034,-612,-1428,-1699,336000);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (499,80763,'Antares','Alpha Scorpii',70,247352,-26432,55375,-811,-1945,-1047,351948);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (500,80816,'Kornephoros','Beta Herculis',40,247555,21490,13915,-419,-1014,432,39009);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (501,80883,'Marfik','Lambda Ophiuchi',57,247728,1984,17312,-498,-1217,46,17123);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (502,80894,'Phi Ophiuchi','Phi Ophiuchi',57,247785,-16613,24358,-565,-1385,-446,126);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (503,80975,'Omega Ophiuchi','Omega Ophiuchi',57,248034,-21466,16864,-452,-1121,-475,356300);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (504,81065,'Gamma Apodis','Gamma Apodis',3,248363,-78897,15628,-89,-224,-1227,312688);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (505,81126,'Sigma Herculis','Sigma Herculis',40,248526,42437,31482,-479,-1219,1197,66911);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (506,81266,'Paikauhale','Tau Scorpii',70,248971,-28216,47406,-688,-1791,-1029,351535);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (507,81377,'Zeta Ophiuchi','Zeta Ophiuchi',57,249290,-10567,36606,-665,-1759,-351,6282);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (508,81693,'Zeta Herculis','Zeta Herculis',40,250322,31603,3495,-170,-474,310,52662);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (509,81833,'Eta Herculis','Eta Herculis',40,250724,38922,10865,-268,-765,655,62281);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (510,81852,'Beta Apodis','Beta Apodis',3,250769,-77517,15696,-89,-256,-1223,314174);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (511,82080,'Epsilon Ursae Minoris','Epsilon Ursae Minoris',82,251493,82037,30397,-77,-229,1727,114996);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (512,82273,'Atria','Alpha Trianguli Australis',78,252166,-69028,39061,-217,-673,-1845,321536);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (513,82363,'Eta Arae','Eta Arae',6,252446,-59041,29923,-268,-848,-1483,329636);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (514,82396,'Larawag','Epsilon Scorpii',70,252541,-34293,6371,-198,-629,-450,348812);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (515,82514,'Xamidimura','Mu1 Scorpii',70,252968,-38047,50101,-516,-1685,-1379,346116);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (516,82545,'Pipirima','Mu2 Scorpii',70,253084,-38018,47406,-499,-1641,-1341,346197);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (517,82673,'Iota Ophiuchi','Iota Ophiuchi',57,253502,10165,24523,-438,-1478,276,28734);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (518,82729,'Zeta2 Scorpii','Zeta2 Scorpii',70,253646,-42361,13231,-239,-816,-775,343097);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (519,83000,'Kappa Ophiuchi','Kappa Ophiuchi',57,254417,9375,9146,-253,-909,156,28365);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (520,83081,'Zeta Arae','Zeta Arae',6,254655,-55990,48608,-326,-1189,-1828,332801);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (521,83153,'Epsilon1 Arae','Epsilon1 Arae',6,254896,-53160,36079,-297,-1100,-1520,335147);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (522,83207,'Epsilon Herculis','Epsilon Herculis',40,255072,30926,15502,-275,-1032,640,52853);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (523,83895,'Aldhibah','Zeta Draconis',34,257197,65715,32846,-165,-727,1652,96010);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (524,84012,'Sabik','Eta Ophiuchi',57,257595,-15725,8837,-194,-884,-255,6722);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (525,84143,'Eta Scorpii','Eta Scorpii',70,258038,-43239,7348,-129,-611,-587,344365);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (526,84345,'Rasalgethi','Alpha1 Herculis',40,258662,14390,35960,-361,-1801,471,35534);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (527,84379,'Sarin','Delta Herculis',40,258758,24839,7513,-153,-771,364,46824);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (528,84380,'Pi Herculis','Pi Herculis',40,258762,36809,37662,-303,-1524,1163,60661);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (529,84405,'Guniibuu','HR 6402',57,258837,-26603,1935,-76,-386,-197,358279);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (530,84880,'Nu Serpentis','Nu Serpentis',72,260207,-12847,20321,-236,-1370,-317,10627);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (531,84893,'Xi Ophiuchi','Xi Ophiuchi',57,260252,-21113,5660,-119,-692,-271,3591);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (532,84970,'Theta Ophiuchi','Theta Ophiuchi',57,260502,-25000,43604,-312,-1867,-882,464);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (533,85258,'Beta Arae','Beta Arae',6,261325,-55530,64585,-217,-1422,-2095,335367);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (534,85267,'Gamma Arae','Gamma Arae',6,261349,-56378,111316,-278,-1826,-2778,334645);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (535,85355,'Sigma Ophiuchi','Sigma Ophiuchi',57,261629,4140,90098,-436,-2962,217,26766);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (536,85670,'Rastaban','Beta Draconis',34,262608,52301,38014,-153,-1182,1543,79579);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (537,85693,'Maasym','Lambda Herculis',40,262685,26111,36937,-220,-1712,846,49484);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (538,85696,'Lesath','Upsilon Scorpii',70,262691,-37296,57625,-243,-1894,-1455,351274);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (539,85727,'Delta Arae','Delta Arae',6,262775,-60684,19791,-87,-683,-1227,331270);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (540,85792,'Alpha Arae','Alpha Arae',6,262960,-49876,26734,-129,-1046,-1250,340755);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (541,85822,'Yildun','Delta Ursae Minoris',82,263054,86586,17211,-9,-78,1310,119286);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (542,85927,'Shaula','Lambda Scorpii',70,263402,-37104,57120,-219,-1893,-1442,351744);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (543,86032,'Rasalhague','Alpha Ophiuchi',57,263734,12560,4859,-74,-676,152,35894);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (544,86228,'Sargas','Theta Scorpii',70,264330,-42998,30033,-125,-1261,-1182,347139);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (545,86263,'Xi Serpentis','Xi Serpentis',72,264397,-15399,10528,-97,-984,-272,10606);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (546,86414,'Iota Herculis','Iota Herculis',40,264866,46006,45489,-133,-1475,1534,72322);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (547,86565,'Omicron Serpentis','Omicron Serpentis',72,265354,-12875,17321,-104,-1279,-293,13279);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (548,86670,'Kappa Scorpii','Kappa Scorpii',70,265622,-39030,48319,-130,-1703,-1384,351044);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (549,86742,'Cebalrai','Beta Ophiuchi',57,265868,4567,8185,-65,-899,72,29213);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (550,86929,'Eta Pavonis','Eta Pavonis',59,266433,-64724,35222,-50,-800,-1697,328401);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (551,86974,'Mu Herculis','Mu Herculis',40,266615,27721,2711,-27,-460,242,52439);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (552,87073,'Iota1 Scorpii','Iota1 Scorpii',70,266896,-40127,192992,-182,-3354,-2831,350609);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (553,87108,'Gamma Ophiuchi','Gamma Ophiuchi',57,266973,2707,10279,-53,-1011,48,28016);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (554,87261,'Fuyue','HR 6630',70,267465,-37043,12583,-40,-894,-676,353503);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (555,87585,'Grumium','Xi Draconis',34,268382,56873,11255,-16,-580,888,85167);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (556,87808,'Theta Herculis','Theta Herculis',40,269063,37251,75325,-36,-2184,1661,63257);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (557,87833,'Eltanin','Gamma Draconis',34,269152,51489,15428,-11,-773,972,79055);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (558,87933,'Xi Herculis','Xi Herculis',40,269441,29248,13681,-10,-1020,571,54910);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (559,87937,'Barnard''s Star','GJ 699',57,269452,4693,595,-2,-243,20,31009);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (560,87998,'Nu Herculis','Nu Herculis',40,269626,30189,86057,-17,-2536,1475,55941);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (561,88048,'Nu Ophiuchi','Nu Ophiuchi',57,269757,-9774,15072,-5,-1210,-208,18168);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (562,88635,'Alnasl','Gamma2 Sagittarii',75,271452,-30424,9687,22,-848,-498,919);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (563,88714,'Theta Arae','Theta Arae',6,271658,-50091,81336,53,-1829,-2188,343331);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (564,88794,'Omicron Herculis','Omicron Herculis',40,271886,28762,33799,53,-1611,885,55186);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (565,88866,'Pi Pavonis','Pi Pavonis',59,272145,-63669,12999,19,-505,-1022,330487);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (566,89642,'Eta Sagittarii','Eta Sagittarii',75,274407,-36762,14593,74,-965,-723,356433);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (567,89826,'Kappa Lyrae','Kappa Lyrae',52,274965,36065,25166,111,-1278,934,63521);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (568,89908,'Phi Draconis','Phi Draconis',34,275189,71338,30284,50,-555,1649,101876);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (569,89931,'Kaus Media','Delta Sagittarii',75,275249,-29828,34771,148,-1611,-927,3001);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (570,89937,'Chi Draconis','Chi Draconis',34,275264,72733,2628,14,-152,490,103457);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (571,89962,'Eta Serpentis','Eta Serpentis',72,275328,-2899,6048,72,-773,-39,26896);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (572,90098,'Xi Pavonis','Xi Pavonis',59,275807,-61494,46862,105,-1028,-1902,333285);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (573,90185,'Kaus Australis','Epsilon Sagittarii',75,276043,-34385,14330,104,-982,-676,359194);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (574,90422,'Alpha Telescopii','Alpha Telescopii',77,276743,-45968,27782,136,-1151,-1198,348666);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (575,90496,'Kaus Borealis','Lambda Sagittarii',75,276993,-25422,7818,97,-793,-380,7660);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (576,90568,'Zeta Telescopii','Zeta Telescopii',77,277208,-49071,12622,92,-730,-849,345822);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (577,91117,'Alpha Scuti','Alpha Scuti',71,278802,-8244,19912,214,-1380,-202,23752);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (578,91262,'Vega','Alpha Lyrae',52,279235,38784,2504,63,-385,313,67449);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (579,91792,'Zeta Pavonis','Zeta Pavonis',59,280759,-71428,21846,88,-462,-1401,323483);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (580,91971,'Zeta1 Lyrae','Zeta1 Lyrae',52,281193,37605,15613,192,-971,762,66854);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (581,92041,'Phi Sagittarii','Phi Sagittarii',75,281414,-26991,23929,273,-1351,-702,7996);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (582,92175,'Beta Scuti','Beta Scuti',71,281794,-4748,91617,617,-2953,-251,28221);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (583,92420,'Sheliak','Beta Lyrae',52,282520,33363,96211,562,-2529,1706,63188);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (584,92609,'Lambda Pavonis','Lambda Pavonis',59,283054,-62188,143051,399,-1719,-3345,333611);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (585,92791,'Delta2 Lyrae','Delta2 Lyrae',52,283626,36899,73624,511,-2109,1629,66926);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (586,92855,'Nunki','Sigma Sagittarii',75,283816,-26297,22776,323,-1314,-669,9556);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (587,93015,'Kappa Pavonis','Kappa Pavonis',59,284238,-67233,50024,213,-839,-2062,328287);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (588,93085,'Xi2 Sagittarii','Xi2 Sagittarii',75,284432,-21107,36524,444,-1727,-688,14610);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (589,93194,'Sulafat','Gamma Lyrae',52,284736,32690,62007,533,-2027,1345,63319);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (590,93244,'Epsilon Aquilae','Epsilon Aquilae',4,284906,15068,15494,309,-1162,324,47294);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (591,93506,'Ascella','Zeta Sagittarii',75,285653,-29880,8820,220,-784,-468,6841);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (592,93683,'Omicron Sagittarii','Omicron Sagittarii',75,286171,-21741,14205,308,-1063,-441,14715);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (593,93747,'Okab','Zeta Aquilae',4,286353,13863,8303,249,-849,218,46854);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (594,93805,'Lambda Aquilae','Lambda Aquilae',4,286562,-4883,12368,316,-1062,-95,30259);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (595,93864,'Tau Sagittarii','Tau Sagittarii',75,286735,-27670,12161,281,-935,-512,9335);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (596,94114,'Meridiana','Alpha Coronae Australis',26,287368,-37904,12535,264,-843,-688,359542);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (597,94141,'Albaldah','Pi Sagittarii',75,287441,-21024,50962,632,-2010,-810,15886);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (598,94160,'Beta Coronae Australis','Beta Coronae Australis',26,287507,-39341,47406,507,-1606,-1380,358139);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (599,94376,'Altais','Delta Draconis',34,288139,67662,9742,117,-356,913,98657);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (600,94481,'Aladfar','Eta Lyrae',52,288440,39146,138790,914,-2741,2352,70617);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (601,94648,'Tau Draconis','Tau Draconis',34,288888,73355,14639,112,-328,1159,104732);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (602,94713,'Theta Lyrae','Theta Lyrae',52,289092,38134,83203,742,-2144,1781,69899);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (603,94779,'Kappa Cygni','Kappa Cygni',31,289276,53368,12416,219,-628,894,84402);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (604,95168,'Rho1 Sagittarii','Rho1 Sagittarii',75,290418,-17847,12696,374,-1005,-345,20058);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (605,95241,'Arkab Prior','Beta1 Sagittarii',75,290660,-44459,31361,446,-1183,-1240,353614);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (606,95294,'Arkab Posterior','Beta2 Sagittarii',75,290805,-44800,13417,292,-768,-816,353283);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (607,95347,'Rukbat','Alpha Sagittarii',75,290972,-40616,18180,366,-956,-878,357711);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (608,95501,'Delta Aquilae','Delta Aquilae',4,291375,3115,5064,259,-662,39,39619);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (609,95771,'Anser','Alpha Vulpeculae',86,292176,24665,29732,591,-1451,720,58989);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (610,95853,'Iota2 Cygni','Iota2 Cygni',31,292426,51730,12134,260,-631,865,83612);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (611,95947,'Albireo','Beta1 Cygni',31,292680,27960,43430,710,-1698,977,62110);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (612,96100,'Alsafi','HR 7462',34,293090,69661,1877,59,-139,406,101303);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (613,96229,'Mu Aquilae','Mu Aquilae',4,293522,7379,10761,411,-943,133,44415);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (614,96441,'Theta Cygni','Theta Cygni',31,294111,50221,5980,202,-452,594,82668);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (615,96468,'Iota Aquilae','Iota Aquilae',4,294180,-1287,39107,810,-1804,-44,36987);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (616,96757,'Sham','Alpha Sagittae',74,295024,18014,42524,830,-1777,638,54454);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (617,96837,'Beta Sagittae','Beta Sagittae',74,295262,17476,43956,853,-1809,630,54097);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (618,97165,'Fawaris','Delta Cygni',31,296244,45131,16498,401,-813,910,78710);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (619,97278,'Tarazed','Gamma Aquilae',4,296565,10613,39486,873,-1747,366,48726);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (620,97365,'Delta Sagittae','Delta Sagittae',74,296847,18534,59409,1044,-2062,775,55767);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (621,97433,'Epsilon Draconis','Epsilon Draconis',34,297043,70268,14798,187,-366,1145,102431);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (622,97649,'Altair','Alpha Aquilae',4,297696,8868,1673,188,-358,63,47744);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (623,98032,'Iota Sagittarii','Iota Sagittarii',75,298815,-41868,18180,484,-880,-900,357969);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (624,98036,'Alshain','Beta Aquilae',4,298828,6407,4468,320,-582,75,46126);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (625,98110,'Eta Cygni','Eta Cygni',31,299077,35083,13494,462,-831,668,71016);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (626,98337,'Gamma Sagittae','Gamma Sagittae',74,299689,19492,25844,751,-1317,536,57967);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (627,98412,'Theta1 Sagittarii','Theta1 Sagittarii',75,299934,-35276,51853,928,-1611,-1315,5523);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (628,98495,'Epsilon Pavonis','Epsilon Pavonis',59,300148,-72911,10508,151,-260,-980,322019);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (629,99240,'Delta Pavonis','Delta Pavonis',59,302182,-66182,1992,96,-153,-408,329767);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (630,99255,'Kappa Cephei','Kappa Cephei',19,302222,77711,31482,201,-319,1734,110378);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (631,99473,'Theta Aquilae','Theta Aquilae',4,302826,-821,28635,917,-1422,-24,41580);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (632,100027,'Alpha1 Capricorni','Alpha1 Capricorni',15,304412,-12508,56921,1316,-1922,-517,31089);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (633,100064,'Algedi','Alpha2 Capricorni',15,304514,-12545,10583,569,-827,-223,31097);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (634,100345,'Dabih','Beta Capricorni',15,305253,-14781,32681,1009,-1427,-461,29150);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (635,100453,'Sadr','Gamma Cygni',31,305557,40257,183234,1900,-2658,2766,78149);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (636,100751,'Peacock','Alpha Pavonis',59,306412,-56735,17881,435,-590,-1118,340904);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (637,101093,'Theta Cephei','Theta Cephei',19,307395,62994,13647,322,-421,1041,97759);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (638,101421,'Aldulfin','Epsilon Delphini',32,308303,11303,33045,1105,-1399,356,55419);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (639,101769,'Rotanev','Beta Delphini',32,309387,14595,10088,617,-751,253,58881);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (640,101772,'Alpha Indi','Alpha Indi',44,309392,-47292,9833,427,-520,-729,352566);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (641,101958,'Sualocin','Alpha Delphini',32,309910,15912,25382,983,-1175,437,60304);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (642,102098,'Deneb','Alpha Cygni',31,310358,45280,261577,2330,-2742,3634,84285);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (643,102281,'Delta Delphini','Delta Delphini',32,310865,15075,22324,944,-1091,389,60136);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (644,102395,'Beta Pavonis','Beta Pavonis',59,311240,-66203,13511,309,-353,-1064,328963);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (645,102422,'Eta Cephei','Eta Cephei',19,311322,61839,4653,213,-242,601,97869);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (646,102485,'Psi Capricorni','Psi Capricorni',15,311524,-25271,4787,415,-468,-295,20004);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (647,102488,'Aljanah','Epsilon Cygni',31,311553,33970,7271,469,-529,476,75952);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (648,102532,'Gamma2 Delphini','Gamma2 Delphini',32,311665,16124,12632,718,-807,312,61494);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (649,102618,'Albali','Epsilon Aquarii',5,311919,-9496,20774,950,-1058,-238,37681);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (650,102978,'Omega Capricorni','Omega Capricorni',15,312955,-26919,84278,1764,-1895,-1314,18464);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (651,103227,'Beta Indi','Beta Indi',44,313703,-58454,61193,894,-936,-2108,338158);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (652,103413,'Nu Cygni','Nu Cygni',31,314293,41167,37446,1017,-1043,1274,82909);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (653,104060,'Xi Cygni','Xi Cygni',31,316233,43928,84278,1510,-1446,2014,85930);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (654,104139,'Theta Capricorni','Theta Capricorni',15,316487,-17233,16219,882,-837,-377,31313);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (655,104214,'61 Cygni','61 Cygni',31,316725,38749,1137,191,-180,211,82320);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (656,104732,'Zeta Cygni','Zeta Cygni',31,318234,30227,14311,771,-688,602,76754);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (657,104858,'Delta Equulei','Delta Equulei',35,318620,10007,6030,574,-506,135,60493);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (658,104887,'Tau Cygni','Tau Cygni',31,318698,38045,6635,482,-423,502,82854);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (659,104987,'Kitalpha','Alpha Equulei',35,318956,5248,19029,1036,-902,126,56375);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (660,105102,'Sigma Cygni','Sigma Cygni',31,319354,39395,288634,3150,-2704,3410,84195);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (661,105138,'Upsilon Cygni','Upsilon Cygni',31,319479,34897,64204,1580,-1350,1450,80977);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (662,105199,'Alderamin','Alpha Cephei',19,319645,62586,4905,246,-209,622,101000);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (663,105319,'Theta Indi','Theta Indi',44,319967,-53449,9878,453,-381,-798,343686);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (664,105515,'Iota Capricorni','Iota Capricorni',15,320562,-16835,19672,1037,-853,-406,33625);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (665,105858,'Gamma Pavonis','Gamma Pavonis',59,321611,-65366,3021,180,-142,-500,328157);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (666,105881,'Zeta Capricorni','Zeta Capricorni',15,321667,-22411,38553,1424,-1126,-749,26979);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (667,106032,'Alfirk','Beta Cephei',19,322165,70561,68520,688,-534,2468,107540);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (668,106278,'Sadalsuud','Beta Aquarii',5,322890,-5571,53733,1840,-1392,-225,48023);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (669,106481,'Rho Cygni','Rho Cygni',31,323495,45592,12359,625,-463,794,90742);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (670,106985,'Nashira','Gamma Capricorni',15,325023,-16662,15703,984,-688,-359,35999);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (671,107089,'Nu Octantis','Nu Octantis',56,325369,-77390,6914,149,-103,-811,314289);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (672,107315,'Enif','Epsilon Pegasi',60,326046,9875,68955,2146,-1445,450,65571);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (673,107354,'Kappa Pegasi','Kappa Pegasi',60,326161,25645,11162,791,-530,457,78410);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (674,107380,'Iota Piscis Austrini','Iota Piscis Austrini',64,326237,-33026,20423,996,-666,-779,12875);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (675,107418,'Nu Cephei','Nu Cephei',19,326362,61121,679492,3314,-2205,7218,102310);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (676,107533,'Pi2 Cygni','Pi2 Cygni',31,326698,49310,110561,1812,-1190,2521,94833);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (677,107556,'Deneb Algedi','Delta Capricorni',15,326760,-16127,3870,500,-328,-173,37601);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (678,108085,'Aldhanab','Gamma Gruis',39,328482,-37365,21110,984,-604,-882,6106);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (679,108431,'Delta Indi','Delta Indi',44,329479,-54993,18809,678,-400,-1123,339107);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (680,108870,'Epsilon Indi','Epsilon Indi',44,330840,-56786,1181,164,-92,-288,336192);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (681,108917,'Kurhah','Xi Cephei',19,330948,64628,9652,368,-204,888,106158);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (682,109074,'Sadalmelik','Alpha Aquarii',5,331446,-320,52353,2010,-1094,-13,59929);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (683,109111,'Lambda Gruis','Lambda Gruis',39,331529,-39543,24214,1055,-572,-991,2216);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (684,109139,'Iota Aquarii','Iota Aquarii',5,331609,-13870,17516,1130,-611,-317,43505);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (685,109176,'Iota Pegasi','Iota Pegasi',60,331753,25345,3825,492,-265,265,82260);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (686,109268,'Alnair','Alpha Gruis',39,332058,-46961,10101,606,-321,-735,349998);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (687,109285,'Mu Piscis Austrini','Mu Piscis Austrini',64,332096,-32988,13584,864,-458,-635,13345);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (688,109410,'Pi2 Pegasi','Pi2 Pegasi',60,332497,33178,26303,1204,-627,888,88061);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (689,109427,'Biham','Theta Pegasi',60,332550,6198,9229,848,-440,104,67417);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (690,109492,'Zeta Cephei','Zeta Cephei',19,332714,58201,83630,1354,-699,2458,103063);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (691,109857,'Epsilon Cephei','Epsilon Cephei',19,333759,57044,8545,451,-222,776,102867);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (692,110003,'Ancha','Theta Aquarii',5,334208,-7783,18745,1221,-590,-185,53508);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (693,110130,'Alpha Tucanae','Alpha Tucanae',80,334625,-60260,19973,633,-300,-1227,330221);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (694,110395,'Sadachbia','Gamma Aquarii',5,335414,-1387,16373,1163,-532,-31,62180);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (695,110538,'Beta Lacertae','Beta Lacertae',46,335890,52229,16996,729,-326,1031,101254);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (696,110838,'Delta Tucanae','Delta Tucanae',80,336833,-64966,25089,616,-264,-1435,323874);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (697,110960,'Zeta2 Aquarii','Zeta2 Aquarii',5,337208,-20,9188,884,-371,0,65355);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (698,110991,'Delta Cephei','Delta Cephei',19,337293,58415,86514,1421,-595,2506,105195);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (699,110997,'Delta1 Gruis','Delta1 Gruis',39,337317,-43496,30945,1177,-492,-1211,353772);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (700,111043,'Delta2 Gruis','Delta2 Gruis',39,337439,-43749,33012,1212,-504,-1256,353279);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (701,111169,'Alpha Lacertae','Alpha Lacertae',46,337823,50282,10260,599,-244,779,101260);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (702,111188,'Beta Piscis Austrini','Beta Piscis Austrini',64,337876,-32346,14280,935,-380,-639,14594);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (703,111497,'Eta Aquarii','Eta Aquarii',5,338839,-117,16786,1208,-468,-3,66843);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (704,111954,'Epsilon Piscis Austrini','Epsilon Piscis Austrini',64,340164,-27044,48680,1849,-667,-1003,25486);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (705,112029,'Homam','Zeta Pegasi',60,340366,10831,20436,1322,-472,269,78859);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (706,112122,'Tiaki','Beta Gruis',39,340667,-46885,17697,858,-301,-971,346266);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (707,112158,'Matar','Eta Pegasi',60,340751,30221,21429,1194,-417,737,92496);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (708,112405,'Beta Octantis','Beta Octantis',56,341515,-81382,14927,174,-58,-1208,308352);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (709,112440,'Lambda Pegasi','Lambda Pegasi',60,341633,23566,36524,1663,-552,764,89259);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (710,112447,'Xi Pegasi','Xi Pegasi',60,341673,12173,5315,677,-224,154,81275);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (711,112623,'Epsilon Gruis','Epsilon Gruis',39,342139,-51317,12892,675,-218,-886,338304);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (712,112716,'Tau2 Aquarii','Tau2 Aquarii',5,342398,-13593,31758,1651,-524,-419,52264);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (713,112724,'Iota Cephei','Iota Cephei',19,342420,66200,11529,413,-131,982,111065);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (714,112748,'Sadalbari','Mu Pegasi',60,342501,24602,10610,893,-282,429,90680);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (715,112948,'Gamma Piscis Austrini','Gamma Piscis Austrini',64,343131,-32876,21543,1180,-358,-797,13057);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (716,112961,'Lambda Aquarii','Lambda Aquarii',5,343154,-7580,38507,1862,-564,-259,62192);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (717,113136,'Skat','Delta Aquarii',5,343663,-15821,16059,1170,-343,-345,49579);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (718,113246,'Delta Piscis Austrini','Delta Piscis Austrini',64,343987,-32540,15414,1006,-289,-668,13689);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (719,113368,'Fomalhaut','Alpha Piscis Austrini',64,344413,-29622,2513,420,-117,-248,20488);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (720,113638,'Zeta Gruis','Zeta Gruis',39,345220,-52754,10886,611,-161,-831,334111);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (721,113726,'Omicron Andromedae','Omicron Andromedae',1,345480,42326,68665,1875,-486,1764,102208);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (722,113881,'Scheat','Beta Pegasi',60,345944,28083,19601,1198,-300,659,95749);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (723,113889,'Fumalsamakah','HR 8773',65,345969,3820,40821,1956,-489,135,78794);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (724,113963,'Markab','Alpha Pegasi',60,346190,15205,13334,1082,-266,303,88280);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (725,114131,'Theta Gruis','Theta Gruis',39,346720,-43520,13189,810,-191,-791,348242);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (726,114222,'Pi Cephei','Pi Cephei',19,346974,75387,24671,386,-89,1520,116419);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (727,114421,'Iota Gruis','Iota Gruis',39,347590,-45247,18323,931,-205,-961,344264);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (728,114724,'Phi Aquarii','Phi Aquarii',5,348581,-6049,20208,1386,-280,-150,70941);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (729,114855,'Psi1 Aquarii','Psi1 Aquarii',5,348973,-9088,14982,1186,-231,-193,67072);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (730,114971,'Gamma Piscium','Gamma Piscium',65,349291,3282,13797,1152,-218,67,82482);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (731,114996,'Gamma Tucanae','Gamma Tucanae',80,349357,-58236,7520,449,-84,-737,324300);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (732,115033,'Psi2 Aquarii','Psi2 Aquarii',5,349476,-9183,40167,1945,-361,-320,67617);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (733,115102,'Gamma Sculptoris','Gamma Sculptoris',69,349706,-32532,18221,1120,-203,-726,12250);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (734,115623,'Alkarab','Upsilon Pegasi',60,351345,23404,17041,1184,-180,519,98558);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (735,115830,'Theta Piscium','Theta Piscium',65,351992,6379,14852,1199,-169,135,88805);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (736,116231,'Beta Sculptoris','Beta Sculptoris',69,353243,-37818,17404,1035,-123,-809,355039);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (737,116584,'Lambda Andromedae','Lambda Andromedae',1,354391,46458,8613,636,-62,673,109903);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (738,116631,'Iota Andromedae','Iota Andromedae',1,354534,43268,49947,1620,-155,1532,109034);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (739,116727,'Errai','Gamma Cephei',19,354837,77632,4600,145,-13,662,118992);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (740,116771,'Iota Piscium','Iota Piscium',65,354988,5626,4473,663,-58,66,92465);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (741,116805,'Kappa Andromedae','Kappa Andromedae',1,355102,44334,16838,925,-79,907,109766);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (742,116928,'Lambda Piscium','Lambda Piscium',65,355512,1780,10662,1029,-81,32,90146);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (743,116971,'Omega2 Aquarii','Omega2 Aquarii',5,355681,-14545,14852,1176,-89,-306,67661);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (744,118268,'Omega Piscium','Omega Piscium',65,359828,6863,10434,1014,-3,122,100690);
INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES (745,118322,'Epsilon Tucanae','Epsilon Tucanae',80,359979,-65577,37318,799,0,-1759,311300);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (1,1,30);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (1,30,53);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (4,619,622);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (4,622,624);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (8,148,161);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (8,161,194);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (8,194,195);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (9,410,419);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (9,410,431);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (9,419,420);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (9,420,438);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (9,431,443);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (9,438,443);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (10,206,218);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (10,218,228);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (10,218,230);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (10,226,230);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (10,230,239);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (11,241,247);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (17,2,16);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (17,16,22);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (17,22,33);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (17,33,47);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (19,662,667);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (19,662,690);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (19,667,739);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (19,690,713);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (19,713,739);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (27,454,455);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (27,454,456);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (27,456,464);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (29,354,377);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (29,362,366);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (31,611,635);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (31,618,635);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (31,635,642);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (31,635,647);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (40,508,509);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (40,508,522);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (40,509,528);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (40,522,528);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (47,297,301);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (47,301,308);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (47,303,305);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (47,303,310);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (47,308,310);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (47,310,327);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (47,327,328);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (47,327,343);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (47,328,343);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (52,578,580);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (52,580,583);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (52,580,585);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (52,583,589);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (52,585,589);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (58,160,169);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (58,160,184);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (58,165,169);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (58,165,173);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (58,169,175);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (58,173,189);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (58,175,179);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (58,179,184);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (58,179,189);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (60,4,722);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (60,4,724);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (60,722,724);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (70,478,480);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (70,480,482);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (70,480,493);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (70,493,499);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (70,499,506);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (70,506,514);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (70,514,515);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (70,515,518);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (70,518,525);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (70,525,544);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (70,538,542);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (70,542,548);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (70,544,552);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (70,548,552);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (75,562,573);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (75,569,573);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (75,569,575);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (75,573,591);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (75,575,581);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (75,581,591);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (76,125,126);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (76,125,136);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (76,126,132);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (76,136,177);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (79,45,55);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (79,45,58);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (79,55,58);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (81,323,324);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (81,323,346);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (81,324,355);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (81,346,355);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (81,355,378);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (81,378,389);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (81,389,395);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (82,63,541);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (82,433,448);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (82,433,465);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (82,465,511);
INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES (82,511,541);
-- END GENERATED CONTENT

-- Hand-authored relations to generated definitions.
WITH v(f,t,x,forced) AS (VALUES
  ('nation.romans','bloc.europe','tech.continental_union',1),
  ('nation.greeks','bloc.europe','tech.continental_union',1),
  ('nation.britons','bloc.europe','tech.continental_union',1),
  ('nation.franks','bloc.europe','tech.continental_union',1),
  ('nation.egyptians','bloc.africa','tech.continental_union',1),
  ('nation.nubians','bloc.africa','tech.continental_union',1),
  ('nation.persians','bloc.asia','tech.continental_union',1),
  ('nation.chinese','bloc.asia','tech.continental_union',1),
  ('nation.japanese','bloc.asia','tech.continental_union',1),
  ('nation.indians','bloc.asia','tech.continental_union',1),
  ('nation.mongols','bloc.asia','tech.continental_union',1),
  ('nation.aztecs','bloc.americas','tech.continental_union',1),
  ('nation.inca','bloc.americas','tech.continental_union',1),
  ('nation.maori','bloc.oceania','tech.continental_union',1),
  ('nation.ashscale','race.reptilian','tech.unification',1),
  ('nation.mirefang','race.reptilian','tech.unification',1),
  ('nation.sunstone','race.reptilian','tech.unification',1),
  ('nation.highwind','race.avian','tech.unification',1),
  ('nation.reedmarsh','race.avian','tech.unification',1),
  ('nation.stormcrest','race.avian','tech.unification',1),
  ('bloc.africa','race.human','tech.unification',1), ('bloc.americas','race.human','tech.unification',1),
  ('bloc.asia','race.human','tech.unification',1), ('bloc.europe','race.human','tech.unification',1),
  ('bloc.oceania','race.human','tech.unification',1)
 )
INSERT INTO faction_transition
 SELECT f.id, t.id, x.id, v.forced FROM v JOIN def f ON f.key=v.f JOIN def t ON t.key=v.t JOIN def x ON x.key=v.x;

-- Continental blocs are a human stage: only humans (and their nations) can research the union.
INSERT INTO availability(def_id,faction_id)
 SELECT t.id, f.id FROM def t, def f WHERE t.key='tech.continental_union' AND f.key='race.human';

-- The same building under each race's own name.
INSERT INTO faction_alias(def_id,faction_id,name)
 SELECT x.id, f.id, v.n FROM (
   SELECT 'bld.city' AS x,'race.reptilian' AS f,'Warren' AS n
   UNION ALL SELECT 'bld.city','race.avian','Roost'
   UNION ALL SELECT 'unit.citizen','race.reptilian','Hatchling'
   UNION ALL SELECT 'unit.citizen','race.avian','Fledgling') v
 JOIN def x ON x.key=v.x JOIN def f ON f.key=v.f;

INSERT INTO ability_grant(entity_type_id,ability_id)
 SELECT (SELECT id FROM def WHERE key='unit.priest'), (SELECT id FROM def WHERE key='abl.convert');

-- Opening books. A step asks to have `count` of the subject (queued work counts); every
-- subject must be reachable when its turn comes, so the technology a building waits for is
-- a step of its own before it.
WITH v(p,seq,a,k,n) AS (VALUES
  ('balanced',0,'train','unit.citizen',7), ('balanced',1,'construct','bld.farm',3),
  ('balanced',2,'construct','bld.woodcutter_camp',1), ('balanced',3,'construct','bld.library',1),
  ('balanced',4,'train','unit.citizen',12), ('balanced',5,'research','tech.military.1',1),
  ('balanced',6,'construct','bld.barracks',1),
  ('rusher',0,'train','unit.citizen',6), ('rusher',1,'construct','bld.farm',2),
  ('rusher',2,'construct','bld.woodcutter_camp',1), ('rusher',3,'construct','bld.library',1),
  ('rusher',4,'research','tech.military.1',1), ('rusher',5,'construct','bld.barracks',1),
  ('rusher',6,'train','unit.spearman',6),
  ('builder',0,'train','unit.citizen',8), ('builder',1,'construct','bld.farm',4),
  ('builder',2,'construct','bld.woodcutter_camp',1), ('builder',3,'construct','bld.library',1),
  ('builder',4,'train','unit.citizen',14), ('builder',5,'research','tech.civic.1',1),
  ('builder',6,'found_city','bld.city',2),
  ('turtle',0,'train','unit.citizen',7), ('turtle',1,'construct','bld.farm',3),
  ('turtle',2,'construct','bld.woodcutter_camp',1), ('turtle',3,'construct','bld.library',1),
  ('turtle',4,'research','tech.military.1',1), ('turtle',5,'construct','bld.barracks',1),
  ('turtle',6,'train','unit.spearman',6))
INSERT INTO ai_opening_step(profile_key,seq,action,subject_def_id,count)
 SELECT v.p, v.seq, v.a, d.id, v.n FROM v JOIN def d ON d.key=v.k;

-- ── surface materials, feature objects and crop rows ────────────────────────
-- Up close the renderer shades every land and sea terrain with its material's
-- program; far out it shows the map colour. Material params (flat JSON):
--   colour, colour_alt   two tones of the near look ("#rrggbb")
--   scale                pattern size in tiles
--   base_colour          map colour of a feature terrain
--   shore_material       material of this terrain's tiles that touch water
--   objects              mesh standing on the terrain's tiles
--   edge_objects         mesh on tiles at the edge of the terrain (else `objects`)
--   object_material      material of those meshes (else this one)
--   snow_material        material of object parts above the mesh's snow_line
-- Mesh params are generator inputs in tiles; all of them are baked before play.
-- Defined after the generated content so existing definition ids stay stable.
INSERT INTO def(kind,key,name) VALUES
 ('material','mat.grass','Grass'), ('material','mat.sand','Sand'),
 ('material','mat.shallows','Shallow water'), ('material','mat.deep_water','Deep water'),
 ('material','mat.foliage','Foliage'), ('material','mat.tar_ground','Tar-stained ground'),
 ('material','mat.crop','Crop'),
 ('mesh','mesh.forest_centre','Forest canopy'), ('mesh','mesh.forest_edge','Forest edge'),
 ('mesh','mesh.rock_massif','Rock massif'), ('mesh','mesh.crop_rows','Crop rows');
WITH v(k,prog,params) AS (VALUES
  ('mat.grass','grass','{"colour":"#4f7d2f","colour_alt":"#86a043","scale":0.9,"shore_material":"mat.sand"}'),
  ('mat.sand','sand','{"colour":"#dcc996","colour_alt":"#b8a06a","scale":0.6}'),
  ('mat.shallows','water','{"colour":"#3ea3b2","colour_alt":"#2a7e98","scale":1.0}'),
  ('mat.deep_water','water','{"colour":"#14426f","colour_alt":"#0b2b55","scale":1.0}'),
  ('mat.foliage','foliage','{"colour":"#2e6428","colour_alt":"#6f9636","scale":0.2}'),
  ('mat.tar_ground','oil','{"base_colour":"#171a29","colour":"#24212b","colour_alt":"#05080e","scale":0.8}'),
  ('mat.crop','crop','{"colour":"#6f9a34","colour_alt":"#c8b45c","scale":0.3}'))
INSERT INTO material(id,program,params) SELECT d.id, v.prog, v.params FROM v JOIN def d ON d.key=v.k;
WITH v(k,prog,params) AS (VALUES
  ('mat.forest','forest_floor','{"base_colour":"#2d5a27","colour":"#3b4a24","colour_alt":"#62502e","scale":0.5,"objects":"mesh.forest_centre","edge_objects":"mesh.forest_edge","object_material":"mat.foliage"}'),
  ('mat.mountain','stone','{"base_colour":"#8a8178","colour":"#a19b93","colour_alt":"#6e6861","scale":1.2,"objects":"mesh.rock_massif","snow_material":"mat.ice"}'),
  ('mat.ice','snow','{"base_colour":"#e8eef2"}'))
UPDATE material SET program=(SELECT prog FROM v JOIN def d ON d.key=v.k WHERE d.id=material.id),
 params=(SELECT params FROM v JOIN def d ON d.key=v.k WHERE d.id=material.id)
 WHERE id IN (SELECT d.id FROM v JOIN def d ON d.key=v.k);
WITH v(t,m) AS (VALUES ('ter.plain','mat.grass'), ('ter.coast','mat.shallows'),
  ('ter.deep','mat.deep_water'), ('ter.oil','mat.tar_ground'))
UPDATE terrain SET material_id=(SELECT m.id FROM v JOIN def m ON m.key=v.m JOIN def t ON t.key=v.t WHERE t.id=terrain.id)
 WHERE id IN (SELECT t.id FROM v JOIN def t ON t.key=v.t);
WITH v(k,gen,params) AS (VALUES
  ('mesh.forest_centre','forest_centre','{"variants":4,"count":3,"height":1.35,"radius":0.5}'),
  ('mesh.forest_edge','forest_edge','{"variants":4,"count":3,"height":1.45,"radius":0.32,"trunk_height":0.62,"trunk_colour":"#5b4331"}'),
  ('mesh.rock_massif','rock_massif','{"variants":4,"count":6,"height":4.6,"radius":2.6,"snow_line":6.4}'),
  ('mesh.crop_rows','crop_rows','{"variants":3,"count":2,"height":0.6,"radius":0.5,"blade_width":0.16,"blades":7}'))
INSERT INTO mesh(id,generator,params,baked) SELECT d.id, v.gen, v.params, 1 FROM v JOIN def d ON d.key=v.k;
INSERT INTO entity_visual(entity_type_id,mesh_id,material_id)
 SELECT (SELECT id FROM def WHERE key='bld.farm'), (SELECT id FROM def WHERE key='mesh.crop_rows'),
 (SELECT id FROM def WHERE key='mat.crop');

-- ── composed models of buildings and units ─────────────────────────────────
-- Each model is its own file in db/models/ (<model id>.sql; shared sub-assemblies in
-- db/models/shared/, the materials models wear in db/models/materials.sql), loaded
-- after this seed by `rom build-rules` and the game. See docs/models-in-meshcraft.md.

-- Orbital bombardment (tasks 43 and 75): generated, see orbital_bombardment() in
-- db/content/generate.py.

-- Task 58: the laser battery also fires on ships in the orbit above it, less accurately
-- (`hit_pct`) than at aircraft.
INSERT INTO et_weapon_target(weapon_id,layer,hit_pct)
 SELECT w.id,'orbit',40 FROM et_weapon w JOIN def d ON d.id=w.entity_type_id
 WHERE d.key='unit.laser_battery' AND w.slot=0;

-- How large ships are drawn in the space views (tasks 57 and 72): generated, see
-- VIEW_SCALE in db/content/generate.py.

-- Task 64: the difficulty handicap. A 'handicap' technology is never researched;
-- the host grants it at match setup to a computer player whose difficulty names
-- it, so the income bonus is an ordinary modifier (docs/database.md).
INSERT INTO def(kind,key,name,descr) VALUES
 ('tech','tech.handicap.income_25','Income bonus +25 %','Granted at setup to the toughest computer players.');
INSERT INTO tech(id,kind,research_ticks)
 SELECT id,'handicap',0 FROM def WHERE key='tech.handicap.income_25';
INSERT INTO modifier(source_def_id,stat_key,op,value)
 SELECT d.id, s.key, 'pct', 25 FROM def d JOIN stat s ON s.key LIKE 'income.%'
 WHERE d.key='tech.handicap.income_25' ORDER BY s.key;
UPDATE ai_difficulty SET handicap_tech_id=(SELECT id FROM def WHERE key='tech.handicap.income_25')
 WHERE income_bonus_pct=25;

-- Task 77: ascended players bring life to moons and asteroids (docs/bringing-life.md). Defined
-- after everything else so every earlier definition id (and existing hashes) stays.
-- Every resource body gets a dormant surface before play: a cube of map_style.life_n tiles a
-- face (the smallest planet that holds a few cities) of barren rock without deposits. Brought
-- to life, the body is a habitable planet its ascended player paints with these brushes, free.
INSERT INTO def(kind,key,name) VALUES
 ('terrain','ter.barren','Barren Rock'), ('terrain','ter.desert','Desert'), ('terrain','ter.snow','Snowfield'),
 ('material','mat.barren','Barren rock'), ('material','mat.desert','Desert sand');
WITH v(k,prog,params) AS (VALUES
  ('mat.barren','stone','{"base_colour":"#6f6862","colour":"#7d766e","colour_alt":"#58524c","scale":1.4}'),
  ('mat.desert','sand','{"base_colour":"#d9bf84","colour":"#e2cb93","colour_alt":"#c2a466","scale":0.7}'))
INSERT INTO material(id,program,params) SELECT d.id, v.prog, v.params FROM v JOIN def d ON d.key=v.k;
-- Barren rock, desert and snow are open ground: walked like the plain, but no building stands
-- on them (every building needs the plain), so a body must be painted before it is settled.
WITH v(k,m) AS (VALUES ('ter.barren','mat.barren'), ('ter.desert','mat.desert'), ('ter.snow','mat.ice'))
INSERT INTO terrain(id,layer,buildable,blocks_los,material_id)
 SELECT t.id,'ground',1,0,m.id FROM v JOIN def t ON t.key=v.k JOIN def m ON m.key=v.m;
INSERT INTO movement_class_terrain(movement_class_id,terrain_id,speed_pct)
 SELECT m.movement_class_id, t.id, m.speed_pct FROM movement_class_terrain m
 JOIN def plain ON plain.id=m.terrain_id AND plain.key='ter.plain'
 JOIN def t ON t.key IN ('ter.barren','ter.desert','ter.snow') ORDER BY t.id, m.movement_class_id;
UPDATE map_style SET life_n=64, life_terrain_id=(SELECT id FROM def WHERE key='ter.barren');
INSERT INTO order_kind(key,needs,target,note) VALUES
 ('bring_life','','entity','an ascended player turns a resource body (its marker) into a habitable planet; no executor, no cost'),
 ('paint','','point','an ascended player paints a planet it brought to life with a brush (the subject); no executor, no cost');
INSERT INTO game_constant(key,value,unit,note) VALUES
 ('life.sea_height',100,'height','sea level of a planet brought to life, on the 0..255 height scale'),
 ('life.ground_height',112,'height','height of the barren ground of a dormant surface and of land painted flat');
-- The brushes: land, water, elevation and biomes by hand, sizes in milli-tiles (a dormant
-- surface is 64 tiles a face, 41 tiles in radius); deposits from the rules below.
INSERT INTO def(kind,key,name,sort) VALUES
 ('brush','brush.land.small','Land',10), ('brush','brush.land.large','Land, wide',11),
 ('brush','brush.land.huge','Land, continent',12),
 ('brush','brush.shallows.small','Shallows',20), ('brush','brush.shallows.large','Shallows, wide',21),
 ('brush','brush.deep.small','Deep sea',22), ('brush','brush.deep.large','Deep sea, wide',23),
 ('brush','brush.deep.huge','Ocean',24),
 ('brush','brush.hills.small','Hills',30), ('brush','brush.hills.large','Hills, wide',31),
 ('brush','brush.mountains.small','Mountains',32), ('brush','brush.mountains.large','Mountain range',33),
 ('brush','brush.grass.small','Grass',40), ('brush','brush.grass.large','Grass, wide',41),
 ('brush','brush.forest.small','Forest',42), ('brush','brush.forest.large','Forest, wide',43),
 ('brush','brush.desert.small','Desert',44), ('brush','brush.desert.large','Desert, wide',45),
 ('brush','brush.snow.small','Snow',46), ('brush','brush.snow.large','Snow, wide',47);
WITH v(k,family,t,op,h,r) AS (VALUES
  ('brush.land.small','land','ter.plain','set',112,3000), ('brush.land.large','land','ter.plain','set',112,8000),
  ('brush.land.huge','land','ter.plain','set',112,16000),
  ('brush.shallows.small','water','ter.coast','lower',92,3000), ('brush.shallows.large','water','ter.coast','lower',92,8000),
  ('brush.deep.small','water','ter.deep','lower',50,3000), ('brush.deep.large','water','ter.deep','lower',50,8000),
  ('brush.deep.huge','water','ter.deep','lower',50,16000),
  ('brush.hills.small','elevation',NULL,'raise',165,4000), ('brush.hills.large','elevation',NULL,'raise',165,9000),
  ('brush.mountains.small','elevation','ter.mountain','raise',240,3000),
  ('brush.mountains.large','elevation','ter.mountain','raise',240,7000),
  ('brush.grass.small','biome','ter.plain','keep',0,4000), ('brush.grass.large','biome','ter.plain','keep',0,9000),
  ('brush.forest.small','biome','ter.forest','keep',0,3000), ('brush.forest.large','biome','ter.forest','keep',0,7000),
  ('brush.desert.small','biome','ter.desert','keep',0,4000), ('brush.desert.large','biome','ter.desert','keep',0,9000),
  ('brush.snow.small','biome','ter.snow','keep',0,4000), ('brush.snow.large','biome','ter.snow','keep',0,9000))
INSERT INTO life_brush(id,family,terrain_id,height_op,height,radius)
 SELECT d.id, v.family, t.id, v.op, v.h, v.r FROM v JOIN def d ON d.key=v.k LEFT JOIN def t ON t.key=v.t;
-- Deposits of every kind the rules define, at two sizes: each terrain a gather site needs
-- beside it or stands on (other than the plain every building needs), and each map node a
-- player takes by keeping or carrying it (not the resource bodies, held from orbit).
CREATE TEMP TABLE deposit(terrain_id INTEGER, node_id INTEGER, stem TEXT, name TEXT, sort INTEGER);
INSERT INTO deposit
 SELECT DISTINCT d.id, NULL, substr(d.key, instr(d.key, '.') + 1), d.name, 50 + d.id % 1000 FROM def d
 WHERE d.id IN (SELECT needs_node_terrain_id FROM et_gather_site WHERE needs_node_terrain_id IS NOT NULL
                UNION SELECT p.ref_def_id FROM placement_rule p JOIN et_gather_site g ON g.entity_type_id=p.entity_type_id
                 WHERE p.rule='on_terrain')
   AND d.key <> 'ter.plain';
INSERT INTO deposit
 SELECT NULL, d.id, substr(d.key, instr(d.key, '.') + 1), d.name, 50 + d.id % 1000 FROM def d
 JOIN et_node n ON n.entity_type_id=d.id WHERE n.claim_mode <> 'presence';
INSERT INTO def(kind,key,name,sort)
 SELECT 'brush', 'brush.deposit.' || stem || s.suffix, name || s.label, sort * 2 + s.step FROM deposit
 JOIN (SELECT '.small' AS suffix, '' AS label, 0 AS step UNION ALL SELECT '.large', ', large', 1) s
 WHERE terrain_id IS NOT NULL OR s.step = 0 ORDER BY sort, s.step;
INSERT INTO life_brush(id,family,terrain_id,node_type_id,height_op,height,radius)
 SELECT d.id, 'deposit', p.terrain_id, p.node_id, 'keep', 0,
        CASE WHEN p.node_id IS NOT NULL THEN 500 WHEN d.key LIKE '%.large' THEN 4000 ELSE 1500 END
 FROM deposit p JOIN def d ON d.key LIKE 'brush.deposit.' || p.stem || '.%' ORDER BY d.id;
DROP TABLE deposit;

-- Task 89: map styles from the star catalogue (docs/world-model.md "Map styles"). Many Worlds is
-- shown as Cluster; its key stays, since saves and scripts name it. Solar puts every race's home
-- planet in one system; Quadrant takes every catalogue star in one galactic quadrant, Galaxy every
-- star. Defined last so every earlier definition id stays.
UPDATE def SET name='Cluster' WHERE key='map.many_worlds';
INSERT INTO def(kind,key,name) VALUES
 ('map_style','map.solar','Solar'), ('map_style','map.quadrant','Quadrant'), ('map_style','map.galaxy','Galaxy');
INSERT INTO map_style(id,planets,fluid_pct,space_pct,polar_cap_pct,start_latitude_max_mdeg,polar_terrain_id,params,
  planet_n_min,planet_n_median,planet_n_max,route_extra_pct,life_n,life_terrain_id)
 SELECT d.id, m.planets, m.fluid_pct, m.space_pct, m.polar_cap_pct, m.start_latitude_max_mdeg, m.polar_terrain_id, m.params,
  m.planet_n_min, m.planet_n_median, m.planet_n_max, m.route_extra_pct, m.life_n, m.life_terrain_id
 FROM def d, map_style m JOIN def md ON md.id=m.id AND md.key='map.many_worlds'
 WHERE d.key IN ('map.solar','map.quadrant','map.galaxy') ORDER BY d.id;
-- Solar: every home planet and 1..3 more planets in one system (habitable counts the home, as in a
-- home system of the other styles), and a richer set of resource bodies. Quadrant and Galaxy:
-- the systems beyond the homes are mostly empty: up to two planets and a few resource bodies.
-- star_map_radius bounds the nearest region only; the others span the catalogue (Nu Cephei at 8 243).
WITH v(k,region,h0,h1,r0,r1,radius) AS (VALUES
  ('map.solar',    'single',   2,4, 6,12, 1000),
  ('map.quadrant', 'quadrant', 0,2, 2,6,  8300),
  ('map.galaxy',   'all',      0,2, 2,6,  8300))
UPDATE map_style SET star_region=v.region, neutral_systems_min=0, neutral_systems_max=0,
  habitable_per_system_min=v.h0, habitable_per_system_max=v.h1,
  resource_bodies_per_system_min=v.r0, resource_bodies_per_system_max=v.r1, star_map_radius=v.radius
 FROM v JOIN def d ON d.key=v.k WHERE map_style.id=d.id;
INSERT INTO map_style_feature(map_style_id,feature_def_id,region_kind,pattern,chance_pct,coverage_pct,clumps_min,clumps_max,
  size_min,size_max,start_min_dist,start_max_dist)
 SELECT d.id, f.feature_def_id, f.region_kind, f.pattern, f.chance_pct, f.coverage_pct, f.clumps_min, f.clumps_max,
  f.size_min, f.size_max, f.start_min_dist, f.start_max_dist
 FROM def d, map_style_feature f JOIN def md ON md.id=f.map_style_id AND md.key='map.many_worlds'
 WHERE d.key IN ('map.solar','map.quadrant','map.galaxy') ORDER BY d.id, f.id;

-- Start placement, a setup option for every map style (owner, 2026-11-18): each race on its own
-- home planet (separate, the default), or players of different races together on as few home
-- planets as fairness allows. Read by rom-mapgen.
INSERT INTO match_option VALUES ('start_placement','Start Placement','separate');
INSERT INTO match_option_choice VALUES
 ('start_placement','separate','Separate Worlds',0), ('start_placement','together','Together',1);

COMMIT;
