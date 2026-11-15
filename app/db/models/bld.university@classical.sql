-- bld.university@classical: Classical university (mesh.university_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.university_classical','Classical university');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.university_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','plinth','mat.limestone',0,'','{"w":3.7,"d":3.7,"h":0.25,"steps":2,"inset":0.1}'),
  ('hall','box','mat.lime_plaster',0,'','{"y":0.25,"z":-1.15,"w":3.4,"d":1.1,"h":2.3,"gable":22,"door":0.5,"door_h":1.0,"windows":5,"window_rows":2,"row_gap":1.0,"window_y":0.45,"window_w":0.18,"window_h":0.38,"side_windows":1}'),
  ('hall_frieze','box','mat.cloth',1,'','{"y":2.33,"z":-1.15,"w":3.42,"d":1.12,"h":0.16}'),
  ('hall_roof','gable_roof','mat.terracotta',0,'','{"y":2.55,"z":-1.15,"w":3.4,"d":1.1,"pitch":22,"overhang":0.1,"t":0.08}'),
  ('ridge_cap','gable_roof','mat.cloth',1,'','{"y":2.737,"z":-1.15,"w":3.6,"d":0.6,"pitch":22,"overhang":0,"t":0.03}'),
  ('porch_columns','column_row','mat.limestone',0,'','{"y":0.25,"z":-0.38,"count":4,"radius":0.09,"spacing":0.5,"h":1.5,"segments":8}'),
  ('porch_beam','box','mat.limestone',0,'','{"y":1.75,"z":-0.48,"w":1.8,"d":0.36,"h":0.14}'),
  ('wing_w','box','mat.lime_plaster',0,'','{"x":-1.3,"y":0.25,"z":0.35,"yaw":90,"w":1.6,"d":0.9,"h":1.5,"gable":22,"windows":3,"window_y":0.6,"window_w":0.16,"window_h":0.36}'),
  ('wing_e','box','mat.lime_plaster',0,'','{"x":1.3,"y":0.25,"z":0.35,"yaw":-90,"w":1.6,"d":0.9,"h":1.5,"gable":22,"windows":3,"window_y":0.6,"window_w":0.16,"window_h":0.36}'),
  ('wing_roof_w','gable_roof','mat.terracotta',0,'','{"x":-1.3,"y":1.75,"z":0.35,"yaw":90,"w":1.6,"d":0.9,"pitch":22,"overhang":0.04,"t":0.08}'),
  ('wing_roof_e','gable_roof','mat.terracotta',0,'','{"x":1.3,"y":1.75,"z":0.35,"yaw":90,"w":1.6,"d":0.9,"pitch":22,"overhang":0.04,"t":0.08}'),
  ('wing_caps','gable_roof','mat.cloth',1,'','{"x":-1.3,"y":1.917,"z":0.35,"yaw":90,"w":1.68,"d":0.5,"pitch":22,"overhang":0,"t":0.03,"repeat":2,"dx":2.6}'),
  ('court_columns','column_row','mat.limestone',0,'','{"x":-0.7,"y":0.25,"z":0.35,"yaw":90,"count":3,"radius":0.08,"spacing":0.55,"h":1.3,"segments":8,"repeat":2,"dx":1.4}'),
  ('court_beams','box','mat.limestone',0,'','{"x":-0.7,"y":1.55,"z":0.35,"yaw":90,"w":1.4,"d":0.22,"h":0.12,"repeat":2,"dx":1.4}'),
  ('gate','arch','mat.limestone',0,'','{"y":0.25,"z":1.0,"w":1.75,"h":1.6,"t":0.3,"span":0.75,"spring":0.75}'),
  ('gate_plaque','box','mat.cloth',1,'','{"y":1.48,"z":1.16,"w":0.9,"d":0.02,"h":0.28}'),
  ('banners','banner','mat.cloth',1,'','{"x":-1.05,"y":0.25,"z":1.4,"h":2.6,"fw":0.45,"fh":0.8,"repeat":2,"dx":2.1}'),
  ('obelisk_base','box','mat.limestone',0,'','{"y":0.25,"z":0.35,"w":0.4,"d":0.4,"h":0.3}'),
  ('obelisk','box','mat.limestone',0,'','{"y":0.55,"z":0.35,"w":0.22,"d":0.22,"h":1.1,"taper":0.55}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.university_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.university'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.university_classical'), (SELECT id FROM def WHERE key='mat.lime_plaster'), 100);
