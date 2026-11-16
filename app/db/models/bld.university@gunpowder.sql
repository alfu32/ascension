-- bld.university@gunpowder: Early modern university (mesh.university_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.university_gunpowder','Early modern university');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.university_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','plinth','mat.ashlar',0,'','{"w":3.7,"d":3.7,"h":0.12,"steps":1}'),
  ('hall','box','mat.lime_plaster',0,'','{"y":0.12,"z":-1.2,"w":3.5,"d":1.1,"h":1.8,"windows":7,"window_y":0.24,"window_w":0.16,"window_h":0.42,"window_rows":2,"row_gap":0.8,"side_windows":1,"opening":"#2c3642"}'),
  ('hall_cornice','box','mat.ashlar',0,'','{"y":1.9,"z":-1.2,"w":3.58,"d":1.18,"h":0.07,"far":0}'),
  ('hall_roof','hip_roof','mat.slate',0,'','{"y":1.97,"z":-1.2,"w":3.5,"d":1.1,"pitch":40,"overhang":0.1,"t":0.08}'),
  ('hall_roof_cap','hip_roof','mat.cloth',1,'','{"y":2.326,"z":-1.2,"w":2.9,"d":0.5,"pitch":40,"overhang":0,"t":0.03}'),
  ('centre','box','mat.ashlar',0,'','{"y":0.12,"z":-0.6,"w":1,"d":0.14,"h":1.85,"door":0.44,"door_h":0.8,"windows":1,"window_y":1,"window_w":0.22,"window_h":0.48,"opening":"#2c3642"}'),
  ('pediment','box','mat.ashlar',0,'','{"y":1.97,"z":-0.6,"yaw":90,"w":0.14,"d":1.1,"h":0.03,"gable":24}'),
  ('tympanum','box','mat.cloth',1,'','{"y":2.02,"z":-0.52,"yaw":90,"w":0.02,"d":0.8,"h":0.01,"gable":24,"far":0}'),
  ('cupola','box','mat.lime_plaster',0,'','{"y":2.3,"z":-1.2,"w":0.42,"d":0.42,"h":0.5,"windows":1,"side_windows":1,"window_y":0.14,"window_w":0.12,"window_h":0.26}'),
  ('cupola_dome','dome','mat.copper',0,'','{"y":2.8,"z":-1.2,"w":0.52,"h":0.36,"segments":8}'),
  ('cupola_finial','cone','mat.brass',0,'','{"y":3.14,"z":-1.2,"w":0.08,"h":0.3,"segments":6}'),
  ('wing_w','box','mat.ochre_plaster',0,'','{"x":-1.35,"y":0.12,"z":0.3,"yaw":90,"w":1.9,"d":0.85,"h":1.45,"windows":4,"window_y":0.24,"window_w":0.15,"window_h":0.36,"window_rows":2,"row_gap":0.62,"opening":"#2c3642"}'),
  ('wing_e','box','mat.ochre_plaster',0,'','{"x":1.35,"y":0.12,"z":0.3,"yaw":-90,"w":1.9,"d":0.85,"h":1.45,"windows":4,"window_y":0.24,"window_w":0.15,"window_h":0.36,"window_rows":2,"row_gap":0.62,"opening":"#2c3642"}'),
  ('wing_roof_w','hip_roof','mat.slate',0,'','{"x":-1.35,"y":1.57,"z":0.3,"yaw":90,"w":1.9,"d":0.85,"pitch":40,"overhang":0.08,"t":0.07}'),
  ('wing_cap_w','hip_roof','mat.cloth',1,'','{"x":-1.35,"y":1.867,"z":0.3,"yaw":90,"w":1.41,"d":0.36,"pitch":40,"overhang":0,"t":0.03}'),
  ('wing_roof_e','hip_roof','mat.slate',0,'','{"x":1.35,"y":1.57,"z":0.3,"yaw":90,"w":1.9,"d":0.85,"pitch":40,"overhang":0.08,"t":0.07}'),
  ('wing_cap_e','hip_roof','mat.cloth',1,'','{"x":1.35,"y":1.867,"z":0.3,"yaw":90,"w":1.41,"d":0.36,"pitch":40,"overhang":0,"t":0.03}'),
  ('railing','wall','mat.ashlar',0,'','{"x":-1.24,"y":0.12,"z":1.7,"w":1.24,"h":0.3,"t":0.12,"far":0}'),
  ('railing_e','wall','mat.ashlar',0,'','{"x":1.24,"y":0.12,"z":1.7,"w":1.24,"h":0.3,"t":0.12,"far":0}'),
  ('gate_piers','box','mat.ashlar',0,'','{"x":-0.5,"y":0.12,"z":1.7,"w":0.22,"d":0.22,"h":0.9,"repeat":2,"dx":1}'),
  ('lawn','box','mat.turf',0,'','{"y":0.12,"z":0.45,"w":1.6,"d":1.7,"h":0.02}'),
  ('banners','banner','mat.cloth',1,'','{"x":-0.95,"y":0.12,"z":1.45,"h":2.4,"fw":0.42,"fh":0.8,"repeat":2,"dx":1.9}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.university_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.university'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.university_gunpowder'), (SELECT id FROM def WHERE key='mat.lime_plaster'), 100);
