-- bld.farm@industrial: Industrial farm (mesh.farm_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.farm_industrial','Industrial farm');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.farm_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('soil','plinth','mat.soil',0,'','{"w":3.8,"d":3.8,"h":0.03,"steps":1}'),
  ('barn','box','mat.red_brick',0,'','{"x":-1.1,"z":-1.25,"w":1.4,"d":1,"h":0.85,"gable":40,"door":0.46,"door_h":0.62,"door_colour":"#3a2a1c","side_windows":1,"window_y":0.4,"window_w":0.12,"window_h":0.22,"opening":"#22282f"}'),
  ('roof','gable_roof','mat.slate',0,'','{"x":-1.1,"y":0.85,"z":-1.25,"w":1.4,"d":1,"pitch":40,"overhang":0.1,"t":0.07}'),
  ('roof_cap','gable_roof','mat.cloth',1,'','{"x":-1.1,"y":1.176,"z":-1.25,"w":1.6,"d":0.44,"pitch":40,"overhang":0,"t":0.03}'),
  ('vent','box','mat.sawn_timber',0,'','{"x":-1.1,"y":1.2,"z":-1.25,"w":0.24,"d":0.24,"h":0.2,"far":0}'),
  ('dutch_posts','box','mat.cast_iron',0,'','{"x":0.05,"z":-1.65,"w":0.07,"d":0.07,"h":1.15,"repeat":3,"dx":0.6}'),
  ('dutch_posts_s','box','mat.cast_iron',0,'','{"x":0.05,"z":-0.75,"w":0.07,"d":0.07,"h":1.15,"repeat":3,"dx":0.6,"far":0}'),
  ('dutch_roof','gable_roof','mat.corrugated',0,'','{"x":0.65,"y":1.15,"z":-1.2,"w":1.3,"d":1,"pitch":24,"overhang":0.12,"t":0.04}'),
  ('dutch_cap','gable_roof','mat.cloth',1,'','{"x":0.65,"y":1.305,"z":-1.2,"w":1.54,"d":0.5,"pitch":24,"overhang":0,"t":0.03}'),
  ('hay','box','mat.thatch',0,'','{"x":0.65,"z":-1.2,"w":1.1,"d":0.8,"h":0.8}'),
  ('pump_legs','box','mat.cast_iron',0,'','{"x":1.25,"z":0.6,"w":0.06,"d":0.06,"h":2.6,"roll":-6,"pitch":6,"repeat":1}'),
  ('pump_legs_e','box','mat.cast_iron',0,'','{"x":1.65,"z":0.6,"w":0.06,"d":0.06,"h":2.6,"roll":6,"pitch":6,"far":0}'),
  ('pump_legs_n','box','mat.cast_iron',0,'','{"x":1.45,"z":0.3,"w":0.06,"d":0.06,"h":2.6,"pitch":6,"far":0}'),
  ('pump_brace','box','mat.cast_iron',0,'','{"x":1.45,"y":1,"z":0.62,"w":0.36,"d":0.04,"h":0.04,"far":0}'),
  ('wheel','shield','mat.steel',0,'','{"x":1.45,"y":2.62,"z":0.92,"w":0.72,"t":0.04,"boss":0.06,"segments":12,"rim":"#5a5e66"}'),
  ('wheel_hub','box','mat.cast_iron',0,'','{"x":1.45,"y":2.62,"z":0.68,"w":0.12,"d":0.4,"h":0.12}'),
  ('vane','box','mat.cloth',1,'','{"x":1.45,"y":2.6,"z":0.25,"w":0.03,"d":0.45,"h":0.32}'),
  ('tank','cylinder','mat.corrugated',0,'','{"x":0.55,"z":0.55,"w":0.75,"h":0.55,"segments":9}'),
  ('tank_rim','cylinder','mat.cast_iron',0,'','{"x":0.55,"y":0.55,"z":0.55,"w":0.78,"h":0.05,"segments":9,"far":0}'),
  ('trough','box','mat.cast_iron',0,'','{"x":0.5,"z":1.1,"w":0.7,"d":0.22,"h":0.22,"far":0}'),
  ('rick','cone','mat.thatch',0,'','{"x":-1.2,"z":0.1,"w":0.7,"h":0.85,"segments":7}'),
  ('rick_2','cone','mat.thatch',0,'','{"x":-0.45,"z":0.25,"w":0.55,"h":0.65,"segments":7,"far":0}'),
  ('flag','banner','mat.cloth',1,'','{"x":-0.3,"z":-0.55,"h":1.9,"fw":0.45,"fh":0.6}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.farm_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.farm'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.farm_industrial'), (SELECT id FROM def WHERE key='mat.red_brick'), 100);
