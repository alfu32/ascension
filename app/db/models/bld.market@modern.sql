-- bld.market@modern: Modern market (mesh.market_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.market_modern','Modern market');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.market_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.concrete',0,'','{"w":3.7,"d":3.7,"h":0.06,"steps":1,"tint":"#c4bfb4"}'),
  ('store','box','mat.precast',0,'','{"y":0.06,"z":-0.95,"w":3.4,"d":1.5,"h":1.35,"door":0.6,"door_h":0.75,"door_colour":"#3a4650","windows":4,"window_w":0.6,"window_h":0.72,"window_y":0.1,"side_windows":1,"opening":"#2b3b49"}'),
  ('sign_band','box','mat.cloth',1,'','{"y":0.98,"z":-0.18,"w":3.44,"d":0.06,"h":0.3}'),
  ('store_roof','flat_roof','mat.tar',0,'','{"y":1.41,"z":-0.95,"w":3.4,"d":1.5,"t":0.04,"overhang":0.03,"parapet":0.1,"parapet_t":0.06}'),
  ('plant','box','mat.white_metal',0,'','{"x":-0.8,"y":1.45,"z":-1.15,"w":0.5,"d":0.4,"h":0.22,"repeat":2,"dx":1.4,"far":0}'),
  ('canopy','flat_roof','mat.cloth',1,'','{"y":0.92,"z":-0.02,"w":3.4,"d":0.36,"t":0.04,"overhang":0}'),
  ('stall_w','include',NULL,0,'','{"mesh":"mesh.stall","x":-1.15,"y":0.06,"z":0.85,"yaw":12,"scale":0.68}'),
  ('stall_c','include',NULL,0,'','{"mesh":"mesh.stall","x":0,"y":0.06,"z":1,"scale":0.68}'),
  ('stall_e','include',NULL,0,'','{"mesh":"mesh.stall","x":1.15,"y":0.06,"z":0.85,"yaw":-12,"scale":0.68}'),
  ('pylon','box','mat.steel',0,'','{"x":1.6,"y":0.06,"z":1.6,"w":0.08,"d":0.08,"h":2}'),
  ('pylon_sign','box','mat.cloth',1,'','{"x":1.6,"y":1.7,"z":1.6,"w":0.12,"d":0.55,"h":0.5}'),
  ('planters','box','mat.concrete',0,'','{"x":-1.6,"y":0.06,"z":1.55,"w":0.36,"d":0.36,"h":0.22,"far":0}'),
  ('shrub','dome','mat.turf',0,'','{"x":-1.6,"y":0.28,"z":1.55,"w":0.36,"h":0.32,"segments":6,"tint":"#7aa060","far":0}'),
  ('trolleys','box','mat.steel',0,'','{"x":1.45,"y":0.06,"z":0.15,"w":0.25,"d":0.4,"h":0.2,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.market_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.market'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.market_modern'), (SELECT id FROM def WHERE key='mat.precast'), 100);
