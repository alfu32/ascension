-- bld.mine@gunpowder: Early modern mine (mesh.mine_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.mine_gunpowder','Early modern mine');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.mine_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('collar','wall','mat.brick',0,'','{"x":-0.55,"z":-0.55,"w":1.2,"d":1.2,"h":0.4,"t":0.2,"ring":1}'),
  ('pit','box','mat.rough_stone',0,'','{"x":-0.55,"y":0.04,"z":-0.55,"w":0.8,"d":0.8,"h":0.03,"tint":"#2a2622"}'),
  ('legs','box','mat.sawn_timber',0,'','{"x":-1.2,"z":-0.55,"w":0.14,"d":0.14,"h":2.5,"roll":12}'),
  ('legs_e','box','mat.sawn_timber',0,'','{"x":0.1,"z":-0.55,"w":0.14,"d":0.14,"h":2.5,"roll":-12}'),
  ('back_legs','box','mat.sawn_timber',0,'','{"x":-0.55,"z":-1.15,"w":0.12,"d":0.12,"h":2.4,"pitch":14}'),
  ('head','box','mat.sawn_timber',0,'','{"x":-0.55,"y":2.42,"z":-0.55,"w":1,"d":0.5,"h":0.12}'),
  ('pulley','cylinder','mat.iron',0,'','{"x":-0.59,"y":2.12,"z":-0.55,"w":0.56,"h":0.08,"segments":10,"roll":90}'),
  ('pulley_roof','gable_roof','mat.pantile',0,'','{"x":-0.55,"y":2.54,"z":-0.55,"w":1,"d":0.62,"pitch":40,"overhang":0.08,"t":0.06}'),
  ('pulley_cap','gable_roof','mat.cloth',1,'','{"x":-0.55,"y":2.761,"z":-0.55,"w":1.16,"d":0.28,"pitch":40,"overhang":0,"t":0.03}'),
  ('rope','cylinder','mat.thatch',0,'','{"x":-0.55,"y":1,"z":-0.3,"w":0.04,"h":1.1,"segments":4,"far":0}'),
  ('bucket','box','mat.iron',0,'','{"x":-0.55,"y":0.75,"z":-0.3,"w":0.3,"d":0.3,"h":0.3}'),
  ('gin_post','cylinder','mat.timber',0,'','{"x":-1,"z":1,"w":0.14,"h":1.3,"segments":6}'),
  ('gin_drum','cylinder','mat.sawn_timber',0,'','{"x":-1,"y":0.9,"z":1,"w":0.85,"h":0.3,"segments":10}'),
  ('gin_roof','cone','mat.cloth',1,'','{"x":-1,"y":1.3,"z":1,"w":0.95,"h":0.4,"segments":8}'),
  ('gin_arm','box','mat.oak',0,'','{"x":-0.4,"y":0.75,"z":1,"w":1.2,"d":0.08,"h":0.08,"far":0}'),
  ('gin_horse','include',NULL,0,'','{"mesh":"mesh.horse","x":0.15,"z":0.85,"yaw":180,"scale":0.75}'),
  ('cloth','box','mat.cloth',1,'','{"x":-0.55,"y":1.5,"z":-0.27,"w":0.9,"d":0.03,"h":0.5}'),
  ('spoil','cone','mat.spoil',0,'','{"x":0.95,"z":-0.1,"w":1.7,"h":0.85,"segments":8}'),
  ('spoil_back','cone','mat.spoil',0,'','{"x":1.25,"z":-1.3,"w":1.2,"h":0.55,"segments":7}'),
  ('cart','box','mat.timber',0,'','{"x":0.45,"z":1.25,"w":0.5,"d":0.7,"h":0.32}'),
  ('ore','dome','mat.ore',0,'','{"x":0.45,"y":0.32,"z":1.25,"w":0.45,"h":0.18,"segments":6,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.3,"z":1.5,"h":2.4,"fw":0.5,"fh":0.75}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.mine_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.mine'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.mine_gunpowder'), (SELECT id FROM def WHERE key='mat.timber'), 100);
