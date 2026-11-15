-- bld.granary@medieval: Medieval granary (mesh.granary_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.granary_medieval','Medieval granary');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.granary_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.rough_stone',0,'','{"w":3.6,"d":3.6,"h":0.08,"steps":1,"tint":"#c4beb2"}'),
  ('silo_c','include',NULL,0,'','{"mesh":"mesh.silo_medieval","y":0.08,"z":-1.12,"scale":1.05}'),
  ('silo_w','include',NULL,0,'','{"mesh":"mesh.silo_medieval","x":-1.15,"y":0.08,"z":-0.5}'),
  ('silo_e','include',NULL,0,'','{"mesh":"mesh.silo_medieval","x":1.15,"y":0.08,"z":-0.5}'),
  ('house','include',NULL,0,'','{"mesh":"mesh.house_medieval","x":0.95,"y":0.08,"z":1.05,"scale":0.85}'),
  ('cart_bed','box','mat.timber',0,'','{"x":-1,"y":0.3,"z":1,"w":0.7,"d":0.95,"h":0.18}'),
  ('cart_sacks','dome','mat.cloth',0,'','{"x":-1,"y":0.48,"z":1,"w":0.6,"h":0.3,"segments":8,"tint":"#c9b48a"}'),
  ('cart_wheels','cylinder','mat.timber',0,'','{"x":-1.38,"y":0.28,"z":1,"w":0.56,"h":0.07,"segments":10,"roll":90,"repeat":2,"dx":0.83}'),
  ('ladder','fence','mat.oak',0,'','{"x":0.17,"y":1.1,"z":-0.3,"w":2,"h":0.35,"spacing":0.3,"rails":2,"roll":-90,"pitch":-14,"far":0}'),
  ('sacks','dome','mat.cloth',0,'','{"x":-0.25,"y":0.08,"z":0.45,"w":0.34,"h":0.3,"segments":6,"tint":"#c9b48a","repeat":3,"dx":0.32,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":0,"y":0.08,"z":1.55,"h":2.4,"fw":0.5,"fh":0.75}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.granary_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.granary'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.granary_medieval'), (SELECT id FROM def WHERE key='mat.rough_stone'), 100);
