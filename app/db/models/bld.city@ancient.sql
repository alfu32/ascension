-- bld.city@ancient: Early city (mesh.city_ancient)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.city_ancient','Early city');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.city_ancient';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":6.4,"d":6.4,"h":0.05,"steps":1}'),
  ('hall_base','plinth','mat.fieldstone',0,'','{"z":-0.55,"w":3.5,"d":2.8,"h":0.36,"steps":2,"inset":0.14}'),
  ('hall','box','mat.mud_brick',0,'','{"y":0.36,"z":-0.55,"w":2.8,"d":2.1,"h":1.75,"gable":40,"door":0.62,"door_h":1.05,"windows":4,"side_windows":2,"window_y":1.0,"window_w":0.2,"window_h":0.3}'),
  ('hall_roof','gable_roof','mat.thatch',0,'','{"y":2.11,"z":-0.55,"w":2.8,"d":2.1,"pitch":40,"overhang":0.32,"t":0.13}'),
  ('hall_ridge','box','mat.cloth',1,'','{"y":2.98,"z":-0.55,"w":3.5,"d":0.26,"h":0.12}'),
  ('porch','awning','mat.cloth',1,'','{"z":0.95,"w":1.7,"d":0.8,"h":1.55,"rise":0.4}'),
  ('banner','banner','mat.cloth',1,'','{"x":-1.25,"z":1.55,"h":3.3,"fw":0.62,"fh":1.15,"repeat":2,"dx":2.5}'),
  ('house_sw','include',NULL,0,'','{"mesh":"mesh.house_ancient","x":-2.2,"z":2.2}'),
  ('house_se','include',NULL,0,'','{"mesh":"mesh.house_ancient","x":2.2,"z":2.2}'),
  ('house_w','include',NULL,0,'','{"mesh":"mesh.house_ancient","x":-2.45,"z":-1.0,"yaw":90}'),
  ('house_e','include',NULL,0,'','{"mesh":"mesh.house_ancient","x":2.45,"z":-1.0,"yaw":-90}'),
  ('house_n','include',NULL,0,'','{"mesh":"mesh.house_ancient","z":-2.6,"yaw":180,"scale":0.9}'),
  ('well','cylinder','mat.fieldstone',0,'','{"z":2.55,"w":0.55,"h":0.32,"segments":8}'),
  ('jars','cylinder','mat.mud_plaster',0,'','{"x":-0.95,"z":1.0,"w":0.2,"h":0.26,"segments":6,"taper":0.25,"repeat":3,"dz":0.26}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.city_ancient'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.city'), NULL, (SELECT id FROM def WHERE key='age.ancient'), (SELECT id FROM def WHERE key='mesh.city_ancient'), (SELECT id FROM def WHERE key='mat.mud_plaster'), 100);
