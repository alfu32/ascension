-- bld.star_fort@industrial: Industrial star fort (mesh.star_fort_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.star_fort_industrial','Industrial star fort');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.star_fort_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('ramparts','custom','mat.red_brick',0,'','{"file":"bld.star_fort@industrial/ramparts.mesh"}'),
  ('parade','plinth','mat.cinders',0,'','{"w":2.4,"d":2.4,"h":0.03,"steps":1}'),
  ('gate','box','mat.red_brick',0,'','{"z":1.55,"w":0.95,"d":0.5,"h":1.05,"door":0.42,"door_h":0.68,"door_colour":"#2a221c"}'),
  ('gate_parapet','crenellation','mat.red_brick',0,'','{"y":1.05,"z":1.55,"w":0.95,"d":0.5,"h":0.16,"t":0.1,"merlon":0.2,"ring":1,"far":0}'),
  ('gate_arms','shield','mat.cloth',1,'','{"y":0.86,"z":1.81,"w":0.26,"h":0.3,"t":0.03,"boss":0,"point":0.4,"rim":"#d6cbb0","far":0}'),
  ('barracks','box','mat.red_brick',0,'','{"z":-0.55,"w":2,"d":0.7,"h":0.8,"door":0.3,"door_h":0.5,"windows":5,"window_y":0.3,"window_w":0.12,"window_h":0.26,"opening":"#22282f"}'),
  ('barracks_roof','gable_roof','mat.corrugated',0,'','{"x":0,"y":0.8,"z":-0.55,"w":2,"d":0.7,"pitch":26,"overhang":0.08,"t":0.04}'),
  ('barracks_roof_cap','gable_roof','mat.cloth',1,'','{"x":0,"y":0.93,"z":-0.55,"w":2.16,"d":0.35,"pitch":26,"overhang":0,"t":0.03}'),
  ('magazine','dome','mat.turf',0,'','{"x":-0.75,"z":0.55,"w":0.7,"h":0.4,"segments":8,"far":0}'),
  ('mast','cylinder','mat.timber',0,'','{"x":0.7,"z":0.45,"w":0.06,"h":2.8,"segments":5}'),
  ('yard_arm','box','mat.timber',0,'','{"x":0.7,"y":2.3,"z":0.45,"w":0.8,"d":0.05,"h":0.05,"far":0}'),
  ('flag','banner','mat.cloth',1,'','{"x":0.7,"y":0,"z":0.45,"h":3.1,"fw":0.95,"fh":0.62}'),
  ('cupola_se','dome','mat.cast_iron',1,'','{"x":1.95,"y":0.76,"z":1.95,"w":0.6,"h":0.32,"segments":6}'),
  ('cupola_ne','dome','mat.cast_iron',0,'','{"x":1.95,"y":0.76,"z":-1.95,"w":0.6,"h":0.32,"segments":6}'),
  ('cupola_nw','dome','mat.cast_iron',0,'','{"x":-1.95,"y":0.76,"z":-1.95,"w":0.6,"h":0.32,"segments":6}'),
  ('cupola_sw','dome','mat.cast_iron',1,'','{"x":-1.95,"y":0.76,"z":1.95,"w":0.6,"h":0.32,"segments":6}'),
  ('gun_se','cylinder','mat.cast_iron',0,'','{"x":1.95,"y":0.92,"z":1.95,"w":0.1,"h":0.5,"segments":6,"yaw":45,"pitch":86,"far":0}'),
  ('gun_ne','cylinder','mat.cast_iron',0,'','{"x":1.95,"y":0.92,"z":-1.95,"w":0.1,"h":0.5,"segments":6,"yaw":135,"pitch":86,"far":0}'),
  ('gun_nw','cylinder','mat.cast_iron',0,'','{"x":-1.95,"y":0.92,"z":-1.95,"w":0.1,"h":0.5,"segments":6,"yaw":225,"pitch":86,"far":0}'),
  ('gun_sw','cylinder','mat.cast_iron',0,'','{"x":-1.95,"y":0.92,"z":1.95,"w":0.1,"h":0.5,"segments":6,"yaw":-45,"pitch":86,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.star_fort_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.star_fort'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.star_fort_industrial'), (SELECT id FROM def WHERE key='mat.red_brick'), 100);
