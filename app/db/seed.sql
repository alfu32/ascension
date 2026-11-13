-- Rise of Man — minimal seed. Proves the schema; NOT the balanced game data.
-- Values are placeholders. Load after schema.sql.
PRAGMA foreign_keys = ON;
BEGIN;

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
 ('city.spacing',24000,'milli-tiles','minimum distance between cities'),
 ('city.assimilate_ticks',2400,'ticks','captured city becomes yours after this'),
 ('player.pop_cap.base',50,'pop','before modifiers (stat pop_cap)'),
 ('player.city_limit.base',1,'cities','before modifiers (stat city_limit)'),
 ('player.commerce_cap.base',70,'per period','before modifiers (stat commerce_cap)'),
 ('build.site_start_hp_pct',10,'%','hit points of a building site when placed'),
 ('build.reach',1500,'milli-tiles','how close a builder must be to the footprint to work'),
 ('path.nodes_per_tick',20000,'nodes','path-search nodes expanded per tick across all searches');

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
 ('healer','role'), ('city','structure'), ('temple','structure'), ('wonder','structure'),
 ('production','structure'), ('gather','structure');

INSERT INTO stat(key,scope,note) VALUES
 ('hp','entity',''), ('armor','entity',''), ('attack','entity',''), ('range','entity',''),
 ('speed','entity',''), ('los','entity',''), ('heal_rate','entity',''), ('build_time','entity','train/construct ticks'),
 ('cost.all','entity','every resource of the cost'), ('cost.food','entity',''), ('cost.timber','entity',''),
 ('cost.metal','entity',''), ('cost.wealth','entity',''), ('cost.knowledge','entity',''),
 ('border_radius','entity',''), ('garrison_capacity','entity',''), ('attrition_taken','entity',''),
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

INSERT INTO def(kind,key,name) VALUES
 ('resource','res.food','Food'), ('resource','res.timber','Timber'), ('resource','res.metal','Metal'),
 ('resource','res.wealth','Wealth'), ('resource','res.knowledge','Knowledge'), ('resource','res.oil','Oil'),
 ('resource','res.energy','Energy'), ('resource','rare.relics','Relics'), ('resource','rare.horses','Horses');
WITH v(k,kind,ab,cap,tr,age) AS (VALUES
  ('res.food','base','F',1,1,NULL), ('res.timber','base','T',1,1,NULL), ('res.metal','base','M',1,1,'age.classical'),
  ('res.wealth','base','W',1,0,NULL), ('res.knowledge','base','K',0,0,NULL), ('res.oil','base','O',1,1,'age.industrial'),
  ('res.energy','base','E',1,1,'age.fusion'), ('rare.relics','rare','Re',0,0,NULL), ('rare.horses','rare','Ho',0,0,NULL)
 )
INSERT INTO resource(id,kind,abbrev,rate_capped,tradeable,first_age_id)
 SELECT d.id, v.kind, v.ab, v.cap, v.tr, (SELECT id FROM def WHERE key=v.age) FROM v JOIN def d ON d.key=v.k;
