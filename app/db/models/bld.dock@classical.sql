-- bld.dock@classical: Classical harbour (mesh.dock_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.dock_classical','Classical harbour');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.dock_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('quay','plinth','mat.limestone',0,'','{"w":3.6,"d":3.6,"h":0.12,"steps":1}'),
  ('shed_back','box','mat.lime_plaster',0,'','{"x":-0.85,"y":0.12,"z":-1.58,"yaw":90,"w":0.16,"d":1.6,"h":1.42,"gable":24}'),
  ('shed_wall','wall','mat.lime_plaster',0,'','{"x":-1.58,"y":0.12,"z":-0.15,"yaw":90,"w":3.0,"h":1.42,"t":0.16,"windows":3,"window_y":0.8,"window_w":0.2,"window_h":0.3}'),
  ('shed_columns','column_row','mat.limestone',0,'','{"x":-0.12,"y":0.12,"z":-0.15,"yaw":90,"count":4,"radius":0.09,"spacing":0.95,"h":1.3,"segments":8}'),
  ('shed_beam','box','mat.limestone',0,'','{"x":-0.12,"y":1.42,"z":-0.15,"w":0.22,"d":3.1,"h":0.12}'),
  ('shed_roof','gable_roof','mat.terracotta',0,'','{"x":-0.85,"y":1.54,"z":-0.15,"yaw":90,"w":3.0,"d":1.6,"pitch":24,"overhang":0.15,"t":0.08}'),
  ('ridge_cap','gable_roof','mat.cloth',1,'','{"x":-0.85,"y":1.872,"z":-0.15,"yaw":90,"w":3.3,"d":0.5,"pitch":24,"overhang":0,"t":0.03}'),
  ('shed_ridge','box','mat.cloth',1,'','{"x":-0.85,"y":1.981,"z":-0.15,"yaw":90,"w":3.3,"d":0.16,"h":0.08}'),
  ('galley','hull','mat.timber',0,'','{"x":-0.85,"y":0.32,"z":0.3,"yaw":90,"w":2.6,"d":0.8,"h":0.42,"sheer":0.3}'),
  ('galley_stripe','box','mat.cloth',1,'','{"x":-0.85,"y":0.66,"z":0.3,"yaw":90,"w":1.4,"d":0.84,"h":0.1}'),
  ('chocks','box','mat.limestone',0,'','{"x":-0.85,"y":0.12,"z":-0.35,"w":0.6,"d":0.15,"h":0.2,"repeat":3,"dz":0.65,"far":0}'),
  ('pier','box','mat.limestone',0,'','{"x":0.95,"y":0.4,"w":1.1,"d":3.5,"h":0.14}'),
  ('pier_piers','box','mat.limestone',0,'','{"x":0.95,"y":0.12,"z":-1.6,"w":1.0,"d":0.28,"h":0.28,"repeat":4,"dz":1.067,"tint":"#d8d2c6"}'),
  ('bollards','cylinder','mat.iron',0,'','{"x":0.5,"y":0.52,"z":-0.9,"w":0.13,"h":0.22,"segments":6,"repeat":3,"dz":1.1}'),
  ('crane_leg_w','cylinder','mat.timber',0,'','{"x":0.62,"y":0.52,"z":-0.9,"w":0.12,"h":3.0,"segments":6,"roll":6,"pitch":22}'),
  ('crane_leg_e','cylinder','mat.timber',0,'','{"x":1.28,"y":0.52,"z":-0.9,"w":0.12,"h":3.0,"segments":6,"roll":-6,"pitch":22}'),
  ('crane_stay','cylinder','mat.thatch',0,'','{"x":0.95,"y":0.52,"z":-1.7,"w":0.04,"h":3.37,"segments":4,"pitch":34.7,"far":0}'),
  ('crane_rope','cylinder','mat.thatch',0,'','{"x":0.95,"y":1.6,"z":0.22,"w":0.04,"h":1.7,"segments":4,"far":0}'),
  ('crane_load','box','mat.timber',0,'','{"x":0.95,"y":1.2,"z":0.22,"w":0.42,"d":0.42,"h":0.4}'),
  ('steps','plinth','mat.limestone',0,'','{"x":0.28,"y":0.12,"z":0.6,"w":0.36,"d":0.6,"h":0.4,"steps":3,"inset":0.06}'),
  ('shelter','awning','mat.cloth',1,'','{"x":0.95,"y":0.52,"z":1.0,"w":1.0,"d":0.8,"h":1.05,"rise":0.25}'),
  ('amphorae','cylinder','mat.terracotta',0,'','{"x":0.7,"y":0.52,"z":-1.45,"w":0.2,"h":0.38,"segments":6,"taper":0.35,"repeat":3,"dx":0.25,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.42,"y":0.52,"z":1.6,"h":2.5,"fw":0.5,"fh":0.8}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.dock_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.dock'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.dock_classical'), (SELECT id FROM def WHERE key='mat.limestone'), 100);
