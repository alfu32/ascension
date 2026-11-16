-- bld.airfield@modern: Modern airfield (mesh.airfield_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.airfield_modern','Modern airfield');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.airfield_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('field','plinth','mat.turf',0,'','{"w":5.6,"d":5.6,"h":0.03,"steps":1,"tint":"#a8b890"}'),
  ('runway','box','mat.tarmac',0,'','{"z":1.4,"w":5.5,"d":1.3,"h":0.04}'),
  ('runway_marks','box','mat.canvas',0,'','{"x":-1.8,"y":0.04,"z":1.4,"w":0.4,"d":0.05,"h":0.01,"repeat":5,"dx":0.9,"far":0}'),
  ('piano_keys','box','mat.canvas',0,'','{"x":-2.45,"y":0.04,"z":1,"w":0.3,"d":0.08,"h":0.01,"repeat":5,"dz":0.2,"far":0}'),
  ('piano_keys_e','box','mat.canvas',0,'','{"x":2.45,"y":0.04,"z":1,"w":0.3,"d":0.08,"h":0.01,"repeat":5,"dz":0.2,"far":0}'),
  ('threshold','box','mat.cloth',1,'','{"x":-2.68,"y":0.04,"z":1.4,"w":0.12,"d":1.2,"h":0.01,"repeat":2,"dx":5.36}'),
  ('edge_lights','box','mat.glow',0,'','{"x":-2.5,"y":0.04,"z":0.78,"w":0.05,"d":0.05,"h":0.03,"repeat":9,"dx":0.625,"far":0}'),
  ('edge_lights_s','box','mat.glow',0,'','{"x":-2.5,"y":0.04,"z":2.02,"w":0.05,"d":0.05,"h":0.03,"repeat":9,"dx":0.625,"far":0}'),
  ('taxiway','box','mat.tarmac',0,'','{"x":-0.9,"z":0.45,"w":0.6,"d":0.7,"h":0.035}'),
  ('apron','box','mat.concrete',0,'','{"x":-0.9,"z":-0.45,"w":3.4,"d":1.2,"h":0.03,"tint":"#c8c4bc"}'),
  ('hangar','box','mat.cladding',0,'','{"x":-1,"y":0.03,"z":-1.85,"w":2.8,"d":1.5,"h":0.8}'),
  ('hangar_roof','cylinder','mat.cladding',0,'','{"x":-1,"y":0.83,"z":-2.6,"w":2.8,"h":1.5,"segments":16,"pitch":90,"bottom":1}'),
  ('hangar_doors','box','mat.cloth',1,'','{"x":-1,"y":0.03,"z":-1.08,"w":2.2,"d":0.04,"h":1.3}'),
  ('hangar_band','box','mat.cloth',1,'','{"x":-1,"y":1.33,"z":-1.08,"w":2,"d":0.04,"h":0.32}'),
  ('tower_base','box','mat.concrete',0,'','{"x":1.6,"y":0.03,"z":-1.6,"w":1.2,"d":0.9,"h":0.7,"door":0.3,"door_h":0.5,"door_colour":"#3a4650","windows":1,"window_w":0.8,"window_h":0.18,"window_y":0.4,"side_windows":1,"opening":"#2b3b49"}'),
  ('tower_base_roof','flat_roof','mat.cloth',1,'','{"x":1.6,"y":0.73,"z":-1.6,"w":1.2,"d":0.9,"t":0.03,"overhang":0.03}'),
  ('tower_shaft','cylinder','mat.concrete',0,'','{"x":1.6,"y":0.73,"z":-1.6,"w":0.5,"h":1.9,"segments":8}'),
  ('cab_floor','cylinder','mat.concrete',0,'','{"x":1.6,"y":2.63,"z":-1.6,"w":1.15,"h":0.1,"segments":8}'),
  ('cab','cylinder','mat.curtain_wall',0,'','{"x":1.6,"y":2.73,"z":-1.6,"w":1.05,"h":0.45,"segments":8}'),
  ('cab_roof','cylinder','mat.cloth',1,'','{"x":1.6,"y":3.18,"z":-1.6,"w":1.2,"h":0.1,"segments":8}'),
  ('cab_aerial','cylinder','mat.steel',0,'','{"x":1.6,"y":3.28,"z":-1.6,"w":0.04,"h":0.7,"segments":4}'),
  ('cab_light','box','mat.signal_red',0,'','{"x":1.6,"y":3.98,"z":-1.6,"w":0.08,"d":0.08,"h":0.08,"far":0}'),
  ('radar_base','box','mat.concrete',0,'','{"x":2.3,"y":0.03,"z":-0.35,"w":0.5,"d":0.5,"h":0.3}'),
  ('radar_mast','cylinder','mat.steel',0,'','{"x":2.3,"y":0.33,"z":-0.35,"w":0.1,"h":1.2,"segments":6}'),
  ('radar_head','box','mat.steel',0,'','{"x":2.3,"y":1.53,"z":-0.35,"w":0.16,"d":0.16,"h":0.1}'),
  ('radar','box','mat.white_metal',0,'','{"x":2.3,"y":1.63,"z":-0.35,"yaw":20,"w":0.9,"d":0.06,"h":0.32,"pitch":-12}'),
  ('sock_pole','cylinder','mat.steel',0,'','{"x":2.45,"y":0.03,"z":0.45,"w":0.05,"h":1.5,"segments":4}'),
  ('sock','cone','mat.cloth',1,'','{"x":2.45,"y":1.4,"z":0.32,"w":0.2,"h":0.6,"segments":6,"pitch":-100}'),
  ('fuel','cylinder','mat.white_metal',0,'','{"x":0.55,"y":0.25,"z":-2.35,"w":0.42,"h":0.75,"segments":8,"roll":90}'),
  ('fuel_2','cylinder','mat.white_metal',0,'','{"x":0.55,"y":0.25,"z":-1.8,"w":0.42,"h":0.75,"segments":8,"roll":90,"far":0}'),
  ('jet','cylinder','mat.white_metal',0,'','{"x":0.2,"y":0.24,"z":-0.95,"w":0.18,"h":1.1,"taper":0.3,"segments":6,"pitch":90,"tint":"#9aa2a8"}'),
  ('jet_nose','cone','mat.white_metal',0,'','{"x":0.2,"y":0.24,"z":0.15,"w":0.126,"h":0.3,"segments":6,"pitch":90,"tint":"#9aa2a8"}'),
  ('jet_wings','box','mat.white_metal',0,'','{"x":0.2,"y":0.22,"z":-0.55,"w":0.95,"d":0.42,"h":0.025,"tint":"#9aa2a8"}'),
  ('jet_fin','box','mat.cloth',1,'','{"x":0.2,"y":0.3,"z":-0.85,"w":0.025,"d":0.26,"h":0.3}'),
  ('jet_gear','box','mat.rubber',0,'','{"x":0.2,"y":0.03,"z":-0.5,"w":0.3,"d":0.06,"h":0.12,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.airfield_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.airfield'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.airfield_modern'), (SELECT id FROM def WHERE key='mat.turf'), 100);