WITH v(r,b,n) AS (VALUES ('rare.relics','res.knowledge',20), ('rare.horses','res.food',10),
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

-- ── entity types ────────────────────────────────────────────────────────────
INSERT INTO def(kind,key,name) VALUES
 ('entity_type','bld.city','City'), ('entity_type','bld.farm','Farm'), ('entity_type','bld.library','Library'),
 ('entity_type','bld.temple','Temple'), ('entity_type','bld.barracks','Barracks'),
 ('entity_type','bld.spaceport','Spaceport'),
 ('entity_type','unit.citizen','Citizen'), ('entity_type','unit.priest','Priest'),
 ('entity_type','unit.medic','Medic'), ('entity_type','unit.spearman','Spearman'),
 ('entity_type','unit.merchant','Merchant'), ('entity_type','unit.repair_drone','Repair Drone'),
 ('entity_type','node.relics','Relic Site'), ('entity_type','node.ore_planet','Ore Planet'),
 ('entity_type','unit.survey_ship','Survey Ship');
WITH v(k,cls,grp,hp,ar,ac,los,pop,w,h,t,ramp,rg,ls,lc,age) AS (VALUES
  ('bld.city','building','economic',1200,3,'structure',12000,0,7,7,600,'per_type','','none',0,NULL),
  ('bld.farm','building','economic',400,0,'structure',6000,0,4,4,150,'per_type','','per_city',5,NULL),
  ('bld.library','building','economic',800,2,'structure',8000,0,4,4,400,'none','','per_city',1,NULL),
  ('bld.temple','building','economic',800,2,'structure',8000,0,4,4,400,'per_type','','per_city',1,'age.classical'),
  ('bld.barracks','building','military',1000,4,'structure',10000,0,4,4,420,'per_type','','none',0,NULL),
  ('bld.spaceport','building','military',2500,6,'structure',16000,0,8,8,1200,'per_type','','none',0,'age.starfaring'),
  ('unit.citizen','unit','civilian',40,0,'flesh',2000,1,1,1,100,'per_type','','none',0,NULL),
  ('unit.priest','unit','civilian',35,0,'flesh',6000,1,1,1,300,'per_type','','none',0,'age.classical'),
  ('unit.medic','unit','foot',60,1,'flesh',6000,1,1,1,260,'per_group','barracks','none',0,'age.industrial'),
  ('unit.spearman','unit','foot',110,2,'plate',5000,1,1,1,190,'per_group','barracks','none',0,NULL),
  ('unit.merchant','unit','civilian',50,0,'flesh',4000,1,1,1,300,'per_type','','none',0,NULL),
  ('unit.repair_drone','unit','mechanized',45,1,'vehicle',6000,1,1,1,200,'per_type','','none',0,'age.information'),
  ('node.relics','node','',1,0,NULL,0,0,3,3,0,'none','','none',0,NULL),
  ('node.ore_planet','node','',1,0,NULL,0,0,2,2,0,'none','','none',0,'age.starfaring'),
  ('unit.survey_ship','unit','starship',300,3,'hull',9000,2,1,1,500,'per_type','','none',0,'age.starfaring')
 )
INSERT INTO entity_type(id,class,ui_group,hp,armor,armor_class_id,los,pop_cost,footprint_w,footprint_h,build_ticks,ramp_mode,ramp_group,limit_scope,limit_count,first_age_id)
 SELECT d.id, v.cls, v.grp, v.hp, v.ar, (SELECT id FROM armor_class WHERE key=v.ac), v.los, v.pop, v.w, v.h, v.t,
        v.ramp, v.rg, v.ls, v.lc, (SELECT id FROM def WHERE key=v.age)
 FROM v JOIN def d ON d.key=v.k;

WITH v(k,t) AS (VALUES
  ('bld.city','city'), ('bld.temple','temple'), ('bld.barracks','production'), ('bld.farm','gather'),
  ('unit.citizen','biological'), ('unit.citizen','civilian'),
  ('unit.priest','biological'), ('unit.priest','civilian'), ('unit.priest','healer'),
  ('unit.medic','biological'), ('unit.medic','military'), ('unit.medic','healer'), ('unit.medic','infantry'),
  ('unit.spearman','biological'), ('unit.spearman','military'), ('unit.spearman','infantry'),
  ('unit.merchant','biological'), ('unit.merchant','civilian'),
  ('unit.repair_drone','mechanical'), ('unit.repair_drone','drone'), ('unit.repair_drone','healer'),
  ('unit.survey_ship','mechanical'), ('unit.survey_ship','starship')
 )
INSERT INTO def_tag SELECT d.id, t.id FROM v JOIN def d ON d.key=v.k JOIN tag t ON t.key=v.t;

WITH v(k,m,sp) AS (VALUES ('unit.citizen','foot',1300), ('unit.priest','foot',1100),
  ('unit.medic','foot',1400), ('unit.spearman','foot',1250), ('unit.merchant','wheeled',1500),
  ('unit.repair_drone','aircraft',2600), ('unit.survey_ship','starship',2200))
INSERT INTO et_movement(entity_type_id,movement_class_id,speed,turn_rate)
 SELECT d.id, m.id, v.sp, 360 FROM v
 JOIN def d ON d.key=v.k JOIN movement_class m ON m.key=v.m;

WITH v(k,dmg,dc,rng,cd) AS (VALUES ('unit.spearman',9,'melee',0,30),
  ('unit.citizen',2,'melee',0,40), ('bld.city',8,'pierce',10000,40))
INSERT INTO et_weapon(entity_type_id,slot,damage,damage_class_id,range_max,cooldown_ticks)
 SELECT d.id, 0, v.dmg, c.id, v.rng, v.cd FROM v
 JOIN def d ON d.key=v.k JOIN damage_class c ON c.key=v.dc;
INSERT INTO et_weapon_target SELECT w.id, 'ground', 100 FROM et_weapon w;

INSERT INTO et_builder(entity_type_id) SELECT id FROM def WHERE key='unit.citizen';
INSERT INTO et_harvester SELECT (SELECT id FROM def WHERE key='unit.citizen'), (SELECT id FROM def WHERE key='res.food'), 10;
INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_max)
 SELECT (SELECT id FROM def WHERE key='bld.farm'), (SELECT id FROM def WHERE key='res.food'), 1, 1;
