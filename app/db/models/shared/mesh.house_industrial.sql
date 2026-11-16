-- mesh.house_industrial: Industrial terraced houses (shared sub-assembly)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.house_industrial','Industrial terraced houses');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.house_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('walls','box',NULL,0,'','{"w":1.6,"d":1,"h":1.3,"door":0.22,"door_h":0.5,"door_x":-0.38,"windows":2,"window_y":0.16,"window_w":0.2,"window_h":0.32,"window_rows":2,"row_gap":0.6,"opening":"#22282f"}'),
  ('roof','gable_roof','mat.slate',0,'','{"x":0,"y":1.3,"z":0,"w":1.6,"d":1,"pitch":36,"overhang":0.08,"t":0.07}'),
  ('roof_cap','gable_roof','mat.cloth',1,'','{"x":0,"y":1.604,"z":0,"w":1.76,"d":0.4,"pitch":36,"overhang":0,"t":0.03}'),
  ('stacks','box','mat.red_brick',0,'','{"x":-0.72,"y":1.3,"w":0.16,"d":0.3,"h":0.72,"repeat":2,"dx":1.44,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.house_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
