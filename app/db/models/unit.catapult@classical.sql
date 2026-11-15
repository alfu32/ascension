-- unit.catapult@classical: Catapult (mesh.catapult_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.catapult_classical','Catapult');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.catapult_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('beams','box','mat.timber',0,'','{"x":-0.32,"y":0.2,"w":0.12,"d":1.5,"h":0.14,"repeat":2,"dx":0.64}'),
  ('ties','box','mat.timber',0,'','{"y":0.2,"z":-0.6,"w":0.76,"d":0.12,"h":0.12,"repeat":2,"dz":1.2,"far":0}'),
  ('wheels_w','wheel','mat.timber',0,'','{"x":-0.45,"z":-0.45,"w":0.5,"t":0.08,"spokes":2,"repeat":2,"dz":0.9}'),
  ('wheels_e','wheel','mat.timber',0,'','{"x":0.45,"z":-0.45,"w":0.5,"t":0.08,"spokes":2,"repeat":2,"dz":0.9}'),
  ('posts','box','mat.timber',0,'','{"x":-0.3,"y":0.34,"z":0.3,"w":0.1,"d":0.1,"h":0.95,"repeat":2,"dx":0.6}'),
  ('braces','box','mat.timber',0,'','{"x":-0.3,"y":0.34,"z":0.72,"w":0.08,"d":0.08,"h":0.85,"pitch":-28,"repeat":2,"dx":0.6,"far":0}'),
  ('crossbar','box','mat.timber',0,'','{"y":1.2,"z":0.3,"w":0.72,"d":0.12,"h":0.1}'),
  ('pad','box','mat.cloth',1,'','{"y":1.0,"z":0.22,"w":0.6,"d":0.06,"h":0.3}'),
  ('skein','cylinder','mat.leather',0,'','{"x":-0.32,"y":0.42,"z":-0.1,"w":0.18,"h":0.64,"roll":90,"segments":6}'),
  ('skein_cover','cylinder','mat.cloth',1,'','{"x":-0.2,"y":0.42,"z":-0.1,"w":0.2,"h":0.4,"roll":90,"segments":6,"far":0}'),
  ('arm','box','mat.timber',0,'','{"y":0.42,"z":-0.1,"w":0.09,"d":0.09,"h":0.92,"pitch":-72,"strike":66}'),
  ('cup','box','mat.leather',0,'arm','{"y":0.8,"z":0.05,"w":0.2,"d":0.1,"h":0.16}'),
  ('stone','dome','mat.fieldstone',0,'arm','{"y":0.88,"z":0.1,"w":0.14,"h":0.12,"segments":6}'),
  ('banner','banner','mat.cloth',1,'','{"x":0.36,"z":-0.66,"h":1.5,"fw":0.42,"fh":0.56}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.catapult_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.catapult'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.catapult_classical'), (SELECT id FROM def WHERE key='mat.timber'), 100);
