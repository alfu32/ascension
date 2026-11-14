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
 ('combat.attrition_per_period',60,'hp','enemy territory damage per income period'),
 ('combat.attrition_floor_hp',1,'hp','attrition cannot reduce hp below this'),
 ('victory.capital_hold_ticks',2400,'ticks','continuous enemy capital occupation before defeat'),
 ('unit.respond_range',12000,'milli-tiles','aggressive stance acquisition'),
 ('citizen.auto_work_delay_ticks',200,'ticks','idle workers look for work after this'),
 ('city.spacing',20000,'milli-tiles','minimum distance between cities; below a city border radius so a second city fits inside the first one''s borders'),
 ('city.radius_per_building',400,'milli-tiles','economic radius a city gains per completed own building inside it'),
 ('city.radius_from_buildings_max',8000,'milli-tiles','cap on the economic radius gained from buildings'),
 ('city.assimilate_ticks',2400,'ticks','captured city becomes yours after this'),
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

INSERT INTO layer VALUES
 ('ground','solid surface'), ('fluid','sea'), ('subfluid','underwater'),
 ('air','atmosphere'), ('orbit','near orbit'), ('space','open space between planets');
INSERT INTO body_kind VALUES
 ('star','centre of a star system'), ('habitable','planet with a full surface'),
 ('resource','moon, asteroid or small planet claimed by starship presence');

INSERT INTO map_scale VALUES
 ('planetary','Planetary',0,60), ('interstellar','Interstellar',1,240);

INSERT INTO tag(key,category) VALUES
 ('biological','body'), ('mechanical','body'),
 ('civilian','role'), ('military','role'), ('infantry','role'), ('mounted','role'), ('vehicle','role'),
 ('siege','role'), ('ship','role'), ('aircraft','role'), ('drone','role'), ('starship','role'),
 ('clergy','role'), ('healer','role'), ('city','structure'), ('temple','structure'), ('wonder','structure'),
 ('production','structure'), ('gather','structure');

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
  ('melee','structure',50), ('pierce','structure',20), ('pierce','plate',60), ('bullet','vehicle',40),
  ('explosive','structure',200), ('explosive','flesh',80), ('energy','shield',50), ('bio','flesh',150),
  ('bio','vehicle',25))
INSERT INTO damage_matrix
 SELECT d.id, a.id, v.pct FROM v
 JOIN damage_class d ON d.key=v.dk JOIN armor_class a ON a.key=v.ak;

INSERT INTO movement_class(key,flies) VALUES
 ('foot',0),('wheeled',0),('ship',0),('submarine',0),('aircraft',1),('amphibious',0),('starship',0);

INSERT INTO order_kind(key,needs,target,note) VALUES
 ('stop','','none',''), ('move','movement','entity_or_point','entity target = follow'),
 ('orbit','movement','entity_or_point','body marker or holding point'),
 ('travel','movement','point','target point body = destination system star body; coordinates ignored'),
 ('attack','weapon','entity',''), ('attack_move','weapon','point',''), ('patrol','movement','point',''),
 ('guard','weapon','entity',''), ('explore','movement','none','auto explore'),
 ('construct','builder','def','subject = building type, target point = site'),
 ('repair','builder','entity',''), ('harvest','harvester','entity','target = gather site'),
 ('heal','healer','entity',''), ('trade','trader','entity','target = city'),
 ('carry','movement','entity','target = carry node; brought to the nearest own keeper'),
 ('claim','trader','entity','target = rare resource node'),
 ('enter','movement','entity','garrison / board'), ('unload','container','entity_or_point',''),
 ('train','producer','def',''), ('research','producer','def',''), ('morph','','def','upgrade in place'),
 ('ability','energy','entity_or_point','subject = ability'), ('set_rally','producer','entity_or_point',''),
 ('raze','','none','self-destruct building'), ('disband','','none','');

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
   CASE WHEN sort<=9 THEN 'planetary' ELSE 'interstellar' END
 FROM def WHERE kind='age';
WITH v(k,r) AS (VALUES ('age.ancient',30000), ('age.classical',36000), ('age.medieval',42000),
  ('age.gunpowder',48000), ('age.enlightenment',55000), ('age.industrial',62000),
  ('age.modern',70000), ('age.information',78000), ('age.automation',110000))
UPDATE age SET view_range = (SELECT v.r FROM v JOIN def d ON d.key = v.k WHERE d.id = age.id)
 WHERE id IN (SELECT d.id FROM v JOIN def d ON d.key = v.k);

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
  ('ter.forest','planet','per_player',1,2,5,9,2,6), ('ter.mountain','planet','per_player',1,1,2,3,5,8),
  ('node.relics','planet','scattered',2,4,1,1,10,0),
  ('node.ore_planet','space','scattered',6,10,1,1,0,0), ('ter.nebula','space','scattered',2,3,6,12,0,0))
INSERT INTO map_style_feature(map_style_id,feature_def_id,region_kind,pattern,clumps_min,clumps_max,size_min,size_max,start_min_dist,start_max_dist)
 SELECT (SELECT id FROM def WHERE key='map.one_world'), d.id, v.rk, v.p, v.c0, v.c1, v.s0, v.s1, v.d0, v.d1 FROM v JOIN def d ON d.key=v.k;
INSERT INTO map_style_feature(map_style_id,feature_def_id,region_kind,pattern,clumps_min,clumps_max,size_min,size_max,start_min_dist,start_max_dist)
 SELECT (SELECT id FROM def WHERE key='map.many_worlds'), feature_def_id, region_kind, pattern, clumps_min, clumps_max, size_min, size_max, start_min_dist, start_max_dist
 FROM map_style_feature WHERE map_style_id=(SELECT id FROM def WHERE key='map.one_world') ORDER BY id;

