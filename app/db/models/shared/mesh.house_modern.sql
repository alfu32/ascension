-- mesh.house_modern: Modern flats (shared sub-assembly)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.house_modern','Modern flats');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.house_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('walls','box',NULL,0,'','{"w":1.6,"d":1.1,"h":1.5,"windows":1,"window_w":1.3,"window_h":0.26,"window_y":0.55,"window_rows":2,"row_gap":0.55,"opening":"#2b3b49"}'),
  ('door','box','mat.cast_iron',0,'','{"x":-0.45,"z":0.55,"w":0.26,"d":0.03,"h":0.46,"far":0}'),
  ('porch','box','mat.concrete',0,'','{"x":-0.45,"y":0.46,"z":0.66,"w":0.46,"d":0.24,"h":0.04,"far":0}'),
  ('fascia','box','mat.cloth',1,'','{"y":1.44,"w":1.68,"d":1.18,"h":0.14}'),
  ('roof','flat_roof','mat.tar',0,'','{"y":1.58,"w":1.6,"d":1.1,"t":0.04,"overhang":0.03}'),
  ('stair_house','box','mat.concrete',0,'','{"x":0.45,"y":1.62,"z":-0.15,"w":0.4,"d":0.4,"h":0.3,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.house_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
