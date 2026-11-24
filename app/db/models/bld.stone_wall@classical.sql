-- bld.stone_wall@classical: Classical wall (mesh.stone_wall_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.stone_wall_classical','Classical wall');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.stone_wall_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('footing','box','mat.limestone',0,'','{"w":1.0,"d":0.8,"h":0.2}'),
  ('wall','wall','mat.ashlar',0,'','{"y":0.2,"w":1.0,"h":1.7,"t":0.62}'),
  ('coping','box','mat.limestone',0,'','{"y":1.9,"w":1.0,"d":0.7,"h":0.1}'),
  ('parapet','crenellation','mat.limestone',0,'','{"y":2.0,"z":0.27,"w":1.0,"h":0.32,"t":0.14,"merlon":0.25}'),
  ('walk_rail','box','mat.limestone',0,'','{"y":2.0,"z":-0.29,"w":1.0,"d":0.1,"h":0.14}'),
  ('band','box','mat.cloth',1,'','{"y":1.45,"z":0.315,"w":1.0,"d":0.02,"h":0.32}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.stone_wall_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.stone_wall'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.stone_wall_classical'), (SELECT id FROM def WHERE key='mat.limestone'), 100);
