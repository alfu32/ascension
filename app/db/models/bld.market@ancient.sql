-- bld.market@ancient: Early market (mesh.market_ancient)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.market_ancient','Early market');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.market_ancient';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.7,"d":3.7,"h":0.05,"steps":1}'),
  ('stall_nw','include',NULL,0,'','{"mesh":"mesh.stall","x":-0.85,"z":-1.2}'),
  ('stall_ne','include',NULL,0,'','{"mesh":"mesh.stall","x":0.85,"z":-1.2}'),
  ('stall_w','include',NULL,0,'','{"mesh":"mesh.stall","x":-1.25,"z":0.55,"yaw":90}'),
  ('stall_e','include',NULL,0,'','{"mesh":"mesh.stall","x":1.25,"z":0.55,"yaw":-90}'),
  ('mast','banner','mat.cloth',1,'','{"x":0.1,"z":0.2,"h":3.0,"fw":0.55,"fh":0.9}'),
  ('amphorae','cylinder','mat.mud_plaster',0,'','{"x":-0.3,"z":0.15,"w":0.22,"h":0.4,"segments":6,"taper":0.35,"repeat":2,"dz":0.26}'),
  ('cart_bed','box','mat.timber',0,'','{"y":0.28,"z":1.3,"w":0.6,"d":0.8,"h":0.18}'),
  ('cart_load','dome','mat.cloth',0,'','{"y":0.46,"z":1.3,"w":0.55,"h":0.25,"segments":8,"tint":"#c9b48a"}'),
  ('cart_wheels','cylinder','mat.timber',0,'','{"x":0.31,"y":0.26,"z":1.4,"w":0.52,"h":0.07,"segments":10,"roll":90}'),
  ('cart_wheel_w','cylinder','mat.timber',0,'','{"x":-0.38,"y":0.26,"z":1.4,"w":0.52,"h":0.07,"segments":10,"roll":90}'),
  ('cart_handles','box','mat.timber',0,'','{"x":-0.2,"y":0.3,"z":0.9,"w":0.05,"d":0.6,"h":0.05,"repeat":2,"dx":0.4,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.market_ancient'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.market'), NULL, (SELECT id FROM def WHERE key='age.ancient'), (SELECT id FROM def WHERE key='mesh.market_ancient'), (SELECT id FROM def WHERE key='mat.timber'), 100);