WITH v(k,pct) AS (VALUES ('ter.forest',24), ('ter.mountain',7))
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
INSERT INTO ai_difficulty(key,name,rank,think_interval_ticks,orders_per_minute,reaction_ticks,mistake_pct,uses_formations,uses_abilities,retreats,income_bonus_pct,sees_all) VALUES
 ('easiest','Easiest',0,100,20,200,30,0,0,0,0,0),
 ('easy','Easy',1,60,40,120,20,0,0,1,0,0),
 ('moderate','Moderate',2,40,80,60,10,1,1,1,0,0),
 ('tough','Tough',3,20,160,30,3,1,1,1,0,0),
 ('tougher','Tougher',4,10,300,10,0,1,1,1,0,0),
 ('toughest','Toughest',5,10,300,10,0,1,1,1,25,0);

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
INSERT INTO tag(key,category) VALUES ('building','structure');
INSERT INTO tag(key,category) VALUES ('fort','structure');
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
INSERT INTO def(kind,key,name) VALUES ('entity_type','bld.auto_university','Automated University');
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.city'),'building','economic',1200,3,(SELECT id FROM armor_class WHERE key='structure'),12000,7,7,150,'per_type','none',0,(SELECT id FROM def WHERE key='age.ancient'));
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
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='bld.city'),8,(SELECT id FROM damage_class WHERE key='pierce'),10000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.city')),'ground');
INSERT INTO et_container(entity_type_id,capacity,heals,adds_firepower) VALUES ((SELECT id FROM def WHERE key='bld.city'),10,1,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.farm'),'building','economic',400,3,(SELECT id FROM armor_class WHERE key='structure'),12000,4,4,150,'per_type','per_city',5,(SELECT id FROM def WHERE key='age.ancient'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.farm'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.farm'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.farm'),(SELECT id FROM def WHERE key='res.timber'),50,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.farm'),'construct','economic',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.farm'),'construct','economic',1,0);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.farm'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.farm'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.farm'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_max,slots_per_node,node_radius,needs_node_terrain_id,self_income) VALUES ((SELECT id FROM def WHERE key='bld.farm'),(SELECT id FROM def WHERE key='res.food'),1,1,0,0,NULL,0);
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.farm'),(SELECT id FROM tag WHERE key='gather'));
INSERT INTO completion_bonus(entity_type_id,resource_id,amount) VALUES ((SELECT id FROM def WHERE key='bld.farm'),(SELECT id FROM def WHERE key='res.food'),20);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.woodcutter_camp'),'building','economic',500,3,(SELECT id FROM armor_class WHERE key='structure'),12000,4,4,150,'per_type','none',0,(SELECT id FROM def WHERE key='age.ancient'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.woodcutter_camp'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.woodcutter_camp'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.woodcutter_camp'),(SELECT id FROM def WHERE key='res.timber'),60,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.woodcutter_camp'),'construct','economic',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.woodcutter_camp'),'construct','economic',2,0);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.woodcutter_camp'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.woodcutter_camp'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.woodcutter_camp'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_max,slots_per_node,node_radius,needs_node_terrain_id,self_income) VALUES ((SELECT id FROM def WHERE key='bld.woodcutter_camp'),(SELECT id FROM def WHERE key='res.timber'),0,10,1,6000,(SELECT id FROM def WHERE key='ter.forest'),0);
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.woodcutter_camp'),(SELECT id FROM tag WHERE key='gather'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.library'),'building','economic',800,3,(SELECT id FROM armor_class WHERE key='structure'),12000,4,4,150,'per_type','per_city',1,(SELECT id FROM def WHERE key='age.ancient'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='res.timber'),60,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.library'),'construct','economic',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.library'),'construct','economic',3,0);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.library'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.library'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.library'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.market'),'building','economic',800,3,(SELECT id FROM armor_class WHERE key='structure'),12000,4,4,150,'per_type','per_city',1,(SELECT id FROM def WHERE key='age.ancient'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.market'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.market'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.market'),(SELECT id FROM def WHERE key='res.timber'),80,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.market'),'construct','economic',4,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.market'),'construct','economic',4,0);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.market'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.market'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.market'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO completion_bonus(entity_type_id,resource_id,amount) VALUES ((SELECT id FROM def WHERE key='bld.market'),(SELECT id FROM def WHERE key='res.wealth'),20);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.granary'),'building','economic',880,3,(SELECT id FROM armor_class WHERE key='structure'),12000,4,4,175,'per_type','per_city',1,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.granary'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.granary'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.granary'),(SELECT id FROM def WHERE key='res.timber'),80,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.granary'),'construct','economic',0,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.granary'),'construct','economic',0,1);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.granary'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.granary'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.granary'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.lumber_mill'),'building','economic',880,3,(SELECT id FROM armor_class WHERE key='structure'),12000,4,4,175,'per_type','per_city',1,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.lumber_mill'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.lumber_mill'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.lumber_mill'),(SELECT id FROM def WHERE key='res.timber'),80,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.lumber_mill'),'construct','economic',1,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.lumber_mill'),'construct','economic',1,1);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.lumber_mill'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.lumber_mill'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.lumber_mill'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.mine'),'building','economic',660,3,(SELECT id FROM armor_class WHERE key='structure'),12000,4,4,175,'per_type','none',0,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.mine'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.mine'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.mine'),(SELECT id FROM def WHERE key='res.timber'),80,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.mine'),'construct','economic',2,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.mine'),'construct','economic',2,1);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.mine'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.mine'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.mine'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_max,slots_per_node,node_radius,needs_node_terrain_id,self_income) VALUES ((SELECT id FROM def WHERE key='bld.mine'),(SELECT id FROM def WHERE key='res.metal'),0,10,1,6000,(SELECT id FROM def WHERE key='ter.mountain'),0);
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.mine'),(SELECT id FROM tag WHERE key='gather'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.university'),'building','economic',880,3,(SELECT id FROM armor_class WHERE key='structure'),12000,4,4,175,'per_type','per_city',1,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM def WHERE key='res.timber'),100,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.university'),'construct','economic',3,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.university'),'construct','economic',3,1);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.university'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.university'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.university'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_max,slots_per_node,node_radius,needs_node_terrain_id,self_income) VALUES ((SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM def WHERE key='res.knowledge'),7,7,0,0,NULL,0);
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM tag WHERE key='gather'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.temple'),'building','economic',880,3,(SELECT id FROM armor_class WHERE key='structure'),12000,4,4,175,'per_type','per_city',1,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.temple'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.temple'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.temple'),(SELECT id FROM def WHERE key='res.timber'),80,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.temple'),'construct','economic',4,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.temple'),'construct','economic',4,1);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.temple'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.temple'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.temple'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.temple'),(SELECT id FROM tag WHERE key='temple'));
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) VALUES ((SELECT id FROM def WHERE key='bld.temple'),0,2000,0,0);
INSERT INTO et_keeper(entity_type_id,capacity) VALUES ((SELECT id FROM def WHERE key='bld.temple'),5);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.smelter'),'building','economic',1200,3,(SELECT id FROM armor_class WHERE key='structure'),12000,4,4,200,'per_type','per_city',1,(SELECT id FROM def WHERE key='age.medieval'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.smelter'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.smelter'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.smelter'),(SELECT id FROM def WHERE key='res.timber'),120,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.smelter'),(SELECT id FROM def WHERE key='res.metal'),60,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.smelter'),'construct','economic',0,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.smelter'),'construct','economic',0,2);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.smelter'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.smelter'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.smelter'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.oil_well'),'building','economic',900,3,(SELECT id FROM armor_class WHERE key='structure'),12000,2,2,275,'per_type','none',0,(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.oil_well'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.oil_well'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.oil_well'),(SELECT id FROM def WHERE key='res.timber'),120,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.oil_well'),(SELECT id FROM def WHERE key='res.metal'),60,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.oil_well'),'construct','economic',1,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.oil_well'),'construct','economic',1,2);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.oil_well'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.oil_well'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.oil_well'),'on_terrain',(SELECT id FROM def WHERE key='ter.oil'));
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_max,slots_per_node,node_radius,needs_node_terrain_id,self_income) VALUES ((SELECT id FROM def WHERE key='bld.oil_well'),(SELECT id FROM def WHERE key='res.oil'),3,3,0,0,NULL,0);
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.oil_well'),(SELECT id FROM tag WHERE key='gather'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.refinery'),'building','economic',1800,3,(SELECT id FROM armor_class WHERE key='structure'),12000,4,4,275,'per_type','per_city',1,(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.refinery'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.refinery'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.refinery'),(SELECT id FROM def WHERE key='res.timber'),160,3);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.refinery'),(SELECT id FROM def WHERE key='res.metal'),80,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.refinery'),'construct','economic',2,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.refinery'),'construct','economic',2,2);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.refinery'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.refinery'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.refinery'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),'building','economic',3420,3,(SELECT id FROM armor_class WHERE key='structure'),12000,4,4,375,'per_type','per_city',1,(SELECT id FROM def WHERE key='age.fusion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),(SELECT id FROM def WHERE key='res.timber'),200,4);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),(SELECT id FROM def WHERE key='res.metal'),100,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.fusion_plant'),'construct','economic',3,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.fusion_plant'),'construct','economic',3,2);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),'in_city_radius');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_max,slots_per_node,node_radius,needs_node_terrain_id,self_income) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),(SELECT id FROM def WHERE key='res.energy'),0,0,0,0,NULL,40);
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),(SELECT id FROM tag WHERE key='gather'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),'building','military',1000,3,(SELECT id FROM armor_class WHERE key='structure'),12000,4,4,150,'per_type','none',0,(SELECT id FROM def WHERE key='age.ancient'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='res.timber'),60,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.barracks'),'construct','military',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.barracks'),'construct','military',0,0);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.barracks'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.tower'),'building','military',1000,3,(SELECT id FROM armor_class WHERE key='structure'),12000,2,2,150,'per_type','none',0,(SELECT id FROM def WHERE key='age.ancient'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.tower'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.tower'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.tower'),(SELECT id FROM def WHERE key='res.timber'),80,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.tower'),'construct','military',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.tower'),'construct','military',1,0);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.tower'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.tower'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.tower'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='bld.tower'),8,(SELECT id FROM damage_class WHERE key='pierce'),10000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.tower')),'ground');
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.dock'),'building','military',1000,3,(SELECT id FROM armor_class WHERE key='structure'),12000,4,4,150,'per_type','none',0,(SELECT id FROM def WHERE key='age.ancient'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='res.timber'),100,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.dock'),'construct','military',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.dock'),'construct','military',2,0);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.dock'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.dock'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.dock'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id,distance) VALUES ((SELECT id FROM def WHERE key='bld.dock'),'adjacent_terrain',(SELECT id FROM def WHERE key='ter.coast'),2000);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.stable'),'building','military',1100,3,(SELECT id FROM armor_class WHERE key='structure'),12000,4,4,175,'per_type','none',0,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM def WHERE key='res.timber'),100,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.stable'),'construct','military',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.stable'),'construct','military',3,0);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.stable'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.stable'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.stable'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),'building','military',1320,3,(SELECT id FROM armor_class WHERE key='structure'),12000,4,4,175,'per_type','none',0,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),(SELECT id FROM def WHERE key='res.timber'),120,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.siege_workshop'),'construct','military',4,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.siege_workshop'),'construct','military',4,0);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.fort'),'building','military',2640,3,(SELECT id FROM armor_class WHERE key='structure'),12000,6,6,200,'per_type','none',0,(SELECT id FROM def WHERE key='age.medieval'));
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
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='bld.fort'),12,(SELECT id FROM damage_class WHERE key='pierce'),10000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fort')),'ground');
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.factory'),'building','military',2700,3,(SELECT id FROM armor_class WHERE key='structure'),12000,4,4,275,'per_type','none',0,(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='res.timber'),200,4);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='res.metal'),100,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.factory'),'construct','military',1,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.factory'),'construct','military',1,1);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.factory'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.factory'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.factory'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),'building','military',2700,3,(SELECT id FROM armor_class WHERE key='structure'),12000,4,4,275,'per_type','none',0,(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='res.timber'),200,4);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='res.metal'),100,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.vehicle_plant'),'construct','military',2,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.vehicle_plant'),'construct','military',2,1);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),'building','military',2700,3,(SELECT id FROM armor_class WHERE key='structure'),12000,6,6,275,'per_type','none',0,(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='res.timber'),200,4);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='res.metal'),100,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.airfield'),'construct','military',3,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.airfield'),'construct','military',3,1);
INSERT INTO et_producer(entity_type_id) VALUES ((SELECT id FROM def WHERE key='bld.airfield'));
INSERT INTO placement_rule(entity_type_id,rule) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),'in_own_territory');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),'on_terrain',(SELECT id FROM def WHERE key='ter.plain'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,ramp_mode,limit_scope,limit_count,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),'building','military',5000,3,(SELECT id FROM armor_class WHERE key='structure'),12000,8,8,400,'per_type','none',0,(SELECT id FROM def WHERE key='age.starfaring'));
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,first_age_id,upgrades_from_id) VALUES ((SELECT id FROM def WHERE key='bld.bastion'),'building','military',2100,5,(SELECT id FROM armor_class WHERE key='structure'),14000,2,2,250,(SELECT id FROM def WHERE key='age.enlightenment'),(SELECT id FROM def WHERE key='bld.tower'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.bastion'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.bastion'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO et_producer(entity_type_id,queue_limit,parallel,has_rally) SELECT (SELECT id FROM def WHERE key='bld.bastion'),queue_limit,parallel,has_rally FROM et_producer WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.tower');
INSERT INTO et_keeper(entity_type_id,capacity) SELECT (SELECT id FROM def WHERE key='bld.bastion'),capacity FROM et_keeper WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.tower');
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) SELECT (SELECT id FROM def WHERE key='bld.bastion'),is_city,border_radius,economic_radius,capturable FROM et_territory WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.tower');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id,layer,distance) SELECT (SELECT id FROM def WHERE key='bld.bastion'),rule,ref_def_id,layer,distance FROM placement_rule WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.tower');
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income) SELECT (SELECT id FROM def WHERE key='bld.bastion'),resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income FROM et_gather_site WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.tower');
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='bld.bastion'),20,(SELECT id FROM damage_class WHERE key='explosive'),10000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bastion')),'ground');
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.bastion','Upgrade to Bastion');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.bastion'),'upgrade','',0,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.bastion'),(SELECT id FROM def WHERE key='res.timber'),300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.bastion'),(SELECT id FROM def WHERE key='res.metal'),150,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.bastion'),'upgrade_type',(SELECT id FROM def WHERE key='bld.tower'),(SELECT id FROM def WHERE key='bld.bastion'));
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.bastion'),'construct','military',0,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.bastion'),'construct','military',0,2);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,first_age_id,upgrades_from_id) VALUES ((SELECT id FROM def WHERE key='bld.bunker'),'building','military',2400,5,(SELECT id FROM armor_class WHERE key='structure'),14000,2,2,300,(SELECT id FROM def WHERE key='age.modern'),(SELECT id FROM def WHERE key='bld.bastion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.bunker'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.bunker'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO et_producer(entity_type_id,queue_limit,parallel,has_rally) SELECT (SELECT id FROM def WHERE key='bld.bunker'),queue_limit,parallel,has_rally FROM et_producer WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bastion');
INSERT INTO et_keeper(entity_type_id,capacity) SELECT (SELECT id FROM def WHERE key='bld.bunker'),capacity FROM et_keeper WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bastion');
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) SELECT (SELECT id FROM def WHERE key='bld.bunker'),is_city,border_radius,economic_radius,capturable FROM et_territory WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bastion');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id,layer,distance) SELECT (SELECT id FROM def WHERE key='bld.bunker'),rule,ref_def_id,layer,distance FROM placement_rule WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bastion');
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income) SELECT (SELECT id FROM def WHERE key='bld.bunker'),resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income FROM et_gather_site WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bastion');
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='bld.bunker'),24,(SELECT id FROM damage_class WHERE key='explosive'),10000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bunker')),'ground');
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.bunker','Upgrade to Bunker');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.bunker'),'upgrade','',0,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.bunker'),(SELECT id FROM def WHERE key='res.timber'),300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.bunker'),(SELECT id FROM def WHERE key='res.metal'),150,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.bunker'),'upgrade_type',(SELECT id FROM def WHERE key='bld.bastion'),(SELECT id FROM def WHERE key='bld.bunker'));
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.bunker'),'construct','military',1,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.bunker'),'construct','military',1,2);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,first_age_id,upgrades_from_id) VALUES ((SELECT id FROM def WHERE key='bld.defence_turret'),'building','military',2700,5,(SELECT id FROM armor_class WHERE key='structure'),14000,2,2,350,(SELECT id FROM def WHERE key='age.automation'),(SELECT id FROM def WHERE key='bld.bunker'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.defence_turret'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.defence_turret'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO et_producer(entity_type_id,queue_limit,parallel,has_rally) SELECT (SELECT id FROM def WHERE key='bld.defence_turret'),queue_limit,parallel,has_rally FROM et_producer WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bunker');
INSERT INTO et_keeper(entity_type_id,capacity) SELECT (SELECT id FROM def WHERE key='bld.defence_turret'),capacity FROM et_keeper WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bunker');
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) SELECT (SELECT id FROM def WHERE key='bld.defence_turret'),is_city,border_radius,economic_radius,capturable FROM et_territory WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bunker');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id,layer,distance) SELECT (SELECT id FROM def WHERE key='bld.defence_turret'),rule,ref_def_id,layer,distance FROM placement_rule WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bunker');
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income) SELECT (SELECT id FROM def WHERE key='bld.defence_turret'),resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income FROM et_gather_site WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.bunker');
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='bld.defence_turret'),28,(SELECT id FROM damage_class WHERE key='energy'),10000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.defence_turret')),'ground');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.defence_turret')),'air');
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.defence_turret','Upgrade to Defence Turret');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.defence_turret'),'upgrade','',0,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.defence_turret'),(SELECT id FROM def WHERE key='res.timber'),300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.defence_turret'),(SELECT id FROM def WHERE key='res.metal'),150,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.defence_turret'),'upgrade_type',(SELECT id FROM def WHERE key='bld.bunker'),(SELECT id FROM def WHERE key='bld.defence_turret'));
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.defence_turret'),'construct','military',2,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.defence_turret'),'construct','military',2,2);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,first_age_id,upgrades_from_id) VALUES ((SELECT id FROM def WHERE key='bld.star_fort'),'building','military',2100,5,(SELECT id FROM armor_class WHERE key='structure'),14000,6,6,250,(SELECT id FROM def WHERE key='age.enlightenment'),(SELECT id FROM def WHERE key='bld.fort'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.star_fort'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.star_fort'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.star_fort'),(SELECT id FROM tag WHERE key='fort'));
INSERT INTO et_producer(entity_type_id,queue_limit,parallel,has_rally) SELECT (SELECT id FROM def WHERE key='bld.star_fort'),queue_limit,parallel,has_rally FROM et_producer WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fort');
INSERT INTO et_keeper(entity_type_id,capacity) SELECT (SELECT id FROM def WHERE key='bld.star_fort'),capacity FROM et_keeper WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fort');
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) SELECT (SELECT id FROM def WHERE key='bld.star_fort'),is_city,border_radius,economic_radius,capturable FROM et_territory WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fort');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id,layer,distance) SELECT (SELECT id FROM def WHERE key='bld.star_fort'),rule,ref_def_id,layer,distance FROM placement_rule WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fort');
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income) SELECT (SELECT id FROM def WHERE key='bld.star_fort'),resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income FROM et_gather_site WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fort');
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='bld.star_fort'),20,(SELECT id FROM damage_class WHERE key='explosive'),10000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.star_fort')),'ground');
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.star_fort','Upgrade to Star Fort');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.star_fort'),'upgrade','',0,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.star_fort'),(SELECT id FROM def WHERE key='res.timber'),300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.star_fort'),(SELECT id FROM def WHERE key='res.metal'),150,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.star_fort'),'upgrade_type',(SELECT id FROM def WHERE key='bld.fort'),(SELECT id FROM def WHERE key='bld.star_fort'));
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.star_fort'),'construct','military',3,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.star_fort'),'construct','military',3,2);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,first_age_id,upgrades_from_id) VALUES ((SELECT id FROM def WHERE key='bld.fortress'),'building','military',2400,5,(SELECT id FROM armor_class WHERE key='structure'),14000,6,6,300,(SELECT id FROM def WHERE key='age.modern'),(SELECT id FROM def WHERE key='bld.star_fort'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.fortress'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.fortress'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.fortress'),(SELECT id FROM tag WHERE key='fort'));
INSERT INTO et_producer(entity_type_id,queue_limit,parallel,has_rally) SELECT (SELECT id FROM def WHERE key='bld.fortress'),queue_limit,parallel,has_rally FROM et_producer WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.star_fort');
INSERT INTO et_keeper(entity_type_id,capacity) SELECT (SELECT id FROM def WHERE key='bld.fortress'),capacity FROM et_keeper WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.star_fort');
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) SELECT (SELECT id FROM def WHERE key='bld.fortress'),is_city,border_radius,economic_radius,capturable FROM et_territory WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.star_fort');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id,layer,distance) SELECT (SELECT id FROM def WHERE key='bld.fortress'),rule,ref_def_id,layer,distance FROM placement_rule WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.star_fort');
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income) SELECT (SELECT id FROM def WHERE key='bld.fortress'),resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income FROM et_gather_site WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.star_fort');
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='bld.fortress'),24,(SELECT id FROM damage_class WHERE key='explosive'),10000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fortress')),'ground');
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.fortress','Upgrade to Fortress');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.fortress'),'upgrade','',0,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.fortress'),(SELECT id FROM def WHERE key='res.timber'),300,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.fortress'),(SELECT id FROM def WHERE key='res.metal'),150,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.fortress'),'upgrade_type',(SELECT id FROM def WHERE key='bld.star_fort'),(SELECT id FROM def WHERE key='bld.fortress'));
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.fortress'),'construct','military',4,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.fortress'),'construct','military',4,2);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,first_age_id,upgrades_from_id) VALUES ((SELECT id FROM def WHERE key='bld.citadel'),'building','military',2850,5,(SELECT id FROM armor_class WHERE key='structure'),14000,6,6,375,(SELECT id FROM def WHERE key='age.fusion'),(SELECT id FROM def WHERE key='bld.fortress'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.citadel'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.citadel'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.citadel'),(SELECT id FROM tag WHERE key='fort'));
INSERT INTO et_producer(entity_type_id,queue_limit,parallel,has_rally) SELECT (SELECT id FROM def WHERE key='bld.citadel'),queue_limit,parallel,has_rally FROM et_producer WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fortress');
INSERT INTO et_keeper(entity_type_id,capacity) SELECT (SELECT id FROM def WHERE key='bld.citadel'),capacity FROM et_keeper WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fortress');
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) SELECT (SELECT id FROM def WHERE key='bld.citadel'),is_city,border_radius,economic_radius,capturable FROM et_territory WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fortress');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id,layer,distance) SELECT (SELECT id FROM def WHERE key='bld.citadel'),rule,ref_def_id,layer,distance FROM placement_rule WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fortress');
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income) SELECT (SELECT id FROM def WHERE key='bld.citadel'),resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income FROM et_gather_site WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.fortress');
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='bld.citadel'),30,(SELECT id FROM damage_class WHERE key='energy'),10000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.citadel')),'ground');
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.musketeer'),'unit','infantry',145,2,(SELECT id FROM armor_class WHERE key='plate'),5000,1,225,'per_type',(SELECT id FROM def WHERE key='unit.pikeman'),(SELECT id FROM def WHERE key='age.gunpowder'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.musketeer'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.musketeer'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.musketeer'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.musketeer'),(SELECT id FROM movement_class WHERE key='foot'),1250);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.musketeer'),13,(SELECT id FROM damage_class WHERE key='bullet'),5000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.musketeer')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.musketeer'),'train','units',3,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.musketeer'),(SELECT id FROM def WHERE key='res.food'),58,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.musketeer'),(SELECT id FROM def WHERE key='res.metal'),29,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.musketeer','Upgrade to Musketeers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.musketeer'),'unit_upgrade','',0,450);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.musketeer'),(SELECT id FROM def WHERE key='res.food'),116,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.musketeer'),(SELECT id FROM def WHERE key='res.metal'),58,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.musketeer'),'upgrade_type',(SELECT id FROM def WHERE key='unit.pikeman'),(SELECT id FROM def WHERE key='unit.musketeer'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.fusilier'),'unit','infantry',160,2,(SELECT id FROM armor_class WHERE key='plate'),5000,1,250,'per_type',(SELECT id FROM def WHERE key='unit.musketeer'),(SELECT id FROM def WHERE key='age.enlightenment'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.fusilier'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.fusilier'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.fusilier'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.fusilier'),(SELECT id FROM movement_class WHERE key='foot'),1250);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.fusilier'),14,(SELECT id FROM damage_class WHERE key='bullet'),5000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.fusilier')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.fusilier'),'train','units',4,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.fusilier'),(SELECT id FROM def WHERE key='res.food'),64,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.fusilier'),(SELECT id FROM def WHERE key='res.metal'),32,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.fusilier','Upgrade to Fusiliers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.fusilier'),'unit_upgrade','',0,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.fusilier'),(SELECT id FROM def WHERE key='res.food'),128,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.fusilier'),(SELECT id FROM def WHERE key='res.metal'),64,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.fusilier'),'upgrade_type',(SELECT id FROM def WHERE key='unit.musketeer'),(SELECT id FROM def WHERE key='unit.fusilier'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.rifleman'),'unit','infantry',175,2,(SELECT id FROM armor_class WHERE key='plate'),5000,1,275,'per_type',(SELECT id FROM def WHERE key='unit.fusilier'),(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.rifleman'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.rifleman'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.rifleman'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.rifleman'),(SELECT id FROM movement_class WHERE key='foot'),1250);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.rifleman'),15,(SELECT id FROM damage_class WHERE key='bullet'),5000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.rifleman')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.rifleman'),'train','units',0,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.rifleman'),(SELECT id FROM def WHERE key='res.food'),70,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.rifleman'),(SELECT id FROM def WHERE key='res.metal'),35,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.rifleman','Upgrade to Riflemen');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.rifleman'),'unit_upgrade','',0,550);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.rifleman'),(SELECT id FROM def WHERE key='res.food'),140,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.rifleman'),(SELECT id FROM def WHERE key='res.metal'),70,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.rifleman'),'upgrade_type',(SELECT id FROM def WHERE key='unit.fusilier'),(SELECT id FROM def WHERE key='unit.rifleman'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.infantry'),'unit','infantry',190,2,(SELECT id FROM armor_class WHERE key='plate'),5000,1,300,'per_type',(SELECT id FROM def WHERE key='unit.rifleman'),(SELECT id FROM def WHERE key='age.modern'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.infantry'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.infantry'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.infantry'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.infantry'),(SELECT id FROM movement_class WHERE key='foot'),1250);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.infantry'),17,(SELECT id FROM damage_class WHERE key='bullet'),5000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.infantry')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.infantry'),'train','units',1,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.infantry'),(SELECT id FROM def WHERE key='res.food'),76,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.infantry'),(SELECT id FROM def WHERE key='res.metal'),38,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.infantry','Upgrade to Infantry');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.infantry'),'unit_upgrade','',0,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.infantry'),(SELECT id FROM def WHERE key='res.food'),152,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.infantry'),(SELECT id FROM def WHERE key='res.metal'),76,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.infantry'),'upgrade_type',(SELECT id FROM def WHERE key='unit.rifleman'),(SELECT id FROM def WHERE key='unit.infantry'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.assault_infantry'),'unit','infantry',205,2,(SELECT id FROM armor_class WHERE key='plate'),5000,1,325,'per_type',(SELECT id FROM def WHERE key='unit.infantry'),(SELECT id FROM def WHERE key='age.information'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.assault_infantry'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.assault_infantry'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.assault_infantry'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.assault_infantry'),(SELECT id FROM movement_class WHERE key='foot'),1250);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.assault_infantry'),18,(SELECT id FROM damage_class WHERE key='bullet'),5000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.assault_infantry')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.assault_infantry'),'train','units',2,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.assault_infantry'),(SELECT id FROM def WHERE key='res.food'),82,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.assault_infantry'),(SELECT id FROM def WHERE key='res.metal'),41,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.assault_infantry','Upgrade to Assault Infantry');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.assault_infantry'),'unit_upgrade','',0,650);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.assault_infantry'),(SELECT id FROM def WHERE key='res.food'),164,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.assault_infantry'),(SELECT id FROM def WHERE key='res.metal'),82,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.assault_infantry'),'upgrade_type',(SELECT id FROM def WHERE key='unit.infantry'),(SELECT id FROM def WHERE key='unit.assault_infantry'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.combat_robot'),'unit','infantry',220,2,(SELECT id FROM armor_class WHERE key='plate'),5000,1,350,'per_type',(SELECT id FROM def WHERE key='unit.assault_infantry'),(SELECT id FROM def WHERE key='age.automation'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.combat_robot'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.combat_robot'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.combat_robot'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.combat_robot'),(SELECT id FROM movement_class WHERE key='wheeled'),1250);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.combat_robot'),19,(SELECT id FROM damage_class WHERE key='bullet'),5000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.combat_robot')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.combat_robot'),'train','units',3,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.combat_robot'),(SELECT id FROM def WHERE key='res.food'),88,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.combat_robot'),(SELECT id FROM def WHERE key='res.metal'),44,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.combat_robot','Upgrade to Combat Robots');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.combat_robot'),'unit_upgrade','',0,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.combat_robot'),(SELECT id FROM def WHERE key='res.food'),176,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.combat_robot'),(SELECT id FROM def WHERE key='res.metal'),88,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.combat_robot'),'upgrade_type',(SELECT id FROM def WHERE key='unit.assault_infantry'),(SELECT id FROM def WHERE key='unit.combat_robot'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.exo_trooper'),'unit','infantry',235,2,(SELECT id FROM armor_class WHERE key='plate'),5000,1,375,'per_type',(SELECT id FROM def WHERE key='unit.combat_robot'),(SELECT id FROM def WHERE key='age.fusion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.exo_trooper'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.exo_trooper'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.exo_trooper'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.exo_trooper'),(SELECT id FROM movement_class WHERE key='foot'),1250);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.exo_trooper'),21,(SELECT id FROM damage_class WHERE key='bullet'),5000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.exo_trooper')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.exo_trooper'),'train','units',4,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.exo_trooper'),(SELECT id FROM def WHERE key='res.food'),94,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.exo_trooper'),(SELECT id FROM def WHERE key='res.metal'),47,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.exo_trooper','Upgrade to Exo-Troopers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.exo_trooper'),'unit_upgrade','',0,750);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.exo_trooper'),(SELECT id FROM def WHERE key='res.food'),188,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.exo_trooper'),(SELECT id FROM def WHERE key='res.metal'),94,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.exo_trooper'),'upgrade_type',(SELECT id FROM def WHERE key='unit.combat_robot'),(SELECT id FROM def WHERE key='unit.exo_trooper'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.marine'),'unit','infantry',250,2,(SELECT id FROM armor_class WHERE key='plate'),5000,1,400,'per_type',(SELECT id FROM def WHERE key='unit.exo_trooper'),(SELECT id FROM def WHERE key='age.starfaring'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.marine'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.marine'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.marine'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.marine'),(SELECT id FROM movement_class WHERE key='foot'),1250);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.marine'),22,(SELECT id FROM damage_class WHERE key='energy'),5000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.marine')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.marine'),'train','units',0,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.marine'),(SELECT id FROM def WHERE key='res.food'),100,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.marine'),(SELECT id FROM def WHERE key='res.metal'),50,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.marine','Upgrade to Marines');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.marine'),'unit_upgrade','',0,800);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.marine'),(SELECT id FROM def WHERE key='res.food'),200,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.marine'),(SELECT id FROM def WHERE key='res.metal'),100,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.marine'),'upgrade_type',(SELECT id FROM def WHERE key='unit.exo_trooper'),(SELECT id FROM def WHERE key='unit.marine'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.shock_marine'),'unit','infantry',280,2,(SELECT id FROM armor_class WHERE key='plate'),5000,1,450,'per_type',(SELECT id FROM def WHERE key='unit.marine'),(SELECT id FROM def WHERE key='age.abundance'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.shock_marine'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.shock_marine'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.shock_marine'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.shock_marine'),(SELECT id FROM movement_class WHERE key='foot'),1250);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.shock_marine'),25,(SELECT id FROM damage_class WHERE key='energy'),5000,30);
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
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.slinger'),(SELECT id FROM movement_class WHERE key='foot'),1200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.slinger'),7,(SELECT id FROM damage_class WHERE key='pierce'),5000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.slinger')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.slinger'),'train','units',2,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.slinger'),(SELECT id FROM def WHERE key='res.food'),30,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.slinger'),(SELECT id FROM def WHERE key='res.timber'),30,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.archer'),'unit','ranged',80,0,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,175,'per_type',(SELECT id FROM def WHERE key='unit.slinger'),(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.archer'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.archer'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.archer'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.archer'),(SELECT id FROM movement_class WHERE key='foot'),1200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.archer'),8,(SELECT id FROM damage_class WHERE key='pierce'),5000,30);
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
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.crossbowman'),(SELECT id FROM movement_class WHERE key='foot'),1200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.crossbowman'),9,(SELECT id FROM damage_class WHERE key='pierce'),5000,30);
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
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.arquebusier'),(SELECT id FROM movement_class WHERE key='foot'),1200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.arquebusier'),10,(SELECT id FROM damage_class WHERE key='bullet'),5000,30);
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
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.light_infantry'),(SELECT id FROM movement_class WHERE key='foot'),1200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.light_infantry'),11,(SELECT id FROM damage_class WHERE key='bullet'),5000,30);
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
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.machine_gunner'),(SELECT id FROM movement_class WHERE key='foot'),1200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.machine_gunner'),12,(SELECT id FROM damage_class WHERE key='bullet'),5000,30);
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
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.heavy_machine_gunner'),(SELECT id FROM movement_class WHERE key='foot'),1200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.heavy_machine_gunner'),13,(SELECT id FROM damage_class WHERE key='bullet'),5000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.heavy_machine_gunner')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.heavy_machine_gunner'),'train','units_2',3,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.heavy_machine_gunner'),(SELECT id FROM def WHERE key='res.food'),57,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.heavy_machine_gunner'),(SELECT id FROM def WHERE key='res.timber'),57,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.heavy_machine_gunner','Upgrade to Heavy Machine Gunners');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.heavy_machine_gunner'),'unit_upgrade','',0,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.heavy_machine_gunner'),(SELECT id FROM def WHERE key='res.food'),114,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.heavy_machine_gunner'),(SELECT id FROM def WHERE key='res.timber'),114,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.heavy_machine_gunner'),'upgrade_type',(SELECT id FROM def WHERE key='unit.machine_gunner'),(SELECT id FROM def WHERE key='unit.heavy_machine_gunner'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.gun_drone'),'unit','ranged',143,3,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,325,'per_type',(SELECT id FROM def WHERE key='unit.heavy_machine_gunner'),(SELECT id FROM def WHERE key='age.information'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.gun_drone'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.gun_drone'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.gun_drone'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.gun_drone'),(SELECT id FROM tag WHERE key='drone'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.gun_drone'),(SELECT id FROM movement_class WHERE key='wheeled'),1200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.gun_drone'),14,(SELECT id FROM damage_class WHERE key='bullet'),6000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.gun_drone')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.gun_drone'),'train','units_2',4,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.gun_drone'),(SELECT id FROM def WHERE key='res.food'),61,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.gun_drone'),(SELECT id FROM def WHERE key='res.timber'),61,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.gun_drone','Upgrade to Gun Drones');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.gun_drone'),'unit_upgrade','',0,650);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.gun_drone'),(SELECT id FROM def WHERE key='res.food'),122,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.gun_drone'),(SELECT id FROM def WHERE key='res.timber'),122,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.gun_drone'),'upgrade_type',(SELECT id FROM def WHERE key='unit.heavy_machine_gunner'),(SELECT id FROM def WHERE key='unit.gun_drone'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.pulse_gunner'),'unit','ranged',164,4,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,375,'per_type',(SELECT id FROM def WHERE key='unit.gun_drone'),(SELECT id FROM def WHERE key='age.fusion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.pulse_gunner'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.pulse_gunner'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.pulse_gunner'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.pulse_gunner'),(SELECT id FROM movement_class WHERE key='foot'),1200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.pulse_gunner'),16,(SELECT id FROM damage_class WHERE key='energy'),6000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.pulse_gunner')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.pulse_gunner'),'train','units_2',0,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.pulse_gunner'),(SELECT id FROM def WHERE key='res.food'),70,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.pulse_gunner'),(SELECT id FROM def WHERE key='res.timber'),70,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.pulse_gunner','Upgrade to Pulse Gunners');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.pulse_gunner'),'unit_upgrade','',0,750);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.pulse_gunner'),(SELECT id FROM def WHERE key='res.food'),140,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.pulse_gunner'),(SELECT id FROM def WHERE key='res.timber'),140,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.pulse_gunner'),'upgrade_type',(SELECT id FROM def WHERE key='unit.gun_drone'),(SELECT id FROM def WHERE key='unit.pulse_gunner'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.beam_trooper'),'unit','ranged',185,5,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,425,'per_type',(SELECT id FROM def WHERE key='unit.pulse_gunner'),(SELECT id FROM def WHERE key='age.expansion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.beam_trooper'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.beam_trooper'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.beam_trooper'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.beam_trooper'),(SELECT id FROM movement_class WHERE key='foot'),1200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.beam_trooper'),18,(SELECT id FROM damage_class WHERE key='energy'),6000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.beam_trooper')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.beam_trooper'),'train','units_2',1,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.beam_trooper'),(SELECT id FROM def WHERE key='res.food'),79,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.beam_trooper'),(SELECT id FROM def WHERE key='res.timber'),79,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.beam_trooper','Upgrade to Beam Troopers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.beam_trooper'),'unit_upgrade','',0,850);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.beam_trooper'),(SELECT id FROM def WHERE key='res.food'),158,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.beam_trooper'),(SELECT id FROM def WHERE key='res.timber'),158,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.beam_trooper'),'upgrade_type',(SELECT id FROM def WHERE key='unit.pulse_gunner'),(SELECT id FROM def WHERE key='unit.beam_trooper'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),'unit','anti_armour',122,2,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,275,'per_type',NULL,(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),(SELECT id FROM movement_class WHERE key='foot'),1100);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),24,(SELECT id FROM damage_class WHERE key='explosive'),6000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.anti_tank_rifleman')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),'train','units_2',2,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),(SELECT id FROM def WHERE key='res.food'),70,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),(SELECT id FROM def WHERE key='res.metal'),70,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.bazooka_team'),'unit','anti_armour',133,3,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,300,'per_type',(SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),(SELECT id FROM def WHERE key='age.modern'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.bazooka_team'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.bazooka_team'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.bazooka_team'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.bazooka_team'),(SELECT id FROM movement_class WHERE key='foot'),1100);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.bazooka_team'),26,(SELECT id FROM damage_class WHERE key='explosive'),6000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.bazooka_team')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.bazooka_team'),'train','units_2',3,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.bazooka_team'),(SELECT id FROM def WHERE key='res.food'),76,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.bazooka_team'),(SELECT id FROM def WHERE key='res.metal'),76,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.bazooka_team','Upgrade to Bazooka Teams');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.bazooka_team'),'unit_upgrade','',0,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.bazooka_team'),(SELECT id FROM def WHERE key='res.food'),152,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.bazooka_team'),(SELECT id FROM def WHERE key='res.metal'),152,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.bazooka_team'),'upgrade_type',(SELECT id FROM def WHERE key='unit.anti_tank_rifleman'),(SELECT id FROM def WHERE key='unit.bazooka_team'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_team'),'unit','anti_armour',143,3,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,325,'per_type',(SELECT id FROM def WHERE key='unit.bazooka_team'),(SELECT id FROM def WHERE key='age.information'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_team'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_team'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_team'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.missile_team'),(SELECT id FROM movement_class WHERE key='foot'),1100);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.missile_team'),28,(SELECT id FROM damage_class WHERE key='explosive'),6000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.missile_team')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.missile_team'),'train','units_2',4,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.missile_team'),(SELECT id FROM def WHERE key='res.food'),82,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.missile_team'),(SELECT id FROM def WHERE key='res.metal'),82,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.missile_team','Upgrade to Missile Teams');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_team'),'unit_upgrade','',0,650);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_team'),(SELECT id FROM def WHERE key='res.food'),164,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_team'),(SELECT id FROM def WHERE key='res.metal'),164,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_team'),'upgrade_type',(SELECT id FROM def WHERE key='unit.bazooka_team'),(SELECT id FROM def WHERE key='unit.missile_team'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.railgun_team'),'unit','anti_armour',164,4,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,375,'per_type',(SELECT id FROM def WHERE key='unit.missile_team'),(SELECT id FROM def WHERE key='age.fusion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.railgun_team'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.railgun_team'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.railgun_team'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.railgun_team'),(SELECT id FROM movement_class WHERE key='foot'),1100);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.railgun_team'),32,(SELECT id FROM damage_class WHERE key='explosive'),6000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.railgun_team')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.railgun_team'),'train','units_2',0,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.railgun_team'),(SELECT id FROM def WHERE key='res.food'),94,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.railgun_team'),(SELECT id FROM def WHERE key='res.metal'),94,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.railgun_team','Upgrade to Railgun Teams');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.railgun_team'),'unit_upgrade','',0,750);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.railgun_team'),(SELECT id FROM def WHERE key='res.food'),188,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.railgun_team'),(SELECT id FROM def WHERE key='res.metal'),188,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.railgun_team'),'upgrade_type',(SELECT id FROM def WHERE key='unit.missile_team'),(SELECT id FROM def WHERE key='unit.railgun_team'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.scout'),'unit','scout',70,0,(SELECT id FROM armor_class WHERE key='flesh'),9000,1,150,'per_type',NULL,(SELECT id FROM def WHERE key='age.ancient'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.scout'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.scout'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.scout'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.scout'),(SELECT id FROM movement_class WHERE key='foot'),2200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.scout'),7,(SELECT id FROM damage_class WHERE key='pierce'),0,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.scout')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.scout'),'train','units_2',1,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.scout'),(SELECT id FROM def WHERE key='res.food'),30,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.scout'),(SELECT id FROM def WHERE key='res.timber'),30,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.ranger'),'unit','scout',112,2,(SELECT id FROM armor_class WHERE key='flesh'),9000,1,250,'per_type',(SELECT id FROM def WHERE key='unit.scout'),(SELECT id FROM def WHERE key='age.enlightenment'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.ranger'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.ranger'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.ranger'),(SELECT id FROM tag WHERE key='infantry'));
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.recon_drone'),'unit','scout',143,3,(SELECT id FROM armor_class WHERE key='flesh'),9000,1,325,'per_type',(SELECT id FROM def WHERE key='unit.ranger'),(SELECT id FROM def WHERE key='age.information'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.recon_drone'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.recon_drone'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.recon_drone'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.recon_drone'),(SELECT id FROM tag WHERE key='drone'));
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.medic'),'unit','medic',60,2,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,275,'per_type',NULL,(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.medic'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.medic'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.medic'),(SELECT id FROM movement_class WHERE key='foot'),1400);
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.medic'),(SELECT id FROM tag WHERE key='healer'));
INSERT INTO et_healer(entity_type_id,target_tag_id,hp_per_second,range,energy_per_hp,can_convert) VALUES ((SELECT id FROM def WHERE key='unit.medic'),(SELECT id FROM tag WHERE key='biological'),8,3000,1,0);
INSERT INTO et_energy(entity_type_id,max,regen_per_second) VALUES ((SELECT id FROM def WHERE key='unit.medic'),100,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='unit.medic'),'train','units_2',4,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.medic'),(SELECT id FROM def WHERE key='res.wealth'),70,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.horseman'),'unit','light_mounted',184,0,(SELECT id FROM armor_class WHERE key='plate'),6000,1,175,'per_type',NULL,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.horseman'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.horseman'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.horseman'),(SELECT id FROM tag WHERE key='mounted'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.horseman'),(SELECT id FROM movement_class WHERE key='foot'),2200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.horseman'),13,(SELECT id FROM damage_class WHERE key='melee'),0,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.horseman')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM def WHERE key='unit.horseman'),'train','units',0,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.horseman'),(SELECT id FROM def WHERE key='res.food'),69,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.horseman'),(SELECT id FROM def WHERE key='res.metal'),46,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.light_cavalry'),'unit','light_mounted',208,1,(SELECT id FROM armor_class WHERE key='plate'),6000,1,200,'per_type',(SELECT id FROM def WHERE key='unit.horseman'),(SELECT id FROM def WHERE key='age.medieval'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.light_cavalry'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.light_cavalry'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.light_cavalry'),(SELECT id FROM tag WHERE key='mounted'));
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.hussar'),'unit','light_mounted',232,1,(SELECT id FROM armor_class WHERE key='plate'),6000,1,225,'per_type',(SELECT id FROM def WHERE key='unit.light_cavalry'),(SELECT id FROM def WHERE key='age.gunpowder'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hussar'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hussar'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hussar'),(SELECT id FROM tag WHERE key='mounted'));
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.dragoon'),'unit','light_mounted',256,2,(SELECT id FROM armor_class WHERE key='plate'),6000,1,250,'per_type',(SELECT id FROM def WHERE key='unit.hussar'),(SELECT id FROM def WHERE key='age.enlightenment'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.dragoon'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.dragoon'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.dragoon'),(SELECT id FROM tag WHERE key='mounted'));
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.armoured_car'),'unit','light_mounted',280,2,(SELECT id FROM armor_class WHERE key='vehicle'),6000,1,275,'per_type',(SELECT id FROM def WHERE key='unit.dragoon'),(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.armoured_car'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.armoured_car'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.armoured_car'),(SELECT id FROM tag WHERE key='vehicle'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.armoured_car'),(SELECT id FROM movement_class WHERE key='wheeled'),1600);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.armoured_car'),21,(SELECT id FROM damage_class WHERE key='explosive'),4000,30);
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.scout_car'),'unit','light_mounted',304,3,(SELECT id FROM armor_class WHERE key='vehicle'),6000,1,300,'per_type',(SELECT id FROM def WHERE key='unit.armoured_car'),(SELECT id FROM def WHERE key='age.modern'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.scout_car'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.scout_car'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.scout_car'),(SELECT id FROM tag WHERE key='vehicle'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.scout_car'),(SELECT id FROM movement_class WHERE key='wheeled'),1600);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.scout_car'),22,(SELECT id FROM damage_class WHERE key='explosive'),4000,30);
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.hovercraft'),'unit','light_mounted',352,4,(SELECT id FROM armor_class WHERE key='vehicle'),6000,1,350,'per_type',(SELECT id FROM def WHERE key='unit.scout_car'),(SELECT id FROM def WHERE key='age.automation'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hovercraft'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hovercraft'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hovercraft'),(SELECT id FROM tag WHERE key='vehicle'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.hovercraft'),(SELECT id FROM movement_class WHERE key='wheeled'),1600);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.hovercraft'),26,(SELECT id FROM damage_class WHERE key='explosive'),4000,30);
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.grav_skimmer'),'unit','light_mounted',400,5,(SELECT id FROM armor_class WHERE key='vehicle'),6000,1,400,'per_type',(SELECT id FROM def WHERE key='unit.hovercraft'),(SELECT id FROM def WHERE key='age.starfaring'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.grav_skimmer'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.grav_skimmer'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.grav_skimmer'),(SELECT id FROM tag WHERE key='vehicle'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.grav_skimmer'),(SELECT id FROM movement_class WHERE key='wheeled'),1600);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.grav_skimmer'),30,(SELECT id FROM damage_class WHERE key='explosive'),4000,30);
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.cataphract'),'unit','heavy_mounted',184,0,(SELECT id FROM armor_class WHERE key='plate'),6000,1,175,'per_type',NULL,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cataphract'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cataphract'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cataphract'),(SELECT id FROM tag WHERE key='mounted'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.cataphract'),(SELECT id FROM movement_class WHERE key='foot'),2200);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.cataphract'),13,(SELECT id FROM damage_class WHERE key='melee'),0,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.cataphract')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.stable'),(SELECT id FROM def WHERE key='unit.cataphract'),'train','units',4,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.cataphract'),(SELECT id FROM def WHERE key='res.food'),69,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.cataphract'),(SELECT id FROM def WHERE key='res.metal'),46,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.knight'),'unit','heavy_mounted',208,1,(SELECT id FROM armor_class WHERE key='plate'),6000,1,200,'per_type',(SELECT id FROM def WHERE key='unit.cataphract'),(SELECT id FROM def WHERE key='age.medieval'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.knight'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.knight'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.knight'),(SELECT id FROM tag WHERE key='mounted'));
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.cuirassier'),'unit','heavy_mounted',232,1,(SELECT id FROM armor_class WHERE key='plate'),6000,1,225,'per_type',(SELECT id FROM def WHERE key='unit.knight'),(SELECT id FROM def WHERE key='age.gunpowder'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cuirassier'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cuirassier'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cuirassier'),(SELECT id FROM tag WHERE key='mounted'));
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.lancer'),'unit','heavy_mounted',256,2,(SELECT id FROM armor_class WHERE key='plate'),6000,1,250,'per_type',(SELECT id FROM def WHERE key='unit.cuirassier'),(SELECT id FROM def WHERE key='age.enlightenment'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.lancer'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.lancer'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.lancer'),(SELECT id FROM tag WHERE key='mounted'));
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.light_tank'),'unit','heavy_mounted',280,2,(SELECT id FROM armor_class WHERE key='vehicle'),6000,1,275,'per_type',(SELECT id FROM def WHERE key='unit.lancer'),(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.light_tank'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.light_tank'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.light_tank'),(SELECT id FROM tag WHERE key='vehicle'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.light_tank'),(SELECT id FROM movement_class WHERE key='wheeled'),1600);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.light_tank'),21,(SELECT id FROM damage_class WHERE key='explosive'),4000,30);
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.tank'),'unit','heavy_mounted',304,3,(SELECT id FROM armor_class WHERE key='vehicle'),6000,1,300,'per_type',(SELECT id FROM def WHERE key='unit.light_tank'),(SELECT id FROM def WHERE key='age.modern'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.tank'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.tank'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.tank'),(SELECT id FROM tag WHERE key='vehicle'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.tank'),(SELECT id FROM movement_class WHERE key='wheeled'),1600);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.tank'),22,(SELECT id FROM damage_class WHERE key='explosive'),4000,30);
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.main_battle_tank'),'unit','heavy_mounted',328,3,(SELECT id FROM armor_class WHERE key='vehicle'),6000,1,325,'per_type',(SELECT id FROM def WHERE key='unit.tank'),(SELECT id FROM def WHERE key='age.information'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.main_battle_tank'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.main_battle_tank'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.main_battle_tank'),(SELECT id FROM tag WHERE key='vehicle'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.main_battle_tank'),(SELECT id FROM movement_class WHERE key='wheeled'),1600);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.main_battle_tank'),24,(SELECT id FROM damage_class WHERE key='explosive'),4000,30);
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.robot_tank'),'unit','heavy_mounted',352,4,(SELECT id FROM armor_class WHERE key='vehicle'),6000,1,350,'per_type',(SELECT id FROM def WHERE key='unit.main_battle_tank'),(SELECT id FROM def WHERE key='age.automation'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.robot_tank'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.robot_tank'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.robot_tank'),(SELECT id FROM tag WHERE key='vehicle'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.robot_tank'),(SELECT id FROM movement_class WHERE key='wheeled'),1600);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.robot_tank'),26,(SELECT id FROM damage_class WHERE key='explosive'),4000,30);
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.fusion_tank'),'unit','heavy_mounted',376,4,(SELECT id FROM armor_class WHERE key='vehicle'),6000,1,375,'per_type',(SELECT id FROM def WHERE key='unit.robot_tank'),(SELECT id FROM def WHERE key='age.fusion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.fusion_tank'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.fusion_tank'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.fusion_tank'),(SELECT id FROM tag WHERE key='vehicle'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.fusion_tank'),(SELECT id FROM movement_class WHERE key='wheeled'),1600);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.fusion_tank'),28,(SELECT id FROM damage_class WHERE key='explosive'),4000,30);
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.grav_tank'),'unit','heavy_mounted',424,5,(SELECT id FROM armor_class WHERE key='vehicle'),6000,1,425,'per_type',(SELECT id FROM def WHERE key='unit.fusion_tank'),(SELECT id FROM def WHERE key='age.expansion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.grav_tank'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.grav_tank'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.grav_tank'),(SELECT id FROM tag WHERE key='vehicle'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.grav_tank'),(SELECT id FROM movement_class WHERE key='wheeled'),1600);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.grav_tank'),31,(SELECT id FROM damage_class WHERE key='explosive'),4000,30);
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.catapult'),'unit','siege',138,0,(SELECT id FROM armor_class WHERE key='structure'),6000,1,175,'per_type',NULL,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.catapult'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.catapult'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.catapult'),(SELECT id FROM tag WHERE key='siege'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.catapult'),(SELECT id FROM movement_class WHERE key='wheeled'),800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.catapult'),34,(SELECT id FROM damage_class WHERE key='explosive'),10000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.catapult')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),(SELECT id FROM def WHERE key='unit.catapult'),'train','units',0,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.catapult'),(SELECT id FROM def WHERE key='res.timber'),69,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.catapult'),(SELECT id FROM def WHERE key='res.metal'),69,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.trebuchet'),'unit','siege',156,1,(SELECT id FROM armor_class WHERE key='structure'),6000,1,200,'per_type',(SELECT id FROM def WHERE key='unit.catapult'),(SELECT id FROM def WHERE key='age.medieval'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.trebuchet'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.trebuchet'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.trebuchet'),(SELECT id FROM tag WHERE key='siege'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.trebuchet'),(SELECT id FROM movement_class WHERE key='wheeled'),800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.trebuchet'),39,(SELECT id FROM damage_class WHERE key='explosive'),10000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.trebuchet')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),(SELECT id FROM def WHERE key='unit.trebuchet'),'train','units',1,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.trebuchet'),(SELECT id FROM def WHERE key='res.timber'),78,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.trebuchet'),(SELECT id FROM def WHERE key='res.metal'),78,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.trebuchet','Upgrade to Trebuchets');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.trebuchet'),'unit_upgrade','',0,400);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.trebuchet'),(SELECT id FROM def WHERE key='res.timber'),156,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.trebuchet'),(SELECT id FROM def WHERE key='res.metal'),156,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.trebuchet'),'upgrade_type',(SELECT id FROM def WHERE key='unit.catapult'),(SELECT id FROM def WHERE key='unit.trebuchet'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.bombard'),'unit','siege',174,1,(SELECT id FROM armor_class WHERE key='structure'),6000,1,225,'per_type',(SELECT id FROM def WHERE key='unit.trebuchet'),(SELECT id FROM def WHERE key='age.gunpowder'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.bombard'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.bombard'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.bombard'),(SELECT id FROM tag WHERE key='siege'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.bombard'),(SELECT id FROM movement_class WHERE key='wheeled'),800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.bombard'),43,(SELECT id FROM damage_class WHERE key='explosive'),11000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.bombard')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),(SELECT id FROM def WHERE key='unit.bombard'),'train','units',2,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.bombard'),(SELECT id FROM def WHERE key='res.timber'),87,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.bombard'),(SELECT id FROM def WHERE key='res.metal'),87,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.bombard','Upgrade to Bombards');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.bombard'),'unit_upgrade','',0,450);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.bombard'),(SELECT id FROM def WHERE key='res.timber'),174,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.bombard'),(SELECT id FROM def WHERE key='res.metal'),174,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.bombard'),'upgrade_type',(SELECT id FROM def WHERE key='unit.trebuchet'),(SELECT id FROM def WHERE key='unit.bombard'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.cannon'),'unit','siege',192,2,(SELECT id FROM armor_class WHERE key='structure'),6000,1,250,'per_type',(SELECT id FROM def WHERE key='unit.bombard'),(SELECT id FROM def WHERE key='age.enlightenment'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cannon'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cannon'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cannon'),(SELECT id FROM tag WHERE key='siege'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.cannon'),(SELECT id FROM movement_class WHERE key='wheeled'),800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.cannon'),48,(SELECT id FROM damage_class WHERE key='explosive'),11000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.cannon')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),(SELECT id FROM def WHERE key='unit.cannon'),'train','units',3,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.cannon'),(SELECT id FROM def WHERE key='res.timber'),96,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.cannon'),(SELECT id FROM def WHERE key='res.metal'),96,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.cannon','Upgrade to Cannons');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.cannon'),'unit_upgrade','',0,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.cannon'),(SELECT id FROM def WHERE key='res.timber'),192,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.cannon'),(SELECT id FROM def WHERE key='res.metal'),192,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.cannon'),'upgrade_type',(SELECT id FROM def WHERE key='unit.bombard'),(SELECT id FROM def WHERE key='unit.cannon'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.field_gun'),'unit','siege',210,2,(SELECT id FROM armor_class WHERE key='structure'),6000,1,275,'per_type',(SELECT id FROM def WHERE key='unit.cannon'),(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.field_gun'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.field_gun'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.field_gun'),(SELECT id FROM tag WHERE key='siege'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.field_gun'),(SELECT id FROM movement_class WHERE key='wheeled'),800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.field_gun'),52,(SELECT id FROM damage_class WHERE key='explosive'),12000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.field_gun')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='unit.field_gun'),'train','units',0,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.field_gun'),(SELECT id FROM def WHERE key='res.timber'),105,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.field_gun'),(SELECT id FROM def WHERE key='res.metal'),105,2);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.field_gun','Upgrade to Field Guns');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.field_gun'),'unit_upgrade','',0,550);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.field_gun'),(SELECT id FROM def WHERE key='res.timber'),210,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.field_gun'),(SELECT id FROM def WHERE key='res.metal'),210,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.field_gun'),'upgrade_type',(SELECT id FROM def WHERE key='unit.cannon'),(SELECT id FROM def WHERE key='unit.field_gun'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.howitzer'),'unit','siege',228,3,(SELECT id FROM armor_class WHERE key='structure'),6000,1,300,'per_type',(SELECT id FROM def WHERE key='unit.field_gun'),(SELECT id FROM def WHERE key='age.modern'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.howitzer'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.howitzer'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.howitzer'),(SELECT id FROM tag WHERE key='siege'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.howitzer'),(SELECT id FROM movement_class WHERE key='wheeled'),800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.howitzer'),57,(SELECT id FROM damage_class WHERE key='explosive'),12000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.howitzer')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='unit.howitzer'),'train','units',1,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.howitzer'),(SELECT id FROM def WHERE key='res.timber'),114,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.howitzer'),(SELECT id FROM def WHERE key='res.metal'),114,2);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.howitzer','Upgrade to Howitzers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.howitzer'),'unit_upgrade','',0,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.howitzer'),(SELECT id FROM def WHERE key='res.timber'),228,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.howitzer'),(SELECT id FROM def WHERE key='res.metal'),228,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.howitzer'),'upgrade_type',(SELECT id FROM def WHERE key='unit.field_gun'),(SELECT id FROM def WHERE key='unit.howitzer'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.rocket_artillery'),'unit','siege',246,3,(SELECT id FROM armor_class WHERE key='structure'),6000,1,325,'per_type',(SELECT id FROM def WHERE key='unit.howitzer'),(SELECT id FROM def WHERE key='age.information'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.rocket_artillery'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.rocket_artillery'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.rocket_artillery'),(SELECT id FROM tag WHERE key='siege'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.rocket_artillery'),(SELECT id FROM movement_class WHERE key='wheeled'),800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.rocket_artillery'),61,(SELECT id FROM damage_class WHERE key='explosive'),13000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.rocket_artillery')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='unit.rocket_artillery'),'train','units',2,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.rocket_artillery'),(SELECT id FROM def WHERE key='res.timber'),123,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.rocket_artillery'),(SELECT id FROM def WHERE key='res.metal'),123,2);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.rocket_artillery','Upgrade to Rocket Artillery');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.rocket_artillery'),'unit_upgrade','',0,650);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.rocket_artillery'),(SELECT id FROM def WHERE key='res.timber'),246,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.rocket_artillery'),(SELECT id FROM def WHERE key='res.metal'),246,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.rocket_artillery'),'upgrade_type',(SELECT id FROM def WHERE key='unit.howitzer'),(SELECT id FROM def WHERE key='unit.rocket_artillery'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.railgun_battery'),'unit','siege',282,4,(SELECT id FROM armor_class WHERE key='structure'),6000,1,375,'per_type',(SELECT id FROM def WHERE key='unit.rocket_artillery'),(SELECT id FROM def WHERE key='age.fusion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.railgun_battery'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.railgun_battery'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.railgun_battery'),(SELECT id FROM tag WHERE key='siege'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.railgun_battery'),(SELECT id FROM movement_class WHERE key='wheeled'),800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.railgun_battery'),70,(SELECT id FROM damage_class WHERE key='explosive'),14000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.railgun_battery')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='unit.railgun_battery'),'train','units',3,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.railgun_battery'),(SELECT id FROM def WHERE key='res.timber'),141,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.railgun_battery'),(SELECT id FROM def WHERE key='res.metal'),141,2);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.railgun_battery','Upgrade to Railgun Batteries');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.railgun_battery'),'unit_upgrade','',0,750);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.railgun_battery'),(SELECT id FROM def WHERE key='res.timber'),282,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.railgun_battery'),(SELECT id FROM def WHERE key='res.metal'),282,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.railgun_battery'),'upgrade_type',(SELECT id FROM def WHERE key='unit.rocket_artillery'),(SELECT id FROM def WHERE key='unit.railgun_battery'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.flak_gun'),'unit','anti_air',133,3,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,300,'per_type',NULL,(SELECT id FROM def WHERE key='age.modern'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.flak_gun'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.flak_gun'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.flak_gun'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.flak_gun'),(SELECT id FROM movement_class WHERE key='wheeled'),1100);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.flak_gun'),26,(SELECT id FROM damage_class WHERE key='explosive'),6000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.flak_gun')),'air');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='unit.flak_gun'),'train','units',4,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.flak_gun'),(SELECT id FROM def WHERE key='res.food'),76,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.flak_gun'),(SELECT id FROM def WHERE key='res.metal'),76,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_launcher'),'unit','anti_air',143,3,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,325,'per_type',(SELECT id FROM def WHERE key='unit.flak_gun'),(SELECT id FROM def WHERE key='age.information'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_launcher'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_launcher'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_launcher'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.missile_launcher'),(SELECT id FROM movement_class WHERE key='wheeled'),1100);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.missile_launcher'),28,(SELECT id FROM damage_class WHERE key='explosive'),6000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.missile_launcher')),'air');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='unit.missile_launcher'),'train','units',0,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.missile_launcher'),(SELECT id FROM def WHERE key='res.food'),82,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.missile_launcher'),(SELECT id FROM def WHERE key='res.metal'),82,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.missile_launcher','Upgrade to Missile Launchers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_launcher'),'unit_upgrade','',0,650);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_launcher'),(SELECT id FROM def WHERE key='res.food'),164,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_launcher'),(SELECT id FROM def WHERE key='res.metal'),164,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_launcher'),'upgrade_type',(SELECT id FROM def WHERE key='unit.flak_gun'),(SELECT id FROM def WHERE key='unit.missile_launcher'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.laser_battery'),'unit','anti_air',164,4,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,375,'per_type',(SELECT id FROM def WHERE key='unit.missile_launcher'),(SELECT id FROM def WHERE key='age.fusion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.laser_battery'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.laser_battery'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.laser_battery'),(SELECT id FROM tag WHERE key='infantry'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.laser_battery'),(SELECT id FROM movement_class WHERE key='wheeled'),1100);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.laser_battery'),32,(SELECT id FROM damage_class WHERE key='explosive'),6000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.laser_battery')),'air');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='unit.laser_battery'),'train','units',1,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.laser_battery'),(SELECT id FROM def WHERE key='res.food'),94,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.laser_battery'),(SELECT id FROM def WHERE key='res.metal'),94,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.laser_battery','Upgrade to Laser Batteries');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.laser_battery'),'unit_upgrade','',0,750);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.laser_battery'),(SELECT id FROM def WHERE key='res.food'),188,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.laser_battery'),(SELECT id FROM def WHERE key='res.metal'),188,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.laser_battery'),'upgrade_type',(SELECT id FROM def WHERE key='unit.missile_launcher'),(SELECT id FROM def WHERE key='unit.laser_battery'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.supply_wagon'),'unit','supply',120,1,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,200,'per_type',NULL,(SELECT id FROM def WHERE key='age.medieval'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.supply_wagon'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.supply_wagon'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.supply_wagon'),(SELECT id FROM movement_class WHERE key='wheeled'),1300);
INSERT INTO et_supply(entity_type_id,radius) VALUES ((SELECT id FROM def WHERE key='unit.supply_wagon'),6000);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.siege_workshop'),(SELECT id FROM def WHERE key='unit.supply_wagon'),'train','units',4,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.supply_wagon'),(SELECT id FROM def WHERE key='res.wealth'),52,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.supply_truck'),'unit','supply',120,2,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,275,'per_type',(SELECT id FROM def WHERE key='unit.supply_wagon'),(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.supply_truck'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.supply_truck'),(SELECT id FROM tag WHERE key='military'));
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
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.repair_drone'),(SELECT id FROM movement_class WHERE key='aircraft'),2600);
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.repair_drone'),(SELECT id FROM tag WHERE key='healer'));
INSERT INTO et_healer(entity_type_id,target_tag_id,hp_per_second,range,energy_per_hp,can_convert) VALUES ((SELECT id FROM def WHERE key='unit.repair_drone'),(SELECT id FROM tag WHERE key='mechanical'),8,3000,0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='unit.repair_drone'),'train','units',3,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.repair_drone'),(SELECT id FROM def WHERE key='res.wealth'),82,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.trireme'),'unit','warship',345,0,(SELECT id FROM armor_class WHERE key='hull'),6000,1,175,'per_type',NULL,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.trireme'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.trireme'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.trireme'),(SELECT id FROM tag WHERE key='ship'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.trireme'),(SELECT id FROM movement_class WHERE key='ship'),1800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.trireme'),20,(SELECT id FROM damage_class WHERE key='pierce'),7000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.trireme')),'fluid');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.trireme')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='unit.trireme'),'train','units',0,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.trireme'),(SELECT id FROM def WHERE key='res.timber'),92,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.trireme'),(SELECT id FROM def WHERE key='res.metal'),46,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.war_galley'),'unit','warship',390,1,(SELECT id FROM armor_class WHERE key='hull'),6000,1,200,'per_type',(SELECT id FROM def WHERE key='unit.trireme'),(SELECT id FROM def WHERE key='age.medieval'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.war_galley'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.war_galley'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.war_galley'),(SELECT id FROM tag WHERE key='ship'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.war_galley'),(SELECT id FROM movement_class WHERE key='ship'),1800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.war_galley'),23,(SELECT id FROM damage_class WHERE key='pierce'),7000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.war_galley')),'fluid');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.war_galley')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='unit.war_galley'),'train','units',1,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.war_galley'),(SELECT id FROM def WHERE key='res.timber'),104,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.war_galley'),(SELECT id FROM def WHERE key='res.metal'),52,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.war_galley','Upgrade to War Galleys');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.war_galley'),'unit_upgrade','',0,400);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.war_galley'),(SELECT id FROM def WHERE key='res.timber'),208,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.war_galley'),(SELECT id FROM def WHERE key='res.metal'),104,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.war_galley'),'upgrade_type',(SELECT id FROM def WHERE key='unit.trireme'),(SELECT id FROM def WHERE key='unit.war_galley'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.carrack'),'unit','warship',435,1,(SELECT id FROM armor_class WHERE key='hull'),6000,1,225,'per_type',(SELECT id FROM def WHERE key='unit.war_galley'),(SELECT id FROM def WHERE key='age.gunpowder'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.carrack'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.carrack'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.carrack'),(SELECT id FROM tag WHERE key='ship'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.carrack'),(SELECT id FROM movement_class WHERE key='ship'),1800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.carrack'),26,(SELECT id FROM damage_class WHERE key='explosive'),7000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.carrack')),'fluid');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.carrack')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='unit.carrack'),'train','units',2,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.carrack'),(SELECT id FROM def WHERE key='res.timber'),116,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.carrack'),(SELECT id FROM def WHERE key='res.metal'),58,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.carrack','Upgrade to Carracks');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.carrack'),'unit_upgrade','',0,450);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.carrack'),(SELECT id FROM def WHERE key='res.timber'),232,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.carrack'),(SELECT id FROM def WHERE key='res.metal'),116,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.carrack'),'upgrade_type',(SELECT id FROM def WHERE key='unit.war_galley'),(SELECT id FROM def WHERE key='unit.carrack'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.frigate'),'unit','warship',480,2,(SELECT id FROM armor_class WHERE key='hull'),6000,1,250,'per_type',(SELECT id FROM def WHERE key='unit.carrack'),(SELECT id FROM def WHERE key='age.enlightenment'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.frigate'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.frigate'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.frigate'),(SELECT id FROM tag WHERE key='ship'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.frigate'),(SELECT id FROM movement_class WHERE key='ship'),1800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.frigate'),28,(SELECT id FROM damage_class WHERE key='explosive'),7000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.frigate')),'fluid');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.frigate')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='unit.frigate'),'train','units',3,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.frigate'),(SELECT id FROM def WHERE key='res.timber'),128,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.frigate'),(SELECT id FROM def WHERE key='res.metal'),64,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.frigate','Upgrade to Frigates');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.frigate'),'unit_upgrade','',0,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.frigate'),(SELECT id FROM def WHERE key='res.timber'),256,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.frigate'),(SELECT id FROM def WHERE key='res.metal'),128,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.frigate'),'upgrade_type',(SELECT id FROM def WHERE key='unit.carrack'),(SELECT id FROM def WHERE key='unit.frigate'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.ironclad'),'unit','warship',525,2,(SELECT id FROM armor_class WHERE key='hull'),6000,1,275,'per_type',(SELECT id FROM def WHERE key='unit.frigate'),(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.ironclad'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.ironclad'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.ironclad'),(SELECT id FROM tag WHERE key='ship'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.ironclad'),(SELECT id FROM movement_class WHERE key='ship'),1800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.ironclad'),31,(SELECT id FROM damage_class WHERE key='explosive'),7000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.ironclad')),'fluid');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.ironclad')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='unit.ironclad'),'train','units',4,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.ironclad'),(SELECT id FROM def WHERE key='res.timber'),140,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.ironclad'),(SELECT id FROM def WHERE key='res.metal'),70,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.ironclad','Upgrade to Ironclads');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.ironclad'),'unit_upgrade','',0,550);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.ironclad'),(SELECT id FROM def WHERE key='res.timber'),280,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.ironclad'),(SELECT id FROM def WHERE key='res.metal'),140,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.ironclad'),'upgrade_type',(SELECT id FROM def WHERE key='unit.frigate'),(SELECT id FROM def WHERE key='unit.ironclad'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.destroyer'),'unit','warship',570,3,(SELECT id FROM armor_class WHERE key='hull'),6000,1,300,'per_type',(SELECT id FROM def WHERE key='unit.ironclad'),(SELECT id FROM def WHERE key='age.modern'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.destroyer'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.destroyer'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.destroyer'),(SELECT id FROM tag WHERE key='ship'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.destroyer'),(SELECT id FROM movement_class WHERE key='ship'),1800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.destroyer'),34,(SELECT id FROM damage_class WHERE key='explosive'),7000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.destroyer')),'fluid');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.destroyer')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='unit.destroyer'),'train','units',0,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.destroyer'),(SELECT id FROM def WHERE key='res.timber'),152,3);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.destroyer'),(SELECT id FROM def WHERE key='res.metal'),76,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.destroyer','Upgrade to Destroyers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.destroyer'),'unit_upgrade','',0,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.destroyer'),(SELECT id FROM def WHERE key='res.timber'),304,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.destroyer'),(SELECT id FROM def WHERE key='res.metal'),152,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.destroyer'),'upgrade_type',(SELECT id FROM def WHERE key='unit.ironclad'),(SELECT id FROM def WHERE key='unit.destroyer'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_cruiser'),'unit','warship',615,3,(SELECT id FROM armor_class WHERE key='hull'),6000,1,325,'per_type',(SELECT id FROM def WHERE key='unit.destroyer'),(SELECT id FROM def WHERE key='age.information'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_cruiser'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_cruiser'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.missile_cruiser'),(SELECT id FROM tag WHERE key='ship'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.missile_cruiser'),(SELECT id FROM movement_class WHERE key='ship'),1800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.missile_cruiser'),36,(SELECT id FROM damage_class WHERE key='explosive'),7000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.missile_cruiser')),'fluid');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.missile_cruiser')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='unit.missile_cruiser'),'train','units',1,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.missile_cruiser'),(SELECT id FROM def WHERE key='res.timber'),164,3);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.missile_cruiser'),(SELECT id FROM def WHERE key='res.metal'),82,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.missile_cruiser','Upgrade to Missile Cruisers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_cruiser'),'unit_upgrade','',0,650);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_cruiser'),(SELECT id FROM def WHERE key='res.timber'),328,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_cruiser'),(SELECT id FROM def WHERE key='res.metal'),164,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_cruiser'),'upgrade_type',(SELECT id FROM def WHERE key='unit.destroyer'),(SELECT id FROM def WHERE key='unit.missile_cruiser'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_cruiser'),'unit','warship',660,4,(SELECT id FROM armor_class WHERE key='hull'),6000,1,350,'per_type',(SELECT id FROM def WHERE key='unit.missile_cruiser'),(SELECT id FROM def WHERE key='age.automation'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_cruiser'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_cruiser'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_cruiser'),(SELECT id FROM tag WHERE key='ship'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_cruiser'),(SELECT id FROM tag WHERE key='drone'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.drone_cruiser'),(SELECT id FROM movement_class WHERE key='ship'),1800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.drone_cruiser'),39,(SELECT id FROM damage_class WHERE key='explosive'),7000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.drone_cruiser')),'fluid');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.drone_cruiser')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='unit.drone_cruiser'),'train','units',2,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.drone_cruiser'),(SELECT id FROM def WHERE key='res.timber'),176,3);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.drone_cruiser'),(SELECT id FROM def WHERE key='res.metal'),88,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.drone_cruiser','Upgrade to Drone Cruisers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_cruiser'),'unit_upgrade','',0,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_cruiser'),(SELECT id FROM def WHERE key='res.timber'),352,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_cruiser'),(SELECT id FROM def WHERE key='res.metal'),176,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_cruiser'),'upgrade_type',(SELECT id FROM def WHERE key='unit.missile_cruiser'),(SELECT id FROM def WHERE key='unit.drone_cruiser'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.hover_frigate'),'unit','warship',705,4,(SELECT id FROM armor_class WHERE key='hull'),6000,1,375,'per_type',(SELECT id FROM def WHERE key='unit.drone_cruiser'),(SELECT id FROM def WHERE key='age.fusion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hover_frigate'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hover_frigate'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.hover_frigate'),(SELECT id FROM tag WHERE key='ship'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.hover_frigate'),(SELECT id FROM movement_class WHERE key='ship'),1800);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.hover_frigate'),42,(SELECT id FROM damage_class WHERE key='explosive'),7000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.hover_frigate')),'fluid');
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.hover_frigate')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='unit.hover_frigate'),'train','units',3,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.hover_frigate'),(SELECT id FROM def WHERE key='res.timber'),188,3);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.hover_frigate'),(SELECT id FROM def WHERE key='res.metal'),94,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.hover_frigate','Upgrade to Hover Frigates');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.hover_frigate'),'unit_upgrade','',0,750);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.hover_frigate'),(SELECT id FROM def WHERE key='res.timber'),376,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.hover_frigate'),(SELECT id FROM def WHERE key='res.metal'),188,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.hover_frigate'),'upgrade_type',(SELECT id FROM def WHERE key='unit.drone_cruiser'),(SELECT id FROM def WHERE key='unit.hover_frigate'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.biplane'),'unit','fighter',262,2,(SELECT id FROM armor_class WHERE key='vehicle'),6000,1,275,'per_type',NULL,(SELECT id FROM def WHERE key='age.industrial'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.biplane'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.biplane'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.biplane'),(SELECT id FROM tag WHERE key='aircraft'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.biplane'),(SELECT id FROM movement_class WHERE key='aircraft'),4000);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.biplane'),35,(SELECT id FROM damage_class WHERE key='bullet'),5000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.biplane')),'air');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='unit.biplane'),'train','units',0,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.biplane'),(SELECT id FROM def WHERE key='res.metal'),140,2);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.biplane'),(SELECT id FROM def WHERE key='res.oil'),70,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.fighter'),'unit','fighter',285,3,(SELECT id FROM armor_class WHERE key='vehicle'),6000,1,300,'per_type',(SELECT id FROM def WHERE key='unit.biplane'),(SELECT id FROM def WHERE key='age.modern'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.fighter'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.fighter'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.fighter'),(SELECT id FROM tag WHERE key='aircraft'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.fighter'),(SELECT id FROM movement_class WHERE key='aircraft'),4000);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.fighter'),38,(SELECT id FROM damage_class WHERE key='bullet'),5000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.fighter')),'air');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='unit.fighter'),'train','units',1,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.fighter'),(SELECT id FROM def WHERE key='res.metal'),152,3);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.fighter'),(SELECT id FROM def WHERE key='res.oil'),76,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.fighter','Upgrade to Fighters');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.fighter'),'unit_upgrade','',0,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.fighter'),(SELECT id FROM def WHERE key='res.metal'),304,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.fighter'),(SELECT id FROM def WHERE key='res.oil'),152,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.fighter'),'upgrade_type',(SELECT id FROM def WHERE key='unit.biplane'),(SELECT id FROM def WHERE key='unit.fighter'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.jet_fighter'),'unit','fighter',307,3,(SELECT id FROM armor_class WHERE key='vehicle'),6000,1,325,'per_type',(SELECT id FROM def WHERE key='unit.fighter'),(SELECT id FROM def WHERE key='age.information'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.jet_fighter'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.jet_fighter'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.jet_fighter'),(SELECT id FROM tag WHERE key='aircraft'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.jet_fighter'),(SELECT id FROM movement_class WHERE key='aircraft'),4000);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.jet_fighter'),41,(SELECT id FROM damage_class WHERE key='bullet'),5000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.jet_fighter')),'air');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='unit.jet_fighter'),'train','units',2,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.jet_fighter'),(SELECT id FROM def WHERE key='res.metal'),164,3);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.jet_fighter'),(SELECT id FROM def WHERE key='res.oil'),82,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.jet_fighter','Upgrade to Jet Fighters');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.jet_fighter'),'unit_upgrade','',0,650);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.jet_fighter'),(SELECT id FROM def WHERE key='res.metal'),328,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.jet_fighter'),(SELECT id FROM def WHERE key='res.oil'),164,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.jet_fighter'),'upgrade_type',(SELECT id FROM def WHERE key='unit.fighter'),(SELECT id FROM def WHERE key='unit.jet_fighter'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_fighter'),'unit','fighter',330,4,(SELECT id FROM armor_class WHERE key='vehicle'),6000,1,350,'per_type',(SELECT id FROM def WHERE key='unit.jet_fighter'),(SELECT id FROM def WHERE key='age.automation'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_fighter'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_fighter'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_fighter'),(SELECT id FROM tag WHERE key='aircraft'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_fighter'),(SELECT id FROM tag WHERE key='drone'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.drone_fighter'),(SELECT id FROM movement_class WHERE key='aircraft'),4000);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.drone_fighter'),44,(SELECT id FROM damage_class WHERE key='bullet'),5000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.drone_fighter')),'air');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='unit.drone_fighter'),'train','units',3,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.drone_fighter'),(SELECT id FROM def WHERE key='res.metal'),176,3);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.drone_fighter'),(SELECT id FROM def WHERE key='res.oil'),88,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.drone_fighter','Upgrade to Drone Fighters');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_fighter'),'unit_upgrade','',0,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_fighter'),(SELECT id FROM def WHERE key='res.metal'),352,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_fighter'),(SELECT id FROM def WHERE key='res.oil'),176,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_fighter'),'upgrade_type',(SELECT id FROM def WHERE key='unit.jet_fighter'),(SELECT id FROM def WHERE key='unit.drone_fighter'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.interceptor'),'unit','fighter',352,4,(SELECT id FROM armor_class WHERE key='vehicle'),6000,1,375,'per_type',(SELECT id FROM def WHERE key='unit.drone_fighter'),(SELECT id FROM def WHERE key='age.fusion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.interceptor'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.interceptor'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.interceptor'),(SELECT id FROM tag WHERE key='aircraft'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.interceptor'),(SELECT id FROM movement_class WHERE key='aircraft'),4000);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.interceptor'),47,(SELECT id FROM damage_class WHERE key='bullet'),5000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.interceptor')),'air');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='unit.interceptor'),'train','units',4,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.interceptor'),(SELECT id FROM def WHERE key='res.metal'),188,3);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.interceptor'),(SELECT id FROM def WHERE key='res.oil'),94,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.interceptor','Upgrade to Interceptors');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.interceptor'),'unit_upgrade','',0,750);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.interceptor'),(SELECT id FROM def WHERE key='res.metal'),376,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.interceptor'),(SELECT id FROM def WHERE key='res.oil'),188,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.interceptor'),'upgrade_type',(SELECT id FROM def WHERE key='unit.drone_fighter'),(SELECT id FROM def WHERE key='unit.interceptor'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.bomber'),'unit','bomber',285,3,(SELECT id FROM armor_class WHERE key='vehicle'),6000,1,300,'per_type',NULL,(SELECT id FROM def WHERE key='age.modern'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.bomber'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.bomber'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.bomber'),(SELECT id FROM tag WHERE key='aircraft'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.bomber'),(SELECT id FROM movement_class WHERE key='aircraft'),4000);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.bomber'),38,(SELECT id FROM damage_class WHERE key='explosive'),5000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.bomber')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='unit.bomber'),'train','units',0,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.bomber'),(SELECT id FROM def WHERE key='res.metal'),152,3);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.bomber'),(SELECT id FROM def WHERE key='res.oil'),76,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.strategic_bomber'),'unit','bomber',307,3,(SELECT id FROM armor_class WHERE key='vehicle'),6000,1,325,'per_type',(SELECT id FROM def WHERE key='unit.bomber'),(SELECT id FROM def WHERE key='age.information'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.strategic_bomber'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.strategic_bomber'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.strategic_bomber'),(SELECT id FROM tag WHERE key='aircraft'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.strategic_bomber'),(SELECT id FROM movement_class WHERE key='aircraft'),4000);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.strategic_bomber'),41,(SELECT id FROM damage_class WHERE key='explosive'),5000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.strategic_bomber')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='unit.strategic_bomber'),'train','units',1,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.strategic_bomber'),(SELECT id FROM def WHERE key='res.metal'),164,3);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.strategic_bomber'),(SELECT id FROM def WHERE key='res.oil'),82,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.strategic_bomber','Upgrade to Strategic Bombers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.strategic_bomber'),'unit_upgrade','',0,650);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.strategic_bomber'),(SELECT id FROM def WHERE key='res.metal'),328,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.strategic_bomber'),(SELECT id FROM def WHERE key='res.oil'),164,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.strategic_bomber'),'upgrade_type',(SELECT id FROM def WHERE key='unit.bomber'),(SELECT id FROM def WHERE key='unit.strategic_bomber'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_bomber'),'unit','bomber',330,4,(SELECT id FROM armor_class WHERE key='vehicle'),6000,1,350,'per_type',(SELECT id FROM def WHERE key='unit.strategic_bomber'),(SELECT id FROM def WHERE key='age.automation'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_bomber'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_bomber'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_bomber'),(SELECT id FROM tag WHERE key='aircraft'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.drone_bomber'),(SELECT id FROM tag WHERE key='drone'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.drone_bomber'),(SELECT id FROM movement_class WHERE key='aircraft'),4000);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.drone_bomber'),44,(SELECT id FROM damage_class WHERE key='explosive'),5000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.drone_bomber')),'ground');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='unit.drone_bomber'),'train','units',2,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.drone_bomber'),(SELECT id FROM def WHERE key='res.metal'),176,3);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.drone_bomber'),(SELECT id FROM def WHERE key='res.oil'),88,1);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.drone_bomber','Upgrade to Drone Bombers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_bomber'),'unit_upgrade','',0,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_bomber'),(SELECT id FROM def WHERE key='res.metal'),352,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_bomber'),(SELECT id FROM def WHERE key='res.oil'),176,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_bomber'),'upgrade_type',(SELECT id FROM def WHERE key='unit.strategic_bomber'),(SELECT id FROM def WHERE key='unit.drone_bomber'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.survey_ship'),'unit','starship',1000,5,(SELECT id FROM armor_class WHERE key='shield'),9000,2,400,'per_type',NULL,(SELECT id FROM def WHERE key='age.starfaring'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.survey_ship'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.survey_ship'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.survey_ship'),(SELECT id FROM tag WHERE key='starship'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.survey_ship'),(SELECT id FROM movement_class WHERE key='starship'),3000);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.survey_ship'),75,(SELECT id FROM damage_class WHERE key='energy'),8000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.survey_ship')),'space');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='unit.survey_ship'),'train','units',0,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.survey_ship'),(SELECT id FROM def WHERE key='res.metal'),500,10);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.survey_ship'),(SELECT id FROM def WHERE key='res.energy'),250,5);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.corvette'),'unit','starship',1000,5,(SELECT id FROM armor_class WHERE key='shield'),9000,2,400,'per_type',(SELECT id FROM def WHERE key='unit.survey_ship'),(SELECT id FROM def WHERE key='age.starfaring'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.corvette'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.corvette'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.corvette'),(SELECT id FROM tag WHERE key='starship'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.corvette'),(SELECT id FROM movement_class WHERE key='starship'),3000);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.corvette'),75,(SELECT id FROM damage_class WHERE key='energy'),8000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.corvette')),'space');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='unit.corvette'),'train','units',1,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.corvette'),(SELECT id FROM def WHERE key='res.metal'),500,10);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.corvette'),(SELECT id FROM def WHERE key='res.energy'),250,5);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.corvette','Upgrade to Corvettes');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.corvette'),'unit_upgrade','',0,800);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.corvette'),(SELECT id FROM def WHERE key='res.metal'),1000,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.corvette'),(SELECT id FROM def WHERE key='res.energy'),500,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.corvette'),'upgrade_type',(SELECT id FROM def WHERE key='unit.survey_ship'),(SELECT id FROM def WHERE key='unit.corvette'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.space_frigate'),'unit','starship',1060,5,(SELECT id FROM armor_class WHERE key='shield'),9000,2,425,'per_type',(SELECT id FROM def WHERE key='unit.corvette'),(SELECT id FROM def WHERE key='age.expansion'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.space_frigate'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.space_frigate'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.space_frigate'),(SELECT id FROM tag WHERE key='starship'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.space_frigate'),(SELECT id FROM movement_class WHERE key='starship'),3000);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.space_frigate'),79,(SELECT id FROM damage_class WHERE key='energy'),8000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.space_frigate')),'space');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='unit.space_frigate'),'train','units',2,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.space_frigate'),(SELECT id FROM def WHERE key='res.metal'),530,10);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.space_frigate'),(SELECT id FROM def WHERE key='res.energy'),265,5);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.space_frigate','Upgrade to Space Frigates');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.space_frigate'),'unit_upgrade','',0,850);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.space_frigate'),(SELECT id FROM def WHERE key='res.metal'),1060,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.space_frigate'),(SELECT id FROM def WHERE key='res.energy'),530,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.space_frigate'),'upgrade_type',(SELECT id FROM def WHERE key='unit.corvette'),(SELECT id FROM def WHERE key='unit.space_frigate'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.space_cruiser'),'unit','starship',1120,6,(SELECT id FROM armor_class WHERE key='shield'),9000,2,450,'per_type',(SELECT id FROM def WHERE key='unit.space_frigate'),(SELECT id FROM def WHERE key='age.abundance'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.space_cruiser'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.space_cruiser'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.space_cruiser'),(SELECT id FROM tag WHERE key='starship'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.space_cruiser'),(SELECT id FROM movement_class WHERE key='starship'),3000);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.space_cruiser'),84,(SELECT id FROM damage_class WHERE key='energy'),8000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.space_cruiser')),'space');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='unit.space_cruiser'),'train','units',3,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.space_cruiser'),(SELECT id FROM def WHERE key='res.metal'),560,11);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.space_cruiser'),(SELECT id FROM def WHERE key='res.energy'),280,5);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.space_cruiser','Upgrade to Space Cruisers');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.space_cruiser'),'unit_upgrade','',0,900);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.space_cruiser'),(SELECT id FROM def WHERE key='res.metal'),1120,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.space_cruiser'),(SELECT id FROM def WHERE key='res.energy'),560,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.space_cruiser'),'upgrade_type',(SELECT id FROM def WHERE key='unit.space_frigate'),(SELECT id FROM def WHERE key='unit.space_cruiser'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.battleship'),'unit','starship',1180,6,(SELECT id FROM armor_class WHERE key='shield'),9000,2,475,'per_type',(SELECT id FROM def WHERE key='unit.space_cruiser'),(SELECT id FROM def WHERE key='age.galactic'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.battleship'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.battleship'),(SELECT id FROM tag WHERE key='military'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.battleship'),(SELECT id FROM tag WHERE key='starship'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.battleship'),(SELECT id FROM movement_class WHERE key='starship'),3000);
INSERT INTO et_weapon(entity_type_id,damage,damage_class_id,range_max,cooldown_ticks) VALUES ((SELECT id FROM def WHERE key='unit.battleship'),88,(SELECT id FROM damage_class WHERE key='energy'),8000,30);
INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.battleship')),'space');
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='unit.battleship'),'train','units',4,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.battleship'),(SELECT id FROM def WHERE key='res.metal'),590,11);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.battleship'),(SELECT id FROM def WHERE key='res.energy'),295,5);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.battleship','Upgrade to Battleships');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.battleship'),'unit_upgrade','',0,950);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.battleship'),(SELECT id FROM def WHERE key='res.metal'),1180,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.battleship'),(SELECT id FROM def WHERE key='res.energy'),590,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.battleship'),'upgrade_type',(SELECT id FROM def WHERE key='unit.space_cruiser'),(SELECT id FROM def WHERE key='unit.battleship'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),'unit','civilian',40,0,(SELECT id FROM armor_class WHERE key='flesh'),2000,1,150,'per_type',NULL,(SELECT id FROM def WHERE key='age.ancient'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM tag WHERE key='civilian'));
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),'unit','civilian',40,4,(SELECT id FROM armor_class WHERE key='flesh'),2000,1,350,'per_type',(SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='age.automation'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM tag WHERE key='civilian'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM tag WHERE key='drone'));
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.scholar'),'unit','scholar',50,0,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,175,'per_type',NULL,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.scholar'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.scholar'),(SELECT id FROM tag WHERE key='civilian'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.scholar'),(SELECT id FROM movement_class WHERE key='foot'),1300);
INSERT INTO et_harvester(entity_type_id,resource_id,amount) VALUES ((SELECT id FROM def WHERE key='unit.scholar'),(SELECT id FROM def WHERE key='res.knowledge'),10);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM def WHERE key='unit.scholar'),'train','units',0,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.scholar'),(SELECT id FROM def WHERE key='res.wealth'),46,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.merchant'),'unit','merchant',50,0,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,150,'per_type',NULL,(SELECT id FROM def WHERE key='age.ancient'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.merchant'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.merchant'),(SELECT id FROM tag WHERE key='civilian'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.merchant'),(SELECT id FROM movement_class WHERE key='foot'),1500);
INSERT INTO et_trader(entity_type_id,runs_routes,claims_nodes,route_resource_id,route_income) VALUES ((SELECT id FROM def WHERE key='unit.merchant'),1,0,(SELECT id FROM def WHERE key='res.wealth'),3);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.market'),(SELECT id FROM def WHERE key='unit.merchant'),'train','units',0,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.merchant'),(SELECT id FROM def WHERE key='res.wealth'),40,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.caravan'),'unit','caravan',90,0,(SELECT id FROM armor_class WHERE key='vehicle'),6000,1,175,'per_type',NULL,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.caravan'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.caravan'),(SELECT id FROM tag WHERE key='civilian'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.caravan'),(SELECT id FROM movement_class WHERE key='wheeled'),1300);
INSERT INTO et_trader(entity_type_id,runs_routes,claims_nodes,route_resource_id,route_income) VALUES ((SELECT id FROM def WHERE key='unit.caravan'),1,0,(SELECT id FROM def WHERE key='res.wealth'),6);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.market'),(SELECT id FROM def WHERE key='unit.caravan'),'train','units',1,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.caravan'),(SELECT id FROM def WHERE key='res.food'),46,1);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.caravan'),(SELECT id FROM def WHERE key='res.timber'),69,1);
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.freight_wagon'),'unit','caravan',130,2,(SELECT id FROM armor_class WHERE key='vehicle'),6000,1,250,'per_type',(SELECT id FROM def WHERE key='unit.caravan'),(SELECT id FROM def WHERE key='age.enlightenment'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.freight_wagon'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.freight_wagon'),(SELECT id FROM tag WHERE key='civilian'));
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.lorry'),'unit','caravan',170,3,(SELECT id FROM armor_class WHERE key='vehicle'),6000,1,300,'per_type',(SELECT id FROM def WHERE key='unit.freight_wagon'),(SELECT id FROM def WHERE key='age.modern'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.lorry'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.lorry'),(SELECT id FROM tag WHERE key='civilian'));
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.cargo_drone'),'unit','caravan',210,4,(SELECT id FROM armor_class WHERE key='vehicle'),6000,1,350,'per_type',(SELECT id FROM def WHERE key='unit.lorry'),(SELECT id FROM def WHERE key='age.automation'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cargo_drone'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cargo_drone'),(SELECT id FROM tag WHERE key='civilian'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.cargo_drone'),(SELECT id FROM tag WHERE key='drone'));
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,build_ticks,ramp_mode,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='unit.priest'),'unit','priest',35,0,(SELECT id FROM armor_class WHERE key='flesh'),6000,1,175,'per_type',NULL,(SELECT id FROM def WHERE key='age.classical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.priest'),(SELECT id FROM tag WHERE key='biological'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.priest'),(SELECT id FROM tag WHERE key='civilian'));
INSERT INTO et_movement(entity_type_id,movement_class_id,speed) VALUES ((SELECT id FROM def WHERE key='unit.priest'),(SELECT id FROM movement_class WHERE key='foot'),1100);
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.priest'),(SELECT id FROM tag WHERE key='clergy'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='unit.priest'),(SELECT id FROM tag WHERE key='healer'));
INSERT INTO et_healer(entity_type_id,target_tag_id,hp_per_second,range,energy_per_hp,can_convert) VALUES ((SELECT id FROM def WHERE key='unit.priest'),(SELECT id FROM tag WHERE key='biological'),4,5000,0,1);
INSERT INTO et_energy(entity_type_id,max,regen_per_second) VALUES ((SELECT id FROM def WHERE key='unit.priest'),100,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.temple'),(SELECT id FROM def WHERE key='unit.priest'),'train','units',0,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='unit.priest'),(SELECT id FROM def WHERE key='res.wealth'),46,1);
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
INSERT INTO def(kind,key,name) VALUES ('tech','tech.bombardment','Bombardment');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.bombardment'),'upgrade','',0,500);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.bombardment'),(SELECT id FROM def WHERE key='res.wealth'),500,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.bombardment'),(SELECT id FROM def WHERE key='res.knowledge'),250,0);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.bombardment'),'border_radius','add',3000,(SELECT id FROM tag WHERE key='fort'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.bombardment'),'range','add',1000,(SELECT id FROM tag WHERE key='fort'));
INSERT INTO def(kind,key,name) VALUES ('tech','tech.strategic_reserves','Strategic Reserves');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.strategic_reserves'),'upgrade','',0,600);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.strategic_reserves'),(SELECT id FROM def WHERE key='res.wealth'),700,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.strategic_reserves'),(SELECT id FROM def WHERE key='res.knowledge'),350,0);
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.strategic_reserves'),'border_radius','add',3000,(SELECT id FROM tag WHERE key='fort'));
INSERT INTO modifier(source_def_id,stat_key,op,value,target_tag_id) VALUES ((SELECT id FROM def WHERE key='tech.strategic_reserves'),'range','add',1000,(SELECT id FROM tag WHERE key='fort'));
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,limit_scope,limit_count,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_farm'),'building','economic',720,3,(SELECT id FROM armor_class WHERE key='structure'),12000,4,4,350,'per_city',5,(SELECT id FROM def WHERE key='bld.farm'),(SELECT id FROM def WHERE key='age.automation'));
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,limit_scope,limit_count,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_woodcutter_camp'),'building','economic',900,3,(SELECT id FROM armor_class WHERE key='structure'),12000,4,4,350,'none',0,(SELECT id FROM def WHERE key='bld.woodcutter_camp'),(SELECT id FROM def WHERE key='age.automation'));
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,limit_scope,limit_count,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_mine'),'building','economic',1080,3,(SELECT id FROM armor_class WHERE key='structure'),12000,4,4,350,'none',0,(SELECT id FROM def WHERE key='bld.mine'),(SELECT id FROM def WHERE key='age.automation'));
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,limit_scope,limit_count,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_oil_well'),'building','economic',1080,3,(SELECT id FROM armor_class WHERE key='structure'),12000,2,2,350,'none',0,(SELECT id FROM def WHERE key='bld.oil_well'),(SELECT id FROM def WHERE key='age.automation'));
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
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,footprint_w,footprint_h,build_ticks,limit_scope,limit_count,upgrades_from_id,first_age_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_university'),'building','economic',1440,3,(SELECT id FROM armor_class WHERE key='structure'),12000,4,4,350,'per_city',1,(SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM def WHERE key='age.automation'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_university'),(SELECT id FROM tag WHERE key='building'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_university'),(SELECT id FROM tag WHERE key='mechanical'));
INSERT INTO def_tag(def_id,tag_id) VALUES ((SELECT id FROM def WHERE key='bld.auto_university'),(SELECT id FROM tag WHERE key='gather'));
INSERT INTO et_producer(entity_type_id,queue_limit,parallel,has_rally) SELECT (SELECT id FROM def WHERE key='bld.auto_university'),queue_limit,parallel,has_rally FROM et_producer WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.university');
INSERT INTO et_keeper(entity_type_id,capacity) SELECT (SELECT id FROM def WHERE key='bld.auto_university'),capacity FROM et_keeper WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.university');
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable) SELECT (SELECT id FROM def WHERE key='bld.auto_university'),is_city,border_radius,economic_radius,capturable FROM et_territory WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.university');
INSERT INTO placement_rule(entity_type_id,rule,ref_def_id,layer,distance) SELECT (SELECT id FROM def WHERE key='bld.auto_university'),rule,ref_def_id,layer,distance FROM placement_rule WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.university');
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income) SELECT (SELECT id FROM def WHERE key='bld.auto_university'),resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income FROM et_gather_site WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.university');
UPDATE et_gather_site SET slots_base=0,slots_max=0,slots_per_node=0,self_income=70 WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.auto_university');
INSERT INTO def(kind,key,name) VALUES ('tech','tech.up.auto_university','Upgrade to Automated University');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_university'),'upgrade','',0,700);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_university'),(SELECT id FROM def WHERE key='res.metal'),400,0);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_university'),(SELECT id FROM def WHERE key='res.knowledge'),400,0);
INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_university'),'upgrade_type',(SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM def WHERE key='bld.auto_university'));
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.citizen'),(SELECT id FROM def WHERE key='bld.auto_university'),'construct','economic_2',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='unit.worker_drone'),(SELECT id FROM def WHERE key='bld.auto_university'),'construct','economic_2',3,0);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.continental_union','Continental Union');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.continental_union'),'project','',0,3000);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.continental_union'),(SELECT id FROM def WHERE key='res.knowledge'),1000,0);
INSERT INTO def(kind,key,name) VALUES ('tech','tech.unification','Unification');
INSERT INTO tech(id,kind,track,level,research_ticks) VALUES ((SELECT id FROM def WHERE key='tech.unification'),'project','',0,4000);
INSERT INTO cost(def_id,resource_id,amount,ramp) VALUES ((SELECT id FROM def WHERE key='tech.unification'),(SELECT id FROM def WHERE key='res.knowledge'),1000,0);
INSERT INTO tech_effect(tech_id,effect,target_def_id) VALUES ((SELECT id FROM def WHERE key='tech.unification'),'change_faction',(SELECT id FROM def WHERE key='race.human'));
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
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='tech.up.carrack'),'research','research',1,0);
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
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='tech.up.frigate'),'research','research',2,0);
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
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.bastion'),(SELECT id FROM def WHERE key='tech.up.bunker'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.star_fort'),(SELECT id FROM def WHERE key='tech.up.fortress'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.infantry'),'research','research',1,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.heavy_machine_gunner'),'research','research',2,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.bazooka_team'),'research','research',3,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='tech.up.scout_car'),'research','research',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='tech.up.tank'),'research','research',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.factory'),(SELECT id FROM def WHERE key='tech.up.howitzer'),'research','research',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='tech.up.destroyer'),'research','research',4,0);
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
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.dock'),(SELECT id FROM def WHERE key='tech.up.missile_cruiser'),'research','research',0,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='tech.up.jet_fighter'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.airfield'),(SELECT id FROM def WHERE key='tech.up.strategic_bomber'),'research','research',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.military.8'),'research','research_3',1,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.civic.8'),'research','research_3',2,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.commerce.8'),'research','research_3',3,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.science.8'),'research','research_3',4,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.age.automation'),'research','research_3',0,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.refinery'),(SELECT id FROM def WHERE key='tech.petrochemistry'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.tower'),(SELECT id FROM def WHERE key='tech.nationalism'),'research','research',4,0);
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
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.university'),(SELECT id FROM def WHERE key='tech.up.auto_university'),'research','research',0,1);
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
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.marine'),'research','research_2',2,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='tech.up.grav_skimmer'),'research','research',3,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='tech.up.corvette'),'research','research',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.military.11'),'research','research_4',1,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.civic.11'),'research','research_4',2,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.commerce.11'),'research','research_4',3,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.science.11'),'research','research_4',4,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.age.expansion'),'research','research_4',0,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),(SELECT id FROM def WHERE key='tech.superconducting_grid'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.unification'),'research','projects',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.beam_trooper'),'research','research_2',3,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.vehicle_plant'),(SELECT id FROM def WHERE key='tech.up.grav_tank'),'research','research',4,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='tech.up.space_frigate'),'research','research',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.military.12'),'research','research_4',1,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.civic.12'),'research','research_4',2,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.commerce.12'),'research','research_4',3,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.science.12'),'research','research_4',4,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.age.abundance'),'research','research_4',0,2);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.fusion_plant'),(SELECT id FROM def WHERE key='tech.stellar_collectors'),'research','research',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.barracks'),(SELECT id FROM def WHERE key='tech.up.shock_marine'),'research','research_2',4,1);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='tech.up.space_cruiser'),'research','research',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.military.13'),'research','research_5',1,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.civic.13'),'research','research_5',2,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.commerce.13'),'research','research_5',3,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.science.13'),'research','research_5',4,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.library'),(SELECT id FROM def WHERE key='tech.age.galactic'),'research','research_5',0,0);
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row) VALUES ((SELECT id FROM def WHERE key='bld.spaceport'),(SELECT id FROM def WHERE key='tech.up.battleship'),'research','research',3,0);
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
INSERT INTO production SELECT (SELECT id FROM def WHERE key='bld.auto_farm'),product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id=(SELECT id FROM def WHERE key='bld.farm');
INSERT INTO production SELECT (SELECT id FROM def WHERE key='bld.auto_woodcutter_camp'),product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id=(SELECT id FROM def WHERE key='bld.woodcutter_camp');
INSERT INTO production SELECT (SELECT id FROM def WHERE key='bld.auto_mine'),product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id=(SELECT id FROM def WHERE key='bld.mine');
INSERT INTO production SELECT (SELECT id FROM def WHERE key='bld.auto_oil_well'),product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id=(SELECT id FROM def WHERE key='bld.oil_well');
INSERT INTO production SELECT (SELECT id FROM def WHERE key='bld.auto_university'),product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id=(SELECT id FROM def WHERE key='bld.university');
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
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.tower'),(SELECT id FROM def WHERE key='age.ancient'),0);
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
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.carrack'),(SELECT id FROM def WHERE key='age.gunpowder'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.carrack'),(SELECT id FROM def WHERE key='tech.up.war_galley'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.carrack'),(SELECT id FROM def WHERE key='tech.up.carrack'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.frigate'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.frigate'),(SELECT id FROM def WHERE key='age.enlightenment'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.frigate'),(SELECT id FROM def WHERE key='tech.up.carrack'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.frigate'),(SELECT id FROM def WHERE key='tech.up.frigate'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.ironclad'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.ironclad'),(SELECT id FROM def WHERE key='age.industrial'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.ironclad'),(SELECT id FROM def WHERE key='tech.up.frigate'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.ironclad'),(SELECT id FROM def WHERE key='tech.up.ironclad'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.destroyer'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.destroyer'),(SELECT id FROM def WHERE key='age.modern'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.destroyer'),(SELECT id FROM def WHERE key='tech.up.ironclad'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.destroyer'),(SELECT id FROM def WHERE key='tech.up.destroyer'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.missile_cruiser'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_cruiser'),(SELECT id FROM def WHERE key='age.information'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.missile_cruiser'),(SELECT id FROM def WHERE key='tech.up.destroyer'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.missile_cruiser'),(SELECT id FROM def WHERE key='tech.up.missile_cruiser'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.drone_cruiser'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_cruiser'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.drone_cruiser'),(SELECT id FROM def WHERE key='tech.up.missile_cruiser'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.drone_cruiser'),(SELECT id FROM def WHERE key='tech.up.drone_cruiser'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.hover_frigate'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.hover_frigate'),(SELECT id FROM def WHERE key='age.fusion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.hover_frigate'),(SELECT id FROM def WHERE key='tech.up.drone_cruiser'),1);
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
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.corvette'),(SELECT id FROM def WHERE key='age.starfaring'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.corvette'),(SELECT id FROM def WHERE key='tech.up.corvette'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.space_frigate'),(SELECT id FROM def WHERE key='age.expansion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.space_frigate'),(SELECT id FROM def WHERE key='age.expansion'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.space_frigate'),(SELECT id FROM def WHERE key='tech.up.corvette'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.space_frigate'),(SELECT id FROM def WHERE key='tech.up.space_frigate'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.space_cruiser'),(SELECT id FROM def WHERE key='age.abundance'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.space_cruiser'),(SELECT id FROM def WHERE key='age.abundance'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.space_cruiser'),(SELECT id FROM def WHERE key='tech.up.space_frigate'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.space_cruiser'),(SELECT id FROM def WHERE key='tech.up.space_cruiser'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.battleship'),(SELECT id FROM def WHERE key='age.galactic'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.battleship'),(SELECT id FROM def WHERE key='age.galactic'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.battleship'),(SELECT id FROM def WHERE key='tech.up.space_cruiser'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='unit.battleship'),(SELECT id FROM def WHERE key='tech.up.battleship'),1);
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
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.up.auto_university'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.auto_university'),(SELECT id FROM def WHERE key='tech.up.auto_university'),1);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='bld.auto_university'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.continental_union'),(SELECT id FROM def WHERE key='age.automation'),0);
INSERT INTO prerequisite(def_id,requires_def_id,alt_group) VALUES ((SELECT id FROM def WHERE key='tech.unification'),(SELECT id FROM def WHERE key='age.starfaring'),0);
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

