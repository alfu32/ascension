-- mesh.silo_modern: Concrete silo (shared sub-assembly)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.silo_modern','Concrete silo');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.silo_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('drum','cylinder','mat.concrete',0,'','{"w":0.9,"h":2.6,"segments":12}'),
  ('band','cylinder','mat.cloth',1,'','{"y":2,"w":0.92,"h":0.32,"segments":12}'),
  ('cap','dome','mat.white_metal',0,'','{"y":2.6,"w":0.92,"h":0.2,"segments":12,"far":0}'),
  ('ladder','box','mat.cast_iron',0,'','{"z":0.46,"w":0.08,"d":0.03,"h":2.6,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.silo_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
