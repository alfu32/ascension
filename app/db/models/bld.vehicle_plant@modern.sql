-- bld.vehicle_plant@modern: Modern vehicle plant (mesh.vehicle_plant_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.vehicle_plant_modern','Modern vehicle plant');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.vehicle_plant_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.tarmac',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('hall','box','mat.cladding',0,'','{"y":0.05,"z":-0.7,"w":3.4,"d":2,"h":2,"gable":10,"windows":0,"side_windows":1,"window_w":1.4,"window_h":0.3,"window_y":1.4,"opening":"#2b3b49","tint":"#b8c4cc"}'),
  ('doors','box','mat.white_metal',0,'','{"x":-0.8,"y":0.05,"z":0.31,"w":1.2,"d":0.04,"h":1.45,"repeat":2,"dx":1.6}'),
  ('door_frames','box','mat.cloth',1,'','{"x":-0.8,"y":1.5,"z":0.33,"w":1.3,"d":0.04,"h":0.16,"repeat":2,"dx":1.6}'),
  ('roof','gable_roof','mat.cladding',0,'','{"y":2.05,"z":-0.7,"w":3.4,"d":2,"pitch":10,"overhang":0.1,"t":0.05}'),
  ('roof_cap','gable_roof','mat.cloth',1,'','{"y":2.2172,"z":-0.7,"w":3.64,"d":0.56,"pitch":10,"overhang":0,"t":0.03}'),
  ('vent_stack','cylinder','mat.steel',0,'','{"x":1.2,"y":0.05,"z":-1.3,"w":0.2,"h":3,"segments":6}'),
  ('crane_posts','box','mat.steel',0,'','{"x":-1.55,"y":0.05,"z":1.65,"w":0.12,"d":0.12,"h":1.6,"repeat":2,"dx":3.1}'),
  ('crane_rails','box','mat.steel',0,'','{"x":-1.55,"y":1.6,"z":1,"w":0.12,"d":1.5,"h":0.12,"repeat":2,"dx":3.1}'),
  ('crane_bridge','box','mat.cloth',1,'','{"y":1.72,"z":1.05,"w":3.24,"d":0.2,"h":0.18}'),
  ('crane_trolley','box','mat.steel',0,'','{"x":-0.8,"y":1.55,"z":1.05,"w":0.3,"d":0.3,"h":0.17,"far":0}'),
  ('crane_hook','cylinder','mat.steel',0,'','{"x":-0.8,"y":1,"z":1.05,"w":0.03,"h":0.55,"segments":3,"far":0}'),
  ('engine','box','mat.cast_iron',0,'','{"x":-0.8,"y":0.75,"z":1.05,"w":0.34,"d":0.4,"h":0.26}'),
  ('chassis','box','mat.white_metal',0,'','{"x":-0.8,"y":0.2,"z":1.05,"w":0.5,"d":1,"h":0.12,"tint":"#8a8e94"}'),
  ('chassis_cab','box','mat.white_metal',0,'','{"x":-0.8,"y":0.32,"z":1.38,"w":0.48,"d":0.3,"h":0.32,"tint":"#c8a040"}'),
  ('chassis_wheels','cylinder','mat.rubber',0,'','{"x":-1.1,"y":0.17,"z":0.7,"w":0.34,"h":0.08,"segments":8,"roll":90,"repeat":2,"dx":0.52}'),
  ('chassis_wheels_b','cylinder','mat.rubber',0,'','{"x":-1.1,"y":0.17,"z":1.4,"w":0.34,"h":0.08,"segments":8,"roll":90,"repeat":2,"dx":0.52,"far":0}'),
  ('tank_hull','box','mat.olive_drab',0,'','{"x":0.75,"y":0.12,"z":1.05,"w":0.52,"d":1,"h":0.2}'),
  ('tank_tracks','box','mat.rubber',0,'','{"x":0.43,"y":0.05,"z":1.05,"w":0.13,"d":1,"h":0.22,"repeat":2,"dx":0.64}'),
  ('tank_wheels','cylinder','mat.olive_drab',0,'','{"x":0.36,"y":0.13,"z":0.69,"w":0.15,"h":0.02,"segments":6,"roll":90,"repeat":4,"dz":0.24,"far":0}'),
  ('tank_wheels_e','cylinder','mat.olive_drab',0,'','{"x":1.13,"y":0.13,"z":0.69,"w":0.15,"h":0.02,"segments":6,"roll":90,"repeat":4,"dz":0.24,"far":0}'),
  ('tank_turret','box','mat.olive_drab',0,'','{"x":0.75,"y":0.32,"z":0.98,"w":0.4,"d":0.46,"h":0.16,"taper":0.25}'),
  ('tank_gun','cylinder','mat.olive_drab',0,'','{"x":0.75,"y":0.4,"z":1.15,"w":0.06,"h":0.6,"segments":6,"pitch":90}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.vehicle_plant_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.vehicle_plant'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.vehicle_plant_modern'), (SELECT id FROM def WHERE key='mat.cladding'), 100);
