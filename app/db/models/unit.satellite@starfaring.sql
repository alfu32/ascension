-- unit.satellite@starfaring: Satellites (mesh.satellite_starfaring)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.satellite_starfaring','Satellites');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.satellite_starfaring';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('bus','box','mat.steel',0,'','{"y":0.3,"w":0.36,"d":0.36,"h":0.42,"tint":"#c9a64a"}'),
  ('band','box','mat.steel',1,'bus','{"y":0.5,"w":0.38,"d":0.38,"h":0.08}'),
  ('mast','cylinder','mat.iron',0,'bus','{"y":0.3,"w":0.05,"h":1.7,"segments":6,"roll":90,"tint":"#7c8592"}'),
  ('wing','box','mat.steel',0,'bus','{"x":-0.62,"y":0.3,"w":0.82,"d":0.42,"h":0.03,"repeat":2,"dx":1.24,"tint":"#24365a"}'),
  ('dish','cone','mat.white_metal',0,'bus','{"y":0.02,"w":0.34,"h":0.14,"segments":10,"pitch":180,"tint":"#d8dde3"}'),
  ('foil_panels','box','mat.steel',0,'','{"x":-0.185,"y":0.33,"z":-0.09,"w":0.015,"d":0.14,"h":0.14,"repeat":2,"dz":0.18,"tint":"#d8b85a","far":0}'),
  ('foil_panels_e','box','mat.steel',0,'','{"x":0.185,"y":0.33,"z":-0.09,"w":0.015,"d":0.14,"h":0.14,"repeat":2,"dz":0.18,"tint":"#d8b85a","far":0}'),
  ('foil_panels_hi','box','mat.steel',0,'','{"x":-0.185,"y":0.53,"z":-0.09,"w":0.015,"d":0.14,"h":0.12,"repeat":2,"dz":0.18,"tint":"#b08c38","far":0}'),
  ('top_deck','box','mat.steel',0,'','{"y":0.72,"w":0.3,"d":0.3,"h":0.04,"tint":"#c3cbd6","far":0}'),
  ('trackers','cylinder','mat.iron',0,'','{"x":-0.08,"y":0.76,"z":-0.08,"w":0.06,"h":0.08,"segments":6,"repeat":2,"dx":0.16,"tint":"#4a5260","far":0}'),
  ('beacon','box','mat.signal_red',0,'','{"x":0.08,"y":0.76,"z":0.08,"w":0.04,"d":0.04,"h":0.04,"far":0}'),
  ('whips','cylinder','mat.iron',0,'','{"x":-0.1,"y":0.76,"z":0.1,"w":0.015,"h":0.3,"segments":3,"repeat":2,"dx":0.06,"far":0}'),
  ('yokes','box','mat.steel',0,'','{"x":-0.24,"y":0.58,"w":0.12,"d":0.05,"h":0.05,"repeat":2,"dx":0.48,"tint":"#4a5260","far":0}'),
  ('wing_frames','box','mat.white_metal',0,'','{"x":-0.62,"y":0.595,"z":-0.215,"w":0.84,"d":0.02,"h":0.04,"repeat":2,"dx":1.24,"far":0}'),
  ('wing_frames_aft','box','mat.white_metal',0,'','{"x":-0.62,"y":0.595,"z":0.215,"w":0.84,"d":0.02,"h":0.04,"repeat":2,"dx":1.24,"far":0}'),
  ('cells','box','mat.white_metal',0,'','{"x":-0.92,"y":0.63,"w":0.012,"d":0.42,"h":0.01,"repeat":5,"dx":0.15,"far":0}'),
  ('cells_e','box','mat.white_metal',0,'','{"x":0.32,"y":0.63,"w":0.012,"d":0.42,"h":0.01,"repeat":5,"dx":0.15,"far":0}'),
  ('tip_light_port','box','mat.signal_red',0,'','{"x":-1.05,"y":0.6,"w":0.04,"d":0.04,"h":0.04,"far":0}'),
  ('tip_light_stbd','box','mat.light_strip',0,'','{"x":1.05,"y":0.6,"w":0.04,"d":0.04,"h":0.04,"far":0}'),
  ('boom_head','box','mat.steel',0,'','{"x":1.62,"y":0.55,"w":0.12,"d":0.1,"h":0.1,"tint":"#c3cbd6","far":0}'),
  ('boom_sensor','dome','mat.white_metal',0,'','{"x":1.62,"y":0.65,"w":0.09,"h":0.05,"segments":8,"far":0}'),
  ('dish_rim','cylinder','mat.steel',0,'','{"y":0.165,"w":0.36,"h":0.025,"segments":12,"tint":"#9aa4b2","far":0}'),
  ('feed','cylinder','mat.iron',0,'bus','{"y":-0.08,"w":0.04,"h":0.12,"segments":5,"far":0}'),
  ('feed_horn','cone','mat.iron',0,'','{"y":0.22,"w":0.06,"h":0.04,"segments":6,"pitch":180,"tint":"#4a5260","far":0}'),
  ('thruster','wheel','mat.glow',0,'','{"y":0.6,"z":-0.2,"yaw":90,"w":0.16,"t":0.03,"spokes":3,"segments":6,"far":0}'),
  ('nozzles','cylinder','mat.iron',0,'','{"x":-0.13,"y":0.36,"z":-0.2,"w":0.05,"h":0.06,"segments":6,"pitch":-90,"repeat":2,"dx":0.26,"tint":"#4a5260","far":0}'),
  ('nozzle_glow','cylinder','mat.glow',0,'','{"x":-0.13,"y":0.36,"z":-0.265,"w":0.04,"h":0.01,"segments":6,"pitch":-90,"repeat":2,"dx":0.26,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.satellite_starfaring'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.satellite'), NULL, (SELECT id FROM def WHERE key='age.starfaring'), (SELECT id FROM def WHERE key='mesh.satellite_starfaring'), (SELECT id FROM def WHERE key='mat.steel'), 100);
