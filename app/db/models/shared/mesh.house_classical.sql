-- mesh.house_classical: Classical house (shared sub-assembly)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.house_classical','Classical house');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.house_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('walls','box','mat.lime_plaster',0,'','{"w":1.4,"d":1.1,"h":1.05,"gable":24,"door":0.3,"door_h":0.66,"windows":2,"side_windows":1,"window_w":0.16,"window_h":0.24,"window_y":0.55}'),
  ('roof','gable_roof','mat.terracotta',0,'','{"y":1.05,"w":1.4,"d":1.1,"pitch":24,"overhang":0.14,"t":0.08}'),
  ('ridge','box','mat.cloth',1,'','{"y":1.3,"w":1.65,"d":0.16,"h":0.08}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.house_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
