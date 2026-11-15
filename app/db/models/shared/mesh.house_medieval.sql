-- mesh.house_medieval: Medieval house (shared sub-assembly)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.house_medieval','Medieval house');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.house_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('ground','box','mat.rough_stone',0,'','{"door":0.3,"door_h":0.5,"w":1.3,"d":1.05,"h":0.62}'),
  ('sill','box','mat.oak',0,'','{"y":0.6,"w":1.42,"d":1.17,"h":0.06,"far":0}'),
  ('upper','box','mat.daub',0,'','{"y":0.62,"w":1.38,"d":1.13,"h":0.52,"gable":52,"windows":1,"window_y":0.2,"window_w":0.16,"window_h":0.2}'),
  ('posts','box','mat.oak',0,'','{"x":-0.665,"y":0.62,"w":0.06,"d":1.15,"h":0.52,"repeat":3,"dx":0.665,"far":0}'),
  ('roof','gable_roof','mat.slate',0,'','{"y":1.14,"w":1.38,"d":1.13,"pitch":52,"overhang":0.12,"t":0.08}'),
  ('roof_cap','gable_roof','mat.cloth',1,'','{"y":1.673,"w":1.66,"d":0.5,"pitch":52,"overhang":0,"t":0.03}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.house_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
