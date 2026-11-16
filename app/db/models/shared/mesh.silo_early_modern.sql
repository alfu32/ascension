-- mesh.silo_early_modern: Early modern silo (shared sub-assembly)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.silo_early_modern','Early modern silo');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.silo_early_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('drum','cylinder','mat.brick',0,'','{"w":1.18,"h":1.5,"segments":10}'),
  ('string','cylinder','mat.ashlar',0,'','{"y":0.55,"w":1.22,"h":0.07,"segments":10,"far":0}'),
  ('band','cylinder','mat.cloth',1,'','{"y":1.24,"w":1.22,"h":0.26,"segments":10}'),
  ('cap','cone','mat.pantile',0,'','{"y":1.5,"w":1.38,"h":0.72,"segments":10}'),
  ('tip','cone','mat.cloth',1,'','{"y":1.78,"w":0.84,"h":0.47,"segments":10}'),
  ('hatch','box','mat.oak',0,'','{"y":0.15,"z":0.575,"w":0.32,"d":0.04,"h":0.48,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.silo_early_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
