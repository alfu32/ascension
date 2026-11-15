-- bld.city@medieval: Medieval city (mesh.city_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.city_medieval','Medieval city');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.city_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.rough_stone',0,'','{"w":6.4,"d":6.4,"h":0.05,"steps":1,"tint":"#b8b2a6"}'),
  ('hall','box','mat.rough_stone',0,'','{"z":-0.6,"w":3,"d":2,"h":1.1,"windows":3,"window_y":0.45,"window_w":0.34,"window_h":0.5,"opening":"#2a221c"}'),
  ('hall_sill','box','mat.oak',0,'','{"y":1.08,"z":-0.6,"w":3.16,"d":2.16,"h":0.07,"far":0}'),
  ('hall_upper','box','mat.daub',0,'','{"y":1.1,"z":-0.6,"w":3.1,"d":2.1,"h":0.9,"gable":50,"windows":4,"window_y":0.32,"window_w":0.16,"window_h":0.26}'),
  ('hall_posts','box','mat.oak',0,'','{"x":-1.53,"y":1.1,"z":-0.6,"w":0.07,"d":2.13,"h":0.9,"repeat":5,"dx":0.765,"far":0}'),
  ('hall_roof','gable_roof','mat.slate',0,'','{"y":2,"z":-0.6,"w":3.1,"d":2.1,"pitch":50,"overhang":0.18,"t":0.1}'),
  ('hall_roof_cap','gable_roof','mat.cloth',1,'','{"y":2.99,"z":-0.6,"w":3.5,"d":0.7,"pitch":50,"overhang":0,"t":0.03}'),
  ('belfry','box','mat.dressed_stone',0,'','{"z":0.75,"w":0.95,"d":0.95,"h":3.5,"door":0.4,"door_h":0.75,"side_windows":1,"window_y":2.6,"window_w":0.2,"window_h":0.42}'),
  ('belfry_top','box','mat.dressed_stone',0,'','{"y":3.5,"z":0.75,"w":1.07,"d":1.07,"h":0.16,"tint":"#d8d2c8"}'),
  ('spire','hip_roof','mat.slate',0,'','{"y":3.66,"z":0.75,"w":0.95,"d":0.95,"pitch":72,"overhang":0.06,"t":0.06}'),
  ('spire_tip','hip_roof','mat.cloth',1,'','{"y":4.64,"z":0.75,"w":0.34,"d":0.34,"pitch":72,"overhang":0,"t":0.03}'),
  ('vane','banner','mat.cloth',1,'','{"y":5.15,"z":0.75,"h":0.75,"fw":0.42,"fh":0.3,"notch":0.08}'),
  ('banner','banner','mat.cloth',1,'','{"x":-1.3,"z":1.55,"h":3.1,"fw":0.6,"fh":1.1,"repeat":2,"dx":2.6}'),
  ('house_sw','include',NULL,0,'','{"mesh":"mesh.house_medieval","x":-2.2,"z":2.2}'),
  ('house_se','include',NULL,0,'','{"mesh":"mesh.house_medieval","x":2.2,"z":2.2}'),
  ('house_w','include',NULL,0,'','{"mesh":"mesh.house_medieval","x":-2.45,"z":-1,"yaw":90}'),
  ('house_e','include',NULL,0,'','{"mesh":"mesh.house_medieval","x":2.45,"z":-1,"yaw":-90}'),
  ('house_n','include',NULL,0,'','{"mesh":"mesh.house_medieval","z":-2.6,"yaw":180,"scale":0.9}'),
  ('well','cylinder','mat.rough_stone',0,'','{"z":2.55,"w":0.6,"h":0.34,"segments":8}'),
  ('well_roof','gable_roof','mat.shingle',0,'','{"y":0.95,"z":2.55,"w":0.7,"d":0.6,"pitch":40,"overhang":0.06,"t":0.05}'),
  ('well_posts','box','mat.oak',0,'','{"x":-0.25,"z":2.55,"w":0.06,"d":0.06,"h":0.95,"repeat":2,"dx":0.5,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.city_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.city'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.city_medieval'), (SELECT id FROM def WHERE key='mat.rough_stone'), 100);
