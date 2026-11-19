-- unit.orbital_fighter@starfaring: Orbital Fighters (mesh.orbital_fighter_starfaring)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.orbital_fighter_starfaring','Orbital Fighters');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.orbital_fighter_starfaring';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','box','mat.steel',0,'','{"y":0.25,"w":0.36,"d":1.5,"h":0.22,"taper":0.3,"tint":"#b4bcc8"}'),
  ('nose','box','mat.steel',0,'','{"y":0.27,"z":0.95,"w":0.22,"d":0.45,"h":0.14,"taper":0.6,"tint":"#d2d8e0"}'),
  ('wing','box','mat.steel',1,'','{"y":0.22,"z":-0.25,"w":1.5,"d":0.6,"h":0.05,"taper":0.55}'),
  ('fin','box','mat.steel',0,'','{"y":0.45,"z":-0.5,"w":0.04,"d":0.42,"h":0.2,"taper":0.5,"tint":"#8e97a3"}'),
  ('canopy','box','mat.steel',0,'','{"y":0.45,"z":0.35,"w":0.18,"d":0.4,"h":0.1,"taper":0.4,"tint":"#3b5a78","far":0}'),
  ('canopy_frame','box','mat.steel',0,'','{"y":0.45,"z":0.35,"w":0.2,"d":0.03,"h":0.09,"repeat":2,"dz":0.18,"tint":"#c3cbd6","far":0}'),
  ('canopy_glow','box','mat.light_strip',0,'','{"y":0.5,"z":0.42,"w":0.08,"d":0.12,"h":0.02,"far":0}'),
  ('spine','box','mat.steel',0,'','{"y":0.45,"z":-0.15,"w":0.1,"d":0.5,"h":0.05,"taper":0.3,"tint":"#9aa4b2","far":0}'),
  ('intakes','box','mat.steel',0,'','{"x":-0.2,"y":0.22,"z":0.25,"w":0.1,"d":0.4,"h":0.12,"taper":0.2,"repeat":2,"dx":0.4,"tint":"#8e97a3","far":0}'),
  ('intake_mouths','box','mat.iron',0,'','{"x":-0.2,"y":0.24,"z":0.45,"w":0.08,"d":0.02,"h":0.08,"repeat":2,"dx":0.4,"tint":"#1e2228","far":0}'),
  ('wing_roots','box','mat.steel',0,'','{"x":-0.25,"y":0.25,"z":-0.35,"w":0.16,"d":0.6,"h":0.06,"repeat":2,"dx":0.5,"tint":"#9aa4b2","far":0}'),
  ('wing_panels','box','mat.composite_panel',0,'','{"x":-0.45,"y":0.27,"z":-0.3,"w":0.2,"d":0.3,"h":0.01,"repeat":2,"dx":0.9,"tint":"#8994a3","far":0}'),
  ('wing_stripes','box','mat.steel',0,'','{"x":-0.6,"y":0.27,"z":-0.42,"w":0.04,"d":0.3,"h":0.012,"repeat":2,"dx":1.2,"tint":"#d2d8e0","far":0}'),
  ('cannons','cylinder','mat.iron',0,'','{"x":-0.62,"y":0.23,"z":-0.35,"w":0.05,"h":0.6,"segments":6,"pitch":90,"repeat":2,"dx":1.24,"tint":"#4a5260","far":0}'),
  ('muzzles','cylinder','mat.iron',0,'','{"x":-0.62,"y":0.23,"z":0.22,"w":0.07,"h":0.05,"segments":6,"pitch":90,"repeat":2,"dx":1.24,"tint":"#353b45","far":0}'),
  ('tip_light_port','box','mat.signal_red',0,'','{"x":-0.68,"y":0.24,"z":-0.45,"w":0.04,"d":0.04,"h":0.04,"far":0}'),
  ('tip_light_stbd','box','mat.light_strip',0,'','{"x":0.68,"y":0.24,"z":-0.45,"w":0.04,"d":0.04,"h":0.04,"far":0}'),
  ('tail_port','box','mat.steel',1,'','{"x":-0.14,"y":0.4,"z":-0.6,"w":0.03,"d":0.3,"h":0.18,"taper":0.5,"roll":-25,"far":0}'),
  ('tail_stbd','box','mat.steel',1,'','{"x":0.14,"y":0.4,"z":-0.6,"w":0.03,"d":0.3,"h":0.18,"taper":0.5,"roll":25,"far":0}'),
  ('strakes','box','mat.steel',0,'','{"x":-0.1,"y":0.14,"z":-0.5,"w":0.03,"d":0.4,"h":0.1,"repeat":2,"dx":0.2,"tint":"#8e97a3","far":0}'),
  ('rcs','box','mat.steel',0,'','{"x":-0.15,"y":0.35,"z":0.75,"w":0.04,"d":0.06,"h":0.04,"repeat":2,"dx":0.3,"tint":"#8e97a3","far":0}'),
  ('beacon','box','mat.signal_red',0,'','{"y":0.66,"z":-0.45,"w":0.03,"d":0.03,"h":0.03,"far":0}'),
  ('engine','cylinder','mat.iron',0,'','{"y":0.25,"z":-0.82,"w":0.24,"h":0.22,"segments":8,"pitch":90,"tint":"#4a5260","far":0}'),
  ('drive','wheel','mat.glow',0,'','{"y":0.25,"z":-0.95,"yaw":90,"w":0.2,"t":0.04,"spokes":3,"segments":8,"far":0}'),
  ('engine_bell','cylinder','mat.cast_iron',0,'','{"y":0.25,"z":-0.88,"w":0.278,"h":0.12,"segments":10,"pitch":90,"taper":0.2,"far":0}'),
  ('engine_band','cylinder','mat.steel',0,'','{"y":0.25,"z":-0.688,"w":0.254,"h":0.05,"segments":10,"pitch":90,"tint":"#9aa4b2","far":0}'),
  ('engine_throat','cylinder','mat.glow',0,'','{"y":0.25,"z":-0.895,"w":0.216,"h":0.012,"segments":10,"pitch":90,"far":0}'),
  ('engine_plume','cone','mat.glow',0,'','{"y":0.25,"z":-0.9,"w":0.16,"h":0.08,"segments":8,"pitch":-90,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.orbital_fighter_starfaring'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.orbital_fighter'), NULL, (SELECT id FROM def WHERE key='age.starfaring'), (SELECT id FROM def WHERE key='mesh.orbital_fighter_starfaring'), (SELECT id FROM def WHERE key='mat.steel'), 100);
