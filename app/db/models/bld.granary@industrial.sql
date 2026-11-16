-- bld.granary@industrial: Industrial granary (mesh.granary_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.granary_industrial','Industrial granary');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.granary_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.dressed_stone',0,'','{"w":3.6,"d":3.6,"h":0.08,"steps":1,"tint":"#a8a296"}'),
  ('silo_w','include',NULL,0,'','{"mesh":"mesh.silo_industrial","x":-1.15,"y":0.08,"z":-1.1}'),
  ('silo_c','include',NULL,0,'','{"mesh":"mesh.silo_industrial","y":0.08,"z":-1.1}'),
  ('silo_e','include',NULL,0,'','{"mesh":"mesh.silo_industrial","x":1.15,"y":0.08,"z":-1.1}'),
  ('elevator','box','mat.iron',0,'','{"x":1.2,"y":0.08,"z":0.35,"w":0.8,"d":0.8,"h":3.1,"door":0.3,"door_h":0.55,"side_windows":1,"window_y":2.4,"window_w":0.14,"window_h":0.2,"opening":"#22282f","tint":"#c8c0b0"}'),
  ('elevator_roof','gable_roof','mat.corrugated',0,'','{"x":1.2,"y":3.18,"z":0.35,"w":0.8,"d":0.8,"pitch":35,"overhang":0.06,"t":0.04}'),
  ('elevator_roof_cap','gable_roof','mat.cloth',1,'','{"x":1.2,"y":3.369,"z":0.35,"w":0.92,"d":0.4,"pitch":35,"overhang":0,"t":0.03}'),
  ('conveyor','box','mat.iron',0,'','{"x":1.15,"y":2.95,"z":-0.4,"w":0.2,"d":1.2,"h":0.16,"tint":"#c8c0b0","far":0}'),
  ('gallery','box','mat.iron',0,'','{"y":2.95,"z":-1.1,"w":2.7,"d":0.26,"h":0.18,"tint":"#c8c0b0"}'),
  ('office','include','mat.red_brick',0,'','{"mesh":"mesh.house_industrial","x":-0.85,"y":0.08,"z":0.85,"scale":0.8}'),
  ('cart_bed','box','mat.timber',0,'','{"x":0.2,"y":0.3,"z":1.2,"w":0.7,"d":0.95,"h":0.18,"far":0}'),
  ('sacks','dome','mat.canvas',0,'','{"x":0.2,"y":0.48,"z":1.2,"w":0.6,"h":0.3,"segments":8,"tint":"#d9c49a","far":0}'),
  ('cart_wheels','cylinder','mat.timber',0,'','{"x":-0.18,"y":0.28,"z":1.2,"w":0.56,"h":0.07,"segments":10,"roll":90,"repeat":2,"dx":0.83,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":-1.6,"y":0.08,"z":1.6,"h":2.4,"fw":0.5,"fh":0.75}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.granary_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.granary'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.granary_industrial'), (SELECT id FROM def WHERE key='mat.red_brick'), 100);
