-- bld.mine@classical: Classical mine (mesh.mine_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.mine_classical','Classical mine');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.mine_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('collar','wall','mat.fieldstone',0,'','{"x":-0.55,"z":-0.55,"w":1.2,"d":1.2,"h":0.35,"t":0.2,"ring":1}'),
  ('pit','box','mat.fieldstone',0,'','{"x":-0.55,"y":0.04,"z":-0.55,"w":0.82,"d":0.82,"h":0.03,"tint":"#2a2622"}'),
  ('leg_nw','cylinder','mat.timber',0,'','{"x":-1.3,"z":-1.3,"w":0.16,"h":2.75,"segments":6,"roll":7,"pitch":7}'),
  ('leg_ne','cylinder','mat.timber',0,'','{"x":0.2,"z":-1.3,"w":0.16,"h":2.75,"segments":6,"roll":-7,"pitch":7}'),
  ('leg_sw','cylinder','mat.timber',0,'','{"x":-1.3,"z":0.2,"w":0.16,"h":2.75,"segments":6,"roll":7,"pitch":-7}'),
  ('leg_se','cylinder','mat.timber',0,'','{"x":0.2,"z":0.2,"w":0.16,"h":2.75,"segments":6,"roll":-7,"pitch":-7}'),
  ('girts','box','mat.timber',0,'','{"x":-0.55,"y":1.4,"z":-1.12,"w":1.2,"d":0.1,"h":0.1,"repeat":2,"dz":1.14,"far":0}'),
  ('girts_side','box','mat.timber',0,'','{"x":-1.12,"y":1.4,"z":-0.55,"yaw":90,"w":1.2,"d":0.1,"h":0.1,"repeat":2,"dx":1.14,"far":0}'),
  ('cloth','box','mat.cloth',1,'','{"x":-0.55,"y":1.0,"z":0.03,"w":0.9,"d":0.03,"h":0.42}'),
  ('head','box','mat.timber',0,'','{"x":-0.55,"y":2.68,"z":-0.55,"w":1.0,"d":1.0,"h":0.14}'),
  ('sheave_frame','box','mat.timber',0,'','{"x":-0.75,"y":2.82,"z":-0.55,"w":0.08,"d":0.1,"h":0.55,"repeat":2,"dx":0.4,"far":0}'),
  ('sheave','cylinder','mat.timber',0,'','{"x":-0.62,"y":3.28,"z":-0.55,"w":0.85,"h":0.14,"segments":12,"roll":90}'),
  ('rope','cylinder','mat.thatch',0,'','{"x":-0.55,"y":1.4,"z":-0.13,"w":0.04,"h":1.9,"segments":4,"far":0}'),
  ('bucket','box','mat.iron',0,'','{"x":-0.55,"y":1.1,"z":-0.13,"w":0.3,"d":0.3,"h":0.3}'),
  ('banner','banner','mat.cloth',1,'','{"x":-0.55,"y":2.82,"z":-0.92,"h":1.8,"fw":0.62,"fh":0.8}'),
  ('spoil','cone','mat.spoil',0,'','{"x":0.95,"z":0.95,"w":1.7,"h":0.8,"segments":8}'),
  ('spoil_back','cone','mat.spoil',0,'','{"x":1.2,"z":-0.15,"w":1.2,"h":0.55,"segments":7}'),
  ('spoil_low','cone','mat.spoil',0,'','{"x":0.3,"z":1.45,"w":0.8,"h":0.4,"segments":6}'),
  ('rails','box','mat.iron',0,'','{"x":-0.75,"y":0.05,"z":0.92,"w":0.05,"d":1.65,"h":0.05,"repeat":2,"dx":0.4}'),
  ('sleepers','box','mat.timber',0,'','{"x":-0.55,"y":0.02,"z":0.2,"w":0.65,"d":0.1,"h":0.04,"repeat":7,"dz":0.25,"far":0}'),
  ('cart','box','mat.timber',0,'','{"x":-0.55,"y":0.18,"z":1.05,"w":0.5,"d":0.7,"h":0.32}'),
  ('ore','dome','mat.spoil',0,'','{"x":-0.55,"y":0.5,"z":1.05,"w":0.45,"h":0.18,"segments":6,"far":0}'),
  ('cart_wheels_w','cylinder','mat.iron',0,'','{"x":-0.83,"y":0.21,"z":0.85,"w":0.22,"h":0.05,"segments":8,"roll":90,"repeat":2,"dz":0.4}'),
  ('cart_wheels_e','cylinder','mat.iron',0,'','{"x":-0.32,"y":0.21,"z":0.85,"w":0.22,"h":0.05,"segments":8,"roll":90,"repeat":2,"dz":0.4}'),
  ('house','include',NULL,0,'','{"mesh":"mesh.house_classical","x":1.1,"z":-1.25,"scale":0.7}'),
  ('shelter','awning','mat.cloth',1,'','{"x":-1.35,"z":0.95,"w":0.8,"d":1.1,"h":1.15,"rise":0.25}'),
  ('baskets','cylinder','mat.thatch',0,'','{"x":-1.5,"z":0.7,"w":0.3,"h":0.3,"segments":6,"repeat":2,"dz":0.45,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.mine_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.mine'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.mine_classical'), (SELECT id FROM def WHERE key='mat.timber'), 100);
