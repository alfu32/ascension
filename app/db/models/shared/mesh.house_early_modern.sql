-- mesh.house_early_modern: Early modern house (shared sub-assembly)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.house_early_modern','Early modern house');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.house_early_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('walls','box',NULL,0,'','{"w":1.3,"d":1,"h":1.15,"door":0.24,"door_h":0.5,"windows":2,"window_y":0.14,"window_w":0.17,"window_h":0.32,"window_rows":2,"row_gap":0.56,"opening":"#2c3642"}'),
  ('cornice','box','mat.ashlar',0,'','{"y":1.12,"w":1.38,"d":1.08,"h":0.07,"far":0}'),
  ('roof','hip_roof','mat.pantile',0,'','{"y":1.19,"w":1.3,"d":1,"pitch":38,"overhang":0.1,"t":0.07}'),
  ('roof_cap','hip_roof','mat.cloth',1,'','{"y":1.498,"w":0.74,"d":0.44,"pitch":38,"overhang":0,"t":0.03}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.house_early_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
