-- unit.dreadnought@modern: Dreadnought (mesh.dreadnought_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.dreadnought_modern','Dreadnought');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.dreadnought_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','hull','mat.haze_grey',0,'','{"yaw":-90,"y":-0.16,"w":4.7,"d":1,"h":0.62,"sheer":0.1,"segments":10,"float":1}'),
  ('deck','box','mat.tarmac',0,'hull','{"y":0.56,"w":3.3,"d":0.86,"h":0.04}'),
  ('boot','box','mat.cloth',1,'hull','{"y":0.4,"z":-0.465,"w":2.5,"d":0.03,"h":0.1,"repeat":2,"dz":0.93,"far":0}'),
  ('belt','box','mat.cast_iron',0,'hull','{"y":0.24,"z":-0.425,"w":2.2,"d":0.03,"h":0.12,"repeat":2,"dz":0.85,"far":0}'),
  ('forecastle','box','mat.haze_grey',0,'hull','{"x":1.3,"y":0.58,"w":1.4,"d":0.76,"h":0.14,"far":0}'),
  ('superstructure','box','mat.haze_grey',0,'hull','{"x":0.25,"y":0.6,"w":1.3,"d":0.3,"h":0.2}'),
  ('bridge','box','mat.haze_grey',0,'hull','{"x":0.75,"y":0.8,"w":0.36,"d":0.5,"h":0.26}'),
  ('bridge_windows','box','mat.felt',0,'hull','{"x":0.75,"y":0.97,"w":0.38,"d":0.52,"h":0.05,"far":0}'),
  ('bridge_roof','box','mat.haze_grey',0,'hull','{"x":0.75,"y":1.06,"w":0.3,"d":0.56,"h":0.04,"far":0}'),
  ('mast','cylinder','mat.haze_grey',0,'hull','{"x":0.5,"y":0.8,"w":0.07,"h":1.75,"segments":5}'),
  ('mast_legs','cylinder','mat.haze_grey',0,'hull','{"x":0.62,"y":0.8,"z":-0.12,"w":0.045,"h":1.25,"segments":4,"roll":-5,"repeat":2,"dz":0.24,"far":0}'),
  ('spotting_top','box','mat.haze_grey',0,'mast','{"y":1.25,"w":0.26,"d":0.26,"h":0.16,"far":0}'),
  ('yard','box','mat.haze_grey',0,'mast','{"y":1.5,"w":0.04,"d":0.7,"h":0.04,"far":0}'),
  ('mast_light','box','mat.signal_red',0,'mast','{"y":1.75,"w":0.05,"d":0.05,"h":0.05,"far":0}'),
  ('funnel_fore','cylinder','mat.haze_grey',0,'hull','{"x":0.33,"y":0.8,"w":0.3,"h":0.62,"segments":8}'),
  ('funnel_aft','cylinder','mat.haze_grey',0,'hull','{"x":-0.33,"y":0.8,"w":0.3,"h":0.58,"segments":8}'),
  ('cap_fore','cylinder','mat.cloth',1,'funnel_fore','{"y":0.48,"w":0.31,"h":0.12,"segments":8,"far":0}'),
  ('cap_aft','cylinder','mat.cloth',1,'funnel_aft','{"y":0.44,"w":0.31,"h":0.12,"segments":8,"far":0}'),
  ('boats','box','mat.white_metal',0,'hull','{"y":0.8,"z":-0.1,"w":0.3,"d":0.1,"h":0.08,"repeat":2,"dz":0.2,"taper":0.3,"far":0}'),
  ('casemates','cylinder','mat.cast_iron',0,'hull','{"x":-0.75,"y":0.38,"z":0.5,"w":0.035,"h":0.12,"segments":4,"pitch":90,"repeat":2,"dx":1.2,"far":0}'),
  ('casemates_port','cylinder','mat.cast_iron',0,'hull','{"x":-0.75,"y":0.38,"z":-0.5,"w":0.035,"h":0.12,"segments":4,"pitch":-90,"repeat":2,"dx":1.2,"far":0}'),
  ('turret_a','box','mat.haze_grey',0,'hull','{"x":1.55,"y":0.72,"w":0.36,"d":0.44,"h":0.2,"yaw":90,"taper":0.18,"traverse":30}'),
  ('roof_a','box','mat.cloth',1,'turret_a','{"y":0.2,"w":0.3,"d":0.36,"h":0.02,"far":0}'),
  ('guns_a','cylinder','mat.cast_iron',0,'turret_a','{"x":-0.07,"y":0.1,"z":0.2,"w":0.06,"h":0.55,"segments":5,"pitch":90,"recoil":0.07,"repeat":2,"dx":0.14,"far":0}'),
  ('spark_a','cone','mat.flash',0,'guns_a','{"y":0.55,"w":0.1,"h":0.1,"segments":3,"flash":1,"far":0}'),
  ('flash_a','cone','mat.flash',0,'spark_a','{"y":0.45,"w":0.42,"h":0.45,"segments":6,"pitch":180,"far":0}'),
  ('turret_p','box','mat.haze_grey',0,'hull','{"y":0.62,"z":-0.36,"w":0.36,"d":0.44,"h":0.2,"yaw":180,"taper":0.18,"traverse":25}'),
  ('roof_p','box','mat.cloth',1,'turret_p','{"y":0.2,"w":0.3,"d":0.36,"h":0.02,"far":0}'),
  ('guns_p','cylinder','mat.cast_iron',0,'turret_p','{"x":-0.07,"y":0.1,"z":0.2,"w":0.06,"h":0.55,"segments":5,"pitch":90,"recoil":0.07,"repeat":2,"dx":0.14,"far":0}'),
  ('turret_q','box','mat.haze_grey',0,'hull','{"y":0.62,"z":0.36,"w":0.36,"d":0.44,"h":0.2,"yaw":0,"taper":0.18,"traverse":-25}'),
  ('roof_q','box','mat.cloth',1,'turret_q','{"y":0.2,"w":0.3,"d":0.36,"h":0.02,"far":0}'),
  ('guns_q','cylinder','mat.cast_iron',0,'turret_q','{"x":-0.07,"y":0.1,"z":0.2,"w":0.06,"h":0.55,"segments":5,"pitch":90,"recoil":0.07,"repeat":2,"dx":0.14,"far":0}'),
  ('turret_x','box','mat.haze_grey',0,'hull','{"x":-1.15,"y":0.74,"w":0.36,"d":0.44,"h":0.2,"yaw":-90,"taper":0.18,"traverse":-30}'),
  ('roof_x','box','mat.cloth',1,'turret_x','{"y":0.2,"w":0.3,"d":0.36,"h":0.02,"far":0}'),
  ('guns_x','cylinder','mat.cast_iron',0,'turret_x','{"x":-0.07,"y":0.1,"z":0.2,"w":0.06,"h":0.55,"segments":5,"pitch":90,"recoil":0.07,"repeat":2,"dx":0.14,"far":0}'),
  ('barbette_x','cylinder','mat.haze_grey',0,'hull','{"x":-1.15,"y":0.58,"w":0.38,"h":0.16,"segments":8,"far":0}'),
  ('turret_y','box','mat.haze_grey',0,'hull','{"x":-1.65,"y":0.6,"w":0.36,"d":0.44,"h":0.2,"yaw":-90,"taper":0.18,"traverse":-30}'),
  ('roof_y','box','mat.cloth',1,'turret_y','{"y":0.2,"w":0.3,"d":0.36,"h":0.02,"far":0}'),
  ('guns_y','cylinder','mat.cast_iron',0,'turret_y','{"x":-0.07,"y":0.1,"z":0.2,"w":0.06,"h":0.55,"segments":5,"pitch":90,"recoil":0.07,"repeat":2,"dx":0.14,"far":0}'),
  ('spark_y','cone','mat.flash',0,'guns_y','{"y":0.55,"w":0.1,"h":0.1,"segments":3,"flash":1,"far":0}'),
  ('flash_y','cone','mat.flash',0,'spark_y','{"y":0.45,"w":0.42,"h":0.45,"segments":6,"pitch":180,"far":0}'),
  ('ensign','box','mat.cloth',1,'hull','{"x":-2.3,"y":1,"w":0.36,"d":0.015,"h":0.24}'),
  ('ensign_staff','cylinder','mat.haze_grey',0,'hull','{"x":-2.15,"y":0.6,"w":0.03,"h":0.62,"segments":4,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.dreadnought_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.dreadnought'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.dreadnought_modern'), (SELECT id FROM def WHERE key='mat.haze_grey'), 100);
