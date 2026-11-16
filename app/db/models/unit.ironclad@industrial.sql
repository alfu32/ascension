-- unit.ironclad@industrial: Ironclad (mesh.ironclad_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.ironclad_industrial','Ironclad');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.ironclad_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','hull','mat.iron',0,'','{"yaw":-90,"y":-0.14,"w":3.7,"d":1.05,"h":0.52,"sheer":0.06,"segments":10,"float":1,"tint":"#9a9ea4"}'),
  ('deck','box','mat.sawn_timber',0,'hull','{"y":0.48,"w":2.3,"d":0.96,"h":0.04}'),
  ('deck_ends','box','mat.sawn_timber',0,'hull','{"x":-1.42,"y":0.48,"w":0.6,"d":0.62,"h":0.04,"repeat":2,"dx":2.84,"far":0}'),
  ('boot','box','mat.cloth',1,'hull','{"y":0.36,"z":-0.5,"w":2,"d":0.03,"h":0.1,"repeat":2,"dz":1,"far":0}'),
  ('casemate','box','mat.iron',0,'hull','{"x":0,"y":0.52,"w":1,"d":0.66,"h":0.3}'),
  ('bridge','box','mat.iron',0,'hull','{"x":0.35,"y":0.82,"w":0.3,"d":0.5,"h":0.18,"windows":0,"side_windows":3,"window_y":0.06,"window_w":0.06,"window_h":0.06,"opening":"#22262a","far":0}'),
  ('funnel','cylinder','mat.cast_iron',0,'hull','{"x":-0.1,"y":0.82,"w":0.28,"h":0.72,"segments":8}'),
  ('funnel_band','cylinder','mat.cloth',1,'funnel','{"y":0.48,"w":0.29,"h":0.14,"segments":8}'),
  ('mast','cylinder','mat.timber',0,'hull','{"x":0.62,"y":0.52,"w":0.06,"h":1.5,"segments":5}'),
  ('top','cylinder','mat.timber',0,'mast','{"y":1,"w":0.24,"h":0.1,"segments":6,"far":0}'),
  ('ensign','box','mat.cloth',1,'hull','{"x":-1.62,"y":0.86,"w":0.42,"d":0.015,"h":0.28,"far":0}'),
  ('ensign_staff','cylinder','mat.timber',0,'hull','{"x":-1.5,"y":0.5,"w":0.03,"h":0.66,"segments":4,"far":0}'),
  ('turret_fore','cylinder','mat.iron',0,'hull','{"yaw":90,"x":1.1,"y":0.52,"w":0.6,"h":0.3,"segments":10,"traverse":30}'),
  ('fore_roof','cylinder','mat.cloth',1,'turret_fore','{"y":0.3,"w":0.6,"h":0.03,"segments":10,"far":0}'),
  ('fore_guns','cylinder','mat.cast_iron',0,'turret_fore','{"x":-0.09,"y":0.15,"z":0.24,"w":0.08,"h":0.52,"segments":6,"pitch":90,"recoil":0.07,"repeat":2,"dx":0.18,"far":0}'),
  ('fore_spark','cone','mat.flash',0,'fore_guns','{"y":0.52,"w":0.12,"h":0.12,"segments":3,"flash":1,"far":0}'),
  ('fore_flash','cone','mat.flash',0,'fore_spark','{"y":0.55,"w":0.5,"h":0.55,"segments":6,"pitch":180,"far":0}'),
  ('turret_aft','cylinder','mat.iron',0,'hull','{"yaw":-90,"x":-1.05,"y":0.52,"w":0.6,"h":0.3,"segments":10,"traverse":-30}'),
  ('aft_roof','cylinder','mat.cloth',1,'turret_aft','{"y":0.3,"w":0.6,"h":0.03,"segments":10,"far":0}'),
  ('aft_guns','cylinder','mat.cast_iron',0,'turret_aft','{"x":-0.09,"y":0.15,"z":0.24,"w":0.08,"h":0.52,"segments":6,"pitch":90,"recoil":0.07,"repeat":2,"dx":0.18,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.ironclad_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.ironclad'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.ironclad_industrial'), (SELECT id FROM def WHERE key='mat.cast_iron'), 100);
