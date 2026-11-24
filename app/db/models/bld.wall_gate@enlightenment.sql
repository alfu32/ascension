-- bld.wall_gate@enlightenment: Rampart gate (mesh.wall_gate_enlightenment)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.wall_gate_enlightenment','Rampart gate');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.wall_gate_enlightenment';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('bed','box','mat.turf',0,'','{"w":0.96,"d":0.86,"h":0.02}'),
  ('pier_l','box','mat.dressed_stone',0,'','{"x":-0.39,"w":0.2,"d":0.6,"h":1.9}'),
  ('pier_r','box','mat.dressed_stone',0,'','{"x":0.39,"w":0.2,"d":0.6,"h":1.9}'),
  ('urn_l','dome','mat.dressed_stone',0,'','{"x":-0.4,"y":1.9,"w":0.18,"h":0.18,"segments":6,"far":0}'),
  ('urn_r','dome','mat.dressed_stone',0,'','{"x":0.4,"y":1.9,"w":0.18,"h":0.18,"segments":6,"far":0}'),
  ('grille','fence','mat.cast_iron',0,'','{"w":0.6,"h":1.4,"spacing":0.12,"rails":3}'),
  ('crest','box','mat.cloth',1,'','{"y":1.4,"w":0.6,"d":0.06,"h":0.4}'),
  ('caps','box','mat.cloth',1,'','{"x":-0.42,"y":1.82,"w":0.22,"d":0.62,"h":0.08,"repeat":2,"dx":0.8}'),
  ('banner','banner','mat.cloth',1,'','{"x":0.19,"y":2.05,"h":0.6,"fw":0.3,"fh":0.2}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.wall_gate_enlightenment'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.wall_gate'), NULL, (SELECT id FROM def WHERE key='age.enlightenment'), (SELECT id FROM def WHERE key='mesh.wall_gate_enlightenment'), (SELECT id FROM def WHERE key='mat.dressed_stone'), 100);
