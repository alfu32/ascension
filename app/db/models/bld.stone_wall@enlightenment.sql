-- bld.stone_wall@enlightenment: Revetted rampart (mesh.stone_wall_enlightenment)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.stone_wall_enlightenment','Revetted rampart');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.stone_wall_enlightenment';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('bed','box','mat.turf',0,'','{"w":0.96,"d":0.86,"h":0.02}'),
  ('glacis','box','mat.turf',0,'','{"z":0.2,"w":0.98,"d":1.15,"h":0.2}'),
  ('glacis_2','box','mat.turf',0,'','{"y":0.2,"z":0.12,"w":0.98,"d":0.95,"h":0.2}'),
  ('revetment','box','mat.dressed_stone',0,'','{"z":-0.08,"w":0.98,"d":0.8,"h":1.3}'),
  ('cordon','box','mat.cloth',1,'','{"y":1.3,"z":-0.08,"w":0.98,"d":0.74,"h":0.1}'),
  ('parapet','box','mat.turf',0,'','{"y":1.4,"z":0.06,"w":0.98,"d":0.4,"h":0.32}'),
  ('fraise','fence','mat.timber',0,'','{"y":1.0,"z":0.42,"w":0.98,"h":0.3,"spacing":0.25,"rails":2,"far":0}'),
  ('team_front','box','mat.cloth',1,'','{"y":0.75,"z":0.335,"w":0.98,"d":0.02,"h":0.45}'),
  ('team_back','box','mat.cloth',1,'','{"y":0.75,"z":-0.495,"w":0.98,"d":0.02,"h":0.45}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.stone_wall_enlightenment'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.stone_wall'), NULL, (SELECT id FROM def WHERE key='age.enlightenment'), (SELECT id FROM def WHERE key='mesh.stone_wall_enlightenment'), (SELECT id FROM def WHERE key='mat.dressed_stone'), 100);
