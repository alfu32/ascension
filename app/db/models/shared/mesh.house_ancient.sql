-- mesh.house_ancient: Early house (shared sub-assembly)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.house_ancient','Early house');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.house_ancient';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('walls','box','mat.mud_plaster',0,'','{"w":1.3,"d":1.05,"h":0.95,"gable":42,"door":0.3,"door_h":0.6,"windows":1,"side_windows":1,"window_w":0.17,"window_h":0.19,"window_y":0.5}'),
  ('roof','gable_roof','mat.thatch',0,'','{"y":0.95,"w":1.3,"d":1.05,"pitch":42,"overhang":0.17,"t":0.1}'),
  ('ridge','box','mat.cloth',1,'','{"y":1.47,"w":1.62,"d":0.2,"h":0.1}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.house_ancient'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
