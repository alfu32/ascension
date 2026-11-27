-- unit.cog@medieval: Cog (mesh.cog_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.cog_medieval','Cog');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.cog_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','hull','mat.oak',0,'','{"yaw":-90,"y":-0.12,"w":3.0,"d":1.2,"h":0.78,"sheer":0.22,"segments":10,"float":1,"tint":"#6e5a48"}'),
  ('strakes','box','mat.timber',0,'hull','{"y":0.5,"z":-0.57,"w":2.2,"d":0.04,"h":0.05,"repeat":2,"dz":1.14,"far":0}'),
  ('deck','box','mat.sawn_timber',0,'hull','{"y":0.66,"w":2.2,"d":0.92,"h":0.05}'),
  ('forecastle','box','mat.sawn_timber',0,'hull','{"x":1.05,"y":0.68,"w":0.6,"d":0.9,"h":0.4}'),
  ('fore_merlons','crenellation','mat.sawn_timber',0,'hull','{"x":1.05,"y":1.08,"w":0.62,"d":0.92,"h":0.12,"t":0.05,"merlon":0.12,"ring":1,"far":0}'),
  ('sterncastle','box','mat.sawn_timber',0,'hull','{"x":-1.05,"y":0.68,"w":0.7,"d":0.94,"h":0.52}'),
  ('stern_merlons','crenellation','mat.sawn_timber',0,'hull','{"x":-1.05,"y":1.2,"w":0.72,"d":0.96,"h":0.12,"t":0.05,"merlon":0.12,"ring":1,"far":0}'),
  ('castle_trim','box','mat.cloth',1,'hull','{"x":-1.05,"y":1.1,"w":0.72,"d":0.96,"h":0.07}'),
  ('rudder','box','mat.timber',0,'hull','{"x":-1.55,"y":0.0,"w":0.1,"d":0.06,"h":0.8,"roll":-12}'),
  ('shields','box','mat.cloth',1,'hull','{"x":-0.55,"y":0.72,"z":-0.49,"w":0.2,"d":0.03,"h":0.22,"repeat":4,"dx":0.36,"far":0}'),
  ('shields_port','box','mat.cloth',1,'hull','{"x":-0.55,"y":0.72,"z":0.49,"w":0.2,"d":0.03,"h":0.22,"repeat":4,"dx":0.36,"far":0}'),
  ('cargo','box','mat.hide',0,'hull','{"x":-0.3,"y":0.71,"w":0.4,"d":0.5,"h":0.2,"far":0}'),
  ('barrels','cylinder','mat.oak',0,'hull','{"x":0.35,"y":0.71,"z":-0.18,"w":0.18,"h":0.24,"segments":6,"repeat":2,"dz":0.36,"far":0}'),
  ('mast','cylinder','mat.timber',0,'hull','{"x":0.05,"y":0.66,"w":0.09,"h":2.3,"segments":5}'),
  ('yard','cylinder','mat.timber',0,'mast','{"y":1.95,"w":0.05,"h":1.5,"pitch":90,"yaw":90,"segments":4,"far":0}'),
  ('sail','sail','mat.cloth',1,'mast','{"y":0.65,"yaw":90,"w":1.45,"h":1.3,"billow":0.22}'),
  ('crows_nest','cylinder','mat.sawn_timber',0,'mast','{"y":2.1,"w":0.24,"h":0.14,"segments":6,"far":0}'),
  ('pennant','box','mat.cloth',1,'mast','{"y":2.3,"z":-0.16,"w":0.015,"d":0.32,"h":0.1,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.cog_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.cog'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.cog_medieval'), (SELECT id FROM def WHERE key='mat.oak'), 100);
