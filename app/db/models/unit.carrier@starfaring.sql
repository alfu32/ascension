-- unit.carrier@starfaring: Carriers (mesh.carrier_starfaring)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.carrier_starfaring','Carriers');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.carrier_starfaring';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','box','mat.steel',0,'','{"y":0.25,"w":1.5,"d":4,"h":0.7,"taper":0.08,"tint":"#9aa4b2"}'),
  ('deck','box','mat.steel',0,'','{"y":0.95,"z":0.1,"w":1.7,"d":3.8,"h":0.08,"tint":"#4b525c"}'),
  ('island','box','mat.steel',0,'','{"x":0.62,"y":1.03,"z":-0.3,"w":0.32,"d":0.9,"h":0.5,"taper":0.15,"tint":"#c3cbd6"}'),
  ('mast','cylinder','mat.iron',1,'','{"x":0.62,"y":1.5,"z":-0.3,"w":0.08,"h":0.35,"segments":6}'),
  ('wing','box','mat.steel',1,'','{"y":0.3,"z":-1.2,"w":2.6,"d":0.8,"h":0.07,"taper":0.3}'),
  ('stripe','box','mat.steel',1,'','{"y":1.03,"z":0.1,"w":0.12,"d":3.4,"h":0.02,"far":0}'),
  ('edge_lights','box','mat.light_strip',0,'','{"x":-0.8,"y":1.03,"z":-1.45,"w":0.03,"d":0.03,"h":0.02,"repeat":8,"dz":0.42,"far":0}'),
  ('edge_lights_e','box','mat.light_strip',0,'','{"x":0.8,"y":1.03,"z":-1.45,"w":0.03,"d":0.03,"h":0.02,"repeat":8,"dz":0.42,"far":0}'),
  ('centre_lights','box','mat.glow',0,'','{"y":1.055,"z":-1.4,"w":0.04,"d":0.04,"h":0.015,"repeat":6,"dz":0.55,"far":0}'),
  ('catapults','box','mat.iron',0,'','{"x":-0.4,"y":1.03,"z":1.2,"w":0.05,"d":0.9,"h":0.015,"repeat":2,"dx":0.5,"tint":"#4a5260","far":0}'),
  ('deck_plates','box','mat.composite_panel',0,'','{"x":-0.5,"y":1.03,"z":-0.9,"w":0.5,"d":0.8,"h":0.012,"repeat":2,"dx":0.62,"tint":"#6a7380","far":0}'),
  ('deck_plates_aft','box','mat.composite_panel',0,'','{"x":-0.5,"y":1.03,"z":-1.75,"w":0.5,"d":0.5,"h":0.012,"repeat":2,"dx":1,"tint":"#6a7380","far":0}'),
  ('lift','box','mat.steel',0,'','{"x":-0.45,"y":1.03,"w":0.4,"d":0.45,"h":0.012,"tint":"#3a4048","far":0}'),
  ('lift_marks','box','mat.steel',1,'','{"x":-0.45,"y":1.045,"z":-0.2,"w":0.42,"d":0.04,"h":0.01,"repeat":2,"dz":0.4,"far":0}'),
  ('hangar','box','mat.iron',0,'','{"y":0.45,"z":1.98,"w":1,"d":0.06,"h":0.38,"tint":"#1e2228","far":0}'),
  ('hangar_frame','box','mat.steel',0,'','{"y":0.83,"z":1.99,"w":1.14,"d":0.06,"h":0.06,"tint":"#c3cbd6","far":0}'),
  ('hangar_frame_sides','box','mat.steel',0,'','{"x":-0.54,"y":0.43,"z":1.99,"w":0.06,"d":0.06,"h":0.42,"repeat":2,"dx":1.08,"tint":"#c3cbd6","far":0}'),
  ('hangar_glow','box','mat.glow',0,'','{"y":0.47,"z":1.965,"w":0.9,"d":0.02,"h":0.03,"far":0}'),
  ('hangar_lights','box','mat.light_strip',0,'','{"x":-0.4,"y":0.78,"z":2,"w":0.04,"d":0.02,"h":0.03,"repeat":5,"dx":0.2,"far":0}'),
  ('side_hangars_port','box','mat.iron',0,'','{"x":-0.73,"y":0.42,"z":-0.1,"w":0.06,"d":0.6,"h":0.3,"repeat":2,"dz":0.9,"roll":4,"tint":"#1e2228","far":0}'),
  ('side_hangar_glow_port','box','mat.glow',0,'','{"x":-0.74,"y":0.44,"z":-0.1,"w":0.02,"d":0.5,"h":0.025,"repeat":2,"dz":0.9,"far":0}'),
  ('plates_port','box','mat.composite_panel',0,'','{"x":-0.74,"y":0.3,"z":-1.35,"w":0.03,"d":0.55,"h":0.5,"roll":4,"repeat":2,"dz":2.4,"tint":"#8994a3","far":0}'),
  ('sponsons_port','box','mat.steel',0,'','{"x":-0.82,"y":0.78,"z":0.9,"w":0.18,"d":0.5,"h":0.14,"tint":"#8e97a3","far":0}'),
  ('side_hangars_stbd','box','mat.iron',0,'','{"x":0.73,"y":0.42,"z":-0.1,"w":0.06,"d":0.6,"h":0.3,"repeat":2,"dz":0.9,"roll":-4,"tint":"#1e2228","far":0}'),
  ('side_hangar_glow_stbd','box','mat.glow',0,'','{"x":0.74,"y":0.44,"z":-0.1,"w":0.02,"d":0.5,"h":0.025,"repeat":2,"dz":0.9,"far":0}'),
  ('plates_stbd','box','mat.composite_panel',0,'','{"x":0.74,"y":0.3,"z":-1.35,"w":0.03,"d":0.55,"h":0.5,"roll":-4,"repeat":2,"dz":2.4,"tint":"#8994a3","far":0}'),
  ('sponsons_stbd','box','mat.steel',0,'','{"x":0.82,"y":0.78,"z":0.9,"w":0.18,"d":0.5,"h":0.14,"tint":"#8e97a3","far":0}'),
  ('keel','box','mat.steel',0,'','{"y":0.15,"z":-0.2,"w":0.9,"d":3.4,"h":0.12,"tint":"#8e97a3","far":0}'),
  ('pd_mounts','cylinder','mat.steel',0,'','{"x":-0.82,"y":0.92,"z":0.9,"w":0.14,"h":0.06,"segments":8,"repeat":2,"dx":1.64,"tint":"#9aa4b2","far":0}'),
  ('pd_guns','cylinder','mat.iron',0,'','{"x":-0.82,"y":0.96,"z":0.9,"w":0.03,"h":0.2,"segments":4,"pitch":90,"repeat":2,"dx":1.64,"tint":"#4a5260","far":0}'),
  ('island_windows','box','mat.light_strip',0,'','{"x":0.62,"y":1.38,"z":0.09,"w":0.24,"d":0.02,"h":0.05,"far":0}'),
  ('island_side_windows','box','mat.light_strip',0,'','{"x":0.47,"y":1.33,"z":-0.3,"w":0.02,"d":0.7,"h":0.04,"far":0}'),
  ('island_upper','box','mat.steel',0,'','{"x":0.62,"y":1.53,"z":-0.35,"w":0.22,"d":0.45,"h":0.14,"taper":0.2,"tint":"#c3cbd6","far":0}'),
  ('island_band','box','mat.steel',1,'','{"x":0.62,"y":1.53,"z":-0.35,"w":0.24,"d":0.47,"h":0.04,"far":0}'),
  ('radar','dome','mat.white_metal',0,'','{"x":0.62,"y":1.85,"z":-0.3,"w":0.24,"h":0.06,"segments":8,"pitch":15,"far":0}'),
  ('yard','box','mat.steel',0,'','{"x":0.62,"y":1.78,"z":-0.3,"w":0.32,"d":0.03,"h":0.03,"tint":"#7c8592","far":0}'),
  ('mast_light','box','mat.signal_red',0,'','{"x":0.62,"y":1.94,"z":-0.3,"w":0.05,"d":0.05,"h":0.05,"far":0}'),
  ('aerials','cylinder','mat.iron',0,'','{"x":0.55,"y":1.67,"z":-0.5,"w":0.02,"h":0.3,"segments":3,"repeat":2,"dx":0.14,"far":0}'),
  ('crane','cylinder','mat.iron',0,'','{"x":0.45,"y":1.03,"z":-0.9,"w":0.05,"h":0.32,"segments":4,"tint":"#4a5260","far":0}'),
  ('crane_jib','box','mat.steel',0,'','{"x":0.35,"y":1.32,"z":-0.9,"w":0.26,"d":0.04,"h":0.04,"roll":-15,"tint":"#4a5260","far":0}'),
  ('wing_light_port','box','mat.signal_red',0,'','{"x":-1.28,"y":0.42,"z":-1.2,"w":0.05,"d":0.06,"h":0.05,"far":0}'),
  ('wing_light_stbd','box','mat.light_strip',0,'','{"x":1.28,"y":0.42,"z":-1.2,"w":0.05,"d":0.06,"h":0.05,"far":0}'),
  ('engine','cylinder','mat.iron',0,'','{"x":-0.45,"y":0.45,"z":-2.08,"w":0.42,"h":0.4,"segments":8,"pitch":90,"repeat":3,"dx":0.45,"tint":"#4a5260","far":0}'),
  ('drive','wheel','mat.glow',0,'','{"x":-0.45,"y":0.45,"z":-2.1,"yaw":90,"w":0.34,"t":0.05,"spokes":3,"segments":8,"repeat":3,"dx":0.45,"far":0}'),
  ('engine_bell','cylinder','mat.cast_iron',0,'','{"x":-0.45,"y":0.45,"z":-2.14,"w":0.487,"h":0.12,"segments":10,"pitch":90,"taper":0.2,"repeat":3,"dx":0.45,"far":0}'),
  ('engine_band','cylinder','mat.steel',0,'','{"x":-0.45,"y":0.45,"z":-1.84,"w":0.445,"h":0.05,"segments":10,"pitch":90,"repeat":3,"dx":0.45,"tint":"#9aa4b2","far":0}'),
  ('engine_throat','cylinder','mat.glow',0,'','{"x":-0.45,"y":0.45,"z":-2.155,"w":0.378,"h":0.012,"segments":10,"pitch":90,"repeat":3,"dx":0.45,"far":0}'),
  ('engine_plume','cone','mat.glow',0,'','{"x":-0.45,"y":0.45,"z":-2.15,"w":0.294,"h":0.189,"segments":10,"pitch":-90,"repeat":3,"dx":0.45,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.carrier_starfaring'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.carrier'), NULL, (SELECT id FROM def WHERE key='age.starfaring'), (SELECT id FROM def WHERE key='mesh.carrier_starfaring'), (SELECT id FROM def WHERE key='mat.steel'), 100);
