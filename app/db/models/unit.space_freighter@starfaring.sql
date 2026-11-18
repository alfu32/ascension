-- unit.space_freighter@starfaring: Space Freighters (mesh.space_freighter_starfaring)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.space_freighter_starfaring','Space Freighters');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.space_freighter_starfaring';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('keel','box','mat.steel',0,'','{"y":0.12,"w":0.5,"d":3.2,"h":0.26,"taper":0.1,"tint":"#8e97a3"}'),
  ('container','box','mat.steel',0,'keel','{"y":0.38,"z":-0.95,"w":0.9,"d":0.78,"h":0.6,"repeat":3,"dz":0.9,"tint":"#b98a4a"}'),
  ('band','box','mat.steel',1,'keel','{"y":0.38,"z":-0.95,"w":0.94,"d":0.3,"h":0.62,"repeat":3,"dz":0.9}'),
  ('fin','box','mat.steel',1,'keel','{"y":0.16,"z":-1.3,"w":1.9,"d":0.55,"h":0.06,"taper":0.3}'),
  ('sides_port','box','mat.corrugated',0,'','{"x":-0.455,"y":0.52,"z":-1.27,"w":0.02,"d":0.22,"h":0.56,"repeat":6,"dz":0.45,"tint":"#d8a060","far":0}'),
  ('sides_stbd','box','mat.corrugated',0,'','{"x":0.455,"y":0.52,"z":-1.27,"w":0.02,"d":0.22,"h":0.56,"repeat":6,"dz":0.45,"tint":"#d8a060","far":0}'),
  ('tops','box','mat.corrugated',0,'','{"y":1.1,"z":-1.27,"w":0.84,"d":0.2,"h":0.015,"repeat":6,"dz":0.45,"tint":"#d8a060","far":0}'),
  ('posts','box','mat.steel',0,'','{"x":-0.44,"y":0.5,"z":-1.33,"w":0.04,"d":0.04,"h":0.6,"repeat":2,"dx":0.88,"tint":"#7a5a30","far":0}'),
  ('posts_fore','box','mat.steel',0,'','{"x":-0.44,"y":0.5,"z":0.82,"w":0.04,"d":0.04,"h":0.6,"repeat":2,"dx":0.88,"tint":"#7a5a30","far":0}'),
  ('clamps','box','mat.steel',0,'','{"x":-0.3,"y":0.4,"z":-1.4,"w":0.1,"d":0.06,"h":0.12,"repeat":4,"dx":0.2,"tint":"#4a5260","far":0}'),
  ('clamps_mid','box','mat.steel',0,'','{"x":-0.3,"y":0.4,"z":0.4,"w":0.1,"d":0.06,"h":0.12,"repeat":4,"dx":0.2,"tint":"#4a5260","far":0}'),
  ('doors','box','mat.iron',0,'','{"x":-0.2,"y":0.55,"z":-1.355,"w":0.36,"d":0.02,"h":0.48,"repeat":2,"dx":0.4,"tint":"#8a6a3a","far":0}'),
  ('truss','box','mat.steel',0,'','{"x":-0.2,"y":0.08,"z":-0.1,"w":0.04,"d":2.8,"h":0.04,"repeat":2,"dx":0.4,"tint":"#4a5260","far":0}'),
  ('truss_ribs','box','mat.steel',0,'','{"y":0.08,"z":-1.3,"w":0.44,"d":0.04,"h":0.04,"repeat":6,"dz":0.45,"tint":"#4a5260","far":0}'),
  ('bridge','box','mat.steel',0,'','{"y":0.52,"z":1.45,"w":0.56,"d":0.45,"h":0.36,"taper":0.25,"tint":"#c3cbd6","far":0}'),
  ('window','box','mat.light_strip',0,'','{"y":0.66,"z":1.68,"w":0.4,"d":0.04,"h":0.08,"far":0}'),
  ('side_windows','box','mat.light_strip',0,'','{"x":-0.255,"y":0.68,"z":1.45,"w":0.02,"d":0.3,"h":0.04,"repeat":2,"dx":0.51,"far":0}'),
  ('cab_band','box','mat.steel',1,'','{"y":0.82,"z":1.42,"w":0.42,"d":0.32,"h":0.05,"far":0}'),
  ('cab_roof','box','mat.steel',0,'','{"y":0.87,"z":1.4,"w":0.3,"d":0.22,"h":0.04,"tint":"#d2d8e0","far":0}'),
  ('mast','cylinder','mat.iron',0,'','{"x":0.12,"y":0.9,"z":1.35,"w":0.025,"h":0.3,"segments":4,"tint":"#7c8592","far":0}'),
  ('mast_light','box','mat.signal_red',0,'','{"x":0.12,"y":1.2,"z":1.35,"w":0.04,"d":0.04,"h":0.04,"far":0}'),
  ('dish','dome','mat.white_metal',0,'','{"x":-0.12,"y":0.9,"z":1.35,"w":0.14,"h":0.05,"segments":8,"pitch":-30,"far":0}'),
  ('nose','box','mat.steel',0,'','{"y":0.12,"z":1.65,"w":0.4,"d":0.15,"h":0.4,"taper":0.5,"pitch":30,"tint":"#9aa4b2","far":0}'),
  ('radiators','box','mat.white_metal',0,'','{"x":-0.36,"y":0.6,"z":-1.6,"w":0.03,"d":0.25,"h":0.4,"repeat":2,"dx":0.72,"far":0}'),
  ('fin_light_port','box','mat.signal_red',0,'','{"x":-0.93,"y":0.32,"z":-1.3,"w":0.05,"d":0.06,"h":0.04,"far":0}'),
  ('fin_light_stbd','box','mat.light_strip',0,'','{"x":0.93,"y":0.32,"z":-1.3,"w":0.05,"d":0.06,"h":0.04,"far":0}'),
  ('engine','cylinder','mat.iron',0,'','{"x":-0.3,"y":0.32,"z":-1.72,"w":0.32,"h":0.4,"segments":8,"pitch":90,"repeat":2,"dx":0.6,"tint":"#4a5260","far":0}'),
  ('drive','wheel','mat.glow',0,'','{"x":-0.3,"y":0.32,"z":-1.74,"yaw":90,"w":0.26,"t":0.05,"spokes":3,"segments":8,"repeat":2,"dx":0.6,"far":0}'),
  ('engine_bell','cylinder','mat.cast_iron',0,'','{"x":-0.3,"y":0.32,"z":-1.78,"w":0.371,"h":0.12,"segments":10,"pitch":90,"taper":0.2,"repeat":2,"dx":0.6,"far":0}'),
  ('engine_band','cylinder','mat.steel',0,'','{"x":-0.3,"y":0.32,"z":-1.48,"w":0.339,"h":0.05,"segments":10,"pitch":90,"repeat":2,"dx":0.6,"tint":"#9aa4b2","far":0}'),
  ('engine_throat','cylinder','mat.glow',0,'','{"x":-0.3,"y":0.32,"z":-1.795,"w":0.288,"h":0.012,"segments":10,"pitch":90,"repeat":2,"dx":0.6,"far":0}'),
  ('engine_plume','cone','mat.glow',0,'','{"x":-0.3,"y":0.32,"z":-1.79,"w":0.224,"h":0.144,"segments":10,"pitch":-90,"repeat":2,"dx":0.6,"far":0}'),
  ('engine_mounts','box','mat.steel',0,'','{"x":-0.3,"y":0.2,"z":-1.45,"w":0.16,"d":0.25,"h":0.12,"repeat":2,"dx":0.6,"tint":"#8e97a3","far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.space_freighter_starfaring'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.space_freighter'), NULL, (SELECT id FROM def WHERE key='age.starfaring'), (SELECT id FROM def WHERE key='mesh.space_freighter_starfaring'), (SELECT id FROM def WHERE key='mat.steel'), 100);
