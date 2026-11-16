-- bld.granary@modern: Modern granary (mesh.granary_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.granary_modern','Modern granary');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.granary_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.concrete',0,'','{"w":3.6,"d":3.6,"h":0.08,"steps":1,"tint":"#b4b0a6"}'),
  ('silo_1','include',NULL,0,'','{"mesh":"mesh.silo_modern","x":-1.35,"y":0.08,"z":-1}'),
  ('silo_2','include',NULL,0,'','{"mesh":"mesh.silo_modern","x":-0.45,"y":0.08,"z":-1}'),
  ('silo_3','include',NULL,0,'','{"mesh":"mesh.silo_modern","x":0.45,"y":0.08,"z":-1}'),
  ('silo_4','include',NULL,0,'','{"mesh":"mesh.silo_modern","x":1.35,"y":0.08,"z":-1}'),
  ('gallery','box','mat.concrete',0,'','{"y":2.8,"z":-1,"w":3.3,"d":0.42,"h":0.32,"windows":1,"window_w":2.8,"window_h":0.1,"window_y":0.12,"opening":"#2b3b49"}'),
  ('gallery_roof','flat_roof','mat.cloth',1,'','{"y":3.12,"z":-1,"w":3.3,"d":0.42,"t":0.03,"overhang":0.03}'),
  ('elevator','box','mat.precast',0,'','{"x":0.95,"y":0.08,"z":0.15,"w":0.7,"d":0.6,"h":3.6,"door":0.34,"door_h":0.6,"door_colour":"#3a4650","windows":1,"window_w":0.12,"window_h":0.4,"window_y":1.4,"window_rows":3,"row_gap":0.65,"side_windows":1,"opening":"#2b3b49"}'),
  ('elevator_crown','box','mat.cloth',1,'','{"x":0.95,"y":3.78,"z":0.15,"w":0.76,"d":0.66,"h":0.22}'),
  ('elevator_roof','flat_roof','mat.concrete',0,'','{"x":0.95,"y":4,"z":0.15,"w":0.76,"d":0.66,"t":0.05,"overhang":0}'),
  ('bridge','box','mat.steel',0,'','{"x":0.95,"y":3.15,"z":-0.5,"w":0.24,"d":0.5,"h":0.2,"far":0}'),
  ('spout','box','mat.steel',0,'','{"x":0.62,"y":1,"z":0.3,"w":0.08,"d":0.08,"h":0.9,"roll":-40,"far":0}'),
  ('weighbridge','box','mat.steel',0,'','{"x":0,"y":0.08,"z":0.95,"w":0.6,"d":1.2,"h":0.03,"far":0}'),
  ('truck','box','mat.white_metal',0,'','{"x":0,"y":0.18,"z":0.85,"w":0.4,"d":0.8,"h":0.3,"tint":"#c8c0a8"}'),
  ('truck_cab','box','mat.white_metal',0,'','{"x":0,"y":0.18,"z":1.38,"w":0.4,"d":0.28,"h":0.38,"tint":"#4a6a8a","far":0}'),
  ('truck_wheels','cylinder','mat.rubber',0,'','{"x":-0.22,"y":0.18,"z":0.65,"w":0.24,"h":0.08,"segments":6,"roll":90,"repeat":2,"dx":0.36,"far":0}'),
  ('office','include','mat.precast',0,'','{"mesh":"mesh.house_modern","x":-1.15,"y":0.08,"z":1,"scale":0.65}'),
  ('banner','banner','mat.cloth',1,'','{"x":-1.65,"y":0.08,"z":1.65,"h":2.4,"fw":0.5,"fh":0.75}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.granary_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.granary'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.granary_modern'), (SELECT id FROM def WHERE key='mat.concrete'), 100);
