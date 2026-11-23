-- bld.city@information: Freedom city (mesh.city_information)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.city_information','Freedom city');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.city_information';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.concrete',0,'','{"w":6.4,"d":6.4,"h":0.05,"steps":1,"tint":"#c0bcb2"}'),
  ('podium','box','mat.curtain_wall',0,'','{"y":0.05,"z":-0.5,"w":3.6,"d":2.2,"h":1.1,"door":0.6,"door_h":0.75,"door_colour":"#3a4650"}'),
  ('podium_band','box','mat.cloth',1,'','{"y":1.15,"z":-0.5,"w":3.66,"d":2.26,"h":0.16}'),
  ('podium_roof','flat_roof','mat.cloth',1,'','{"y":1.31,"z":-0.5,"w":3.6,"d":2.2,"t":0.05,"overhang":0.03}'),
  ('canopy','flat_roof','mat.white_metal',0,'','{"y":0.9,"z":0.95,"w":1.5,"d":0.7,"t":0.05,"overhang":0,"far":0}'),
  ('canopy_posts','cylinder','mat.steel',0,'','{"x":-0.65,"y":0.05,"z":1.2,"w":0.06,"h":0.85,"segments":5,"repeat":2,"dx":1.3,"far":0}'),
  ('tower','box','mat.curtain_wall',0,'','{"x":-0.55,"y":1.36,"z":-0.85,"w":1.5,"d":1.3,"h":5.6}'),
  ('tower_floor_1','box','mat.steel',0,'','{"x":-0.55,"y":2.16,"z":-0.85,"w":1.52,"d":1.32,"h":0.05,"far":0}'),
  ('tower_floor_2','box','mat.steel',0,'','{"x":-0.55,"y":2.96,"z":-0.85,"w":1.52,"d":1.32,"h":0.05,"far":0}'),
  ('tower_floor_3','box','mat.steel',0,'','{"x":-0.55,"y":3.76,"z":-0.85,"w":1.52,"d":1.32,"h":0.05,"far":0}'),
  ('tower_floor_4','box','mat.steel',0,'','{"x":-0.55,"y":4.56,"z":-0.85,"w":1.52,"d":1.32,"h":0.05,"far":0}'),
  ('tower_floor_5','box','mat.steel',0,'','{"x":-0.55,"y":5.36,"z":-0.85,"w":1.52,"d":1.32,"h":0.05,"far":0}'),
  ('tower_floor_6','box','mat.steel',0,'','{"x":-0.55,"y":6.16,"z":-0.85,"w":1.52,"d":1.32,"h":0.05,"far":0}'),
  ('tower_crown','box','mat.cloth',1,'','{"x":-0.55,"y":6.96,"z":-0.85,"w":1.56,"d":1.36,"h":0.3}'),
  ('tower_top','box','mat.curtain_wall',0,'','{"x":-0.55,"y":7.26,"z":-0.85,"w":1.1,"d":0.9,"h":0.5,"taper":0.3}'),
  ('spire','cylinder','mat.steel',0,'','{"x":-0.55,"y":7.76,"z":-0.85,"w":0.1,"h":1.5,"taper":0.8,"segments":6}'),
  ('spire_light','box','mat.signal_red',0,'','{"x":-0.55,"y":9.26,"z":-0.85,"w":0.08,"d":0.08,"h":0.08,"far":0}'),
  ('tower_b','box','mat.curtain_wall',0,'','{"x":0.95,"y":1.36,"z":-0.55,"w":1.2,"d":1,"h":3.6,"tint":"#a8d0c8"}'),
  ('tower_b_floor_1','box','mat.steel',0,'','{"x":0.95,"y":2.26,"z":-0.55,"w":1.22,"d":1.02,"h":0.05,"far":0}'),
  ('tower_b_floor_2','box','mat.steel',0,'','{"x":0.95,"y":3.16,"z":-0.55,"w":1.22,"d":1.02,"h":0.05,"far":0}'),
  ('tower_b_floor_3','box','mat.steel',0,'','{"x":0.95,"y":4.06,"z":-0.55,"w":1.22,"d":1.02,"h":0.05,"far":0}'),
  ('tower_b_crown','box','mat.cloth',1,'','{"x":0.95,"y":4.96,"z":-0.55,"w":1.26,"d":1.06,"h":0.22}'),
  ('tower_b_roof','flat_roof','mat.concrete',0,'','{"x":0.95,"y":5.18,"z":-0.55,"w":1.26,"d":1.06,"t":0.04,"overhang":0}'),
  ('dish_post','cylinder','mat.steel',0,'','{"x":0.75,"y":5.22,"z":-0.45,"w":0.05,"h":0.25,"segments":4,"far":0}'),
  ('dish','dome','mat.white_metal',0,'','{"x":0.75,"y":5.47,"z":-0.45,"yaw":0,"w":0.55,"h":0.154,"segments":8,"pitch":-135}'),
  ('dish_2_post','cylinder','mat.steel',0,'','{"x":1.2,"y":5.22,"z":-0.8,"w":0.05,"h":0.25,"segments":4,"far":0}'),
  ('dish_2','dome','mat.white_metal',0,'','{"x":1.2,"y":5.47,"z":-0.8,"yaw":40,"w":0.4,"h":0.112,"segments":8,"pitch":-135}'),
  ('mast','cylinder','mat.steel',0,'','{"x":1.3,"y":5.22,"z":-0.3,"w":0.04,"h":0.9,"segments":4,"far":0}'),
  ('flags','banner','mat.cloth',1,'','{"x":-0.6,"y":0.05,"z":1.55,"h":2.4,"fw":0.5,"fh":0.34,"repeat":3,"dx":0.6,"far":0}'),
  ('house_sw','include','mat.precast',0,'','{"mesh":"mesh.house_modern","x":-2.2,"y":0.05,"z":2.3}'),
  ('house_se','include','mat.precast',0,'','{"mesh":"mesh.house_modern","x":2.2,"y":0.05,"z":2.3,"tint":"#e4dcd0"}'),
  ('house_w','include','mat.precast',0,'','{"mesh":"mesh.house_modern","x":-2.6,"y":0.05,"z":-0.6,"yaw":90,"tint":"#e4dcd0"}'),
  ('house_e','include','mat.precast',0,'','{"mesh":"mesh.house_modern","x":2.6,"y":0.05,"z":-0.6,"yaw":-90}'),
  ('panels','box','mat.curtain_wall',0,'','{"x":-2.6,"y":1.66,"z":-0.6,"w":0.9,"d":1.3,"h":0.03,"tint":"#40506a","far":0}'),
  ('parking','box','mat.tarmac',0,'','{"y":0.05,"z":-2.65,"w":5.4,"d":0.9,"h":0.02,"far":0}'),
  ('parking_lines','box','mat.canvas',0,'','{"x":-2.4,"y":0.07,"z":-2.65,"w":0.03,"d":0.6,"h":0.005,"repeat":7,"dx":0.8,"far":0}'),
  ('cars','box','mat.white_metal',0,'','{"x":-2,"y":0.08,"z":-2.6,"w":0.28,"d":0.52,"h":0.13,"repeat":3,"dx":1.6,"tint":"#e0e0e0","far":0}'),
  ('cars_2','box','mat.white_metal',0,'','{"x":-1.2,"y":0.08,"z":-2.7,"w":0.28,"d":0.52,"h":0.13,"repeat":3,"dx":1.6,"tint":"#30343a","far":0}'),
  ('car_tops','box','mat.curtain_wall',0,'','{"x":-2,"y":0.21,"z":-2.6,"w":0.24,"d":0.28,"h":0.09,"repeat":6,"dx":0.8,"far":0}'),
  ('lamps','cylinder','mat.steel',0,'','{"x":-1.5,"y":0.05,"z":1.3,"w":0.04,"h":1.2,"segments":4,"repeat":2,"dx":3,"far":0}'),
  ('lamp_heads','box','mat.white_metal',0,'','{"x":-1.5,"y":1.25,"z":1.3,"w":0.22,"d":0.1,"h":0.05,"repeat":2,"dx":3,"far":0}'),
  ('trunks','cylinder','mat.timber',0,'','{"x":-0.8,"y":0.05,"z":2.55,"w":0.08,"h":0.5,"segments":4,"repeat":2,"dx":1.6,"far":0}'),
  ('trees','dome','mat.turf',0,'','{"x":-0.8,"y":0.45,"z":2.55,"w":0.62,"h":0.7,"segments":6,"repeat":2,"dx":1.6,"tint":"#7aa060","far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.city_information'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.city'), NULL, (SELECT id FROM def WHERE key='age.information'), (SELECT id FROM def WHERE key='mesh.city_information'), (SELECT id FROM def WHERE key='mat.concrete'), 100);
