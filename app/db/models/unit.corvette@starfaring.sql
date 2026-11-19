-- unit.corvette@starfaring: Corvette (mesh.corvette_starfaring)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.corvette_starfaring','Corvette');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.corvette_starfaring';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','box','mat.steel',0,'','{"y":0.2,"z":-0.05,"w":0.64,"d":2,"h":0.34,"taper":0.25,"tint":"#a6b0bd"}'),
  ('prow','box','mat.steel',0,'','{"y":0.22,"z":1.17,"w":0.5,"d":0.5,"h":0.27,"taper":0.75,"tint":"#c3cbd6"}'),
  ('spine','box','mat.steel',1,'','{"y":0.54,"z":-0.25,"w":0.16,"d":1.2,"h":0.05}'),
  ('sponsons','box','mat.steel',0,'','{"x":-0.42,"y":0.2,"z":-0.35,"w":0.22,"d":1.2,"h":0.24,"taper":0.2,"repeat":2,"dx":0.84,"tint":"#8e97a3"}'),
  ('stripes','box','mat.steel',1,'','{"x":-0.42,"y":0.44,"z":-0.35,"w":0.09,"d":1.05,"h":0.025,"repeat":2,"dx":0.84}'),
  ('bridge','box','mat.steel',0,'','{"y":0.54,"z":0.1,"w":0.3,"d":0.34,"h":0.14,"taper":0.3,"tint":"#c3cbd6"}'),
  ('fins','box','mat.steel',1,'','{"x":-0.53,"y":0.22,"z":-0.95,"w":0.04,"d":0.4,"h":0.42,"taper":0.55,"roll":-35}'),
  ('fin_e','box','mat.steel',1,'','{"x":0.53,"y":0.22,"z":-0.95,"w":0.04,"d":0.4,"h":0.42,"taper":0.55,"roll":35}'),
  ('engine','cylinder','mat.iron',0,'','{"y":0.37,"z":-1.38,"w":0.42,"h":0.36,"segments":8,"pitch":90,"tint":"#4a5260"}'),
  ('drive','wheel','mat.glow',0,'','{"y":0.37,"z":-1.4,"yaw":90,"w":0.34,"t":0.05,"spokes":0,"segments":8}'),
  ('turret','box','mat.steel',0,'','{"y":0.54,"z":0.6,"w":0.3,"d":0.32,"h":0.13,"taper":0.25,"traverse":22,"tint":"#9aa4b2"}'),
  ('window','box','mat.light_strip',0,'','{"y":0.6,"z":0.27,"w":0.22,"d":0.02,"h":0.04,"far":0}'),
  ('side_windows','box','mat.light_strip',0,'','{"x":-0.13,"y":0.6,"z":0.1,"w":0.02,"d":0.16,"h":0.03,"repeat":2,"dx":0.26,"far":0}'),
  ('bridge_roof','box','mat.steel',0,'','{"y":0.68,"z":0.06,"w":0.18,"d":0.18,"h":0.03,"tint":"#d2d8e0","far":0}'),
  ('mast','cylinder','mat.iron',0,'','{"y":0.71,"w":0.025,"h":0.32,"segments":4,"tint":"#7c8592","far":0}'),
  ('mast_yard','box','mat.steel',0,'','{"y":0.92,"w":0.2,"d":0.02,"h":0.02,"tint":"#7c8592","far":0}'),
  ('mast_light','box','mat.signal_red',0,'','{"y":1.03,"w":0.035,"d":0.035,"h":0.035,"far":0}'),
  ('sensor','dome','mat.white_metal',0,'','{"y":0.68,"z":-0.12,"w":0.12,"h":0.06,"segments":8,"far":0}'),
  ('plates_port','box','mat.composite_panel',0,'','{"x":-0.535,"y":0.23,"z":-0.74,"w":0.03,"d":0.36,"h":0.17,"roll":5,"repeat":3,"dz":0.4,"tint":"#8994a3","far":0}'),
  ('plates_stbd','box','mat.composite_panel',0,'','{"x":0.535,"y":0.23,"z":-0.74,"w":0.03,"d":0.36,"h":0.17,"roll":-5,"repeat":3,"dz":0.4,"tint":"#8994a3","far":0}'),
  ('deck_plates','box','mat.composite_panel',0,'','{"y":0.535,"z":-0.82,"w":0.38,"d":0.3,"h":0.02,"repeat":2,"dz":0.34,"tint":"#8994a3","far":0}'),
  ('keel','box','mat.steel',0,'','{"y":0.12,"z":-0.2,"w":0.22,"d":1.6,"h":0.1,"tint":"#8e97a3","far":0}'),
  ('prow_cheeks','box','mat.steel',0,'','{"x":-0.2,"y":0.26,"z":0.9,"w":0.06,"d":0.42,"h":0.14,"taper":0.4,"repeat":2,"dx":0.4,"tint":"#9aa4b2","far":0}'),
  ('prow_sensor','cone','mat.white_metal',0,'','{"y":0.36,"z":1.32,"w":0.08,"h":0.18,"segments":6,"pitch":90,"far":0}'),
  ('intakes','box','mat.iron',0,'','{"x":-0.42,"y":0.24,"z":0.27,"w":0.16,"d":0.06,"h":0.12,"repeat":2,"dx":0.84,"tint":"#2c3138","far":0}'),
  ('thrusters','cylinder','mat.iron',0,'','{"x":-0.42,"y":0.32,"z":-1.02,"w":0.13,"h":0.1,"segments":8,"pitch":90,"repeat":2,"dx":0.84,"tint":"#4a5260","far":0}'),
  ('thruster_glow','cylinder','mat.glow',0,'','{"x":-0.42,"y":0.32,"z":-1.035,"w":0.1,"h":0.02,"segments":8,"pitch":90,"repeat":2,"dx":0.84,"far":0}'),
  ('vanes','box','mat.white_metal',0,'','{"x":-0.42,"y":0.445,"z":-0.55,"w":0.16,"d":0.035,"h":0.07,"repeat":4,"dz":0.14,"far":0}'),
  ('vanes_e','box','mat.white_metal',0,'','{"x":0.42,"y":0.445,"z":-0.55,"w":0.16,"d":0.035,"h":0.07,"repeat":4,"dz":0.14,"far":0}'),
  ('engine_bell','cylinder','mat.cast_iron',0,'','{"y":0.37,"z":-1.47,"w":0.46,"h":0.1,"segments":12,"pitch":90,"taper":0.12,"far":0}'),
  ('engine_band','cylinder','mat.steel',0,'','{"y":0.37,"z":-1.2,"w":0.45,"h":0.05,"segments":12,"pitch":90,"tint":"#9aa4b2","far":0}'),
  ('engine_glow','cylinder','mat.glow',0,'','{"y":0.37,"z":-1.48,"w":0.38,"h":0.012,"segments":12,"pitch":90,"far":0}'),
  ('fin_lights_port','box','mat.signal_red',0,'','{"x":-0.66,"y":0.405,"z":-1.03,"w":0.04,"d":0.05,"h":0.04,"far":0}'),
  ('fin_lights_stbd','box','mat.light_strip',0,'','{"x":0.66,"y":0.405,"z":-1.03,"w":0.04,"d":0.05,"h":0.04,"far":0}'),
  ('bow_lights','box','mat.light_strip',0,'','{"x":-0.2,"y":0.36,"z":1.05,"w":0.03,"d":0.04,"h":0.03,"repeat":2,"dx":0.4,"far":0}'),
  ('light_strips','box','mat.light_strip',0,'','{"x":-0.25,"y":0.47,"z":-0.35,"w":0.02,"d":0.8,"h":0.015,"repeat":2,"dx":0.5,"far":0}'),
  ('pd_base','cylinder','mat.steel',0,'','{"y":0.59,"z":-0.55,"w":0.14,"h":0.06,"segments":8,"tint":"#9aa4b2","far":0}'),
  ('pd_gun','cylinder','mat.iron',0,'','{"y":0.63,"z":-0.55,"w":0.03,"h":0.18,"segments":4,"pitch":90,"tint":"#4a5260","far":0}'),
  ('turret_ring','cylinder','mat.steel',0,'','{"y":0.52,"z":0.6,"w":0.3,"h":0.03,"segments":10,"tint":"#8e97a3","far":0}'),
  ('turret_hood','box','mat.steel',0,'turret','{"y":0.13,"z":-0.05,"w":0.16,"d":0.12,"h":0.035,"tint":"#c3cbd6","far":0}'),
  ('turret_mantlet','box','mat.iron',0,'turret','{"y":0.015,"z":0.16,"w":0.24,"d":0.05,"h":0.1,"tint":"#4a5260","far":0}'),
  ('turret_guns','cylinder','mat.iron',0,'turret','{"x":-0.055,"y":0.06,"z":0.096,"w":0.055,"h":0.42,"segments":4,"pitch":90,"repeat":2,"dx":0.11,"tint":"#4a5260","far":0}'),
  ('turret_muzzles','cylinder','mat.iron',0,'turret','{"x":-0.055,"y":0.06,"z":0.47,"w":0.075,"h":0.06,"segments":6,"pitch":90,"repeat":2,"dx":0.11,"tint":"#353b45","far":0}'),
  ('turret_flash','box','mat.flash',0,'turret','{"y":0.03,"z":0.536,"w":0.27,"d":0.12,"h":0.1,"flash":1,"far":0}'),
  ('turret_bolts','cylinder','mat.light_strip',0,'turret_flash','{"x":-0.055,"y":0.035,"z":0.14,"w":0.07,"h":0.6,"segments":3,"pitch":90,"repeat":2,"dx":0.11,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.corvette_starfaring'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.corvette'), NULL, (SELECT id FROM def WHERE key='age.starfaring'), (SELECT id FROM def WHERE key='mesh.corvette_starfaring'), (SELECT id FROM def WHERE key='mat.steel'), 100);