WITH v(p,seq,a,k,n) AS (VALUES
  ('balanced',0,'train','unit.citizen',6), ('balanced',1,'construct','bld.farm',3),
  ('balanced',2,'construct','bld.library',1), ('balanced',3,'train','unit.citizen',10),
  ('balanced',4,'construct','bld.barracks',1), ('balanced',5,'research','tech.age.classical',1),
  ('rusher',0,'train','unit.citizen',5), ('rusher',1,'construct','bld.farm',2),
  ('rusher',2,'construct','bld.barracks',1), ('rusher',3,'train','unit.spearman',6))
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
  ('mat.tar_ground','stone','{"base_colour":"#3b3631","colour":"#4a433b","colour_alt":"#25211e","scale":0.5}'),
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

-- ── composed models of buildings and units ──────────────────────────────────
-- A composed mesh (generator 'composed') is a list of named parts in mesh_part;
-- an entity_visual row gives it to an entity type from an age on (no age: any
-- age). Shapes, knobs and material programs are listed in docs/database.md;
-- positions and sizes are tiles in the model's frame (x east, y up, z south,
-- origin on the ground at the footprint centre, the front facing south).
-- The look and review loop are in docs/art-direction.md.
INSERT INTO def(kind,key,name) VALUES
 ('material','mat.mud_plaster','Mud plaster'), ('material','mat.mud_brick','Mud brick'),
 ('material','mat.packed_earth','Packed earth'), ('material','mat.thatch','Thatch'),
 ('material','mat.timber','Timber'), ('material','mat.cloth','Cloth'),
 ('material','mat.fieldstone','Fieldstone'), ('material','mat.limestone','Limestone'),
 ('material','mat.lime_plaster','Lime plaster'), ('material','mat.terracotta','Terracotta tiles'),
 ('material','mat.bronze','Bronze'), ('material','mat.iron','Iron'),
 ('mesh','mesh.house_ancient','Early house'), ('mesh','mesh.city_ancient','Early city'),
 ('mesh','mesh.barracks_ancient','Early barracks'), ('mesh','mesh.library_ancient','Early library'),
 ('mesh','mesh.house_classical','Classical house'), ('mesh','mesh.city_classical','Classical city'),
 ('mesh','mesh.citizen','Citizen');
