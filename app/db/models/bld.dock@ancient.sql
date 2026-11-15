-- bld.dock@ancient: Early dock (mesh.dock_ancient)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.dock_ancient','Early dock');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.dock_ancient';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.04,"steps":1}'),
  ('shed_back','box','mat.mud_brick',0,'','{"x":-0.85,"z":-1.58,"yaw":90,"w":0.14,"d":1.6,"h":1.1,"gable":36}'),
  ('shed_wall','wall','mat.mud_brick',0,'','{"x":-1.58,"z":-0.15,"yaw":90,"w":3.0,"h":1.1,"t":0.14,"windows":3,"window_y":0.6,"window_w":0.18,"window_h":0.22}'),
  ('shed_posts','cylinder','mat.timber',0,'','{"x":-0.12,"z":-1.5,"w":0.13,"h":1.1,"segments":6,"repeat":3,"dz":1.4}'),
  ('shed_roof','gable_roof','mat.thatch',0,'','{"x":-0.85,"y":1.1,"z":-0.15,"yaw":90,"w":3.0,"d":1.6,"pitch":36,"overhang":0.18,"t":0.1}'),
  ('ridge_cap','gable_roof','mat.cloth',1,'','{"x":-0.85,"y":1.623,"z":-0.15,"yaw":90,"w":3.36,"d":0.5,"pitch":36,"overhang":0,"t":0.03}'),
  ('shed_ridge','box','mat.cloth',1,'','{"x":-0.85,"y":1.769,"z":-0.15,"yaw":90,"w":3.36,"d":0.2,"h":0.1}'),
  ('boat','hull','mat.timber',0,'','{"x":-0.85,"y":0.18,"z":0.45,"yaw":90,"w":2.3,"d":0.8,"h":0.38,"sheer":0.22}'),
  ('boat_stripe','box','mat.cloth',1,'','{"x":-0.85,"y":0.5,"z":0.45,"yaw":90,"w":1.2,"d":0.84,"h":0.08}'),
  ('chocks','box','mat.timber',0,'','{"x":-0.85,"z":-0.15,"w":0.6,"d":0.15,"h":0.2,"repeat":3,"dz":0.6,"far":0}'),
  ('pier','box','mat.timber',0,'','{"x":0.95,"y":0.4,"w":1.1,"d":3.5,"h":0.1}'),
  ('pier_beams','box','mat.timber',0,'','{"x":0.95,"y":0.28,"w":0.9,"d":3.4,"h":0.12,"tint":"#a89888"}'),
  ('pier_posts','cylinder','mat.timber',0,'','{"x":0.45,"z":-1.6,"w":0.15,"h":0.8,"segments":6,"repeat":4,"dz":1.067}'),
  ('pier_posts_e','cylinder','mat.timber',0,'','{"x":1.45,"z":-1.6,"w":0.15,"h":0.8,"segments":6,"repeat":4,"dz":1.067,"far":0}'),
  ('steps','plinth','mat.timber',0,'','{"x":0.28,"z":0.3,"w":0.36,"d":0.6,"h":0.45,"steps":3,"inset":0.06}'),
  ('shelter','awning','mat.cloth',1,'','{"x":0.95,"y":0.5,"z":-1.2,"w":1.0,"d":0.8,"h":1.0,"rise":0.25}'),
  ('crate','box','mat.timber',0,'','{"x":1.2,"y":0.5,"z":-1.3,"w":0.35,"d":0.35,"h":0.35,"far":0}'),
  ('amphorae','cylinder','mat.mud_plaster',0,'','{"x":0.7,"y":0.5,"z":-1.4,"w":0.2,"h":0.36,"segments":6,"taper":0.35,"repeat":2,"dz":0.25,"far":0}'),
  ('rope','cylinder','mat.thatch',0,'','{"x":1.15,"y":0.5,"z":0.75,"w":0.32,"h":0.08,"segments":8,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.4,"y":0.5,"z":1.5,"h":2.4,"fw":0.5,"fh":0.75}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.dock_ancient'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.dock'), NULL, (SELECT id FROM def WHERE key='age.ancient'), (SELECT id FROM def WHERE key='mesh.dock_ancient'), (SELECT id FROM def WHERE key='mat.timber'), 100);
