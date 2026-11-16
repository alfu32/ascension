-- mesh.silo_industrial: Industrial silo (shared sub-assembly)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.silo_industrial','Industrial silo');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.silo_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('drum','cylinder','mat.iron',0,'','{"w":1.1,"h":2.3,"segments":10,"tint":"#c4c8cc"}'),
  ('hoops','cylinder','mat.cast_iron',0,'','{"y":0.55,"w":1.12,"h":0.05,"segments":10,"repeat":1,"far":0}'),
  ('hoops_2','cylinder','mat.cast_iron',0,'','{"y":1.25,"w":1.12,"h":0.05,"segments":10,"far":0}'),
  ('band','cylinder','mat.cloth',1,'','{"y":1.7,"w":1.13,"h":0.45,"segments":10}'),
  ('cap','cone','mat.iron',0,'','{"y":2.3,"w":1.2,"h":0.5,"segments":10,"tint":"#a0a4aa"}'),
  ('vent','cylinder','mat.cast_iron',0,'','{"y":2.75,"w":0.16,"h":0.16,"segments":6,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.silo_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