WITH v(k,prog,params) AS (VALUES
  ('mat.mud_plaster','plaster','{"colour":"#c4a27a","colour_alt":"#a3825c","scale":0.7}'),
  ('mat.mud_brick','brick','{"colour":"#b58a5e","colour_alt":"#8c6b4c","scale":0.32}'),
  ('mat.packed_earth','plaster','{"colour":"#b49b78","colour_alt":"#998262","scale":1.4}'),
  ('mat.thatch','thatch','{"colour":"#c4a465","colour_alt":"#86693a","scale":0.22}'),
  ('mat.timber','timber','{"colour":"#8a6240","colour_alt":"#5a3d26","scale":0.16}'),
  ('mat.cloth','cloth','{"colour":"#e4dccb","colour_alt":"#b7ad99","scale":0.25}'),
  ('mat.fieldstone','stone','{"colour":"#a39b8e","colour_alt":"#736d64","scale":0.35}'),
  ('mat.limestone','brick','{"colour":"#e3dccb","colour_alt":"#bdb4a0","scale":0.62}'),
  ('mat.lime_plaster','plaster','{"colour":"#ebe4d4","colour_alt":"#d0c6b0","scale":0.9}'),
  ('mat.terracotta','roof_tile','{"colour":"#bf5f36","colour_alt":"#8a3b1f","scale":0.2}'),
  ('mat.bronze','metal','{"colour":"#b98d4a","colour_alt":"#7d5a2a","scale":0.3}'),
  ('mat.iron','metal','{"colour":"#8e9196","colour_alt":"#5d6066","scale":0.3}'))
