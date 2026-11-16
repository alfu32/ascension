-- unit.destroyer@modern: Destroyer (mesh.destroyer_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.destroyer_modern','Destroyer');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.destroyer_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','hull','mat.haze_grey',0,'','{"yaw":-90,"y":-0.16,"w":4.2,"d":0.86,"h":0.56,"sheer":0.12,"segments":12,"float":1}'),
  ('deck','box','mat.tarmac',0,'hull','{"y":0.5,"w":2.8,"d":0.74,"h":0.04,"far":0}'),
  ('deck_ends','box','mat.tarmac',0,'hull','{"x":-1.6,"y":0.5,"w":0.4,"d":0.46,"h":0.04,"repeat":2,"dx":3.2,"far":0}'),
  ('boot','box','mat.cloth',1,'hull','{"y":0.36,"z":-0.42,"w":2.6,"d":0.03,"h":0.1,"repeat":2,"dz":0.84,"far":0}'),
  ('forecastle','box','mat.haze_grey',0,'hull','{"x":1,"y":0.52,"w":1.2,"d":0.66,"h":0.14,"far":0}'),
  ('bridge_top','box','mat.haze_grey',0,'hull','{"x":0.62,"y":0.66,"w":0.46,"d":0.52,"h":0.38,"windows":0,"side_windows":3,"window_y":0.28,"window_w":0.08,"window_h":0.06,"opening":"#22262a"}'),
  ('bridge_roof','box','mat.haze_grey',0,'hull','{"x":0.6,"y":1.04,"w":0.36,"d":0.44,"h":0.1,"taper":0.15,"far":0}'),
  ('mast','cylinder','mat.haze_grey',0,'hull','{"x":0.28,"y":0.52,"w":0.06,"h":1.55,"segments":5}'),
  ('mast_legs','cylinder','mat.haze_grey',0,'hull','{"x":0.28,"y":0.52,"z":-0.18,"w":0.04,"h":1,"segments":4,"pitch":10,"repeat":2,"dz":0.36,"far":0}'),
  ('yard','box','mat.haze_grey',0,'mast','{"y":1.2,"w":0.04,"d":0.6,"h":0.04,"far":0}'),
  ('radar','box','mat.haze_grey',0,'mast','{"y":1.4,"w":0.06,"d":0.3,"h":0.12,"far":0}'),
  ('funnel_fore','cylinder','mat.haze_grey',0,'hull','{"x":0.05,"y":0.52,"w":0.28,"h":0.62,"segments":8,"roll":-10}'),
  ('funnel_aft','cylinder','mat.haze_grey',0,'hull','{"x":-0.45,"y":0.52,"w":0.28,"h":0.55,"segments":8,"roll":-10}'),
  ('cap_fore','cylinder','mat.cloth',1,'funnel_fore','{"y":0.5,"w":0.29,"h":0.12,"segments":8}'),
  ('cap_aft','cylinder','mat.cloth',1,'funnel_aft','{"y":0.43,"w":0.29,"h":0.12,"segments":8}'),
  ('tubes','cylinder','mat.haze_grey',0,'hull','{"x":-0.95,"y":0.6,"z":-0.08,"w":0.07,"h":0.5,"segments":5,"pitch":90,"repeat":3,"dz":0.08,"far":0}'),
  ('turret_a','box','mat.haze_grey',0,'hull','{"yaw":90,"x":1.35,"y":0.66,"w":0.34,"d":0.42,"h":0.2,"taper":0.18,"traverse":30}'),
  ('roof_a','box','mat.cloth',1,'turret_a','{"y":0.2,"w":0.28,"d":0.34,"h":0.02,"far":0}'),
  ('gun_a','cylinder','mat.cast_iron',0,'turret_a','{"y":0.1,"z":0.2,"w":0.06,"h":0.5,"segments":5,"pitch":90,"recoil":0.07}'),
  ('spark_a','cone','mat.flash',0,'gun_a','{"y":0.5,"w":0.1,"h":0.1,"segments":3,"flash":1,"far":0}'),
  ('flash_a','cone','mat.flash',0,'spark_a','{"y":0.45,"w":0.42,"h":0.45,"segments":6,"pitch":180,"far":0}'),
  ('barbette_b','cylinder','mat.haze_grey',0,'hull','{"x":1,"y":0.66,"w":0.36,"h":0.14,"segments":8,"far":0}'),
  ('turret_b','box','mat.haze_grey',0,'hull','{"yaw":90,"x":1,"y":0.8,"w":0.34,"d":0.42,"h":0.2,"taper":0.18,"traverse":30}'),
  ('roof_b','box','mat.cloth',1,'turret_b','{"y":0.2,"w":0.28,"d":0.34,"h":0.02,"far":0}'),
  ('gun_b','cylinder','mat.cast_iron',0,'turret_b','{"y":0.1,"z":0.2,"w":0.06,"h":0.5,"segments":5,"pitch":90,"recoil":0.07,"far":0}'),
  ('turret_y','box','mat.haze_grey',0,'hull','{"yaw":-90,"x":-1.4,"y":0.54,"w":0.34,"d":0.42,"h":0.2,"taper":0.18,"traverse":-30}'),
  ('roof_y','box','mat.cloth',1,'turret_y','{"y":0.2,"w":0.28,"d":0.34,"h":0.02,"far":0}'),
  ('gun_y','cylinder','mat.cast_iron',0,'turret_y','{"y":0.1,"z":0.2,"w":0.06,"h":0.5,"segments":5,"pitch":90,"recoil":0.07,"far":0}'),
  ('ensign','box','mat.cloth',1,'hull','{"x":-1.95,"y":0.92,"w":0.36,"d":0.015,"h":0.24,"far":0}'),
  ('ensign_staff','cylinder','mat.haze_grey',0,'hull','{"x":-1.8,"y":0.52,"w":0.03,"h":0.62,"segments":4,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.destroyer_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.destroyer'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.destroyer_modern'), (SELECT id FROM def WHERE key='mat.haze_grey'), 100);
