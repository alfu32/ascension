-- mesh.round_tower: Round tower (shared sub-assembly)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.round_tower','Round tower');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.round_tower';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('drum','cylinder','mat.dressed_stone',0,'','{"w":1,"h":2.2,"segments":8}'),
  ('corbel','cylinder','mat.dressed_stone',0,'','{"y":2,"w":1.14,"h":0.22,"segments":8,"far":0,"tint":"#d8d2c8"}'),
  ('slits','box','mat.oak',0,'','{"y":1.1,"z":0.48,"w":0.07,"d":0.06,"h":0.36,"tint":"#2a2420","far":0}'),
  ('roof','cone','mat.slate',0,'','{"y":2.22,"w":1.26,"h":1.15,"segments":8}'),
  ('tip','cone','mat.cloth',1,'','{"y":2.795,"w":0.67,"h":0.6,"segments":8,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.round_tower'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
