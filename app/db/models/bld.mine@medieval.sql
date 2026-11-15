-- bld.mine@medieval: Medieval mine (mesh.mine_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.mine_medieval','Medieval mine');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.mine_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('collar','wall','mat.rough_stone',0,'','{"x":-0.55,"z":-0.55,"w":1.2,"d":1.2,"h":0.4,"t":0.22,"ring":1}'),
  ('pit','box','mat.rough_stone',0,'','{"x":-0.55,"y":0.04,"z":-0.55,"w":0.8,"d":0.8,"h":0.03,"tint":"#2a2622"}'),
  ('posts','box','mat.oak',0,'','{"x":-1.25,"z":-1.25,"w":0.16,"d":0.16,"h":2.3,"repeat":2,"dx":1.4}'),
  ('posts_front','box','mat.oak',0,'','{"x":-1.25,"z":0.15,"w":0.16,"d":0.16,"h":2.3,"repeat":2,"dx":1.4}'),
  ('girts','box','mat.oak',0,'','{"x":-0.55,"y":1.3,"z":-1.25,"w":1.4,"d":0.1,"h":0.1,"repeat":2,"dz":1.4,"far":0}'),
  ('braces','box','mat.oak',0,'','{"x":-1.2,"y":1.75,"z":0.15,"w":0.07,"d":0.07,"h":0.6,"roll":45,"far":0}'),
  ('braces_e','box','mat.oak',0,'','{"x":0.1,"y":1.75,"z":0.15,"w":0.07,"d":0.07,"h":0.6,"roll":-45,"far":0}'),
  ('head','box','mat.oak',0,'','{"x":-0.55,"y":2.22,"z":-0.55,"w":1.6,"d":1.6,"h":0.12}'),
  ('roof','gable_roof','mat.shingle',0,'','{"x":-0.55,"y":2.34,"z":-0.55,"w":1.6,"d":1.6,"pitch":48,"overhang":0.16,"t":0.08}'),
  ('roof_cap','gable_roof','mat.cloth',1,'','{"x":-0.55,"y":3.07,"z":-0.55,"w":1.96,"d":0.5,"pitch":48,"overhang":0,"t":0.03}'),
  ('rope','cylinder','mat.thatch',0,'','{"x":-0.55,"y":1,"z":-0.55,"w":0.04,"h":1.25,"segments":4,"far":0}'),
  ('bucket','box','mat.iron',0,'','{"x":-0.55,"y":0.75,"z":-0.55,"w":0.3,"d":0.3,"h":0.3}'),
  ('wheel','cylinder','mat.oak',0,'','{"x":-1.45,"y":1,"z":0.95,"w":1.4,"h":0.18,"segments":10,"roll":90,"bottom":1}'),
  ('wheel_hub','cylinder','mat.iron',0,'','{"x":-1.5,"y":1,"z":0.95,"w":0.16,"h":0.3,"segments":6,"roll":90,"far":0}'),
  ('wheel_posts','box','mat.oak',0,'','{"x":-1.6,"z":0.6,"w":0.1,"d":0.1,"h":1.05,"repeat":2,"dz":0.7,"far":0}'),
  ('cloth','box','mat.cloth',1,'','{"x":-0.55,"y":1.35,"z":0.25,"w":1.2,"d":0.03,"h":0.5}'),
  ('spoil','cone','mat.spoil',0,'','{"x":0.95,"z":0.95,"w":1.7,"h":0.85,"segments":8}'),
  ('spoil_back','cone','mat.spoil',0,'','{"x":1.25,"z":-0.1,"w":1.2,"h":0.55,"segments":7}'),
  ('rails','box','mat.iron',0,'','{"x":-0.75,"y":0.05,"z":1.1,"w":0.05,"d":1.3,"h":0.05,"repeat":2,"dx":0.4}'),
  ('sleepers','box','mat.oak',0,'','{"x":-0.55,"y":0.02,"z":0.6,"w":0.65,"d":0.1,"h":0.04,"repeat":5,"dz":0.25,"far":0}'),
  ('cart','box','mat.timber',0,'','{"x":-0.55,"y":0.18,"z":1.2,"w":0.5,"d":0.7,"h":0.32}'),
  ('ore','dome','mat.spoil',0,'','{"x":-0.55,"y":0.5,"z":1.2,"w":0.45,"h":0.18,"segments":6,"far":0}'),
  ('house','include',NULL,0,'','{"mesh":"mesh.house_medieval","x":1.1,"z":-1.25,"scale":0.7}'),
  ('banner','banner','mat.cloth',1,'','{"x":0.35,"z":1.55,"h":2.3,"fw":0.5,"fh":0.75}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.mine_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.mine'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.mine_medieval'), (SELECT id FROM def WHERE key='mat.oak'), 100);
