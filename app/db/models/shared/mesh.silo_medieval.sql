-- mesh.silo_medieval: Medieval silo (shared sub-assembly)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.silo_medieval','Medieval silo');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.silo_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('drum','cylinder','mat.rough_stone',0,'','{"w":1.18,"h":1.55,"segments":10}'),
  ('band','cylinder','mat.cloth',1,'','{"y":1.2,"w":1.21,"h":0.3,"segments":10}'),
  ('cap','cone','mat.slate',0,'','{"y":1.55,"w":1.36,"h":0.85,"segments":10}'),
  ('tip','cone','mat.cloth',1,'','{"y":1.975,"w":0.72,"h":0.44,"segments":10}'),
  ('hatch','box','mat.oak',0,'','{"y":0.2,"z":0.575,"w":0.3,"d":0.04,"h":0.45,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.silo_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