INSERT INTO material(id,program,params) SELECT d.id, v.prog, v.params FROM v JOIN def d ON d.key=v.k;
INSERT INTO mesh(id,generator,params,baked)
 SELECT id, 'composed', '{}', 1 FROM def WHERE kind='mesh' AND key IN
  ('mesh.house_ancient','mesh.city_ancient','mesh.barracks_ancient','mesh.library_ancient',
   'mesh.house_classical','mesh.city_classical','mesh.citizen');
WITH v(m,name,shape,mat,team,attach,params) AS (VALUES
  -- a mud-plastered house under a thatched gable, its ridge in the owner's colour
  ('mesh.house_ancient','walls','box','mat.mud_plaster',0,'','{"w":1.3,"d":1.05,"h":0.95,"gable":42,"door":0.3,"door_h":0.6,"windows":1,"side_windows":1,"window_w":0.17,"window_h":0.19,"window_y":0.5}'),
  ('mesh.house_ancient','roof','gable_roof','mat.thatch',0,'','{"y":0.95,"w":1.3,"d":1.05,"pitch":42,"overhang":0.17,"t":0.1}'),
  ('mesh.house_ancient','ridge','box','mat.cloth',1,'','{"y":1.47,"w":1.62,"d":0.2,"h":0.1}'),
  -- the early city: a hall on a stone base among houses, a porch and two banners
  ('mesh.city_ancient','yard','plinth','mat.packed_earth',0,'','{"w":6.4,"d":6.4,"h":0.05,"steps":1}'),
  ('mesh.city_ancient','hall_base','plinth','mat.fieldstone',0,'','{"z":-0.55,"w":3.5,"d":2.8,"h":0.36,"steps":2,"inset":0.14}'),
  ('mesh.city_ancient','hall','box','mat.mud_brick',0,'','{"y":0.36,"z":-0.55,"w":2.8,"d":2.1,"h":1.75,"gable":40,"door":0.62,"door_h":1.05,"windows":4,"side_windows":2,"window_y":1.0,"window_w":0.2,"window_h":0.3}'),
  ('mesh.city_ancient','hall_roof','gable_roof','mat.thatch',0,'','{"y":2.11,"z":-0.55,"w":2.8,"d":2.1,"pitch":40,"overhang":0.32,"t":0.13}'),
  ('mesh.city_ancient','hall_ridge','box','mat.cloth',1,'','{"y":2.98,"z":-0.55,"w":3.5,"d":0.26,"h":0.12}'),
  ('mesh.city_ancient','porch','awning','mat.cloth',1,'','{"z":0.95,"w":1.7,"d":0.8,"h":1.55,"rise":0.4}'),
  ('mesh.city_ancient','banner','banner','mat.cloth',1,'','{"x":-1.25,"z":1.55,"h":3.3,"fw":0.62,"fh":1.15,"repeat":2,"dx":2.5}'),
  ('mesh.city_ancient','house_sw','include',NULL,0,'','{"mesh":"mesh.house_ancient","x":-2.2,"z":2.2}'),
  ('mesh.city_ancient','house_se','include',NULL,0,'','{"mesh":"mesh.house_ancient","x":2.2,"z":2.2}'),
  ('mesh.city_ancient','house_w','include',NULL,0,'','{"mesh":"mesh.house_ancient","x":-2.45,"z":-1.0,"yaw":90}'),
  ('mesh.city_ancient','house_e','include',NULL,0,'','{"mesh":"mesh.house_ancient","x":2.45,"z":-1.0,"yaw":-90}'),
  ('mesh.city_ancient','house_n','include',NULL,0,'','{"mesh":"mesh.house_ancient","z":-2.6,"yaw":180,"scale":0.9}'),
  ('mesh.city_ancient','well','cylinder','mat.fieldstone',0,'','{"z":2.55,"w":0.55,"h":0.32,"segments":8}'),
  ('mesh.city_ancient','jars','cylinder','mat.mud_plaster',0,'','{"x":-0.95,"z":1.0,"w":0.2,"h":0.26,"segments":6,"taper":0.25,"repeat":3,"dz":0.26}'),
  -- the early barracks: a palisaded yard, a gate with the owner's cloth, a hut and a tall banner
  ('mesh.barracks_ancient','yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('mesh.barracks_ancient','palisade','palisade','mat.timber',0,'','{"w":3.6,"d":3.6,"h":1.2,"t":0.16,"ring":1,"gap":1.1,"spacing":0.24}'),
  ('mesh.barracks_ancient','gate_posts','cylinder','mat.timber',0,'','{"x":-0.62,"z":1.72,"w":0.2,"h":1.75,"segments":6,"repeat":2,"dx":1.24}'),
  ('mesh.barracks_ancient','gate_beam','box','mat.timber',0,'','{"y":1.55,"z":1.72,"w":1.6,"d":0.18,"h":0.15}'),
  ('mesh.barracks_ancient','gate_cloth','box','mat.cloth',1,'','{"y":1.2,"z":1.84,"w":1.15,"d":0.04,"h":0.34}'),
  ('mesh.barracks_ancient','hut','box','mat.mud_plaster',0,'','{"z":-0.85,"w":2.3,"d":1.25,"h":1.05,"gable":38,"door":0.38,"door_h":0.7,"windows":2,"window_y":0.55,"window_w":0.18,"window_h":0.2}'),
  ('mesh.barracks_ancient','hut_roof','gable_roof','mat.thatch',0,'','{"y":1.05,"z":-0.85,"w":2.3,"d":1.25,"pitch":38,"overhang":0.18,"t":0.1}'),
  ('mesh.barracks_ancient','banner','banner','mat.cloth',1,'','{"x":1.15,"z":0.95,"h":3.6,"fw":0.75,"fh":1.3,"notch":0.25}'),
  ('mesh.barracks_ancient','rack','fence','mat.timber',0,'','{"x":-1.1,"z":0.75,"w":1.0,"h":0.55,"spacing":0.5,"rails":1}'),
  ('mesh.barracks_ancient','spears','cylinder','mat.timber',0,'','{"x":-1.43,"z":0.7,"w":0.045,"h":1.45,"segments":4,"repeat":4,"dx":0.22}'),
  ('mesh.barracks_ancient','dummy','cylinder','mat.timber',0,'','{"x":0.2,"z":0.6,"w":0.1,"h":1.1,"segments":5}'),
  ('mesh.barracks_ancient','dummy_body','box','mat.thatch',0,'','{"x":0.2,"y":0.55,"z":0.6,"w":0.36,"d":0.26,"h":0.45,"taper":0.25}'),
  -- the early library: a solid mud-brick block with few windows under a stacked roof
  ('mesh.library_ancient','base','plinth','mat.fieldstone',0,'','{"w":3.6,"d":3.45,"h":0.42,"steps":2,"inset":0.16}'),
  ('mesh.library_ancient','block','box','mat.mud_brick',0,'','{"y":0.42,"w":2.8,"d":2.5,"h":1.65,"door":0.55,"door_h":1.0,"windows":2,"side_windows":1,"window_y":1.12,"window_w":0.15,"window_h":0.24}'),
  ('mesh.library_ancient','porch_posts','column_row','mat.timber',0,'','{"y":0.42,"z":1.52,"count":2,"radius":0.07,"spacing":1.0,"h":1.25,"segments":6,"capital":0}'),
  ('mesh.library_ancient','porch_roof','box','mat.timber',0,'','{"y":1.67,"z":1.4,"w":1.4,"d":0.5,"h":0.1}'),
  ('mesh.library_ancient','lower_roof','hip_roof','mat.thatch',0,'','{"y":2.07,"w":2.8,"d":2.5,"pitch":26,"overhang":0.3,"t":0.12}'),
  ('mesh.library_ancient','upper_block','box','mat.mud_brick',0,'','{"y":2.35,"w":1.5,"d":1.2,"h":0.7,"windows":3,"side_windows":2,"window_y":0.36,"window_w":0.12,"window_h":0.16}'),
  ('mesh.library_ancient','upper_roof','hip_roof','mat.thatch',1,'','{"y":3.05,"w":1.5,"d":1.2,"pitch":34,"overhang":0.22,"t":0.1}'),
  ('mesh.library_ancient','finial','cone','mat.bronze',0,'','{"y":3.38,"w":0.18,"h":0.38,"segments":6}'),
  -- a classical house: lime-plastered walls under a low terracotta roof
  ('mesh.house_classical','walls','box','mat.lime_plaster',0,'','{"w":1.4,"d":1.1,"h":1.05,"gable":24,"door":0.3,"door_h":0.66,"windows":2,"side_windows":1,"window_w":0.16,"window_h":0.24,"window_y":0.55}'),
  ('mesh.house_classical','roof','gable_roof','mat.terracotta',0,'','{"y":1.05,"w":1.4,"d":1.1,"pitch":24,"overhang":0.14,"t":0.08}'),
  ('mesh.house_classical','ridge','box','mat.cloth',1,'','{"y":1.3,"w":1.65,"d":0.16,"h":0.08}'),
  -- the classical city: a columned hall with a pediment on a stepped base among houses
  ('mesh.city_classical','yard','plinth','mat.limestone',0,'','{"w":6.4,"d":6.4,"h":0.05,"steps":1}'),
  ('mesh.city_classical','hall_base','plinth','mat.limestone',0,'','{"z":-0.5,"w":3.7,"d":2.9,"h":0.5,"steps":3,"inset":0.12}'),
  ('mesh.city_classical','cella','box','mat.lime_plaster',0,'','{"y":0.5,"z":-0.65,"w":2.4,"d":1.6,"h":1.7,"door":0.62,"door_h":1.15}'),
  ('mesh.city_classical','front_columns','column_row','mat.limestone',0,'','{"y":0.5,"z":0.62,"count":6,"radius":0.11,"spacing":0.6,"h":1.75,"segments":8}'),
  ('mesh.city_classical','back_columns','column_row','mat.limestone',0,'','{"y":0.5,"z":-1.62,"count":6,"radius":0.11,"spacing":0.6,"h":1.75,"segments":8,"far":0}'),
  ('mesh.city_classical','entablature','box','mat.limestone',0,'','{"y":2.25,"z":-0.5,"w":3.45,"d":2.6,"h":0.22}'),
  ('mesh.city_classical','pediment','box','mat.limestone',0,'','{"y":2.45,"z":-0.5,"w":3.4,"d":2.55,"h":0.02,"gable":20}'),
  ('mesh.city_classical','roof','gable_roof','mat.terracotta',0,'','{"y":2.47,"z":-0.5,"w":3.4,"d":2.55,"pitch":20,"overhang":0.14,"t":0.1}'),
  ('mesh.city_classical','ridge','box','mat.cloth',1,'','{"y":2.96,"z":-0.5,"w":3.7,"d":0.22,"h":0.1}'),
  ('mesh.city_classical','banner','banner','mat.cloth',1,'','{"x":-1.4,"z":1.55,"h":3.3,"fw":0.6,"fh":1.15,"repeat":2,"dx":2.8}'),
  ('mesh.city_classical','house_sw','include',NULL,0,'','{"mesh":"mesh.house_classical","x":-2.2,"z":2.2}'),
  ('mesh.city_classical','house_se','include',NULL,0,'','{"mesh":"mesh.house_classical","x":2.2,"z":2.2}'),
  ('mesh.city_classical','house_w','include',NULL,0,'','{"mesh":"mesh.house_classical","x":-2.5,"z":-1.0,"yaw":90}'),
  ('mesh.city_classical','house_e','include',NULL,0,'','{"mesh":"mesh.house_classical","x":2.5,"z":-1.0,"yaw":-90}'),
  ('mesh.city_classical','house_n','include',NULL,0,'','{"mesh":"mesh.house_classical","z":-2.65,"yaw":180,"scale":0.9}'),
  ('mesh.city_classical','fountain','cylinder','mat.limestone',0,'','{"z":2.5,"w":0.7,"h":0.25,"segments":8}'),
  ('mesh.city_classical','statue','box','mat.bronze',0,'','{"z":2.5,"y":0.25,"w":0.16,"d":0.16,"h":0.55,"taper":0.3}'),
  -- the citizen: a standing figure in a tunic of the owner's colour, holding a hoe
  ('mesh.citizen','body','figure','mat.cloth',1,'','{"h":1.0}'),
  ('mesh.citizen','hoe','cylinder','mat.timber',0,'right_hand','{"y":-0.4,"w":0.035,"h":0.8,"segments":4}'),
  ('mesh.citizen','blade','box','mat.iron',0,'hoe','{"z":0.06,"w":0.06,"d":0.17,"h":0.08}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT m.id, v.name, v.shape, mt.id, v.team, v.attach, v.params
 FROM v JOIN def m ON m.key=v.m LEFT JOIN def mt ON mt.key=v.mat;
WITH v(e,a,m,mat) AS (VALUES
  ('bld.city','age.ancient','mesh.city_ancient','mat.mud_plaster'),
  ('bld.city','age.classical','mesh.city_classical','mat.limestone'),
  ('bld.barracks','age.ancient','mesh.barracks_ancient','mat.mud_plaster'),
  ('bld.library','age.ancient','mesh.library_ancient','mat.mud_brick'),
  ('unit.citizen',NULL,'mesh.citizen','mat.cloth'))
INSERT INTO entity_visual(entity_type_id,age_id,mesh_id,material_id)
 SELECT e.id, a.id, m.id, mt.id FROM v JOIN def e ON e.key=v.e LEFT JOIN def a ON a.key=v.a
 JOIN def m ON m.key=v.m JOIN def mt ON mt.key=v.mat;

-- ── buildings of the ancient and classical ages (task 38) ──────────────────
-- First looks of the remaining early buildings and the classical looks of the
-- ancient ones, in the style of the approved sample above. `pitch` and `roll`
-- lean a part (logs, braces, a crane); `arch` and `hull` are kit shapes too.
INSERT INTO def(kind,key,name) VALUES
 ('material','mat.sawn_timber','Sawn timber'), ('material','mat.spoil','Spoil'),
 ('material','mat.soil','Tilled soil'), ('material','mat.hide','Horse hide'),
 ('mesh','mesh.log_pile','Log pile'), ('mesh','mesh.stall','Market stall'),
 ('mesh','mesh.silo','Silo'), ('mesh','mesh.horse','Horse'),
 ('mesh','mesh.farm_ancient','Early farm'),
 ('mesh','mesh.woodcutter_ancient','Early woodcutter''s camp'),
 ('mesh','mesh.woodcutter_classical','Classical woodcutter''s camp'),
 ('mesh','mesh.market_ancient','Early market'), ('mesh','mesh.market_classical','Classical market'),
 ('mesh','mesh.tower_ancient','Early watchtower'), ('mesh','mesh.tower_classical','Classical tower'),
 ('mesh','mesh.dock_ancient','Early dock'), ('mesh','mesh.dock_classical','Classical harbour'),
 ('mesh','mesh.barracks_classical','Classical barracks'),
 ('mesh','mesh.library_classical','Classical library'),
 ('mesh','mesh.granary_classical','Classical granary'),
 ('mesh','mesh.lumber_mill_classical','Classical lumber mill'),
 ('mesh','mesh.mine_classical','Classical mine'),
 ('mesh','mesh.university_classical','Classical university'),
 ('mesh','mesh.temple_classical','Classical temple'),
 ('mesh','mesh.stable_classical','Classical stable'),
 ('mesh','mesh.siege_workshop_classical','Classical siege workshop'),
 ('mesh','mesh.catapult_frame','Catapult frame');
WITH v(k,prog,params) AS (VALUES
  ('mat.sawn_timber','timber','{"colour":"#c39a68","colour_alt":"#94704a","scale":0.12}'),
  ('mat.spoil','stone','{"colour":"#857a6c","colour_alt":"#5b5248","scale":0.28}'),
  ('mat.soil','plaster','{"colour":"#6b4f34","colour_alt":"#4f3925","scale":0.8}'),
  ('mat.hide','plaster','{"colour":"#8a5a36","colour_alt":"#5f3c24","scale":0.35}'))
INSERT INTO material(id,program,params) SELECT d.id, v.prog, v.params FROM v JOIN def d ON d.key=v.k;
INSERT INTO mesh(id,generator,params,baked)
 SELECT id, 'composed', '{}', 1 FROM def WHERE kind='mesh' AND key IN
  ('mesh.log_pile','mesh.stall','mesh.silo','mesh.horse','mesh.farm_ancient',
   'mesh.woodcutter_ancient','mesh.woodcutter_classical','mesh.market_ancient',
   'mesh.market_classical','mesh.tower_ancient','mesh.tower_classical','mesh.dock_ancient',
   'mesh.dock_classical','mesh.barracks_classical','mesh.library_classical',
   'mesh.granary_classical','mesh.lumber_mill_classical','mesh.mine_classical',
   'mesh.university_classical','mesh.temple_classical','mesh.stable_classical',
   'mesh.siege_workshop_classical','mesh.catapult_frame');
WITH v(m,name,shape,mat,team,attach,params) AS (VALUES
  -- a pile of logs lying along x, 1.6 long, three rows high
  ('mesh.log_pile','logs_low','cylinder','mat.timber',0,'','{"x":-0.8,"y":0.15,"z":-0.45,"w":0.3,"h":1.6,"segments":6,"roll":90,"repeat":4,"dz":0.3}'),
  ('mesh.log_pile','logs_mid','cylinder','mat.timber',0,'','{"x":-0.72,"y":0.41,"z":-0.3,"w":0.3,"h":1.5,"segments":6,"roll":90,"repeat":3,"dz":0.3,"far":0,"tint":"#e0d2c0"}'),
  ('mesh.log_pile','logs_top','cylinder','mat.timber',0,'','{"x":-0.84,"y":0.67,"z":-0.15,"w":0.3,"h":1.62,"segments":6,"roll":90,"repeat":2,"dz":0.3,"far":0}'),
  ('mesh.log_pile','stakes','cylinder','mat.timber',0,'','{"x":-0.5,"z":-0.66,"w":0.07,"h":0.8,"segments":4,"repeat":2,"dx":1.0,"far":0}'),
  ('mesh.log_pile','stakes_front','cylinder','mat.timber',0,'','{"x":-0.5,"z":0.66,"w":0.07,"h":0.8,"segments":4,"repeat":2,"dx":1.0,"far":0}'),
  -- a market stall: a table of goods under an awning striped in the owner's colour
  ('mesh.stall','table','box','mat.timber',0,'','{"z":0.15,"w":1.3,"d":0.55,"h":0.5}'),
  ('mesh.stall','jars','cylinder','mat.mud_plaster',0,'','{"x":-0.45,"y":0.5,"z":0.15,"w":0.16,"h":0.24,"segments":6,"taper":0.3,"repeat":3,"dx":0.17}'),
  ('mesh.stall','basket','box','mat.thatch',0,'','{"x":0.2,"y":0.5,"z":0.15,"w":0.32,"d":0.3,"h":0.1,"far":0}'),
  ('mesh.stall','greens','dome','mat.cloth',0,'','{"x":0.2,"y":0.6,"z":0.15,"w":0.28,"h":0.1,"segments":6,"tint":"#8fc04a"}'),
  ('mesh.stall','fruit','dome','mat.cloth',0,'','{"x":0.48,"y":0.5,"z":0.15,"w":0.26,"h":0.13,"segments":6,"tint":"#f2a03a"}'),
  ('mesh.stall','awning','awning','mat.cloth',0,'','{"w":1.6,"d":1.1,"h":1.3,"rise":0.35,"posts":0}'),
  ('mesh.stall','stripes','awning','mat.cloth',1,'','{"x":-0.53,"y":0.012,"w":0.18,"d":1.1,"h":1.3,"rise":0.35,"posts":0,"repeat":3,"dx":0.53}'),
  ('mesh.stall','posts_front','cylinder','mat.timber',0,'','{"x":-0.75,"z":0.5,"w":0.07,"h":1.3,"segments":4,"repeat":2,"dx":1.5,"far":0}'),
  ('mesh.stall','posts_back','cylinder','mat.timber',0,'','{"x":-0.75,"z":-0.5,"w":0.07,"h":1.65,"segments":4,"repeat":2,"dx":1.5,"far":0}'),
  ('mesh.stall','crate','box','mat.timber',0,'','{"x":0.6,"z":-0.3,"w":0.3,"d":0.3,"h":0.3,"far":0}'),
  ('mesh.stall','sacks','dome','mat.cloth',0,'','{"x":-0.55,"z":-0.25,"w":0.32,"h":0.36,"segments":6,"tint":"#c9b48a","far":0}'),
  -- a silo: a plastered drum on a stone footing, a band in the owner's colour, a tiled cone
  ('mesh.silo','footing','cylinder','mat.fieldstone',0,'','{"w":1.25,"h":0.22,"segments":10}'),
  ('mesh.silo','drum','cylinder','mat.lime_plaster',0,'','{"y":0.22,"w":1.14,"h":1.4,"segments":10}'),
  ('mesh.silo','band','cylinder','mat.cloth',1,'','{"y":1.12,"w":1.17,"h":0.3,"segments":10}'),
  ('mesh.silo','cap','cone','mat.terracotta',0,'','{"y":1.62,"w":1.32,"h":0.62,"segments":10}'),
  ('mesh.silo','tip','cone','mat.cloth',1,'','{"y":1.925,"w":0.69,"h":0.325,"segments":10}'),
  ('mesh.silo','hatch','box','mat.timber',0,'','{"y":0.35,"z":0.555,"w":0.3,"d":0.04,"h":0.45,"far":0}'),
  -- a horse standing along z, a blanket in the owner's colour
  ('mesh.horse','body','box','mat.hide',0,'','{"y":0.52,"w":0.34,"d":0.92,"h":0.36}'),
  ('mesh.horse','legs_front','box','mat.hide',0,'','{"x":-0.11,"z":0.32,"w":0.09,"d":0.1,"h":0.55,"repeat":2,"dx":0.22,"far":0}'),
  ('mesh.horse','legs_back','box','mat.hide',0,'','{"x":-0.11,"z":-0.32,"w":0.09,"d":0.1,"h":0.55,"repeat":2,"dx":0.22,"far":0}'),
  ('mesh.horse','neck','box','mat.hide',0,'','{"y":0.7,"z":0.36,"w":0.16,"d":0.24,"h":0.5,"pitch":38}'),
  ('mesh.horse','head','box','mat.hide',0,'neck','{"y":0.4,"z":0.08,"w":0.15,"d":0.4,"h":0.17,"pitch":14}'),
  ('mesh.horse','mane','box','mat.timber',0,'neck','{"y":0.05,"z":-0.12,"w":0.05,"d":0.05,"h":0.5,"tint":"#5a4a40","far":0}'),
  ('mesh.horse','tail','box','mat.timber',0,'','{"y":0.84,"z":-0.46,"w":0.07,"d":0.07,"h":0.45,"pitch":-160,"tint":"#5a4a40","far":0}'),
  ('mesh.horse','blanket','box','mat.cloth',1,'','{"y":0.66,"z":-0.02,"w":0.38,"d":0.44,"h":0.24}'),
  -- the early farm: the crop rows (drawn by their own visual) and a small barn in a corner
  ('mesh.farm_ancient','soil','plinth','mat.soil',0,'','{"w":3.8,"d":3.8,"h":0.03,"steps":1}'),
  ('mesh.farm_ancient','barn','box','mat.timber',0,'','{"x":-1.2,"z":-1.25,"w":1.1,"d":0.9,"h":0.8,"gable":40,"door":0.42,"door_h":0.6,"door_colour":"#3a2a1c"}'),
  ('mesh.farm_ancient','barn_roof','gable_roof','mat.thatch',0,'','{"x":-1.2,"y":0.8,"z":-1.25,"w":1.1,"d":0.9,"pitch":40,"overhang":0.12,"t":0.08}'),
  ('mesh.farm_ancient','ridge_cap','gable_roof','mat.cloth',1,'','{"x":-1.2,"y":1.156,"z":-1.25,"w":1.34,"d":0.3,"pitch":40,"overhang":0,"t":0.03}'),
  ('mesh.farm_ancient','barn_ridge','box','mat.cloth',1,'','{"x":-1.2,"y":1.254,"z":-1.25,"w":1.35,"d":0.16,"h":0.08}'),
  ('mesh.farm_ancient','flag','banner','mat.cloth',1,'','{"x":-0.55,"z":-0.75,"h":1.7,"fw":0.36,"fh":0.5}'),
  -- the early woodcutter's camp: log piles and a saw-horse under an open thatched shed
  ('mesh.woodcutter_ancient','yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('mesh.woodcutter_ancient','posts_back','cylinder','mat.timber',0,'','{"x":-1.5,"z":-1.55,"w":0.14,"h":1.3,"segments":6,"repeat":2,"dx":1.9}'),
  ('mesh.woodcutter_ancient','posts_front','cylinder','mat.timber',0,'','{"x":-1.5,"z":-0.35,"w":0.14,"h":1.3,"segments":6,"repeat":2,"dx":1.9}'),
  ('mesh.woodcutter_ancient','beams','box','mat.timber',0,'','{"x":-0.55,"y":1.22,"z":-1.55,"w":2.15,"d":0.12,"h":0.12,"repeat":2,"dz":1.2}'),
  ('mesh.woodcutter_ancient','roof','gable_roof','mat.thatch',0,'','{"x":-0.55,"y":1.34,"z":-0.95,"w":2.0,"d":1.2,"pitch":38,"overhang":0.22,"t":0.1}'),
  ('mesh.woodcutter_ancient','ridge_cap','gable_roof','mat.cloth',1,'','{"x":-0.55,"y":1.662,"z":-0.95,"w":2.44,"d":0.7,"pitch":38,"overhang":0,"t":0.03}'),
  ('mesh.woodcutter_ancient','ridge','box','mat.cloth',1,'','{"x":-0.55,"y":1.896,"z":-0.95,"w":2.44,"d":0.2,"h":0.1}'),
  ('mesh.woodcutter_ancient','valance','box','mat.cloth',1,'','{"x":-0.55,"y":0.98,"z":-0.12,"w":2.44,"d":0.04,"h":0.2,"far":0}'),
  ('mesh.woodcutter_ancient','saw_legs','box','mat.timber',0,'','{"x":-1.0,"z":-1.1,"w":0.07,"d":0.07,"h":0.72,"pitch":26,"repeat":2,"dx":0.85,"far":0}'),
  ('mesh.woodcutter_ancient','saw_legs_back','box','mat.timber',0,'','{"x":-1.0,"z":-0.8,"w":0.07,"d":0.07,"h":0.72,"pitch":-26,"repeat":2,"dx":0.85,"far":0}'),
  ('mesh.woodcutter_ancient','saw_log','cylinder','mat.timber',0,'','{"x":-1.35,"y":0.62,"z":-0.95,"w":0.26,"h":1.45,"segments":6,"roll":90}'),
  ('mesh.woodcutter_ancient','logs_e','include',NULL,0,'','{"mesh":"mesh.log_pile","x":0.75,"z":0.7}'),
  ('mesh.woodcutter_ancient','logs_w','include',NULL,0,'','{"mesh":"mesh.log_pile","x":-1.05,"z":0.85,"yaw":90,"scale":0.85}'),
  ('mesh.woodcutter_ancient','block','cylinder','mat.timber',0,'','{"x":1.2,"z":-1.0,"w":0.45,"h":0.38,"segments":7,"tint":"#d6c4a6"}'),
  ('mesh.woodcutter_ancient','axe','box','mat.timber',0,'','{"x":1.2,"y":0.34,"z":-1.0,"w":0.05,"d":0.05,"h":0.55,"roll":35,"far":0}'),
  ('mesh.woodcutter_ancient','axe_head','box','mat.iron',0,'','{"x":1.2,"y":0.33,"z":-1.0,"w":0.2,"d":0.04,"h":0.12}'),
  ('mesh.woodcutter_ancient','stumps','cylinder','mat.timber',0,'','{"x":1.35,"z":-0.2,"w":0.4,"h":0.22,"segments":7,"tint":"#c8b395","far":0}'),
  ('mesh.woodcutter_ancient','stump_n','cylinder','mat.timber',0,'','{"x":1.45,"z":-1.55,"w":0.36,"h":0.2,"segments":7,"tint":"#c8b395","far":0}'),
  ('mesh.woodcutter_ancient','banner','banner','mat.cloth',1,'','{"x":1.5,"z":-0.35,"h":2.4,"fw":0.5,"fh":0.75}'),
  -- the classical woodcutter's camp: a tiled shed with a plastered back wall, squared beams, a house
  ('mesh.woodcutter_classical','yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('mesh.woodcutter_classical','footings','box','mat.limestone',0,'','{"x":-1.5,"z":-0.35,"w":0.26,"d":0.26,"h":0.2,"repeat":2,"dx":1.8,"far":0}'),
  ('mesh.woodcutter_classical','posts','box','mat.timber',0,'','{"x":-1.5,"y":0.2,"z":-0.35,"w":0.14,"d":0.14,"h":1.25,"repeat":2,"dx":1.8}'),
  ('mesh.woodcutter_classical','back_wall','wall','mat.lime_plaster',0,'','{"x":-0.6,"z":-1.55,"w":2.0,"h":1.45,"t":0.16}'),
  ('mesh.woodcutter_classical','beam','box','mat.timber',0,'','{"x":-0.6,"y":1.45,"z":-0.35,"w":2.0,"d":0.12,"h":0.12}'),
  ('mesh.woodcutter_classical','roof','gable_roof','mat.terracotta',0,'','{"x":-0.6,"y":1.57,"z":-0.95,"w":1.85,"d":1.2,"pitch":24,"overhang":0.18,"t":0.08}'),
  ('mesh.woodcutter_classical','ridge_cap','gable_roof','mat.cloth',1,'','{"x":-0.6,"y":1.747,"z":-0.95,"w":2.21,"d":0.8,"pitch":24,"overhang":0,"t":0.03}'),
  ('mesh.woodcutter_classical','ridge','box','mat.cloth',1,'','{"x":-0.6,"y":1.922,"z":-0.95,"w":2.25,"d":0.16,"h":0.08}'),
  ('mesh.woodcutter_classical','valance','box','mat.cloth',1,'','{"x":-0.6,"y":1.3,"z":-0.16,"w":2.25,"d":0.04,"h":0.18,"far":0}'),
  ('mesh.woodcutter_classical','saw_legs','box','mat.timber',0,'','{"x":-1.05,"z":-1.1,"w":0.07,"d":0.07,"h":0.72,"pitch":26,"repeat":2,"dx":0.85,"far":0}'),
  ('mesh.woodcutter_classical','saw_legs_back','box','mat.timber',0,'','{"x":-1.05,"z":-0.8,"w":0.07,"d":0.07,"h":0.72,"pitch":-26,"repeat":2,"dx":0.85,"far":0}'),
  ('mesh.woodcutter_classical','saw_log','cylinder','mat.timber',0,'','{"x":-1.4,"y":0.62,"z":-0.95,"w":0.26,"h":1.45,"segments":6,"roll":90}'),
  ('mesh.woodcutter_classical','logs','include',NULL,0,'','{"mesh":"mesh.log_pile","x":0.75,"z":0.7}'),
  ('mesh.woodcutter_classical','beams_low','box','mat.sawn_timber',0,'','{"x":-1.45,"z":0.85,"yaw":90,"w":1.5,"d":0.2,"h":0.2,"repeat":4,"dx":0.24}'),
  ('mesh.woodcutter_classical','beams_high','box','mat.sawn_timber',0,'','{"x":-1.33,"y":0.2,"z":0.85,"yaw":90,"w":1.4,"d":0.2,"h":0.2,"repeat":3,"dx":0.24,"tint":"#e6dccc"}'),
  ('mesh.woodcutter_classical','house','include',NULL,0,'','{"mesh":"mesh.house_classical","x":1.2,"z":-1.2,"scale":0.72}'),
  ('mesh.woodcutter_classical','block','cylinder','mat.timber',0,'','{"x":1.3,"z":-0.3,"w":0.45,"h":0.38,"segments":7,"tint":"#d6c4a6"}'),
  ('mesh.woodcutter_classical','axe','box','mat.timber',0,'','{"x":1.3,"y":0.34,"z":-0.3,"w":0.05,"d":0.05,"h":0.55,"roll":35,"far":0}'),
  ('mesh.woodcutter_classical','axe_head','box','mat.iron',0,'','{"x":1.3,"y":0.33,"z":-0.3,"w":0.2,"d":0.04,"h":0.12}'),
  ('mesh.woodcutter_classical','banner','banner','mat.cloth',1,'','{"x":0.55,"z":-0.2,"h":2.6,"fw":0.5,"fh":0.8}'),
  -- the early market: four stalls round a square, a mast with the owner's flag, a handcart
  ('mesh.market_ancient','yard','plinth','mat.packed_earth',0,'','{"w":3.7,"d":3.7,"h":0.05,"steps":1}'),
  ('mesh.market_ancient','stall_nw','include',NULL,0,'','{"mesh":"mesh.stall","x":-0.85,"z":-1.2}'),
  ('mesh.market_ancient','stall_ne','include',NULL,0,'','{"mesh":"mesh.stall","x":0.85,"z":-1.2}'),
  ('mesh.market_ancient','stall_w','include',NULL,0,'','{"mesh":"mesh.stall","x":-1.25,"z":0.55,"yaw":90}'),
  ('mesh.market_ancient','stall_e','include',NULL,0,'','{"mesh":"mesh.stall","x":1.25,"z":0.55,"yaw":-90}'),
  ('mesh.market_ancient','mast','banner','mat.cloth',1,'','{"x":0.1,"z":0.2,"h":3.0,"fw":0.55,"fh":0.9}'),
  ('mesh.market_ancient','amphorae','cylinder','mat.mud_plaster',0,'','{"x":-0.3,"z":0.15,"w":0.22,"h":0.4,"segments":6,"taper":0.35,"repeat":2,"dz":0.26}'),
  ('mesh.market_ancient','cart_bed','box','mat.timber',0,'','{"y":0.28,"z":1.3,"w":0.6,"d":0.8,"h":0.18}'),
  ('mesh.market_ancient','cart_load','dome','mat.cloth',0,'','{"y":0.46,"z":1.3,"w":0.55,"h":0.25,"segments":8,"tint":"#c9b48a"}'),
  ('mesh.market_ancient','cart_wheels','cylinder','mat.timber',0,'','{"x":0.31,"y":0.26,"z":1.4,"w":0.52,"h":0.07,"segments":10,"roll":90}'),
  ('mesh.market_ancient','cart_wheel_w','cylinder','mat.timber',0,'','{"x":-0.38,"y":0.26,"z":1.4,"w":0.52,"h":0.07,"segments":10,"roll":90}'),
  ('mesh.market_ancient','cart_handles','box','mat.timber',0,'','{"x":-0.2,"y":0.3,"z":0.9,"w":0.05,"d":0.6,"h":0.05,"repeat":2,"dx":0.4,"far":0}'),
  -- the classical market: a stoa with a frieze in the owner's colour, three stalls, a basin
  ('mesh.market_classical','yard','plinth','mat.limestone',0,'','{"w":3.7,"d":3.7,"h":0.08,"steps":1}'),
  ('mesh.market_classical','stoa_back','box','mat.lime_plaster',0,'','{"y":0.08,"z":-1.58,"w":3.4,"d":0.24,"h":1.74,"windows":0}'),
  ('mesh.market_classical','stoa_ends','box','mat.lime_plaster',0,'','{"x":-1.55,"y":0.08,"z":-1.1,"w":0.3,"d":1.2,"h":1.74,"gable":20,"repeat":2,"dx":3.1}'),
  ('mesh.market_classical','stoa_columns','column_row','mat.limestone',0,'','{"y":0.08,"z":-0.58,"count":5,"radius":0.1,"spacing":0.66,"h":1.56,"segments":8}'),
  ('mesh.market_classical','stoa_beam','box','mat.limestone',0,'','{"y":1.64,"z":-0.58,"w":3.4,"d":0.24,"h":0.18}'),
  ('mesh.market_classical','stoa_frieze','box','mat.cloth',1,'','{"y":1.68,"z":-0.58,"w":3.42,"d":0.26,"h":0.11}'),
  ('mesh.market_classical','stoa_roof','gable_roof','mat.terracotta',0,'','{"y":1.82,"z":-1.1,"w":3.4,"d":1.2,"pitch":20,"overhang":0.1,"t":0.08}'),
  ('mesh.market_classical','ridge_cap','gable_roof','mat.cloth',1,'','{"y":2.033,"z":-1.1,"w":3.6,"d":0.5,"pitch":20,"overhang":0,"t":0.03}'),
  ('mesh.market_classical','stall_w','include',NULL,0,'','{"mesh":"mesh.stall","x":-1.3,"y":0.08,"z":0.55,"yaw":90,"scale":0.85}'),
  ('mesh.market_classical','stall_e','include',NULL,0,'','{"mesh":"mesh.stall","x":1.3,"y":0.08,"z":0.55,"yaw":-90,"scale":0.85}'),
  ('mesh.market_classical','stall_s','include',NULL,0,'','{"mesh":"mesh.stall","y":0.08,"z":1.3,"yaw":180,"scale":0.85}'),
  ('mesh.market_classical','basin','cylinder','mat.limestone',0,'','{"y":0.08,"z":0.25,"w":0.75,"h":0.28,"segments":10}'),
  ('mesh.market_classical','spout','cylinder','mat.bronze',0,'','{"y":0.36,"z":0.25,"w":0.12,"h":0.5,"segments":6,"taper":0.4}'),
  ('mesh.market_classical','amphorae','cylinder','mat.terracotta',0,'','{"x":-0.55,"y":0.08,"z":-0.25,"w":0.22,"h":0.42,"segments":6,"taper":0.35,"repeat":3,"dx":0.26}'),
  -- the early watchtower: four leaning posts, braces, a fenced platform under a thatched cap
  ('mesh.tower_ancient','footings','box','mat.fieldstone',0,'','{"x":-0.68,"z":-0.68,"w":0.3,"d":0.3,"h":0.14,"repeat":2,"dx":1.36,"far":0}'),
  ('mesh.tower_ancient','footings_front','box','mat.fieldstone',0,'','{"x":-0.68,"z":0.68,"w":0.3,"d":0.3,"h":0.14,"repeat":2,"dx":1.36,"far":0}'),
  ('mesh.tower_ancient','leg_nw','cylinder','mat.timber',0,'','{"x":-0.68,"z":-0.68,"w":0.17,"h":3.25,"segments":6,"roll":3,"pitch":3}'),
  ('mesh.tower_ancient','leg_ne','cylinder','mat.timber',0,'','{"x":0.68,"z":-0.68,"w":0.17,"h":3.25,"segments":6,"roll":-3,"pitch":3}'),
  ('mesh.tower_ancient','leg_sw','cylinder','mat.timber',0,'','{"x":-0.68,"z":0.68,"w":0.17,"h":3.25,"segments":6,"roll":3,"pitch":-3}'),
  ('mesh.tower_ancient','leg_se','cylinder','mat.timber',0,'','{"x":0.68,"z":0.68,"w":0.17,"h":3.25,"segments":6,"roll":-3,"pitch":-3}'),
  ('mesh.tower_ancient','brace_a','box','mat.timber',0,'','{"x":-0.662,"y":0.35,"z":0.6,"w":0.07,"d":0.07,"h":2.43,"roll":30,"repeat":2,"dz":-1.2}'),
  ('mesh.tower_ancient','brace_b','box','mat.timber',0,'','{"x":0.662,"y":0.35,"z":0.6,"w":0.07,"d":0.07,"h":2.43,"roll":-30,"repeat":2,"dz":-1.2}'),
  ('mesh.tower_ancient','brace_c','box','mat.timber',0,'','{"x":-0.6,"y":0.35,"z":-0.662,"w":0.07,"d":0.07,"h":2.43,"pitch":30,"repeat":2,"dx":1.2,"far":0}'),
  ('mesh.tower_ancient','brace_d','box','mat.timber',0,'','{"x":-0.6,"y":0.35,"z":0.662,"w":0.07,"d":0.07,"h":2.43,"pitch":-30,"repeat":2,"dx":1.2,"far":0}'),
  ('mesh.tower_ancient','platform','box','mat.timber',0,'','{"y":3.2,"w":1.5,"d":1.5,"h":0.12}'),
  ('mesh.tower_ancient','parapet','palisade','mat.timber',0,'','{"y":3.32,"w":1.5,"d":1.5,"h":0.55,"t":0.08,"spacing":0.25,"ring":1}'),
  ('mesh.tower_ancient','shield_front','box','mat.cloth',1,'','{"y":3.4,"z":0.78,"w":0.9,"d":0.04,"h":0.4}'),
  ('mesh.tower_ancient','shield_sides','box','mat.cloth',1,'','{"x":-0.78,"y":3.4,"yaw":90,"w":0.9,"d":0.04,"h":0.4,"repeat":2,"dx":1.56}'),
  ('mesh.tower_ancient','roof_posts','cylinder','mat.timber',0,'','{"x":-0.62,"y":3.32,"z":-0.62,"w":0.08,"h":0.95,"segments":4,"repeat":2,"dx":1.24,"far":0}'),
  ('mesh.tower_ancient','roof_posts_front','cylinder','mat.timber',0,'','{"x":-0.62,"y":3.32,"z":0.62,"w":0.08,"h":0.95,"segments":4,"repeat":2,"dx":1.24,"far":0}'),
  ('mesh.tower_ancient','roof','hip_roof','mat.thatch',0,'','{"y":4.27,"w":1.3,"d":1.3,"pitch":42,"overhang":0.25,"t":0.1}'),
  ('mesh.tower_ancient','roof_cap','hip_roof','mat.cloth',1,'','{"y":4.612,"w":0.84,"d":0.84,"pitch":42,"overhang":0,"t":0.03}'),
  ('mesh.tower_ancient','banner','banner','mat.cloth',1,'','{"y":4.75,"h":1.25,"fw":0.5,"fh":0.55}'),
  ('mesh.tower_ancient','ladder','fence','mat.timber',0,'','{"x":0.2,"y":1.6,"z":0.82,"w":3.2,"h":0.4,"spacing":0.35,"rails":2,"roll":-90,"pitch":-3,"far":0}'),
  -- the classical tower: an ashlar shaft with a crenellated top, a banner and a long hanging cloth
  ('mesh.tower_classical','base','plinth','mat.limestone',0,'','{"w":1.8,"d":1.8,"h":0.3,"steps":2,"inset":0.08}'),
  ('mesh.tower_classical','shaft','box','mat.limestone',0,'','{"y":0.3,"w":1.45,"d":1.45,"h":3.3,"door":0.42,"door_h":0.8,"side_windows":1,"window_w":0.12,"window_h":0.34,"window_y":1.6,"window_rows":2,"row_gap":1.1}'),
  ('mesh.tower_classical','corbel','box','mat.limestone',0,'','{"y":3.6,"w":1.65,"d":1.65,"h":0.2}'),
  ('mesh.tower_classical','merlons','crenellation','mat.limestone',0,'','{"y":3.8,"w":1.65,"d":1.65,"h":0.35,"t":0.15,"merlon":0.24,"ring":1}'),
  ('mesh.tower_classical','hanging','box','mat.cloth',1,'','{"y":1.95,"z":0.735,"w":0.5,"d":0.03,"h":1.45}'),
  ('mesh.tower_classical','roof_posts','column_row','mat.limestone',0,'','{"y":3.8,"z":-0.5,"count":2,"radius":0.07,"spacing":1.0,"h":0.85,"segments":6,"capital":0,"repeat":2,"dz":1.0}'),
  ('mesh.tower_classical','roof','hip_roof','mat.terracotta',0,'','{"y":4.65,"w":1.2,"d":1.2,"pitch":30,"overhang":0.18,"t":0.08}'),
  ('mesh.tower_classical','roof_cap','hip_roof','mat.cloth',1,'','{"y":4.8,"w":1,"d":1,"pitch":30,"overhang":0,"t":0.03}'),
  ('mesh.tower_classical','banner','banner','mat.cloth',1,'','{"y":4.95,"h":1.3,"fw":0.5,"fh":0.6}'),
  -- the early dock: a thatched boat shed with a boat on chocks, a raised pier with mooring posts
  ('mesh.dock_ancient','yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.04,"steps":1}'),
  ('mesh.dock_ancient','shed_back','box','mat.mud_brick',0,'','{"x":-0.85,"z":-1.58,"yaw":90,"w":0.14,"d":1.6,"h":1.1,"gable":36}'),
  ('mesh.dock_ancient','shed_wall','wall','mat.mud_brick',0,'','{"x":-1.58,"z":-0.15,"yaw":90,"w":3.0,"h":1.1,"t":0.14,"windows":3,"window_y":0.6,"window_w":0.18,"window_h":0.22}'),
  ('mesh.dock_ancient','shed_posts','cylinder','mat.timber',0,'','{"x":-0.12,"z":-1.5,"w":0.13,"h":1.1,"segments":6,"repeat":3,"dz":1.4}'),
  ('mesh.dock_ancient','shed_roof','gable_roof','mat.thatch',0,'','{"x":-0.85,"y":1.1,"z":-0.15,"yaw":90,"w":3.0,"d":1.6,"pitch":36,"overhang":0.18,"t":0.1}'),
  ('mesh.dock_ancient','ridge_cap','gable_roof','mat.cloth',1,'','{"x":-0.85,"y":1.623,"z":-0.15,"yaw":90,"w":3.36,"d":0.5,"pitch":36,"overhang":0,"t":0.03}'),
  ('mesh.dock_ancient','shed_ridge','box','mat.cloth',1,'','{"x":-0.85,"y":1.769,"z":-0.15,"yaw":90,"w":3.36,"d":0.2,"h":0.1}'),
  ('mesh.dock_ancient','boat','hull','mat.timber',0,'','{"x":-0.85,"y":0.18,"z":0.45,"yaw":90,"w":2.3,"d":0.8,"h":0.38,"sheer":0.22}'),
  ('mesh.dock_ancient','boat_stripe','box','mat.cloth',1,'','{"x":-0.85,"y":0.5,"z":0.45,"yaw":90,"w":1.2,"d":0.84,"h":0.08}'),
  ('mesh.dock_ancient','chocks','box','mat.timber',0,'','{"x":-0.85,"z":-0.15,"w":0.6,"d":0.15,"h":0.2,"repeat":3,"dz":0.6,"far":0}'),
  ('mesh.dock_ancient','pier','box','mat.timber',0,'','{"x":0.95,"y":0.4,"w":1.1,"d":3.5,"h":0.1}'),
  ('mesh.dock_ancient','pier_beams','box','mat.timber',0,'','{"x":0.95,"y":0.28,"w":0.9,"d":3.4,"h":0.12,"tint":"#a89888"}'),
  ('mesh.dock_ancient','pier_posts','cylinder','mat.timber',0,'','{"x":0.45,"z":-1.6,"w":0.15,"h":0.8,"segments":6,"repeat":4,"dz":1.067}'),
  ('mesh.dock_ancient','pier_posts_e','cylinder','mat.timber',0,'','{"x":1.45,"z":-1.6,"w":0.15,"h":0.8,"segments":6,"repeat":4,"dz":1.067,"far":0}'),
  ('mesh.dock_ancient','steps','plinth','mat.timber',0,'','{"x":0.28,"z":0.3,"w":0.36,"d":0.6,"h":0.45,"steps":3,"inset":0.06}'),
  ('mesh.dock_ancient','shelter','awning','mat.cloth',1,'','{"x":0.95,"y":0.5,"z":-1.2,"w":1.0,"d":0.8,"h":1.0,"rise":0.25}'),
  ('mesh.dock_ancient','crate','box','mat.timber',0,'','{"x":1.2,"y":0.5,"z":-1.3,"w":0.35,"d":0.35,"h":0.35,"far":0}'),
  ('mesh.dock_ancient','amphorae','cylinder','mat.mud_plaster',0,'','{"x":0.7,"y":0.5,"z":-1.4,"w":0.2,"h":0.36,"segments":6,"taper":0.35,"repeat":2,"dz":0.25,"far":0}'),
  ('mesh.dock_ancient','rope','cylinder','mat.thatch',0,'','{"x":1.15,"y":0.5,"z":0.75,"w":0.32,"h":0.08,"segments":8,"far":0}'),
  ('mesh.dock_ancient','banner','banner','mat.cloth',1,'','{"x":1.4,"y":0.5,"z":1.5,"h":2.4,"fw":0.5,"fh":0.75}'),
  -- the classical harbour: a colonnaded ship shed with a galley, a stone pier and a crane
  ('mesh.dock_classical','quay','plinth','mat.limestone',0,'','{"w":3.6,"d":3.6,"h":0.12,"steps":1}'),
  ('mesh.dock_classical','shed_back','box','mat.lime_plaster',0,'','{"x":-0.85,"y":0.12,"z":-1.58,"yaw":90,"w":0.16,"d":1.6,"h":1.42,"gable":24}'),
  ('mesh.dock_classical','shed_wall','wall','mat.lime_plaster',0,'','{"x":-1.58,"y":0.12,"z":-0.15,"yaw":90,"w":3.0,"h":1.42,"t":0.16,"windows":3,"window_y":0.8,"window_w":0.2,"window_h":0.3}'),
  ('mesh.dock_classical','shed_columns','column_row','mat.limestone',0,'','{"x":-0.12,"y":0.12,"z":-0.15,"yaw":90,"count":4,"radius":0.09,"spacing":0.95,"h":1.3,"segments":8}'),
  ('mesh.dock_classical','shed_beam','box','mat.limestone',0,'','{"x":-0.12,"y":1.42,"z":-0.15,"w":0.22,"d":3.1,"h":0.12}'),
  ('mesh.dock_classical','shed_roof','gable_roof','mat.terracotta',0,'','{"x":-0.85,"y":1.54,"z":-0.15,"yaw":90,"w":3.0,"d":1.6,"pitch":24,"overhang":0.15,"t":0.08}'),
  ('mesh.dock_classical','ridge_cap','gable_roof','mat.cloth',1,'','{"x":-0.85,"y":1.872,"z":-0.15,"yaw":90,"w":3.3,"d":0.5,"pitch":24,"overhang":0,"t":0.03}'),
  ('mesh.dock_classical','shed_ridge','box','mat.cloth',1,'','{"x":-0.85,"y":1.981,"z":-0.15,"yaw":90,"w":3.3,"d":0.16,"h":0.08}'),
  ('mesh.dock_classical','galley','hull','mat.timber',0,'','{"x":-0.85,"y":0.32,"z":0.3,"yaw":90,"w":2.6,"d":0.8,"h":0.42,"sheer":0.3}'),
  ('mesh.dock_classical','galley_stripe','box','mat.cloth',1,'','{"x":-0.85,"y":0.66,"z":0.3,"yaw":90,"w":1.4,"d":0.84,"h":0.1}'),
  ('mesh.dock_classical','chocks','box','mat.limestone',0,'','{"x":-0.85,"y":0.12,"z":-0.35,"w":0.6,"d":0.15,"h":0.2,"repeat":3,"dz":0.65,"far":0}'),
  ('mesh.dock_classical','pier','box','mat.limestone',0,'','{"x":0.95,"y":0.4,"w":1.1,"d":3.5,"h":0.14}'),
  ('mesh.dock_classical','pier_piers','box','mat.limestone',0,'','{"x":0.95,"y":0.12,"z":-1.6,"w":1.0,"d":0.28,"h":0.28,"repeat":4,"dz":1.067,"tint":"#d8d2c6"}'),
  ('mesh.dock_classical','bollards','cylinder','mat.iron',0,'','{"x":0.5,"y":0.52,"z":-0.9,"w":0.13,"h":0.22,"segments":6,"repeat":3,"dz":1.1}'),
  ('mesh.dock_classical','crane_leg_w','cylinder','mat.timber',0,'','{"x":0.62,"y":0.52,"z":-0.9,"w":0.12,"h":3.0,"segments":6,"roll":6,"pitch":22}'),
  ('mesh.dock_classical','crane_leg_e','cylinder','mat.timber',0,'','{"x":1.28,"y":0.52,"z":-0.9,"w":0.12,"h":3.0,"segments":6,"roll":-6,"pitch":22}'),
  ('mesh.dock_classical','crane_stay','cylinder','mat.thatch',0,'','{"x":0.95,"y":0.52,"z":-1.7,"w":0.04,"h":3.37,"segments":4,"pitch":34.7,"far":0}'),
  ('mesh.dock_classical','crane_rope','cylinder','mat.thatch',0,'','{"x":0.95,"y":1.6,"z":0.22,"w":0.04,"h":1.7,"segments":4,"far":0}'),
  ('mesh.dock_classical','crane_load','box','mat.timber',0,'','{"x":0.95,"y":1.2,"z":0.22,"w":0.42,"d":0.42,"h":0.4}'),
  ('mesh.dock_classical','steps','plinth','mat.limestone',0,'','{"x":0.28,"y":0.12,"z":0.6,"w":0.36,"d":0.6,"h":0.4,"steps":3,"inset":0.06}'),
  ('mesh.dock_classical','shelter','awning','mat.cloth',1,'','{"x":0.95,"y":0.52,"z":1.0,"w":1.0,"d":0.8,"h":1.05,"rise":0.25}'),
  ('mesh.dock_classical','amphorae','cylinder','mat.terracotta',0,'','{"x":0.7,"y":0.52,"z":-1.45,"w":0.2,"h":0.38,"segments":6,"taper":0.35,"repeat":3,"dx":0.25,"far":0}'),
  ('mesh.dock_classical','banner','banner','mat.cloth',1,'','{"x":1.42,"y":0.52,"z":1.6,"h":2.5,"fw":0.5,"fh":0.8}'),
  -- the classical barracks: an ashlar-walled yard, an arched gate, a tiled block and a tall banner
  ('mesh.barracks_classical','yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('mesh.barracks_classical','walls','wall','mat.limestone',0,'','{"w":3.6,"d":3.6,"h":1.15,"t":0.2,"ring":1,"gap":1.3}'),
  ('mesh.barracks_classical','gate','arch','mat.limestone',0,'','{"z":1.7,"w":1.7,"h":1.75,"t":0.26,"span":0.85,"spring":0.85}'),
  ('mesh.barracks_classical','gate_cloth','box','mat.cloth',1,'','{"y":1.32,"z":1.835,"w":0.75,"d":0.02,"h":0.36}'),
  ('mesh.barracks_classical','block','box','mat.lime_plaster',0,'','{"y":0.05,"z":-0.95,"w":2.7,"d":1.2,"h":1.35,"gable":22,"door":0.45,"door_h":0.9,"windows":4,"window_y":0.7,"window_w":0.18,"window_h":0.28,"side_windows":1}'),
  ('mesh.barracks_classical','roof','gable_roof','mat.terracotta',0,'','{"y":1.4,"z":-0.95,"w":2.7,"d":1.2,"pitch":22,"overhang":0.15,"t":0.08}'),
  ('mesh.barracks_classical','ridge_cap','gable_roof','mat.cloth',1,'','{"y":1.587,"z":-0.95,"w":3,"d":0.7,"pitch":22,"overhang":0,"t":0.03}'),
  ('mesh.barracks_classical','ridge','box','mat.cloth',1,'','{"y":1.729,"z":-0.95,"w":2.95,"d":0.16,"h":0.08}'),
  ('mesh.barracks_classical','banner','banner','mat.cloth',1,'','{"x":1.15,"z":0.6,"h":3.8,"fw":0.8,"fh":1.35,"notch":0.25}'),
  ('mesh.barracks_classical','shields','cylinder','mat.cloth',1,'','{"x":-1.05,"y":0.19,"z":-0.22,"w":0.38,"h":0.05,"segments":8,"pitch":-80,"repeat":3,"dx":0.34,"far":0}'),
  ('mesh.barracks_classical','rack','fence','mat.timber',0,'','{"x":0.95,"z":-0.15,"w":0.9,"h":0.55,"spacing":0.45,"rails":1,"far":0}'),
  ('mesh.barracks_classical','spears','cylinder','mat.timber',0,'','{"x":0.65,"z":-0.2,"w":0.045,"h":1.5,"segments":4,"repeat":4,"dx":0.2,"far":0}'),
  ('mesh.barracks_classical','posts','cylinder','mat.timber',0,'','{"x":-1.0,"z":0.8,"w":0.13,"h":1.2,"segments":6,"repeat":2,"dx":0.8}'),
  -- the classical library: an ashlar block on a stepped base, a pedimented porch, a drum and dome
  ('mesh.library_classical','base','plinth','mat.limestone',0,'','{"w":3.5,"d":3.5,"h":0.45,"steps":3,"inset":0.14}'),
  ('mesh.library_classical','block','box','mat.limestone',0,'','{"y":0.45,"z":-0.2,"w":2.7,"d":2.2,"h":1.75,"door":0.5,"door_h":1.05,"side_windows":2,"window_y":1.1,"window_w":0.14,"window_h":0.34}'),
  ('mesh.library_classical','porch_columns','column_row','mat.limestone',0,'','{"y":0.45,"z":1.22,"count":4,"radius":0.1,"spacing":0.55,"h":1.6,"segments":8}'),
  ('mesh.library_classical','porch_beam','box','mat.limestone',0,'','{"y":2.05,"z":1.07,"w":1.95,"d":0.5,"h":0.15}'),
  ('mesh.library_classical','pediment','box','mat.cloth',1,'','{"y":2.2,"z":1.07,"yaw":90,"w":0.5,"d":1.95,"h":0.02,"gable":22}'),
  ('mesh.library_classical','porch_roof','gable_roof','mat.terracotta',0,'','{"y":2.22,"z":1.07,"yaw":90,"w":0.5,"d":1.95,"pitch":22,"overhang":0.06,"t":0.07}'),
  ('mesh.library_classical','top','flat_roof','mat.limestone',0,'','{"y":2.2,"z":-0.2,"w":2.7,"d":2.2,"t":0.12,"overhang":0.06}'),
  ('mesh.library_classical','drum','cylinder','mat.limestone',0,'','{"y":2.32,"z":-0.2,"w":1.5,"h":0.45,"segments":12}'),
  ('mesh.library_classical','drum_band','cylinder','mat.cloth',1,'','{"y":2.6,"z":-0.2,"w":1.53,"h":0.17,"segments":12}'),
  ('mesh.library_classical','dome','dome','mat.cloth',1,'','{"y":2.77,"z":-0.2,"w":1.5,"h":0.8,"segments":12}'),
  ('mesh.library_classical','finial','cone','mat.bronze',0,'','{"y":3.55,"z":-0.2,"w":0.16,"h":0.35,"segments":6}'),
  -- the classical granary: three conical silos and a domed one, a ladder, the keeper's house
  ('mesh.granary_classical','yard','plinth','mat.limestone',0,'','{"w":3.6,"d":3.6,"h":0.1,"steps":1}'),
  ('mesh.granary_classical','silo_c','include',NULL,0,'','{"mesh":"mesh.silo","y":0.1,"z":-1.15,"scale":1.05}'),
  ('mesh.granary_classical','silo_w','include',NULL,0,'','{"mesh":"mesh.silo","x":-1.15,"y":0.1,"z":-0.55}'),
  ('mesh.granary_classical','silo_e','include',NULL,0,'','{"mesh":"mesh.silo","x":1.15,"y":0.1,"z":-0.55}'),
  ('mesh.granary_classical','hive_footing','cylinder','mat.fieldstone',0,'','{"x":-1.1,"y":0.1,"z":0.95,"w":1.15,"h":0.18,"segments":10}'),
  ('mesh.granary_classical','hive','cylinder','mat.lime_plaster',0,'','{"x":-1.1,"y":0.28,"z":0.95,"w":1.05,"h":0.55,"segments":10}'),
  ('mesh.granary_classical','hive_band','cylinder','mat.cloth',1,'','{"x":-1.1,"y":0.6,"z":0.95,"w":1.08,"h":0.2,"segments":10}'),
  ('mesh.granary_classical','hive_dome','dome','mat.lime_plaster',0,'','{"x":-1.1,"y":0.83,"z":0.95,"w":1.05,"h":0.75,"segments":10}'),
  ('mesh.granary_classical','house','include',NULL,0,'','{"mesh":"mesh.house_classical","x":1.0,"y":0.1,"z":1.05,"scale":0.85}'),
  ('mesh.granary_classical','ladder','fence','mat.timber',0,'','{"x":0.17,"y":1.1,"z":-0.3,"w":2.0,"h":0.35,"spacing":0.3,"rails":2,"roll":-90,"pitch":-14,"far":0}'),
  ('mesh.granary_classical','sacks','dome','mat.cloth',0,'','{"x":-0.3,"y":0.1,"z":0.55,"w":0.34,"h":0.3,"segments":6,"tint":"#c9b48a","repeat":3,"dx":0.32,"far":0}'),
  ('mesh.granary_classical','banner','banner','mat.cloth',1,'','{"x":0.3,"y":0.1,"z":1.55,"h":2.4,"fw":0.5,"fh":0.75}'),
  -- the classical lumber mill: a tiled mill house with a wheel, a frame saw over a log, plank stacks
  ('mesh.lumber_mill_classical','yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('mesh.lumber_mill_classical','mill_base','plinth','mat.limestone',0,'','{"x":-0.8,"z":-0.95,"w":1.85,"d":1.5,"h":0.2,"steps":1}'),
  ('mesh.lumber_mill_classical','mill','box','mat.lime_plaster',0,'','{"x":-0.8,"y":0.2,"z":-0.95,"w":1.7,"d":1.35,"h":1.45,"gable":26,"door":0.45,"door_h":0.85,"door_x":-0.4,"windows":2,"side_windows":1,"window_y":0.85,"window_w":0.18,"window_h":0.25}'),
  ('mesh.lumber_mill_classical','mill_roof','gable_roof','mat.terracotta',0,'','{"x":-0.8,"y":1.65,"z":-0.95,"w":1.7,"d":1.35,"pitch":26,"overhang":0.12,"t":0.08}'),
  ('mesh.lumber_mill_classical','ridge_cap','gable_roof','mat.cloth',1,'','{"x":-0.8,"y":1.946,"z":-0.95,"w":1.94,"d":0.5,"pitch":26,"overhang":0,"t":0.03}'),
  ('mesh.lumber_mill_classical','mill_ridge','box','mat.cloth',1,'','{"x":-0.8,"y":2.063,"z":-0.95,"w":1.95,"d":0.16,"h":0.08}'),
  ('mesh.lumber_mill_classical','porch','awning','mat.cloth',1,'','{"x":-1.2,"z":-0.05,"w":0.9,"d":0.45,"h":1.05,"rise":0.2}'),
  ('mesh.lumber_mill_classical','wheel','cylinder','mat.timber',0,'','{"x":0.1,"y":0.85,"z":-0.95,"w":1.5,"h":0.22,"segments":12,"roll":90}'),
  ('mesh.lumber_mill_classical','hub','cylinder','mat.iron',0,'','{"x":0.04,"y":0.85,"z":-0.95,"w":0.18,"h":0.34,"segments":6,"roll":90}'),
  ('mesh.lumber_mill_classical','paddle_0','box','mat.timber',0,'','{"x":0.21,"y":0.85,"z":-0.95,"w":0.3,"d":0.06,"h":0.88}'),
  ('mesh.lumber_mill_classical','paddle_90','box','mat.timber',0,'','{"x":0.21,"y":0.85,"z":-0.95,"w":0.3,"d":0.06,"h":0.88,"pitch":90}'),
  ('mesh.lumber_mill_classical','paddle_180','box','mat.timber',0,'','{"x":0.21,"y":0.85,"z":-0.95,"w":0.3,"d":0.06,"h":0.88,"pitch":180}'),
  ('mesh.lumber_mill_classical','paddle_270','box','mat.timber',0,'','{"x":0.21,"y":0.85,"z":-0.95,"w":0.3,"d":0.06,"h":0.88,"pitch":-90}'),
  ('mesh.lumber_mill_classical','paddle_45','box','mat.timber',0,'','{"x":0.21,"y":0.85,"z":-0.95,"w":0.3,"d":0.06,"h":0.88,"pitch":45,"far":0}'),
  ('mesh.lumber_mill_classical','paddle_135','box','mat.timber',0,'','{"x":0.21,"y":0.85,"z":-0.95,"w":0.3,"d":0.06,"h":0.88,"pitch":135,"far":0}'),
  ('mesh.lumber_mill_classical','paddle_225','box','mat.timber',0,'','{"x":0.21,"y":0.85,"z":-0.95,"w":0.3,"d":0.06,"h":0.88,"pitch":-135,"far":0}'),
  ('mesh.lumber_mill_classical','paddle_315','box','mat.timber',0,'','{"x":0.21,"y":0.85,"z":-0.95,"w":0.3,"d":0.06,"h":0.88,"pitch":-45,"far":0}'),
  ('mesh.lumber_mill_classical','saw_posts','box','mat.timber',0,'','{"x":0.1,"z":0.55,"w":0.15,"d":0.15,"h":2.2,"repeat":2,"dx":1.2}'),
  ('mesh.lumber_mill_classical','saw_head','box','mat.timber',0,'','{"x":0.7,"y":2.08,"z":0.55,"w":1.45,"d":0.17,"h":0.17}'),
  ('mesh.lumber_mill_classical','saw_sash','box','mat.timber',0,'','{"x":0.7,"y":1.3,"z":0.55,"w":1.2,"d":0.1,"h":0.1}'),
  ('mesh.lumber_mill_classical','saw_blades','box','mat.iron',0,'','{"x":0.55,"y":0.5,"z":0.55,"w":0.03,"d":0.05,"h":0.8,"repeat":3,"dx":0.15,"far":0}'),
  ('mesh.lumber_mill_classical','saw_log','cylinder','mat.timber',0,'','{"x":0.7,"y":0.62,"z":-0.15,"w":0.36,"h":1.9,"segments":7,"pitch":90}'),
  ('mesh.lumber_mill_classical','trestles','box','mat.timber',0,'','{"x":0.7,"z":0.1,"w":0.5,"d":0.1,"h":0.45,"repeat":3,"dz":0.65,"far":0}'),
  ('mesh.lumber_mill_classical','planks_1','box','mat.sawn_timber',0,'','{"x":-1.05,"z":0.9,"w":1.3,"d":0.95,"h":0.13}'),
  ('mesh.lumber_mill_classical','sticks_1','box','mat.timber',0,'','{"x":-1.05,"y":0.13,"z":0.55,"w":1.3,"d":0.06,"h":0.04,"repeat":3,"dz":0.35,"far":0}'),
  ('mesh.lumber_mill_classical','planks_2','box','mat.sawn_timber',0,'','{"x":-1.05,"y":0.17,"z":0.9,"yaw":2,"w":1.3,"d":0.95,"h":0.13,"tint":"#f0e6d8"}'),
  ('mesh.lumber_mill_classical','sticks_2','box','mat.timber',0,'','{"x":-1.05,"y":0.3,"z":0.55,"w":1.3,"d":0.06,"h":0.04,"repeat":3,"dz":0.35,"far":0}'),
  ('mesh.lumber_mill_classical','planks_3','box','mat.sawn_timber',0,'','{"x":-1.05,"y":0.34,"z":0.9,"yaw":-1.5,"w":1.25,"d":0.95,"h":0.13}'),
  ('mesh.lumber_mill_classical','sticks_3','box','mat.timber',0,'','{"x":-1.05,"y":0.47,"z":0.55,"w":1.25,"d":0.06,"h":0.04,"repeat":3,"dz":0.35,"far":0}'),
  ('mesh.lumber_mill_classical','planks_4','box','mat.sawn_timber',0,'','{"x":-1.05,"y":0.51,"z":0.9,"yaw":1,"w":1.2,"d":0.95,"h":0.13,"tint":"#f0e6d8"}'),
  ('mesh.lumber_mill_classical','planks_ne','box','mat.sawn_timber',0,'','{"x":1.15,"z":-1.2,"w":1.1,"d":0.8,"h":0.13}'),
  ('mesh.lumber_mill_classical','sticks_ne','box','mat.timber',0,'','{"x":1.15,"y":0.13,"z":-1.45,"w":1.1,"d":0.06,"h":0.04,"repeat":3,"dz":0.25,"far":0}'),
  ('mesh.lumber_mill_classical','planks_ne_2','box','mat.sawn_timber',0,'','{"x":1.15,"y":0.17,"z":-1.2,"yaw":-2,"w":1.05,"d":0.8,"h":0.13,"tint":"#f0e6d8"}'),
  ('mesh.lumber_mill_classical','banner','banner','mat.cloth',1,'','{"x":1.55,"z":1.5,"h":2.4,"fw":0.5,"fh":0.75}'),
  -- the classical mine: a leaning timber headframe with a sheave over a walled shaft, spoil, rails
  ('mesh.mine_classical','yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('mesh.mine_classical','collar','wall','mat.fieldstone',0,'','{"x":-0.55,"z":-0.55,"w":1.2,"d":1.2,"h":0.35,"t":0.2,"ring":1}'),
  ('mesh.mine_classical','pit','box','mat.fieldstone',0,'','{"x":-0.55,"y":0.04,"z":-0.55,"w":0.82,"d":0.82,"h":0.03,"tint":"#2a2622"}'),
  ('mesh.mine_classical','leg_nw','cylinder','mat.timber',0,'','{"x":-1.3,"z":-1.3,"w":0.16,"h":2.75,"segments":6,"roll":7,"pitch":7}'),
  ('mesh.mine_classical','leg_ne','cylinder','mat.timber',0,'','{"x":0.2,"z":-1.3,"w":0.16,"h":2.75,"segments":6,"roll":-7,"pitch":7}'),
  ('mesh.mine_classical','leg_sw','cylinder','mat.timber',0,'','{"x":-1.3,"z":0.2,"w":0.16,"h":2.75,"segments":6,"roll":7,"pitch":-7}'),
  ('mesh.mine_classical','leg_se','cylinder','mat.timber',0,'','{"x":0.2,"z":0.2,"w":0.16,"h":2.75,"segments":6,"roll":-7,"pitch":-7}'),
  ('mesh.mine_classical','girts','box','mat.timber',0,'','{"x":-0.55,"y":1.4,"z":-1.12,"w":1.2,"d":0.1,"h":0.1,"repeat":2,"dz":1.14,"far":0}'),
  ('mesh.mine_classical','girts_side','box','mat.timber',0,'','{"x":-1.12,"y":1.4,"z":-0.55,"yaw":90,"w":1.2,"d":0.1,"h":0.1,"repeat":2,"dx":1.14,"far":0}'),
  ('mesh.mine_classical','cloth','box','mat.cloth',1,'','{"x":-0.55,"y":1.0,"z":0.03,"w":0.9,"d":0.03,"h":0.42}'),
  ('mesh.mine_classical','head','box','mat.timber',0,'','{"x":-0.55,"y":2.68,"z":-0.55,"w":1.0,"d":1.0,"h":0.14}'),
  ('mesh.mine_classical','sheave_frame','box','mat.timber',0,'','{"x":-0.75,"y":2.82,"z":-0.55,"w":0.08,"d":0.1,"h":0.55,"repeat":2,"dx":0.4,"far":0}'),
  ('mesh.mine_classical','sheave','cylinder','mat.timber',0,'','{"x":-0.62,"y":3.28,"z":-0.55,"w":0.85,"h":0.14,"segments":12,"roll":90}'),
  ('mesh.mine_classical','rope','cylinder','mat.thatch',0,'','{"x":-0.55,"y":1.4,"z":-0.13,"w":0.04,"h":1.9,"segments":4,"far":0}'),
  ('mesh.mine_classical','bucket','box','mat.iron',0,'','{"x":-0.55,"y":1.1,"z":-0.13,"w":0.3,"d":0.3,"h":0.3}'),
  ('mesh.mine_classical','banner','banner','mat.cloth',1,'','{"x":-0.55,"y":2.82,"z":-0.92,"h":1.8,"fw":0.62,"fh":0.8}'),
  ('mesh.mine_classical','spoil','cone','mat.spoil',0,'','{"x":0.95,"z":0.95,"w":1.7,"h":0.8,"segments":8}'),
  ('mesh.mine_classical','spoil_back','cone','mat.spoil',0,'','{"x":1.2,"z":-0.15,"w":1.2,"h":0.55,"segments":7}'),
  ('mesh.mine_classical','spoil_low','cone','mat.spoil',0,'','{"x":0.3,"z":1.45,"w":0.8,"h":0.4,"segments":6}'),
  ('mesh.mine_classical','rails','box','mat.iron',0,'','{"x":-0.75,"y":0.05,"z":0.92,"w":0.05,"d":1.65,"h":0.05,"repeat":2,"dx":0.4}'),
  ('mesh.mine_classical','sleepers','box','mat.timber',0,'','{"x":-0.55,"y":0.02,"z":0.2,"w":0.65,"d":0.1,"h":0.04,"repeat":7,"dz":0.25,"far":0}'),
  ('mesh.mine_classical','cart','box','mat.timber',0,'','{"x":-0.55,"y":0.18,"z":1.05,"w":0.5,"d":0.7,"h":0.32}'),
  ('mesh.mine_classical','ore','dome','mat.spoil',0,'','{"x":-0.55,"y":0.5,"z":1.05,"w":0.45,"h":0.18,"segments":6,"far":0}'),
  ('mesh.mine_classical','cart_wheels_w','cylinder','mat.iron',0,'','{"x":-0.83,"y":0.21,"z":0.85,"w":0.22,"h":0.05,"segments":8,"roll":90,"repeat":2,"dz":0.4}'),
  ('mesh.mine_classical','cart_wheels_e','cylinder','mat.iron',0,'','{"x":-0.32,"y":0.21,"z":0.85,"w":0.22,"h":0.05,"segments":8,"roll":90,"repeat":2,"dz":0.4}'),
  ('mesh.mine_classical','house','include',NULL,0,'','{"mesh":"mesh.house_classical","x":1.1,"z":-1.25,"scale":0.7}'),
  ('mesh.mine_classical','shelter','awning','mat.cloth',1,'','{"x":-1.35,"z":0.95,"w":0.8,"d":1.1,"h":1.15,"rise":0.25}'),
  ('mesh.mine_classical','baskets','cylinder','mat.thatch',0,'','{"x":-1.5,"z":0.7,"w":0.3,"h":0.3,"segments":6,"repeat":2,"dz":0.45,"far":0}'),
  -- the classical university: a two-storey hall and wings round a court, an arched gate
  ('mesh.university_classical','base','plinth','mat.limestone',0,'','{"w":3.7,"d":3.7,"h":0.25,"steps":2,"inset":0.1}'),
  ('mesh.university_classical','hall','box','mat.lime_plaster',0,'','{"y":0.25,"z":-1.15,"w":3.4,"d":1.1,"h":2.3,"gable":22,"door":0.5,"door_h":1.0,"windows":5,"window_rows":2,"row_gap":1.0,"window_y":0.45,"window_w":0.18,"window_h":0.38,"side_windows":1}'),
  ('mesh.university_classical','hall_frieze','box','mat.cloth',1,'','{"y":2.33,"z":-1.15,"w":3.42,"d":1.12,"h":0.16}'),
  ('mesh.university_classical','hall_roof','gable_roof','mat.terracotta',0,'','{"y":2.55,"z":-1.15,"w":3.4,"d":1.1,"pitch":22,"overhang":0.1,"t":0.08}'),
  ('mesh.university_classical','ridge_cap','gable_roof','mat.cloth',1,'','{"y":2.737,"z":-1.15,"w":3.6,"d":0.6,"pitch":22,"overhang":0,"t":0.03}'),
  ('mesh.university_classical','porch_columns','column_row','mat.limestone',0,'','{"y":0.25,"z":-0.38,"count":4,"radius":0.09,"spacing":0.5,"h":1.5,"segments":8}'),
  ('mesh.university_classical','porch_beam','box','mat.limestone',0,'','{"y":1.75,"z":-0.48,"w":1.8,"d":0.36,"h":0.14}'),
  ('mesh.university_classical','wing_w','box','mat.lime_plaster',0,'','{"x":-1.3,"y":0.25,"z":0.35,"yaw":90,"w":1.6,"d":0.9,"h":1.5,"gable":22,"windows":3,"window_y":0.6,"window_w":0.16,"window_h":0.36}'),
  ('mesh.university_classical','wing_e','box','mat.lime_plaster',0,'','{"x":1.3,"y":0.25,"z":0.35,"yaw":-90,"w":1.6,"d":0.9,"h":1.5,"gable":22,"windows":3,"window_y":0.6,"window_w":0.16,"window_h":0.36}'),
  ('mesh.university_classical','wing_roof_w','gable_roof','mat.terracotta',0,'','{"x":-1.3,"y":1.75,"z":0.35,"yaw":90,"w":1.6,"d":0.9,"pitch":22,"overhang":0.04,"t":0.08}'),
  ('mesh.university_classical','wing_roof_e','gable_roof','mat.terracotta',0,'','{"x":1.3,"y":1.75,"z":0.35,"yaw":90,"w":1.6,"d":0.9,"pitch":22,"overhang":0.04,"t":0.08}'),
  ('mesh.university_classical','wing_caps','gable_roof','mat.cloth',1,'','{"x":-1.3,"y":1.917,"z":0.35,"yaw":90,"w":1.68,"d":0.5,"pitch":22,"overhang":0,"t":0.03,"repeat":2,"dx":2.6}'),
  ('mesh.university_classical','court_columns','column_row','mat.limestone',0,'','{"x":-0.7,"y":0.25,"z":0.35,"yaw":90,"count":3,"radius":0.08,"spacing":0.55,"h":1.3,"segments":8,"repeat":2,"dx":1.4}'),
  ('mesh.university_classical','court_beams','box','mat.limestone',0,'','{"x":-0.7,"y":1.55,"z":0.35,"yaw":90,"w":1.4,"d":0.22,"h":0.12,"repeat":2,"dx":1.4}'),
  ('mesh.university_classical','gate','arch','mat.limestone',0,'','{"y":0.25,"z":1.0,"w":1.75,"h":1.6,"t":0.3,"span":0.75,"spring":0.75}'),
  ('mesh.university_classical','gate_plaque','box','mat.cloth',1,'','{"y":1.48,"z":1.16,"w":0.9,"d":0.02,"h":0.28}'),
  ('mesh.university_classical','banners','banner','mat.cloth',1,'','{"x":-1.05,"y":0.25,"z":1.4,"h":2.6,"fw":0.45,"fh":0.8,"repeat":2,"dx":2.1}'),
  ('mesh.university_classical','obelisk_base','box','mat.limestone',0,'','{"y":0.25,"z":0.35,"w":0.4,"d":0.4,"h":0.3}'),
  ('mesh.university_classical','obelisk','box','mat.limestone',0,'','{"y":0.55,"z":0.35,"w":0.22,"d":0.22,"h":1.1,"taper":0.55}'),
  -- the classical temple: columns all round on a high stepped base, a painted pediment to the front
  ('mesh.temple_classical','base','plinth','mat.limestone',0,'','{"w":3.7,"d":3.7,"h":0.85,"steps":5,"inset":0.1}'),
  ('mesh.temple_classical','cella','box','mat.lime_plaster',0,'','{"y":0.85,"z":-0.15,"w":1.7,"d":1.9,"h":2.4,"door":0.6,"door_h":1.35}'),
  ('mesh.temple_classical','curtain','box','mat.cloth',1,'','{"y":0.85,"z":0.815,"w":0.55,"d":0.02,"h":1.27}'),
  ('mesh.temple_classical','front_columns','column_row','mat.limestone',0,'','{"y":0.85,"z":1.3,"count":6,"radius":0.11,"spacing":0.52,"h":2.4,"segments":8}'),
  ('mesh.temple_classical','back_columns','column_row','mat.limestone',0,'','{"y":0.85,"z":-1.3,"count":6,"radius":0.11,"spacing":0.52,"h":2.4,"segments":8,"far":0}'),
  ('mesh.temple_classical','side_columns','column_row','mat.limestone',0,'','{"x":-1.3,"y":0.85,"yaw":90,"count":4,"radius":0.11,"spacing":0.52,"h":2.4,"segments":8,"repeat":2,"dx":2.6}'),
  ('mesh.temple_classical','entablature','box','mat.limestone',0,'','{"y":3.25,"w":2.95,"d":2.95,"h":0.24}'),
  ('mesh.temple_classical','frieze','box','mat.cloth',1,'','{"y":3.3,"w":2.97,"d":2.97,"h":0.12}'),
  ('mesh.temple_classical','pediment','box','mat.cloth',1,'','{"y":3.49,"yaw":90,"w":2.95,"d":2.95,"h":0.02,"gable":20}'),
  ('mesh.temple_classical','roof','gable_roof','mat.terracotta',0,'','{"y":3.51,"yaw":90,"w":2.95,"d":2.95,"pitch":20,"overhang":0.1,"t":0.1}'),
  ('mesh.temple_classical','ridge_cap','gable_roof','mat.cloth',1,'','{"y":4.008,"yaw":90,"w":3.15,"d":0.8,"pitch":20,"overhang":0,"t":0.03}'),
  ('mesh.temple_classical','ridge','box','mat.cloth',1,'','{"y":4.156,"yaw":90,"w":3.2,"d":0.16,"h":0.08}'),
  ('mesh.temple_classical','acroteria','cone','mat.bronze',0,'','{"y":4.06,"z":-1.52,"w":0.2,"h":0.38,"segments":6,"repeat":2,"dz":3.04}'),
  -- the classical stable: a long low tiled barn of stalls, a fenced paddock with horses
  ('mesh.stable_classical','yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('mesh.stable_classical','barn_base','plinth','mat.limestone',0,'','{"z":-1.0,"w":3.5,"d":1.35,"h":0.12,"steps":1}'),
  ('mesh.stable_classical','barn','box','mat.lime_plaster',0,'','{"y":0.12,"z":-1.0,"w":3.4,"d":1.2,"h":1.1,"gable":22,"door":0.5,"door_h":0.85,"windows":6,"window_w":0.3,"window_h":0.42,"window_y":0.3,"side_windows":1,"opening":"#2a221c"}'),
  ('mesh.stable_classical','roof','gable_roof','mat.terracotta',0,'','{"y":1.22,"z":-1.0,"w":3.4,"d":1.2,"pitch":22,"overhang":0.1,"t":0.08}'),
  ('mesh.stable_classical','ridge_cap','gable_roof','mat.cloth',1,'','{"y":1.468,"z":-1,"w":3.6,"d":0.4,"pitch":22,"overhang":0,"t":0.03}'),
  ('mesh.stable_classical','ridge','box','mat.cloth',1,'','{"y":1.549,"z":-1.0,"w":3.6,"d":0.16,"h":0.08}'),
  ('mesh.stable_classical','paddock','fence','mat.timber',0,'','{"z":0.75,"w":3.5,"d":1.9,"h":0.6,"spacing":0.85,"rails":2,"ring":1,"gap":0.8,"far":0}'),
  ('mesh.stable_classical','horse_w','include',NULL,0,'','{"mesh":"mesh.horse","x":-0.8,"z":0.75,"yaw":70,"scale":0.9}'),
  ('mesh.stable_classical','horse_e','include',NULL,0,'','{"mesh":"mesh.horse","x":0.7,"z":0.95,"yaw":-120,"scale":0.9}'),
  ('mesh.stable_classical','trough','box','mat.timber',0,'','{"x":1.0,"z":0.15,"w":0.9,"d":0.28,"h":0.3,"far":0}'),
  ('mesh.stable_classical','hay','box','mat.thatch',0,'','{"x":-1.35,"z":0.25,"w":0.55,"d":0.4,"h":0.35}'),
  ('mesh.stable_classical','banner','banner','mat.cloth',1,'','{"x":0.55,"z":1.7,"h":2.2,"fw":0.45,"fh":0.65}'),
  -- a catapult frame half built: a wheeled base, an A-frame and the arm, no skein yet
  ('mesh.catapult_frame','frame','box','mat.timber',0,'','{"x":-0.35,"y":0.22,"w":0.16,"d":1.5,"h":0.16,"repeat":2,"dx":0.7}'),
  ('mesh.catapult_frame','frame_cross','box','mat.timber',0,'','{"y":0.22,"z":-0.55,"w":0.86,"d":0.14,"h":0.14,"repeat":2,"dz":1.1,"far":0}'),
  ('mesh.catapult_frame','wheels_w','cylinder','mat.timber',0,'','{"x":-0.55,"y":0.27,"z":-0.45,"w":0.54,"h":0.08,"segments":10,"roll":90,"repeat":2,"dz":0.9}'),
  ('mesh.catapult_frame','wheel_e','cylinder','mat.timber',0,'','{"x":0.47,"y":0.27,"z":0.45,"w":0.54,"h":0.08,"segments":10,"roll":90}'),
  ('mesh.catapult_frame','uprights_f','box','mat.timber',0,'','{"x":-0.35,"y":0.38,"z":-0.45,"w":0.1,"d":0.1,"h":1.1,"pitch":25,"repeat":2,"dx":0.7}'),
  ('mesh.catapult_frame','uprights_b','box','mat.timber',0,'','{"x":-0.35,"y":0.38,"z":0.45,"w":0.1,"d":0.1,"h":1.1,"pitch":-25,"repeat":2,"dx":0.7,"far":0}'),
  ('mesh.catapult_frame','crossbar','box','mat.timber',0,'','{"y":1.33,"w":0.86,"d":0.1,"h":0.1,"far":0}'),
  ('mesh.catapult_frame','arm','box','mat.timber',0,'','{"y":0.4,"z":0.5,"w":0.12,"d":0.12,"h":1.75,"pitch":-30}'),
  ('mesh.catapult_frame','cup','box','mat.timber',0,'arm','{"y":1.65,"w":0.28,"d":0.28,"h":0.12,"far":0}'),
  -- the classical siege workshop: a tall open shed of posts and ties, a half-built catapult
  ('mesh.siege_workshop_classical','yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('mesh.siege_workshop_classical','posts_back','box','mat.timber',0,'','{"x":-1.55,"z":-1.6,"w":0.16,"d":0.16,"h":2.3,"repeat":3,"dx":1.55}'),
  ('mesh.siege_workshop_classical','posts_front','box','mat.timber',0,'','{"x":-1.55,"z":-0.55,"w":0.16,"d":0.16,"h":2.3,"repeat":3,"dx":1.55}'),
  ('mesh.siege_workshop_classical','ties','box','mat.timber',0,'','{"y":2.16,"z":-1.6,"w":3.3,"d":0.14,"h":0.14,"repeat":2,"dz":1.05}'),
  ('mesh.siege_workshop_classical','cross_ties','box','mat.timber',0,'','{"x":-1.55,"y":2.16,"z":-1.075,"yaw":90,"w":1.2,"d":0.14,"h":0.14,"repeat":3,"dx":1.55,"far":0}'),
  ('mesh.siege_workshop_classical','braces_r','box','mat.timber',0,'','{"x":-1.55,"y":1.6,"z":-0.55,"w":0.08,"d":0.08,"h":0.75,"roll":45,"repeat":2,"dx":1.55,"far":0}'),
  ('mesh.siege_workshop_classical','braces_l','box','mat.timber',0,'','{"x":1.55,"y":1.6,"z":-0.55,"w":0.08,"d":0.08,"h":0.75,"roll":-45,"repeat":2,"dx":-1.55,"far":0}'),
  ('mesh.siege_workshop_classical','gable_ends','box','mat.timber',0,'','{"x":-1.55,"y":2.28,"z":-1.075,"w":0.08,"d":1.05,"h":0.02,"gable":28,"repeat":2,"dx":3.1}'),
  ('mesh.siege_workshop_classical','roof','gable_roof','mat.terracotta',0,'','{"y":2.3,"z":-1.075,"w":3.1,"d":1.05,"pitch":28,"overhang":0.18,"t":0.08}'),
  ('mesh.siege_workshop_classical','ridge_cap','gable_roof','mat.cloth',1,'','{"y":2.55,"z":-1.075,"w":3.46,"d":0.45,"pitch":28,"overhang":0,"t":0.03}'),
  ('mesh.siege_workshop_classical','ridge','box','mat.cloth',1,'','{"y":2.661,"z":-1.075,"w":3.4,"d":0.16,"h":0.08}'),
  ('mesh.siege_workshop_classical','hanging','box','mat.cloth',1,'','{"x":0.78,"y":1.0,"z":-1.6,"w":1.2,"d":0.03,"h":1.1}'),
  ('mesh.siege_workshop_classical','bench','box','mat.timber',0,'','{"x":-0.8,"z":-1.2,"w":1.2,"d":0.5,"h":0.55}'),
  ('mesh.siege_workshop_classical','beams','box','mat.sawn_timber',0,'','{"x":0.8,"z":-1.42,"w":1.3,"d":0.18,"h":0.18,"repeat":3,"dz":0.22}'),
  ('mesh.siege_workshop_classical','beams_top','box','mat.sawn_timber',0,'','{"x":0.8,"y":0.18,"z":-1.31,"w":1.25,"d":0.18,"h":0.18,"repeat":2,"dz":0.22,"tint":"#e6dccc","far":0}'),
  ('mesh.siege_workshop_classical','catapult','include',NULL,0,'','{"mesh":"mesh.catapult_frame","x":-0.2,"z":0.55,"scale":1.2}'),
  ('mesh.siege_workshop_classical','spare_wheel','cylinder','mat.timber',0,'','{"x":1.15,"z":0.75,"w":0.6,"h":0.08,"segments":10,"far":0}'),
  ('mesh.siege_workshop_classical','banner','banner','mat.cloth',1,'','{"x":1.55,"z":1.55,"h":2.8,"fw":0.55,"fh":0.9}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT m.id, v.name, v.shape, mt.id, v.team, v.attach, v.params
 FROM v JOIN def m ON m.key=v.m LEFT JOIN def mt ON mt.key=v.mat;
WITH v(e,a,m,mat) AS (VALUES
  ('bld.farm','age.ancient','mesh.farm_ancient','mat.mud_plaster'),
  ('bld.woodcutter_camp','age.ancient','mesh.woodcutter_ancient','mat.timber'),
  ('bld.woodcutter_camp','age.classical','mesh.woodcutter_classical','mat.timber'),
  ('bld.market','age.ancient','mesh.market_ancient','mat.timber'),
  ('bld.market','age.classical','mesh.market_classical','mat.limestone'),
  ('bld.tower','age.ancient','mesh.tower_ancient','mat.timber'),
  ('bld.tower','age.classical','mesh.tower_classical','mat.limestone'),
  ('bld.dock','age.ancient','mesh.dock_ancient','mat.timber'),
  ('bld.dock','age.classical','mesh.dock_classical','mat.limestone'),
  ('bld.barracks','age.classical','mesh.barracks_classical','mat.lime_plaster'),
  ('bld.library','age.classical','mesh.library_classical','mat.limestone'),
  ('bld.granary','age.classical','mesh.granary_classical','mat.lime_plaster'),
  ('bld.lumber_mill','age.classical','mesh.lumber_mill_classical','mat.lime_plaster'),
  ('bld.mine','age.classical','mesh.mine_classical','mat.timber'),
  ('bld.university','age.classical','mesh.university_classical','mat.lime_plaster'),
  ('bld.temple','age.classical','mesh.temple_classical','mat.limestone'),
  ('bld.stable','age.classical','mesh.stable_classical','mat.lime_plaster'),
  ('bld.siege_workshop','age.classical','mesh.siege_workshop_classical','mat.timber'))
INSERT INTO entity_visual(entity_type_id,age_id,mesh_id,material_id)
 SELECT e.id, a.id, m.id, mt.id FROM v JOIN def e ON e.key=v.e LEFT JOIN def a ON a.key=v.a
 JOIN def m ON m.key=v.m JOIN def mt ON mt.key=v.mat;

COMMIT;
