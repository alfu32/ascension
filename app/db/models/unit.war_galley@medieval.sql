-- unit.war_galley@medieval: War galley (mesh.war_galley_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.war_galley_medieval','War galley');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.war_galley_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','hull','mat.timber',0,'','{"yaw":-90,"y":-0.08,"w":3.6,"d":0.8,"h":0.4,"sheer":0.26,"segments":10,"float":1,"tint":"#7a6a5c"}'),
  ('oars','oar_bank','mat.timber',0,'hull','{"y":0.3,"w":2.2,"count":7,"beam":0.82,"length":0.85,"drop":28}'),
  ('stripes','box','mat.cloth',1,'hull','{"y":0.27,"z":-0.41,"w":2.4,"d":0.05,"h":0.09,"repeat":2,"dz":0.82}'),
  ('deck','box','mat.sawn_timber',0,'hull','{"y":0.34,"w":2.8,"d":0.4,"h":0.05}'),
  ('forecastle','box','mat.sawn_timber',0,'hull','{"x":1.25,"y":0.38,"w":0.6,"d":0.62,"h":0.36}'),
  ('fore_merlons','crenellation','mat.sawn_timber',0,'hull','{"x":1.25,"y":0.74,"w":0.62,"d":0.64,"h":0.14,"t":0.05,"merlon":0.12,"ring":1,"far":0}'),
  ('sterncastle','box','mat.sawn_timber',0,'hull','{"x":-1.25,"y":0.38,"w":0.7,"d":0.66,"h":0.5}'),
  ('stern_merlons','crenellation','mat.sawn_timber',0,'hull','{"x":-1.25,"y":0.88,"w":0.72,"d":0.68,"h":0.14,"t":0.05,"merlon":0.12,"ring":1,"far":0}'),
  ('castle_trim','box','mat.cloth',1,'hull','{"x":-1.25,"y":0.74,"w":0.72,"d":0.68,"h":0.08}'),
  ('beak','box','mat.timber',0,'hull','{"x":1.55,"y":0.3,"w":0.08,"d":0.08,"h":0.7,"roll":78}'),
  ('mast','cylinder','mat.timber',0,'hull','{"x":0.3,"y":0.3,"w":0.07,"h":2,"segments":5}'),
  ('sail','sail','mat.cloth',1,'mast','{"y":0.35,"roll":28,"w":1.7,"h":1.5,"billow":0.14,"lateen":1}'),
  ('pennant','box','mat.cloth',1,'mast','{"y":2,"z":-0.15,"w":0.015,"d":0.3,"h":0.1}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.war_galley_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.war_galley'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.war_galley_medieval'), (SELECT id FROM def WHERE key='mat.timber'), 100);
