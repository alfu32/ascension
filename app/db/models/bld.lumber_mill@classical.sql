-- bld.lumber_mill@classical: Classical lumber mill (mesh.lumber_mill_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.lumber_mill_classical','Classical lumber mill');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.lumber_mill_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('mill_base','plinth','mat.limestone',0,'','{"x":-0.8,"z":-0.95,"w":1.85,"d":1.5,"h":0.2,"steps":1}'),
  ('mill','box','mat.lime_plaster',0,'','{"x":-0.8,"y":0.2,"z":-0.95,"w":1.7,"d":1.35,"h":1.45,"gable":26,"door":0.45,"door_h":0.85,"door_x":-0.4,"windows":2,"side_windows":1,"window_y":0.85,"window_w":0.18,"window_h":0.25}'),
  ('mill_roof','gable_roof','mat.terracotta',0,'','{"x":-0.8,"y":1.65,"z":-0.95,"w":1.7,"d":1.35,"pitch":26,"overhang":0.12,"t":0.08}'),
  ('ridge_cap','gable_roof','mat.cloth',1,'','{"x":-0.8,"y":1.946,"z":-0.95,"w":1.94,"d":0.5,"pitch":26,"overhang":0,"t":0.03}'),
  ('mill_ridge','box','mat.cloth',1,'','{"x":-0.8,"y":2.063,"z":-0.95,"w":1.95,"d":0.16,"h":0.08}'),
  ('porch','awning','mat.cloth',1,'','{"x":-1.2,"z":-0.05,"w":0.9,"d":0.45,"h":1.05,"rise":0.2}'),
  ('wheel','cylinder','mat.timber',0,'','{"x":0.1,"y":0.85,"z":-0.95,"w":1.5,"h":0.22,"segments":12,"roll":90}'),
  ('hub','cylinder','mat.iron',0,'','{"x":0.04,"y":0.85,"z":-0.95,"w":0.18,"h":0.34,"segments":6,"roll":90}'),
  ('paddle_0','box','mat.timber',0,'','{"x":0.21,"y":0.85,"z":-0.95,"w":0.3,"d":0.06,"h":0.88}'),
  ('paddle_90','box','mat.timber',0,'','{"x":0.21,"y":0.85,"z":-0.95,"w":0.3,"d":0.06,"h":0.88,"pitch":90}'),
  ('paddle_180','box','mat.timber',0,'','{"x":0.21,"y":0.85,"z":-0.95,"w":0.3,"d":0.06,"h":0.88,"pitch":180}'),
  ('paddle_270','box','mat.timber',0,'','{"x":0.21,"y":0.85,"z":-0.95,"w":0.3,"d":0.06,"h":0.88,"pitch":-90}'),
  ('paddle_45','box','mat.timber',0,'','{"x":0.21,"y":0.85,"z":-0.95,"w":0.3,"d":0.06,"h":0.88,"pitch":45,"far":0}'),
  ('paddle_135','box','mat.timber',0,'','{"x":0.21,"y":0.85,"z":-0.95,"w":0.3,"d":0.06,"h":0.88,"pitch":135,"far":0}'),
  ('paddle_225','box','mat.timber',0,'','{"x":0.21,"y":0.85,"z":-0.95,"w":0.3,"d":0.06,"h":0.88,"pitch":-135,"far":0}'),
  ('paddle_315','box','mat.timber',0,'','{"x":0.21,"y":0.85,"z":-0.95,"w":0.3,"d":0.06,"h":0.88,"pitch":-45,"far":0}'),
  ('saw_posts','box','mat.timber',0,'','{"x":0.1,"z":0.55,"w":0.15,"d":0.15,"h":2.2,"repeat":2,"dx":1.2}'),
  ('saw_head','box','mat.timber',0,'','{"x":0.7,"y":2.08,"z":0.55,"w":1.45,"d":0.17,"h":0.17}'),
  ('saw_sash','box','mat.timber',0,'','{"x":0.7,"y":1.3,"z":0.55,"w":1.2,"d":0.1,"h":0.1}'),
  ('saw_blades','box','mat.iron',0,'','{"x":0.55,"y":0.5,"z":0.55,"w":0.03,"d":0.05,"h":0.8,"repeat":3,"dx":0.15,"far":0}'),
  ('saw_log','cylinder','mat.timber',0,'','{"x":0.7,"y":0.62,"z":-0.15,"w":0.36,"h":1.9,"segments":7,"pitch":90}'),
  ('trestles','box','mat.timber',0,'','{"x":0.7,"z":0.1,"w":0.5,"d":0.1,"h":0.45,"repeat":3,"dz":0.65,"far":0}'),
  ('planks_1','box','mat.sawn_timber',0,'','{"x":-1.05,"z":0.9,"w":1.3,"d":0.95,"h":0.13}'),
  ('sticks_1','box','mat.timber',0,'','{"x":-1.05,"y":0.13,"z":0.55,"w":1.3,"d":0.06,"h":0.04,"repeat":3,"dz":0.35,"far":0}'),
  ('planks_2','box','mat.sawn_timber',0,'','{"x":-1.05,"y":0.17,"z":0.9,"yaw":2,"w":1.3,"d":0.95,"h":0.13,"tint":"#f0e6d8"}'),
  ('sticks_2','box','mat.timber',0,'','{"x":-1.05,"y":0.3,"z":0.55,"w":1.3,"d":0.06,"h":0.04,"repeat":3,"dz":0.35,"far":0}'),
  ('planks_3','box','mat.sawn_timber',0,'','{"x":-1.05,"y":0.34,"z":0.9,"yaw":-1.5,"w":1.25,"d":0.95,"h":0.13}'),
  ('sticks_3','box','mat.timber',0,'','{"x":-1.05,"y":0.47,"z":0.55,"w":1.25,"d":0.06,"h":0.04,"repeat":3,"dz":0.35,"far":0}'),
  ('planks_4','box','mat.sawn_timber',0,'','{"x":-1.05,"y":0.51,"z":0.9,"yaw":1,"w":1.2,"d":0.95,"h":0.13,"tint":"#f0e6d8"}'),
  ('planks_ne','box','mat.sawn_timber',0,'','{"x":1.15,"z":-1.2,"w":1.1,"d":0.8,"h":0.13}'),
  ('sticks_ne','box','mat.timber',0,'','{"x":1.15,"y":0.13,"z":-1.45,"w":1.1,"d":0.06,"h":0.04,"repeat":3,"dz":0.25,"far":0}'),
  ('planks_ne_2','box','mat.sawn_timber',0,'','{"x":1.15,"y":0.17,"z":-1.2,"yaw":-2,"w":1.05,"d":0.8,"h":0.13,"tint":"#f0e6d8"}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.55,"z":1.5,"h":2.4,"fw":0.5,"fh":0.75}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.lumber_mill_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.lumber_mill'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.lumber_mill_classical'), (SELECT id FROM def WHERE key='mat.lime_plaster'), 100);
