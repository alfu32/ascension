-- bld.oil_well@industrial: Oil well (mesh.oil_well_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.oil_well_industrial','Oil well');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.oil_well_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":1.8,"d":1.8,"h":0.04,"steps":1,"tint":"#8a7a68"}'),
  ('pool','cylinder','mat.crude',0,'','{"x":0.45,"y":0.04,"z":0.55,"w":0.6,"h":0.01,"segments":8,"far":0}'),
  ('derrick_floor','box','mat.sawn_timber',0,'','{"x":-0.3,"y":0.04,"z":-0.25,"w":0.95,"d":0.95,"h":0.1}'),
  ('leg_sw','box','mat.timber',0,'','{"x":-0.7,"y":0.14,"z":0.15,"w":0.07,"d":0.07,"h":3.1,"roll":7,"pitch":-7}'),
  ('leg_se','box','mat.timber',0,'','{"x":0.1,"y":0.14,"z":0.15,"w":0.07,"d":0.07,"h":3.1,"roll":-7,"pitch":-7}'),
  ('leg_nw','box','mat.timber',0,'','{"x":-0.7,"y":0.14,"z":-0.65,"w":0.07,"d":0.07,"h":3.1,"roll":7,"pitch":7}'),
  ('leg_ne','box','mat.timber',0,'','{"x":0.1,"y":0.14,"z":-0.65,"w":0.07,"d":0.07,"h":3.1,"roll":-7,"pitch":7}'),
  ('girts','box','mat.timber',0,'','{"x":-0.3,"y":0.9,"z":0.07,"w":0.72,"d":0.04,"h":0.05,"repeat":1}'),
  ('girts_2','box','mat.timber',0,'','{"x":-0.3,"y":1.7,"z":0,"w":0.53,"d":0.04,"h":0.05,"far":0}'),
  ('girts_3','box','mat.timber',0,'','{"x":-0.3,"y":2.5,"z":-0.08,"w":0.35,"d":0.04,"h":0.05,"far":0}'),
  ('cross','box','mat.timber',0,'','{"x":-0.62,"y":0.16,"z":0.13,"w":0.04,"d":0.03,"h":1,"roll":42,"far":0}'),
  ('cross_2','box','mat.timber',0,'','{"x":0.02,"y":0.16,"z":0.13,"w":0.04,"d":0.03,"h":1,"roll":-42,"far":0}'),
  ('crown','box','mat.sawn_timber',0,'','{"x":-0.3,"y":3.2,"z":-0.25,"w":0.34,"d":0.34,"h":0.1}'),
  ('crown_flag','banner','mat.cloth',1,'','{"x":-0.3,"y":3.3,"z":-0.25,"h":0.75,"fw":0.5,"fh":0.32}'),
  ('sheave','shield','mat.steel',0,'','{"x":-0.3,"y":3.15,"z":-0.25,"yaw":90,"w":0.28,"t":0.04,"boss":0.03,"segments":8,"rim":"#4a4e56","far":0}'),
  ('casing','cylinder','mat.cast_iron',0,'','{"x":-0.3,"y":0.14,"z":-0.25,"w":0.12,"h":0.35,"segments":6}'),
  ('walking_beam','box','mat.oak',0,'','{"x":0.25,"y":0.95,"z":-0.25,"w":0.85,"d":0.1,"h":0.1,"roll":8}'),
  ('samson_post','box','mat.oak',0,'','{"x":0.3,"y":0.14,"z":-0.25,"w":0.1,"d":0.14,"h":0.8}'),
  ('engine_house','box','mat.sawn_timber',0,'','{"x":0.55,"z":-0.45,"w":0.6,"d":0.65,"h":0.7,"door":0.2,"door_h":0.45,"opening":"#22282f","yaw":180}'),
  ('engine_roof','gable_roof','mat.corrugated',0,'','{"x":0.55,"y":0.7,"z":-0.45,"w":0.6,"d":0.65,"pitch":28,"overhang":0.05,"t":0.03}'),
  ('engine_roof_cap','gable_roof','mat.cloth',1,'','{"x":0.55,"y":0.812,"z":-0.45,"w":0.7,"d":0.358,"pitch":28,"overhang":0,"t":0.03}'),
  ('engine_stack','cylinder','mat.cast_iron',0,'','{"x":0.72,"y":0.7,"z":-0.6,"w":0.08,"h":0.7,"segments":5}'),
  ('tank','cylinder','mat.rust',0,'','{"x":0.5,"z":0.45,"w":0.62,"h":0.6,"segments":9}'),
  ('tank_band','cylinder','mat.cloth',1,'','{"x":0.5,"y":0.36,"z":0.45,"w":0.64,"h":0.24,"segments":9}'),
  ('tank_roof','cone','mat.cast_iron',0,'','{"x":0.5,"y":0.6,"z":0.45,"w":0.66,"h":0.12,"segments":9}'),
  ('barrels','cylinder','mat.oak',0,'','{"x":-0.65,"y":0.04,"z":0.65,"w":0.2,"h":0.28,"segments":6,"repeat":2,"dx":0.24,"far":0,"tint":"#7a5a40"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.oil_well_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.oil_well'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.oil_well_industrial'), (SELECT id FROM def WHERE key='mat.timber'), 100);
