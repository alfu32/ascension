-- bld.mine@modern: Modern mine (mesh.mine_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.mine_modern','Modern mine');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.mine_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.cinders',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('collar','wall','mat.concrete',0,'','{"x":-0.6,"y":0.05,"z":-0.4,"w":1.1,"d":1.1,"h":0.3,"t":0.16,"ring":1}'),
  ('pit','box','mat.rough_stone',0,'','{"x":-0.6,"y":0.06,"z":-0.4,"w":0.75,"d":0.75,"h":0.03,"tint":"#2a2622"}'),
  ('legs','box','mat.steel',0,'','{"x":-1.05,"y":0.05,"z":-0.4,"w":0.14,"d":0.16,"h":3.3,"roll":6,"tint":"#c8b8a0"}'),
  ('legs_e','box','mat.steel',0,'','{"x":-0.15,"y":0.05,"z":-0.4,"w":0.14,"d":0.16,"h":3.3,"roll":-6,"tint":"#c8b8a0"}'),
  ('stays','box','mat.steel',0,'','{"x":-0.75,"y":0.05,"z":-1.6,"w":0.12,"d":0.12,"h":3.3,"pitch":20,"roll":4,"tint":"#c8b8a0","far":0}'),
  ('stays_e','box','mat.steel',0,'','{"x":-0.45,"y":0.05,"z":-1.6,"w":0.12,"d":0.12,"h":3.3,"pitch":20,"roll":-4,"tint":"#c8b8a0","far":0}'),
  ('braces','box','mat.steel',0,'','{"x":-0.6,"y":1.05,"z":-0.4,"w":0.86,"d":0.07,"h":0.07,"far":0}'),
  ('braces_2','box','mat.steel',0,'','{"x":-0.6,"y":2.1,"z":-0.4,"w":0.76,"d":0.07,"h":0.07,"far":0}'),
  ('head','box','mat.steel',0,'','{"x":-0.6,"y":3.3,"z":-0.45,"w":0.95,"d":0.55,"h":0.14}'),
  ('sheave','shield','mat.steel',0,'','{"x":-0.6,"y":3.55,"z":-0.3,"yaw":90,"w":0.56,"t":0.04,"boss":0.06,"segments":12,"rim":"#5a5e66"}'),
  ('sheave_2','shield','mat.steel',0,'','{"x":-0.6,"y":3.55,"z":-0.6,"yaw":90,"w":0.56,"t":0.04,"boss":0.06,"segments":12,"rim":"#5a5e66","far":0}'),
  ('head_cap','flat_roof','mat.cloth',1,'','{"x":-0.6,"y":3.44,"z":-0.45,"w":1,"d":0.6,"t":0.06,"overhang":0.02}'),
  ('ropes','box','mat.steel',0,'','{"x":-0.6,"y":0.95,"z":0.75,"w":0.03,"d":0.03,"h":2.95,"pitch":-25,"far":0}'),
  ('cage','box','mat.white_metal',0,'','{"x":-0.6,"y":0.4,"z":-0.4,"w":0.4,"d":0.4,"h":0.5,"tint":"#e0a828"}'),
  ('winding_house','box','mat.precast',0,'','{"x":-0.6,"y":0.05,"z":1.2,"w":1.3,"d":0.9,"h":1,"door":0.3,"door_h":0.55,"door_colour":"#3a4650","windows":1,"window_w":0.9,"window_h":0.2,"window_y":0.62,"side_windows":1,"opening":"#2b3b49"}'),
  ('winding_fascia','box','mat.concrete',0,'','{"x":-0.6,"y":1.05,"z":1.2,"w":1.36,"d":0.96,"h":0.14}'),
  ('winding_roof','flat_roof','mat.cloth',1,'','{"x":-0.6,"y":1.19,"z":1.2,"w":1.36,"d":0.96,"t":0.04,"overhang":0}'),
  ('bin_legs','box','mat.steel',0,'','{"x":1,"y":0.05,"z":-1.45,"w":0.07,"d":0.07,"h":1.15,"repeat":2,"dx":0.45}'),
  ('bin_legs_s','box','mat.steel',0,'','{"x":1,"y":0.05,"z":-1,"w":0.07,"d":0.07,"h":1.15,"repeat":2,"dx":0.45,"far":0}'),
  ('bin_cone','cone','mat.white_metal',0,'','{"x":1.22,"y":1.2,"z":-1.22,"w":0.75,"h":0.4,"segments":10,"pitch":180}'),
  ('bin','cylinder','mat.white_metal',0,'','{"x":1.22,"y":1.2,"z":-1.22,"w":0.75,"h":0.8,"segments":10}'),
  ('bin_band','cylinder','mat.steel',0,'','{"x":1.22,"y":1.65,"z":-1.22,"w":0.77,"h":0.22,"segments":10}'),
  ('conveyor','box','mat.cladding',0,'','{"x":0.475,"y":1.55,"z":-0.8,"yaw":28.9,"roll":-22.9,"w":1.8,"d":0.22,"h":0.2}'),
  ('spoil','cone','mat.spoil',0,'','{"x":1.05,"y":0.05,"z":0.55,"w":1.5,"h":0.95,"segments":8}'),
  ('spoil_back','cone','mat.spoil',0,'','{"x":1.35,"y":0.05,"z":1.35,"w":0.9,"h":0.55,"segments":7}'),
  ('truck','box','mat.white_metal',0,'','{"x":0.3,"y":0.22,"z":1.25,"w":0.42,"d":0.85,"h":0.12,"tint":"#e0a828"}'),
  ('truck_bed','box','mat.white_metal',0,'','{"x":0.3,"y":0.34,"z":1.1,"w":0.42,"d":0.55,"h":0.22,"taper":0.1,"tint":"#e0a828","far":0}'),
  ('truck_cab','box','mat.curtain_wall',0,'','{"x":0.3,"y":0.34,"z":1.53,"w":0.36,"d":0.24,"h":0.22,"far":0}'),
  ('truck_wheels','cylinder','mat.rubber',0,'','{"x":0.06,"y":0.17,"z":0.95,"w":0.32,"h":0.1,"segments":8,"roll":90,"repeat":2,"dx":0.38,"far":0}'),
  ('truck_wheels_f','cylinder','mat.rubber',0,'','{"x":0.06,"y":0.17,"z":1.5,"w":0.32,"h":0.1,"segments":8,"roll":90,"repeat":2,"dx":0.38,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.65,"y":0.05,"z":-0.35,"h":2.4,"fw":0.5,"fh":0.75}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.mine_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.mine'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.mine_modern'), (SELECT id FROM def WHERE key='mat.steel'), 100);
