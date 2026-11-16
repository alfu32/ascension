-- bld.lumber_mill@gunpowder: Early modern lumber mill (mesh.lumber_mill_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.lumber_mill_gunpowder','Early modern lumber mill');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.lumber_mill_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('mill','box','mat.brick',0,'','{"x":-0.8,"z":-0.95,"w":1.75,"d":1.4,"h":1.45,"gable":44,"door":0.44,"door_h":0.62,"door_x":-0.4,"windows":2,"window_y":0.2,"window_w":0.16,"window_h":0.3,"window_rows":2,"row_gap":0.66,"opening":"#2c3642"}'),
  ('mill_roof','gable_roof','mat.pantile',0,'','{"x":-0.8,"y":1.45,"z":-0.95,"w":1.75,"d":1.4,"pitch":44,"overhang":0.12,"t":0.08}'),
  ('mill_roof_cap','gable_roof','mat.cloth',1,'','{"x":-0.8,"y":1.996,"z":-0.95,"w":2,"d":0.5,"pitch":44,"overhang":0,"t":0.03}'),
  ('race','box','mat.ashlar',0,'','{"x":0.3,"z":-0.95,"w":0.4,"d":1.6,"h":0.18}'),
  ('wheel','cylinder','mat.timber',0,'','{"x":0.3,"y":0.85,"z":-0.95,"w":1.5,"h":0.22,"segments":12,"roll":90}'),
  ('hub','cylinder','mat.iron',0,'','{"x":0.24,"y":0.85,"z":-0.95,"w":0.18,"h":0.34,"segments":6,"roll":90}'),
  ('paddle_0','box','mat.timber',0,'','{"x":0.41,"y":0.85,"z":-0.95,"w":0.3,"d":0.06,"h":0.88}'),
  ('paddle_90','box','mat.timber',0,'','{"x":0.41,"y":0.85,"z":-0.95,"w":0.3,"d":0.06,"h":0.88,"pitch":90}'),
  ('paddle_180','box','mat.timber',0,'','{"x":0.41,"y":0.85,"z":-0.95,"w":0.3,"d":0.06,"h":0.88,"pitch":180}'),
  ('paddle_270','box','mat.timber',0,'','{"x":0.41,"y":0.85,"z":-0.95,"w":0.3,"d":0.06,"h":0.88,"pitch":-90}'),
  ('saw_posts','box','mat.sawn_timber',0,'','{"x":0.1,"z":0.55,"w":0.15,"d":0.15,"h":2.2,"repeat":2,"dx":1.2}'),
  ('saw_head','box','mat.sawn_timber',0,'','{"x":0.7,"y":2.08,"z":0.55,"w":1.45,"d":0.17,"h":0.17}'),
  ('saw_cloth','box','mat.cloth',1,'','{"x":0.7,"y":1.55,"z":0.62,"w":0.9,"d":0.03,"h":0.45}'),
  ('saw_sash','box','mat.oak',0,'','{"x":0.7,"y":1.3,"z":0.55,"w":1.2,"d":0.1,"h":0.1}'),
  ('saw_blades','box','mat.iron',0,'','{"x":0.55,"y":0.5,"z":0.55,"w":0.03,"d":0.05,"h":0.8,"repeat":3,"dx":0.15,"far":0}'),
  ('saw_log','cylinder','mat.timber',0,'','{"x":0.7,"y":0.62,"z":-0.15,"w":0.36,"h":1.9,"segments":7,"pitch":90}'),
  ('trestles','box','mat.oak',0,'','{"x":0.7,"z":0.1,"w":0.5,"d":0.1,"h":0.45,"repeat":3,"dz":0.65,"far":0}'),
  ('planks_1','box','mat.sawn_timber',0,'','{"x":-1.05,"z":0.9,"w":1.3,"d":0.95,"h":0.13}'),
  ('planks_2','box','mat.sawn_timber',0,'','{"x":-1.05,"y":0.17,"z":0.9,"w":1.27,"d":0.95,"h":0.13,"tint":"#f0e6d8","yaw":2}'),
  ('planks_3','box','mat.sawn_timber',0,'','{"x":-1.05,"y":0.34,"z":0.9,"w":1.24,"d":0.95,"h":0.13,"yaw":-1.5}'),
  ('planks_ne','box','mat.sawn_timber',0,'','{"x":1.15,"z":-1.2,"w":1.1,"d":0.8,"h":0.13}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.55,"z":1.5,"h":2.4,"fw":0.5,"fh":0.75}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.lumber_mill_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.lumber_mill'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.lumber_mill_gunpowder'), (SELECT id FROM def WHERE key='mat.brick'), 100);
