-- unit.space_probe@information: Space Probes (mesh.space_probe_information)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.space_probe_information','Space Probes');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.space_probe_information';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('bus','cylinder','mat.gold',0,'','{"y":0.22,"w":0.34,"h":0.3,"segments":8}'),
  ('band','cylinder','mat.steel',1,'','{"y":0.26,"w":0.36,"h":0.18,"segments":8}'),
  ('deck','cylinder','mat.white_metal',1,'','{"y":0.52,"w":0.3,"h":0.04,"segments":8}'),
  ('dish','cone','mat.white_metal',1,'','{"y":0.56,"w":0.72,"h":0.16,"segments":14,"pitch":180}'),
  ('dish_face','cylinder','mat.white_metal',0,'','{"y":0.7,"w":0.74,"h":0.02,"segments":14,"tint":"#e4e8ec"}'),
  ('feed_struts','box','mat.iron',0,'','{"x":-0.13,"y":0.6,"w":0.015,"d":0.015,"h":0.3,"repeat":2,"dx":0.26,"tint":"#4a5260","far":0}'),
  ('feed','cylinder','mat.iron',0,'','{"y":0.88,"w":0.07,"h":0.06,"segments":6,"tint":"#4a5260","far":0}'),
  ('feed_horn','cone','mat.iron',0,'','{"y":0.84,"w":0.06,"h":0.05,"segments":6,"pitch":180,"far":0}'),
  ('rtg_boom','box','mat.steel',0,'','{"x":-0.48,"y":0.2,"w":0.62,"d":0.03,"h":0.03,"tint":"#7c8592"}'),
  ('rtg','cylinder','mat.iron',1,'','{"x":-0.86,"y":0.08,"w":0.13,"h":0.26,"segments":8,"tint":"#4a5260"}'),
  ('rtg_fins','box','mat.iron',0,'','{"x":-0.86,"y":0.1,"w":0.24,"d":0.015,"h":0.22,"tint":"#3a404a","far":0}'),
  ('rtg_fins_x','box','mat.iron',0,'','{"x":-0.86,"y":0.1,"w":0.015,"d":0.24,"h":0.22,"tint":"#3a404a","far":0}'),
  ('rtg_cap','dome','mat.steel',0,'','{"x":-0.86,"y":0.34,"w":0.1,"h":0.04,"segments":8,"far":0}'),
  ('mag_boom','box','mat.white_metal',0,'','{"x":0.7,"y":0.26,"w":1.06,"d":0.018,"h":0.018}'),
  ('mag_sensor','dome','mat.white_metal',0,'','{"x":1.24,"y":0.24,"w":0.07,"h":0.05,"segments":8,"far":0}'),
  ('mag_mid','box','mat.steel',0,'','{"x":0.74,"y":0.25,"w":0.05,"d":0.05,"h":0.04,"tint":"#4a5260","far":0}'),
  ('scan_arm','box','mat.steel',0,'','{"x":0.06,"y":0.1,"z":0.27,"w":0.06,"d":0.22,"h":0.05,"tint":"#7c8592","far":0}'),
  ('scan_platform','box','mat.steel',1,'','{"x":0.06,"y":0.06,"z":0.42,"w":0.16,"d":0.12,"h":0.1,"tint":"#c3cbd6"}'),
  ('camera','cylinder','mat.iron',0,'','{"x":0.06,"y":0.11,"z":0.52,"w":0.07,"h":0.12,"segments":8,"pitch":90,"tint":"#2a2e36","far":0}'),
  ('camera_lens','cylinder','mat.glass',0,'','{"x":0.06,"y":0.11,"z":0.64,"w":0.05,"h":0.01,"segments":8,"pitch":90,"far":0}'),
  ('spectrometer','box','mat.steel',0,'','{"x":-0.06,"y":0.16,"z":0.42,"w":0.06,"d":0.08,"h":0.06,"tint":"#d8b85a","far":0}'),
  ('low_gain','cylinder','mat.white_metal',0,'','{"x":0.12,"y":0.92,"w":0.012,"h":0.2,"segments":3,"far":0}'),
  ('low_gain_tip','dome','mat.white_metal',0,'','{"x":0.12,"y":1.12,"w":0.04,"h":0.03,"segments":6,"far":0}'),
  ('foil_panels','box','mat.gold',0,'','{"x":-0.17,"y":0.26,"z":-0.07,"w":0.012,"d":0.12,"h":0.16,"repeat":2,"dz":0.14,"tint":"#b08c38","far":0}'),
  ('foil_panels_e','box','mat.gold',0,'','{"x":0.17,"y":0.26,"z":-0.07,"w":0.012,"d":0.12,"h":0.16,"repeat":2,"dz":0.14,"tint":"#b08c38","far":0}'),
  ('nozzles','cylinder','mat.iron',0,'','{"x":-0.09,"y":0.02,"z":-0.1,"w":0.05,"h":0.07,"segments":6,"pitch":180,"repeat":2,"dx":0.18,"tint":"#4a5260","far":0}'),
  ('nozzle_glow','cylinder','mat.light_strip',0,'','{"x":-0.09,"y":-0.06,"z":-0.1,"w":0.04,"h":0.01,"segments":6,"repeat":2,"dx":0.18,"far":0}'),
  ('main_engine','cone','mat.iron',0,'','{"y":-0.06,"w":0.14,"h":0.1,"segments":8,"tint":"#4a5260","far":0}'),
  ('engine_glow','cylinder','mat.light_strip',0,'','{"y":-0.08,"w":0.1,"h":0.02,"segments":8,"far":0}'),
  ('reaction_wheel','wheel','mat.steel',0,'','{"y":0.4,"z":0.18,"yaw":90,"w":0.14,"t":0.03,"spokes":3,"segments":6,"tint":"#7c8592","far":0}'),
  ('light_port','box','mat.signal_red',0,'','{"x":-0.92,"y":0.36,"w":0.04,"d":0.04,"h":0.04,"far":0}'),
  ('light_stbd','box','mat.light_strip',0,'','{"x":1.29,"y":0.22,"w":0.04,"d":0.04,"h":0.04,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.space_probe_information'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.space_probe'), NULL, (SELECT id FROM def WHERE key='age.information'), (SELECT id FROM def WHERE key='mesh.space_probe_information'), (SELECT id FROM def WHERE key='mat.steel'), 100);