INSERT INTO et_node(entity_type_id,resource_id,amount,claim_mode,claim_tag_id,claim_radius,income) VALUES
 ((SELECT id FROM def WHERE key='node.relics'), (SELECT id FROM def WHERE key='rare.relics'), NULL, 'trader', NULL, 0, 0),
 -- resource planet: small on the map, owned by whoever keeps a starship next to it
 ((SELECT id FROM def WHERE key='node.ore_planet'), (SELECT id FROM def WHERE key='res.metal'), NULL, 'presence',
  (SELECT id FROM tag WHERE key='starship'), 3000, 40);

WITH v(k,t,hps,rng,e,conv) AS (VALUES
  ('unit.priest','biological',4,5000,0,1), ('unit.medic','biological',8,3000,1,0),
  ('unit.repair_drone','mechanical',8,2000,0,0))
INSERT INTO et_healer(entity_type_id,target_tag_id,hp_per_second,range,energy_per_hp,can_convert)
 SELECT d.id, t.id, v.hps, v.rng, v.e, v.conv FROM v
 JOIN def d ON d.key=v.k JOIN tag t ON t.key=v.t;
INSERT INTO et_energy SELECT id, 100, 2 FROM def WHERE key IN ('unit.medic','unit.priest');
INSERT INTO et_trader(entity_type_id,claims_nodes) SELECT id, 1 FROM def WHERE key='unit.merchant';
INSERT INTO et_container(entity_type_id,capacity,heals,adds_firepower) SELECT id, 10, 1, 1 FROM def WHERE key='bld.city';
INSERT INTO et_producer(entity_type_id)
 SELECT id FROM def WHERE key IN ('bld.city','bld.library','bld.temple','bld.barracks','bld.spaceport');
WITH v(k,c,b,e,cap) AS (VALUES ('bld.city',1,24000,20000,1), ('bld.temple',0,2000,0,0),
  ('bld.spaceport',0,12000,0,0))
INSERT INTO et_territory(entity_type_id,is_city,border_radius,economic_radius,capturable)
 SELECT d.id, v.c, v.b, v.e, v.cap FROM v JOIN def d ON d.key=v.k;

WITH v(k,r,l,dist) AS (VALUES ('bld.farm','in_city_radius',NULL,0),
  ('bld.library','in_city_radius',NULL,0), ('bld.temple','in_city_radius',NULL,0),
  ('bld.barracks','in_own_territory',NULL,0), ('bld.city','in_own_territory',NULL,0),
  ('bld.spaceport','in_own_territory',NULL,0), ('bld.spaceport','on_layer','ground',0))
INSERT INTO placement_rule(entity_type_id,rule,layer,distance)
 SELECT d.id, v.r, v.l, v.dist FROM v
 JOIN def d ON d.key=v.k;
INSERT INTO completion_bonus SELECT (SELECT id FROM def WHERE key='bld.farm'), (SELECT id FROM def WHERE key='res.food'), 20;

