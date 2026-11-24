-- bld.stone_wall@medieval: Curtain wall (mesh.stone_wall_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.stone_wall_medieval','Curtain wall');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.stone_wall_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('bed','box','mat.rough_stone',0,'','{"w":0.96,"d":0.86,"h":0.02}'),
  ('batter','box','mat.rough_stone',0,'','{"w":0.98,"d":0.95,"h":0.25}'),
  ('batter_2','box','mat.rough_stone',0,'','{"y":0.25,"w":0.98,"d":0.8,"h":0.25}'),
  ('wall','wall','mat.rough_stone',0,'','{"y":0.5,"w":0.98,"h":1.75,"t":0.66}'),
  ('walk','box','mat.slate',0,'','{"y":2.25,"w":0.98,"d":0.7,"h":0.06}'),
  ('walk_cloth','box','mat.cloth',1,'','{"y":2.31,"w":0.98,"d":0.4,"h":0.02}'),
  ('merlons','crenellation','mat.rough_stone',0,'','{"y":2.31,"z":0.27,"w":0.98,"h":0.4,"t":0.16,"merlon":0.22}'),
  ('hoarding','box','mat.timber',0,'','{"y":2.31,"z":-0.3,"w":0.98,"d":0.08,"h":0.22}'),
  ('team_front','box','mat.cloth',1,'','{"y":1.3,"z":0.345,"w":0.98,"d":0.02,"h":0.15}'),
  ('team_back','box','mat.cloth',1,'','{"y":1.3,"z":-0.345,"w":0.98,"d":0.02,"h":0.15}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.stone_wall_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.stone_wall'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.stone_wall_medieval'), (SELECT id FROM def WHERE key='mat.rough_stone'), 100);
