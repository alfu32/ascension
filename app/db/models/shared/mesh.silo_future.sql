-- mesh.silo_future: Composite silo (shared sub-assembly)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.silo_future','Composite silo');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.silo_future';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('drum','cylinder','mat.white_composite',0,'','{"w":0.9,"h":2.4,"segments":12}'),
  ('seam','cylinder','mat.smart_glass',0,'','{"y":0.9,"w":0.92,"h":0.12,"segments":12,"far":0}'),
  ('band','cylinder','mat.white_composite',1,'','{"y":1.6,"w":0.92,"h":0.45,"segments":12}'),
  ('glow','cylinder','mat.light_strip',0,'','{"y":2.1,"w":0.93,"h":0.03,"segments":12,"far":0}'),
  ('cap','dome','mat.white_composite',0,'','{"y":2.4,"w":0.9,"h":0.32,"segments":12,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.silo_future'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