-- ── techs ───────────────────────────────────────────────────────────────────
INSERT INTO def(kind,key,name) VALUES
 ('tech','tech.age.classical','Advance to Classical Age'), ('tech','tech.age.abundance','Advance to Age of Abundance'),
 ('tech','tech.religion','Religion'), ('tech','tech.continental_union','Continental Union'),
 ('tech','tech.unification','Unification of Humanity');
WITH v(k,kind,tr,lv,t,ex) AS (VALUES
  ('tech.age.classical','age','',1,600,''), ('tech.age.abundance','age','',12,3000,''),
  ('tech.religion','upgrade','',1,500,''), ('tech.continental_union','project','',0,3000,''),
  ('tech.unification','project','',0,4000,'')
 )
INSERT INTO tech(id,kind,track,level,research_ticks,exclusive_group)
 SELECT d.id, v.kind, v.tr, v.lv, v.t, v.ex FROM v JOIN def d ON d.key=v.k;
WITH v(t,eff,x) AS (VALUES ('tech.age.classical','set_age','age.classical'),
  ('tech.age.abundance','set_age','age.abundance'), ('tech.unification','change_faction','race.human')
 )
INSERT INTO tech_effect(tech_id,effect,target_def_id)
 SELECT t.id, v.eff, x.id FROM v JOIN def t ON t.key=v.t JOIN def x ON x.key=v.x;
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

WITH v(a,b) AS (VALUES ('bld.temple','age.classical'), ('unit.priest','tech.religion'),
  ('tech.unification','age.starfaring'), ('tech.continental_union','age.automation'), ('bld.spaceport','age.starfaring'))
INSERT INTO prerequisite(def_id,requires_def_id)
 SELECT a.id, b.id FROM v
 JOIN def a ON a.key=v.a JOIN def b ON b.key=v.b;

WITH v(k,r,n,ramp) AS (VALUES
  ('unit.citizen','res.food',20,1), ('unit.spearman','res.food',40,2), ('unit.spearman','res.timber',20,1),
  ('unit.priest','res.wealth',60,5), ('unit.medic','res.food',50,2), ('unit.medic','res.wealth',30,1),
  ('unit.merchant','res.wealth',30,10), ('unit.survey_ship','res.metal',200,20), ('unit.survey_ship','res.energy',100,10), ('bld.farm','res.timber',50,10), ('bld.temple','res.timber',80,20),
  ('bld.temple','res.wealth',40,10), ('bld.barracks','res.timber',60,15), ('bld.library','res.timber',60,0),
  ('bld.city','res.food',100,50), ('bld.city','res.timber',100,50),
  ('tech.age.classical','res.food',250,0), ('tech.religion','res.wealth',100,0)
 )
INSERT INTO cost(def_id,resource_id,amount,ramp)
 SELECT d.id, r.id, v.n, v.ramp FROM v JOIN def d ON d.key=v.k JOIN def r ON r.key=v.r;

WITH v(p,x,mode,page,c,r) AS (VALUES
  ('bld.city','unit.citizen','train','',0,0),
  ('bld.city','unit.merchant','train','',1,0),
  ('bld.temple','unit.priest','train','',0,0),
  ('bld.temple','tech.religion','research','',0,1),
  ('bld.barracks','unit.spearman','train','',0,0),
  ('bld.barracks','unit.medic','train','',1,0),
  ('bld.barracks','unit.repair_drone','train','',2,0),
  ('bld.spaceport','unit.survey_ship','train','',0,0),
  ('bld.library','tech.age.classical','research','',0,0),
  ('bld.library','tech.age.abundance','research','',0,0+1),
  ('bld.library','tech.unification','research','',1,1),
  ('bld.library','tech.continental_union','research','',4,1),
  ('unit.citizen','bld.city','construct','economic',0,0),
  ('unit.citizen','bld.farm','construct','economic',1,0),
  ('unit.citizen','bld.library','construct','economic',2,0),
  ('unit.citizen','bld.temple','construct','economic',3,0),
  ('unit.citizen','bld.barracks','construct','military',0,0),
  ('unit.citizen','bld.spaceport','construct','military',1,0)
 )
INSERT INTO production(producer_type_id,product_def_id,mode,ui_page,ui_col,ui_row)
 SELECT p.id, x.id, v.mode, v.page, v.c, v.r FROM v JOIN def p ON p.key=v.p JOIN def x ON x.key=v.x;

