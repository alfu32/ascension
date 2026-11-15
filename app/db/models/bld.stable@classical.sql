-- bld.stable@classical: Classical stable (mesh.stable_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.stable_classical','Classical stable');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.stable_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('barn_base','plinth','mat.limestone',0,'','{"z":-1.0,"w":3.5,"d":1.35,"h":0.12,"steps":1}'),
  ('barn','box','mat.lime_plaster',0,'','{"y":0.12,"z":-1.0,"w":3.4,"d":1.2,"h":1.1,"gable":22,"door":0.5,"door_h":0.85,"windows":6,"window_w":0.3,"window_h":0.42,"window_y":0.3,"side_windows":1,"opening":"#2a221c"}'),
  ('roof','gable_roof','mat.terracotta',0,'','{"y":1.22,"z":-1.0,"w":3.4,"d":1.2,"pitch":22,"overhang":0.1,"t":0.08}'),
  ('ridge_cap','gable_roof','mat.cloth',1,'','{"y":1.468,"z":-1,"w":3.6,"d":0.4,"pitch":22,"overhang":0,"t":0.03}'),
  ('ridge','box','mat.cloth',1,'','{"y":1.549,"z":-1.0,"w":3.6,"d":0.16,"h":0.08}'),
  ('paddock','fence','mat.timber',0,'','{"z":0.75,"w":3.5,"d":1.9,"h":0.6,"spacing":0.85,"rails":2,"ring":1,"gap":0.8,"far":0}'),
  ('horse_w','include',NULL,0,'','{"mesh":"mesh.horse","x":-0.8,"z":0.75,"yaw":70,"scale":0.9}'),
  ('horse_e','include',NULL,0,'','{"mesh":"mesh.horse","x":0.7,"z":0.95,"yaw":-120,"scale":0.9}'),
  ('trough','box','mat.timber',0,'','{"x":1.0,"z":0.15,"w":0.9,"d":0.28,"h":0.3,"far":0}'),
  ('hay','box','mat.thatch',0,'','{"x":-1.35,"z":0.25,"w":0.55,"d":0.4,"h":0.35}'),
  ('banner','banner','mat.cloth',1,'','{"x":0.55,"z":1.7,"h":2.2,"fw":0.45,"fh":0.65}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.stable_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.stable'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.stable_classical'), (SELECT id FROM def WHERE key='mat.lime_plaster'), 100);
