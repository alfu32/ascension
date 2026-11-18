-- unit.orbital_bomber@starfaring: Orbital Bombers (mesh.orbital_bomber_starfaring)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.orbital_bomber_starfaring','Orbital Bombers');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.orbital_bomber_starfaring';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','box','mat.steel',0,'','{"y":0.3,"w":0.62,"d":1.9,"h":0.34,"taper":0.2,"tint":"#9aa3ae"}'),
  ('nose','box','mat.steel',0,'','{"y":0.33,"z":1.15,"w":0.4,"d":0.45,"h":0.22,"taper":0.5,"tint":"#c3cbd6"}'),
  ('wing','box','mat.steel',1,'','{"y":0.26,"z":-0.2,"w":2.1,"d":0.85,"h":0.07,"taper":0.45}'),
  ('bay','box','mat.iron',0,'','{"y":0.16,"z":0.1,"w":0.42,"d":1,"h":0.16,"tint":"#5d6470"}'),
  ('canopy','box','mat.steel',0,'','{"y":0.62,"z":0.7,"w":0.3,"d":0.36,"h":0.1,"taper":0.4,"tint":"#3b5a78","far":0}'),
  ('canopy_frames','box','mat.steel',0,'','{"y":0.62,"z":0.7,"w":0.32,"d":0.03,"h":0.09,"repeat":2,"dz":0.16,"tint":"#c3cbd6","far":0}'),
  ('canopy_glow','box','mat.light_strip',0,'','{"y":0.68,"z":0.76,"w":0.12,"d":0.12,"h":0.02,"far":0}'),
  ('sensor','dome','mat.smart_glass',0,'','{"y":0.14,"z":1,"w":0.16,"h":0.08,"segments":8,"pitch":180,"far":0}'),
  ('dorsal','box','mat.steel',0,'','{"y":0.62,"z":-0.2,"w":0.22,"d":0.9,"h":0.08,"taper":0.3,"tint":"#9aa4b2","far":0}'),
  ('dorsal_stripe','box','mat.steel',1,'','{"y":0.7,"z":-0.2,"w":0.06,"d":0.8,"h":0.012,"far":0}'),
  ('body_plates','box','mat.composite_panel',0,'','{"x":-0.25,"y":0.64,"z":0.25,"w":0.14,"d":0.4,"h":0.012,"repeat":2,"dx":0.5,"tint":"#8994a3","far":0}'),
  ('bay_doors','box','mat.steel',0,'','{"x":-0.12,"y":0.13,"z":0.1,"w":0.02,"d":0.95,"h":0.12,"repeat":2,"dx":0.24,"roll":0,"tint":"#8e97a3","far":0}'),
  ('ordnance','cylinder','mat.iron',0,'','{"x":-0.6,"y":0.12,"z":-0.1,"w":0.14,"h":0.5,"segments":6,"pitch":90,"repeat":2,"dx":1.2,"tint":"#3d434c","far":0}'),
  ('ordnance_tips','cone','mat.iron',0,'','{"x":-0.6,"y":0.12,"z":0.4,"w":0.14,"h":0.12,"segments":6,"pitch":90,"repeat":2,"dx":1.2,"tint":"#c9a64a","far":0}'),
  ('pylons','box','mat.steel',0,'','{"x":-0.6,"y":0.18,"z":-0.05,"w":0.04,"d":0.3,"h":0.1,"repeat":2,"dx":1.2,"tint":"#4a5260","far":0}'),
  ('missiles','cylinder','mat.iron',0,'','{"x":-0.85,"y":0.17,"z":-0.3,"w":0.07,"h":0.42,"segments":6,"pitch":90,"repeat":2,"dx":1.7,"tint":"#e2e5e8","far":0}'),
  ('missile_tips','cone','mat.iron',0,'','{"x":-0.85,"y":0.17,"z":0.12,"w":0.07,"h":0.08,"segments":6,"pitch":90,"repeat":2,"dx":1.7,"tint":"#c3533a","far":0}'),
  ('missile_pylons','box','mat.steel',0,'','{"x":-0.85,"y":0.21,"z":-0.15,"w":0.03,"d":0.2,"h":0.07,"repeat":2,"dx":1.7,"tint":"#4a5260","far":0}'),
  ('fences','box','mat.steel',0,'','{"x":-0.45,"y":0.33,"z":-0.25,"w":0.025,"d":0.6,"h":0.06,"repeat":2,"dx":0.9,"tint":"#9aa4b2","far":0}'),
  ('booms','box','mat.steel',0,'','{"x":-0.38,"y":0.3,"z":-1.05,"w":0.12,"d":0.6,"h":0.12,"taper":0.2,"repeat":2,"dx":0.76,"tint":"#9aa4b2","far":0}'),
  ('tail_fins','box','mat.steel',1,'','{"x":-0.38,"y":0.42,"z":-1.15,"w":0.03,"d":0.32,"h":0.22,"taper":0.5,"repeat":2,"dx":0.76,"far":0}'),
  ('tailplane','box','mat.steel',0,'','{"y":0.38,"z":-1.2,"w":0.9,"d":0.22,"h":0.03,"taper":0.3,"tint":"#9aa4b2","far":0}'),
  ('tip_light_port','box','mat.signal_red',0,'','{"x":-1.04,"y":0.29,"z":-0.4,"w":0.04,"d":0.05,"h":0.04,"far":0}'),
  ('tip_light_stbd','box','mat.light_strip',0,'','{"x":1.04,"y":0.29,"z":-0.4,"w":0.04,"d":0.05,"h":0.04,"far":0}'),
  ('beacon','box','mat.signal_red',0,'','{"y":0.73,"z":0.1,"w":0.035,"d":0.035,"h":0.035,"far":0}'),
  ('missiles_outer','cylinder','mat.iron',0,'','{"x":-0.98,"y":0.18,"z":-0.4,"w":0.06,"h":0.36,"segments":6,"pitch":90,"repeat":2,"dx":1.96,"tint":"#e2e5e8","far":0}'),
  ('missile_tips_outer','cone','mat.iron',0,'','{"x":-0.98,"y":0.18,"z":-0.04,"w":0.06,"h":0.07,"segments":6,"pitch":90,"repeat":2,"dx":1.96,"tint":"#c3533a","far":0}'),
  ('missile_pylons_outer','box','mat.steel',0,'','{"x":-0.98,"y":0.22,"z":-0.25,"w":0.03,"d":0.18,"h":0.06,"repeat":2,"dx":1.96,"tint":"#4a5260","far":0}'),
  ('probe','cylinder','mat.iron',0,'','{"y":0.42,"z":1.3,"w":0.03,"h":0.3,"segments":4,"pitch":90,"tint":"#4a5260","far":0}'),
  ('side_windows','box','mat.light_strip',0,'','{"x":-0.16,"y":0.55,"z":0.85,"w":0.02,"d":0.2,"h":0.04,"repeat":2,"dx":0.32,"far":0}'),
  ('whips','cylinder','mat.iron',0,'','{"x":-0.06,"y":0.7,"z":-0.5,"w":0.015,"h":0.2,"segments":3,"repeat":2,"dx":0.12,"far":0}'),
  ('bay_glow','box','mat.glow',0,'','{"y":0.12,"z":0.1,"w":0.18,"d":0.9,"h":0.02,"far":0}'),
  ('nacelles','box','mat.steel',0,'','{"x":-0.18,"y":0.2,"z":-0.9,"w":0.26,"d":0.4,"h":0.26,"taper":0.15,"repeat":2,"dx":0.36,"tint":"#8e97a3","far":0}'),
  ('engine','cylinder','mat.iron',0,'','{"x":-0.18,"y":0.32,"z":-1.05,"w":0.24,"h":0.3,"segments":8,"pitch":90,"repeat":2,"dx":0.36,"tint":"#4a5260","far":0}'),
  ('drive','wheel','mat.glow',0,'','{"x":-0.18,"y":0.32,"z":-1.07,"yaw":90,"w":0.2,"t":0.04,"spokes":3,"segments":8,"repeat":2,"dx":0.36,"far":0}'),
  ('engine_bell','cylinder','mat.cast_iron',0,'','{"x":-0.18,"y":0.32,"z":-1.11,"w":0.278,"h":0.12,"segments":10,"pitch":90,"taper":0.2,"repeat":2,"dx":0.36,"far":0}'),
  ('engine_band','cylinder','mat.steel',0,'','{"x":-0.18,"y":0.32,"z":-0.87,"w":0.254,"h":0.05,"segments":10,"pitch":90,"repeat":2,"dx":0.36,"tint":"#9aa4b2","far":0}'),
  ('engine_throat','cylinder','mat.glow',0,'','{"x":-0.18,"y":0.32,"z":-1.125,"w":0.216,"h":0.012,"segments":10,"pitch":90,"repeat":2,"dx":0.36,"far":0}'),
  ('engine_plumes','cone','mat.glow',0,'','{"x":-0.18,"y":0.32,"z":-1.13,"w":0.16,"h":0.1,"segments":8,"pitch":-90,"repeat":2,"dx":0.36,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.orbital_bomber_starfaring'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.orbital_bomber'), NULL, (SELECT id FROM def WHERE key='age.starfaring'), (SELECT id FROM def WHERE key='mesh.orbital_bomber_starfaring'), (SELECT id FROM def WHERE key='mat.steel'), 100);
