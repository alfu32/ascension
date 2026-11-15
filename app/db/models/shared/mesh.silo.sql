-- mesh.silo: Silo (shared sub-assembly)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.silo','Silo');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.silo';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('footing','cylinder','mat.fieldstone',0,'','{"w":1.25,"h":0.22,"segments":10}'),
  ('drum','cylinder','mat.lime_plaster',0,'','{"y":0.22,"w":1.14,"h":1.4,"segments":10}'),
  ('band','cylinder','mat.cloth',1,'','{"y":1.12,"w":1.17,"h":0.3,"segments":10}'),
  ('cap','cone','mat.terracotta',0,'','{"y":1.62,"w":1.32,"h":0.62,"segments":10}'),
  ('tip','cone','mat.cloth',1,'','{"y":1.925,"w":0.69,"h":0.325,"segments":10}'),
  ('hatch','box','mat.timber',0,'','{"y":0.35,"z":0.555,"w":0.3,"d":0.04,"h":0.45,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.silo'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
