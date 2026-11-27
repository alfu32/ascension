-- unit.troopship@industrial: Troopship (mesh.troopship_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.troopship_industrial','Troopship');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.troopship_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','hull','mat.iron',0,'','{"yaw":-90,"y":-0.14,"w":4.0,"d":1.0,"h":0.62,"sheer":0.1,"segments":12,"float":1,"tint":"#2c2e32"}'),
  ('band','box','mat.cloth',1,'hull','{"y":0.3,"z":-0.49,"w":3.0,"d":0.03,"h":0.2,"repeat":2,"dz":0.98}'),
  ('deck','box','mat.sawn_timber',0,'hull','{"y":0.56,"w":3.0,"d":0.9,"h":0.04}'),
  ('deckhouse','box','mat.lime_plaster',0,'hull','{"x":-0.05,"y":0.58,"w":2.0,"d":0.72,"h":0.32,"windows":0,"side_windows":6,"window_y":0.16,"window_w":0.1,"window_h":0.08,"opening":"#22262a"}'),
  ('upper_house','box','mat.lime_plaster',0,'hull','{"x":0.25,"y":0.9,"w":1.0,"d":0.6,"h":0.22,"far":0}'),
  ('house_roof','box','mat.cloth',1,'hull','{"x":0.25,"y":1.12,"w":1.04,"d":0.64,"h":0.04,"far":0}'),
  ('bridge','box','mat.lime_plaster',0,'hull','{"x":0.65,"y":1.16,"w":0.24,"d":0.7,"h":0.14,"far":0}'),
  ('funnel_fore','cylinder','mat.cloth',1,'hull','{"x":0.05,"y":0.9,"w":0.28,"h":0.8,"segments":8,"roll":-8}'),
  ('funnel_aft','cylinder','mat.cloth',1,'hull','{"x":-0.55,"y":0.9,"w":0.28,"h":0.8,"segments":8,"roll":-8}'),
  ('cap_fore','cylinder','mat.cast_iron',0,'funnel_fore','{"y":0.66,"w":0.29,"h":0.14,"segments":8,"far":0}'),
  ('cap_aft','cylinder','mat.cast_iron',0,'funnel_aft','{"y":0.66,"w":0.29,"h":0.14,"segments":8,"far":0}'),
  ('lifeboats','box','mat.white_metal',0,'hull','{"x":-0.75,"y":0.92,"z":-0.33,"w":0.36,"d":0.12,"h":0.08,"repeat":2,"dx":0.7,"far":0}'),
  ('lifeboats_port','box','mat.white_metal',0,'hull','{"x":-0.75,"y":0.92,"z":0.33,"w":0.36,"d":0.12,"h":0.08,"repeat":2,"dx":0.7,"far":0}'),
  ('foremast','cylinder','mat.timber',0,'hull','{"x":1.35,"y":0.56,"w":0.06,"h":1.6,"segments":5}'),
  ('mainmast','cylinder','mat.timber',0,'hull','{"x":-1.4,"y":0.56,"w":0.06,"h":1.4,"segments":5}'),
  ('ensign','box','mat.cloth',1,'hull','{"x":-1.9,"y":0.9,"w":0.36,"d":0.015,"h":0.24,"far":0}'),
  ('ensign_staff','cylinder','mat.timber',0,'hull','{"x":-1.8,"y":0.56,"w":0.03,"h":0.6,"segments":4,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.troopship_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.troopship'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.troopship_industrial'), (SELECT id FROM def WHERE key='mat.iron'), 100);