-- ── modifiers ───────────────────────────────────────────────────────────────
-- Post-scarcity: every cost becomes zero; production TIME is untouched.
INSERT INTO modifier(source_def_id,stat_key,op,value)
 SELECT id,'cost.all','mul',0 FROM def WHERE key='tech.age.abundance';
-- Religion: temples push borders further, cities are tougher.
WITH v(k,t,s,op,n) AS (VALUES ('tech.religion','temple','border_radius','add',2000),
  ('tech.religion','city','hp','pct',25))
INSERT INTO modifier(source_def_id,target_tag_id,stat_key,op,value)
 SELECT d.id, t.id, v.s, v.op, v.n FROM v JOIN def d ON d.key=v.k JOIN tag t ON t.key=v.t;
-- Continental blocs are a human stage: only humans (and their nations) can research the union.
INSERT INTO availability(def_id,faction_id)
 SELECT t.id, f.id FROM def t, def f WHERE t.key='tech.continental_union' AND f.key='race.human';
-- Race traits (placeholders): the same roster, tilted by a few modifiers.
INSERT INTO modifier(source_def_id,target_tag_id,stat_key,op,value)
 SELECT d.id, t.id, v.s, v.op, v.n FROM (SELECT 1) JOIN (
   SELECT 'race.reptilian' AS k,'biological' AS t,'hp' AS s,'pct' AS op,15 AS n
   UNION ALL SELECT 'race.reptilian','biological','speed','pct',-5
   UNION ALL SELECT 'race.avian','biological','speed','pct',15
   UNION ALL SELECT 'race.avian','biological','los','pct',25
   UNION ALL SELECT 'race.avian','biological','hp','pct',-10) v
 JOIN def d ON d.key=v.k JOIN tag t ON t.key=v.t;
-- The same building under each race's own name.
INSERT INTO faction_alias(def_id,faction_id,name)
 SELECT x.id, f.id, v.n FROM (
   SELECT 'bld.city' AS x,'race.reptilian' AS f,'Warren' AS n
   UNION ALL SELECT 'bld.city','race.avian','Roost'
   UNION ALL SELECT 'unit.citizen','race.reptilian','Hatchling'
   UNION ALL SELECT 'unit.citizen','race.avian','Fledgling') v
 JOIN def x ON x.key=v.x JOIN def f ON f.key=v.f;
-- Rare resource bonus: holding Relics makes research faster.
INSERT INTO modifier(source_def_id,stat_key,op,value)
 SELECT id,'research_time','pct',-33 FROM def WHERE key='rare.relics';

-- ── abilities, formations, stances ──────────────────────────────────────────
INSERT INTO def(kind,key,name) VALUES ('ability','abl.convert','Convert');
INSERT INTO ability(id,target,target_relation,target_tag_id,range,energy_cost,cast_ticks,cooldown_ticks,effect)
 SELECT d.id,'entity','enemy',(SELECT id FROM tag WHERE key='biological'),5000,100,80,200,'convert'
 FROM def d WHERE d.key='abl.convert';
INSERT INTO ability_grant(entity_type_id,ability_id)
 SELECT (SELECT id FROM def WHERE key='unit.priest'), (SELECT id FROM def WHERE key='abl.convert');

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
WITH v(p,seq,a,k,n) AS (VALUES
  ('balanced',0,'train','unit.citizen',6), ('balanced',1,'construct','bld.farm',3),
  ('balanced',2,'construct','bld.library',1), ('balanced',3,'train','unit.citizen',10),
  ('balanced',4,'construct','bld.barracks',1), ('balanced',5,'research','tech.age.classical',1),
  ('rusher',0,'train','unit.citizen',5), ('rusher',1,'construct','bld.farm',2),
  ('rusher',2,'construct','bld.barracks',1), ('rusher',3,'train','unit.spearman',6))
INSERT INTO ai_opening_step(profile_key,seq,action,subject_def_id,count)
 SELECT v.p, v.seq, v.a, d.id, v.n FROM v JOIN def d ON d.key=v.k;
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

COMMIT;
