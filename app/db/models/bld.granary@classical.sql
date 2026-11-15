-- bld.granary@classical: Classical granary (mesh.granary_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.granary_classical','Classical granary');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.granary_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.limestone',0,'','{"w":3.6,"d":3.6,"h":0.1,"steps":1}'),
  ('silo_c','include',NULL,0,'','{"mesh":"mesh.silo","y":0.1,"z":-1.15,"scale":1.05}'),
  ('silo_w','include',NULL,0,'','{"mesh":"mesh.silo","x":-1.15,"y":0.1,"z":-0.55}'),
  ('silo_e','include',NULL,0,'','{"mesh":"mesh.silo","x":1.15,"y":0.1,"z":-0.55}'),
  ('hive_footing','cylinder','mat.fieldstone',0,'','{"x":-1.1,"y":0.1,"z":0.95,"w":1.15,"h":0.18,"segments":10}'),
  ('hive','cylinder','mat.lime_plaster',0,'','{"x":-1.1,"y":0.28,"z":0.95,"w":1.05,"h":0.55,"segments":10}'),
  ('hive_band','cylinder','mat.cloth',1,'','{"x":-1.1,"y":0.6,"z":0.95,"w":1.08,"h":0.2,"segments":10}'),
  ('hive_dome','dome','mat.lime_plaster',0,'','{"x":-1.1,"y":0.83,"z":0.95,"w":1.05,"h":0.75,"segments":10}'),
  ('house','include',NULL,0,'','{"mesh":"mesh.house_classical","x":1.0,"y":0.1,"z":1.05,"scale":0.85}'),
  ('ladder','fence','mat.timber',0,'','{"x":0.17,"y":1.1,"z":-0.3,"w":2.0,"h":0.35,"spacing":0.3,"rails":2,"roll":-90,"pitch":-14,"far":0}'),
  ('sacks','dome','mat.cloth',0,'','{"x":-0.3,"y":0.1,"z":0.55,"w":0.34,"h":0.3,"segments":6,"tint":"#c9b48a","repeat":3,"dx":0.32,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":0.3,"y":0.1,"z":1.55,"h":2.4,"fw":0.5,"fh":0.75}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.granary_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.granary'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.granary_classical'), (SELECT id FROM def WHERE key='mat.lime_plaster'), 100);
