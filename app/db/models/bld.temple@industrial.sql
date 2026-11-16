-- bld.temple@industrial: Industrial temple (mesh.temple_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.temple_industrial','Industrial temple');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.temple_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','plinth','mat.ashlar',0,'','{"w":3.7,"d":3.7,"h":0.45,"steps":3,"inset":0.1}'),
  ('nave','box','mat.red_brick',0,'','{"y":0.45,"z":-0.35,"yaw":90,"w":2.7,"d":2.1,"h":1.9,"gable":34,"windows":3,"side_windows":1,"window_y":0.5,"window_w":0.2,"window_h":0.85,"opening":"#22282f"}'),
  ('buttresses','box','mat.ashlar',0,'','{"x":-1.08,"y":0.45,"z":-1.6,"w":0.12,"d":0.14,"h":1.6,"repeat":4,"dz":0.84,"far":0}'),
  ('buttresses_e','box','mat.ashlar',0,'','{"x":1.08,"y":0.45,"z":-1.6,"w":0.12,"d":0.14,"h":1.6,"repeat":4,"dz":0.84,"far":0}'),
  ('nave_roof','gable_roof','mat.slate',0,'','{"x":0,"y":2.35,"z":-0.35,"yaw":90,"w":2.7,"d":2.1,"pitch":40,"overhang":0.1,"t":0.08}'),
  ('nave_roof_cap','gable_roof','mat.cloth',1,'','{"x":0,"y":3.018,"z":-0.35,"yaw":90,"w":2.9,"d":0.756,"pitch":40,"overhang":0,"t":0.03}'),
  ('rose','shield','mat.glass',0,'','{"y":2.5,"z":-1.72,"w":0.6,"t":0.02,"boss":0.05,"segments":10,"rim":"#d6cbb0","far":0}'),
  ('tower','box','mat.red_brick',0,'','{"y":2.3,"z":0.45,"w":0.82,"d":0.82,"h":1.2,"windows":1,"side_windows":1,"window_y":0.5,"window_w":0.16,"window_h":0.4,"opening":"#22282f"}'),
  ('tower_band','box','mat.ashlar',0,'','{"y":3.5,"z":0.45,"w":0.9,"d":0.9,"h":0.08,"far":0}'),
  ('belfry','box','mat.ashlar',0,'','{"y":3.58,"z":0.45,"w":0.66,"d":0.66,"h":0.62,"windows":1,"side_windows":1,"window_y":0.14,"window_w":0.2,"window_h":0.38,"opening":"#2a221c"}'),
  ('pinnacles','cone','mat.ashlar',0,'','{"x":-0.38,"y":4.2,"z":0.07,"w":0.12,"h":0.45,"segments":4,"repeat":2,"dx":0.76,"far":0}'),
  ('spire','cone','mat.slate',0,'','{"y":4.2,"z":0.45,"w":0.62,"h":1.7,"segments":8}'),
  ('spire_band','cylinder','mat.cloth',1,'','{"y":4.2,"z":0.45,"w":0.6,"h":0.32,"segments":8,"taper":0.18}'),
  ('finial','cone','mat.cast_iron',0,'','{"y":5.85,"z":0.45,"w":0.06,"h":0.35,"segments":4,"far":0}'),
  ('columns','column_row','mat.ashlar',0,'','{"y":0.45,"z":1.42,"count":4,"radius":0.11,"spacing":0.56,"h":1.7,"segments":8,"capital":1}'),
  ('entablature','box','mat.ashlar',0,'','{"y":2.15,"z":1.2,"w":2.2,"d":0.62,"h":0.2}'),
  ('pediment','box','mat.ashlar',0,'','{"y":2.35,"z":1.2,"yaw":90,"w":0.62,"d":2.2,"h":0.02,"gable":26}'),
  ('portico_roof','gable_roof','mat.cloth',1,'','{"y":2.37,"z":1.2,"yaw":90,"w":0.62,"d":2.2,"pitch":26,"overhang":0.05,"t":0.05}'),
  ('tympanum','box','mat.cloth',1,'','{"y":2.39,"z":1.52,"yaw":90,"w":0.02,"d":1.7,"h":0.01,"gable":26,"far":0}'),
  ('portal','box','mat.oak',0,'','{"y":0.45,"z":1.02,"w":0.6,"d":0.04,"h":1.05,"tint":"#5a3d26","far":0}'),
  ('railings','fence','mat.cast_iron',0,'','{"y":0,"z":1.75,"w":3.6,"h":0.35,"spacing":0.14,"rails":1,"gap":1,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.temple_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.temple'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.temple_industrial'), (SELECT id FROM def WHERE key='mat.red_brick'), 100);
