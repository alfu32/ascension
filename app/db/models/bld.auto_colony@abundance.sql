-- bld.auto_colony@abundance: Automated Colony (mesh.auto_colony_abundance)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.auto_colony_abundance','Automated Colony');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.auto_colony_abundance';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('pad','plinth','mat.paving',0,'','{"w":4.5,"d":4.5,"h":0.1,"steps":1}'),
  ('hub','cylinder','mat.composite_panel',0,'','{"x":0.2,"y":0.1,"z":-0.2,"w":2.3,"h":0.62,"segments":16}'),
  ('hub_glow','cylinder','mat.light_strip',0,'','{"x":0.2,"y":0.66,"z":-0.2,"w":2.32,"h":0.03,"segments":16,"far":0}'),
  ('hub_band','cylinder','mat.white_composite',1,'','{"x":0.2,"y":0.72,"z":-0.2,"w":2.36,"h":0.22,"segments":16}'),
  ('hub_dome','dome','mat.smart_glass',0,'','{"x":0.2,"y":0.94,"z":-0.2,"w":2.2,"h":1.05,"segments":16}'),
  ('hub_cap','cylinder','mat.white_composite',0,'','{"x":0.2,"y":1.95,"z":-0.2,"w":0.5,"h":0.1,"segments":10,"far":0}'),
  ('mast','cylinder','mat.white_composite',0,'','{"x":-1.45,"y":0.1,"z":-1.45,"w":0.34,"h":3.8,"taper":0.55,"segments":8}'),
  ('mast_foot','cylinder','mat.graphite',0,'','{"x":-1.45,"y":0.1,"z":-1.45,"w":0.8,"h":0.4,"taper":0.3,"segments":8,"far":0}'),
  ('mast_band','cylinder','mat.white_composite',1,'','{"x":-1.45,"y":1.2,"z":-1.45,"w":0.34,"h":0.36,"segments":8}'),
  ('mast_ring','torus','mat.light_strip',0,'','{"x":-1.45,"y":2.1,"z":-1.45,"w":0.9,"t":0.07,"segments":12,"sides":4}'),
  ('mast_ring_2','torus','mat.light_strip',0,'','{"x":-1.45,"y":2.9,"z":-1.45,"w":0.7,"t":0.06,"segments":12,"sides":4,"far":0}'),
  ('mast_dish','dome','mat.white_composite',0,'','{"x":-1.45,"y":3.6,"z":-1.45,"yaw":45,"w":0.9,"h":0.14,"segments":10,"pitch":50,"far":0}'),
  ('mast_tip','box','mat.signal_red',0,'','{"x":-1.45,"y":3.9,"z":-1.45,"w":0.08,"d":0.08,"h":0.12,"far":0}'),
  ('drone_pad','cylinder','mat.concrete',1,'','{"x":1.35,"y":0.1,"z":1.35,"w":1.45,"h":0.08,"segments":12}'),
  ('drone_pad_ring','torus','mat.light_strip',0,'','{"x":1.35,"y":0.18,"z":1.35,"w":1.3,"t":0.05,"segments":14,"sides":4,"far":0}'),
  ('drone','box','mat.graphite',0,'','{"x":1.35,"y":0.42,"z":1.35,"w":0.5,"d":0.5,"h":0.16,"taper":0.2}'),
  ('drone_band','box','mat.white_composite',1,'','{"x":1.35,"y":0.58,"z":1.35,"w":0.36,"d":0.36,"h":0.05,"far":0}'),
  ('drone_arms','box','mat.white_composite',0,'','{"x":1.35,"y":0.5,"z":1.35,"yaw":45,"w":1.0,"d":0.07,"h":0.05,"far":0}'),
  ('drone_arms_2','box','mat.white_composite',0,'','{"x":1.35,"y":0.5,"z":1.35,"yaw":-45,"w":1.0,"d":0.07,"h":0.05,"far":0}'),
  ('rotor_ne','cylinder','mat.graphite',0,'','{"x":1.7,"y":0.56,"z":1.0,"w":0.36,"h":0.02,"segments":8,"far":0}'),
  ('rotor_nw','cylinder','mat.graphite',0,'','{"x":1.0,"y":0.56,"z":1.0,"w":0.36,"h":0.02,"segments":8,"far":0}'),
  ('rotor_se','cylinder','mat.graphite',0,'','{"x":1.7,"y":0.56,"z":1.7,"w":0.36,"h":0.02,"segments":8,"far":0}'),
  ('rotor_sw','cylinder','mat.graphite',0,'','{"x":1.0,"y":0.56,"z":1.7,"w":0.36,"h":0.02,"segments":8,"far":0}'),
  ('tanks','cylinder','mat.white_metal',0,'','{"x":-1.7,"y":0.1,"z":0.75,"w":0.72,"h":1.05,"segments":10,"repeat":2,"dz":0.95}'),
  ('tank_caps','dome','mat.white_metal',1,'','{"x":-1.7,"y":1.15,"z":0.75,"w":0.72,"h":0.3,"segments":10,"repeat":2,"dz":0.95}'),
  ('tank_bands','cylinder','mat.white_composite',1,'','{"x":-1.7,"y":0.62,"z":0.75,"w":0.74,"h":0.16,"segments":10,"repeat":2,"dz":0.95,"far":0}'),
  ('pipe','box','mat.graphite',0,'','{"x":-1.05,"y":0.3,"z":1.2,"w":0.9,"d":0.1,"h":0.1,"far":0}'),
  ('solar','box','mat.solar_panel',0,'','{"x":1.45,"y":0.55,"z":-1.45,"w":1.45,"d":1.0,"h":0.03,"pitch":24}'),
  ('solar_legs','box','mat.graphite',0,'','{"x":0.9,"y":0.1,"z":-1.45,"w":0.06,"d":0.06,"h":0.48,"repeat":2,"dx":1.1,"far":0}'),
  ('arm_base','cylinder','mat.graphite',0,'','{"x":-0.3,"y":0.1,"z":1.65,"w":0.42,"h":0.22,"segments":8,"far":0}'),
  ('arm_lower','box','mat.white_composite',0,'','{"x":-0.3,"y":0.32,"z":1.65,"w":0.14,"d":0.14,"h":0.85,"roll":25,"far":0}'),
  ('arm_upper','box','mat.white_composite',0,'','{"x":0.05,"y":1.08,"z":1.65,"w":0.12,"d":0.12,"h":0.65,"roll":-70,"far":0}'),
  ('crates','box','mat.composite_panel',0,'','{"x":0.55,"y":0.1,"z":1.9,"w":0.36,"d":0.36,"h":0.3,"repeat":2,"dx":0.42,"far":0}'),
  ('walk_glow','box','mat.light_strip',0,'','{"x":0.2,"y":0.1,"z":1.25,"w":0.06,"d":0.6,"h":0.01,"far":0}'),
  ('flag','banner','mat.cloth',1,'','{"x":-0.75,"y":0.1,"z":-2.0,"h":2.2,"fw":0.5,"fh":0.34,"pole_colour":"#e8ecec"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.auto_colony_abundance'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.auto_colony'), NULL, (SELECT id FROM def WHERE key='age.abundance'), (SELECT id FROM def WHERE key='mesh.auto_colony_abundance'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
