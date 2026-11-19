-- unit.transport_ship@starfaring: Transport Ship (mesh.transport_ship_starfaring)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.transport_ship_starfaring','Transport Ship');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.transport_ship_starfaring';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','box','mat.steel',0,'','{"y":0.2,"w":1.1,"d":3,"h":0.62,"taper":0.12,"tint":"#a6b0bd"}'),
  ('pod','box','mat.steel',0,'hull','{"x":-0.78,"y":0.05,"w":0.46,"d":1.9,"h":0.5,"taper":0.06,"repeat":2,"dx":1.56,"tint":"#7c8592"}'),
  ('spine','box','mat.steel',1,'hull','{"y":0.62,"z":-0.15,"w":0.42,"d":2,"h":0.14}'),
  ('wing','box','mat.steel',1,'hull','{"y":0.22,"z":-0.75,"w":2.8,"d":0.7,"h":0.07,"taper":0.3}'),
  ('bridge','box','mat.steel',0,'hull','{"y":0.62,"z":1.05,"w":0.62,"d":0.55,"h":0.3,"taper":0.15,"tint":"#c3cbd6","far":0}'),
  ('bridge_windows','box','mat.light_strip',0,'','{"y":1.05,"z":1.33,"w":0.44,"d":0.02,"h":0.06,"far":0}'),
  ('bridge_side_windows','box','mat.light_strip',0,'','{"x":-0.28,"y":1,"z":1.05,"w":0.02,"d":0.4,"h":0.04,"repeat":2,"dx":0.56,"far":0}'),
  ('bridge_roof','box','mat.steel',0,'','{"y":1.12,"z":1,"w":0.4,"d":0.3,"h":0.04,"tint":"#d2d8e0","far":0}'),
  ('mast','cylinder','mat.iron',0,'','{"x":0.15,"y":1.16,"z":0.95,"w":0.03,"h":0.35,"segments":4,"tint":"#7c8592","far":0}'),
  ('mast_light','box','mat.signal_red',0,'','{"x":0.15,"y":1.51,"z":0.95,"w":0.04,"d":0.04,"h":0.04,"far":0}'),
  ('dish','dome','mat.white_metal',0,'','{"x":-0.15,"y":1.16,"z":0.9,"w":0.16,"h":0.06,"segments":8,"pitch":-30,"far":0}'),
  ('ramp','box','mat.steel',0,'','{"y":0.2,"z":1.42,"w":0.6,"d":0.18,"h":0.3,"pitch":-12,"tint":"#8e97a3","far":0}'),
  ('ramp_light','box','mat.light_strip',0,'','{"y":0.42,"z":1.53,"w":0.4,"d":0.02,"h":0.03,"far":0}'),
  ('plates_port','box','mat.composite_panel',0,'','{"x":-0.535,"y":0.62,"z":-0.9,"w":0.025,"d":0.5,"h":0.18,"repeat":3,"dz":0.6,"tint":"#8994a3","far":0}'),
  ('plates_stbd','box','mat.composite_panel',0,'','{"x":0.535,"y":0.62,"z":-0.9,"w":0.025,"d":0.5,"h":0.18,"repeat":3,"dz":0.6,"tint":"#8994a3","far":0}'),
  ('spine_plates','box','mat.composite_panel',0,'','{"x":-0.12,"y":0.96,"z":-0.85,"w":0.16,"d":0.4,"h":0.02,"repeat":2,"dx":0.24,"tint":"#8994a3","far":0}'),
  ('spine_plates_fore','box','mat.composite_panel',0,'','{"x":-0.12,"y":0.96,"z":0.3,"w":0.16,"d":0.5,"h":0.02,"repeat":2,"dx":0.24,"tint":"#8994a3","far":0}'),
  ('keel','box','mat.steel',0,'','{"y":0.12,"z":-0.1,"w":0.6,"d":2.6,"h":0.1,"tint":"#8e97a3","far":0}'),
  ('hatches_port','box','mat.steel',0,'','{"x":-1.015,"y":0.3,"z":-0.55,"w":0.02,"d":0.24,"h":0.3,"repeat":3,"dz":0.55,"tint":"#8e97a3","far":0}'),
  ('hatch_lights_port','box','mat.light_strip',0,'','{"x":-1.015,"y":0.62,"z":-0.55,"w":0.02,"d":0.2,"h":0.025,"repeat":3,"dz":0.55,"far":0}'),
  ('pod_windows_port','box','mat.light_strip',0,'','{"x":-1.015,"y":0.5,"w":0.02,"d":1.5,"h":0.03,"far":0}'),
  ('hatches_stbd','box','mat.steel',0,'','{"x":1.015,"y":0.3,"z":-0.55,"w":0.02,"d":0.24,"h":0.3,"repeat":3,"dz":0.55,"tint":"#8e97a3","far":0}'),
  ('hatch_lights_stbd','box','mat.light_strip',0,'','{"x":1.015,"y":0.62,"z":-0.55,"w":0.02,"d":0.2,"h":0.025,"repeat":3,"dz":0.55,"far":0}'),
  ('pod_windows_stbd','box','mat.light_strip',0,'','{"x":1.015,"y":0.5,"w":0.02,"d":1.5,"h":0.03,"far":0}'),
  ('pod_caps','box','mat.steel',0,'','{"x":-0.78,"y":0.25,"z":0.95,"w":0.42,"d":0.12,"h":0.45,"taper":0.3,"pitch":20,"repeat":2,"dx":1.56,"tint":"#c3cbd6","far":0}'),
  ('pod_plates','box','mat.composite_panel',0,'','{"x":-0.78,"y":0.745,"z":-0.3,"w":0.32,"d":0.5,"h":0.02,"repeat":2,"dx":1.56,"tint":"#8994a3","far":0}'),
  ('pod_plates_fore','box','mat.composite_panel',0,'','{"x":-0.78,"y":0.745,"z":0.4,"w":0.32,"d":0.5,"h":0.02,"repeat":2,"dx":1.56,"tint":"#8994a3","far":0}'),
  ('legs','box','mat.steel',0,'','{"x":-0.78,"y":0.13,"z":-0.7,"w":0.06,"d":0.06,"h":0.12,"repeat":2,"dx":1.56,"tint":"#4a5260","far":0}'),
  ('legs_fore','box','mat.steel',0,'','{"x":-0.78,"y":0.13,"z":0.7,"w":0.06,"d":0.06,"h":0.12,"repeat":2,"dx":1.56,"tint":"#4a5260","far":0}'),
  ('pads','box','mat.steel',0,'','{"x":-0.78,"y":0.1,"z":-0.7,"w":0.18,"d":0.14,"h":0.03,"repeat":2,"dx":1.56,"tint":"#8e97a3","far":0}'),
  ('pads_fore','box','mat.steel',0,'','{"x":-0.78,"y":0.1,"z":0.7,"w":0.18,"d":0.14,"h":0.03,"repeat":2,"dx":1.56,"tint":"#8e97a3","far":0}'),
  ('wing_fences','box','mat.steel',0,'','{"x":-1,"y":0.49,"z":-0.75,"w":0.025,"d":0.55,"h":0.08,"repeat":2,"dx":2,"tint":"#9aa4b2","far":0}'),
  ('wing_light_port','box','mat.signal_red',0,'','{"x":-1.38,"y":0.45,"z":-0.85,"w":0.05,"d":0.05,"h":0.04,"far":0}'),
  ('wing_light_stbd','box','mat.light_strip',0,'','{"x":1.38,"y":0.45,"z":-0.85,"w":0.05,"d":0.05,"h":0.04,"far":0}'),
  ('rcs','box','mat.steel',0,'','{"x":-0.5,"y":0.75,"z":1.25,"w":0.06,"d":0.08,"h":0.06,"repeat":2,"dx":1,"tint":"#8e97a3","far":0}'),
  ('engine','cylinder','mat.iron',0,'','{"x":-0.42,"y":0.52,"z":-1.7,"w":0.38,"h":0.42,"segments":8,"pitch":90,"repeat":2,"dx":0.84,"tint":"#4a5260","far":0}'),
  ('drive','wheel','mat.glow',0,'','{"x":-0.42,"y":0.52,"z":-1.72,"yaw":90,"w":0.3,"t":0.05,"spokes":3,"segments":8,"repeat":2,"dx":0.84,"far":0}'),
  ('engine_bell','cylinder','mat.cast_iron',0,'','{"x":-0.42,"y":0.52,"z":-1.76,"w":0.441,"h":0.12,"segments":10,"pitch":90,"taper":0.2,"repeat":2,"dx":0.84,"far":0}'),
  ('engine_band','cylinder','mat.steel',0,'','{"x":-0.42,"y":0.52,"z":-1.448,"w":0.403,"h":0.05,"segments":10,"pitch":90,"repeat":2,"dx":0.84,"tint":"#9aa4b2","far":0}'),
  ('engine_throat','cylinder','mat.glow',0,'','{"x":-0.42,"y":0.52,"z":-1.775,"w":0.342,"h":0.012,"segments":10,"pitch":90,"repeat":2,"dx":0.84,"far":0}'),
  ('engine_plume','cone','mat.glow',0,'','{"x":-0.42,"y":0.52,"z":-1.77,"w":0.266,"h":0.171,"segments":10,"pitch":-90,"repeat":2,"dx":0.84,"far":0}'),
  ('engine_mounts','box','mat.steel',0,'','{"x":-0.42,"y":0.4,"z":-1.45,"w":0.2,"d":0.3,"h":0.14,"repeat":2,"dx":0.84,"tint":"#8e97a3","far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.transport_ship_starfaring'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.transport_ship'), NULL, (SELECT id FROM def WHERE key='age.starfaring'), (SELECT id FROM def WHERE key='mesh.transport_ship_starfaring'), (SELECT id FROM def WHERE key='mat.steel'), 100);
