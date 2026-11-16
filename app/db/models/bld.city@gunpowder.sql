-- bld.city@gunpowder: Early modern city (mesh.city_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.city_gunpowder','Early modern city');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.city_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.ashlar',0,'','{"w":6.4,"d":6.4,"h":0.05,"steps":1,"tint":"#c8c0ae"}'),
  ('hall','box','mat.ochre_plaster',0,'','{"z":-0.6,"w":3.2,"d":2,"h":1.9,"windows":4,"window_y":0.2,"window_w":0.22,"window_h":0.46,"window_rows":2,"row_gap":0.86,"side_windows":1,"opening":"#2c3642"}'),
  ('hall_cornice','box','mat.ashlar',0,'','{"y":1.88,"z":-0.6,"w":3.3,"d":2.1,"h":0.09,"far":0}'),
  ('hall_roof','hip_roof','mat.slate',0,'','{"y":1.95,"z":-0.6,"w":3.2,"d":2,"pitch":40,"overhang":0.12,"t":0.1}'),
  ('hall_roof_cap','hip_roof','mat.cloth',1,'','{"y":2.416,"z":-0.6,"w":2.4,"d":1.2,"pitch":40,"overhang":0,"t":0.03}'),
  ('centre','box','mat.ashlar',0,'','{"z":0.47,"w":1.2,"d":0.16,"h":1.9,"door":0.46,"door_h":0.86,"windows":1,"window_y":1.06,"window_w":0.26,"window_h":0.52,"opening":"#2c3642"}'),
  ('pediment','box','mat.ashlar',0,'','{"y":1.9,"z":0.47,"yaw":90,"w":0.16,"d":1.3,"h":0.03,"gable":24}'),
  ('tympanum','box','mat.cloth',1,'','{"y":1.95,"z":0.56,"yaw":90,"w":0.02,"d":0.95,"h":0.01,"gable":24,"far":0}'),
  ('tower','box','mat.ashlar',0,'','{"y":2.3,"z":-0.6,"w":0.82,"d":0.82,"h":1.2}'),
  ('clock','shield','mat.canvas',0,'','{"y":3.02,"z":-0.18,"w":0.46,"t":0.03,"boss":0.05,"segments":10,"rim":"#3a3530","far":0}'),
  ('tower_cornice','box','mat.ashlar',0,'','{"y":3.5,"z":-0.6,"w":0.92,"d":0.92,"h":0.08,"far":0}'),
  ('lantern','cylinder','mat.ochre_plaster',0,'','{"y":3.58,"z":-0.6,"w":0.62,"h":0.45,"segments":8}'),
  ('dome','dome','mat.copper',0,'','{"y":4.03,"z":-0.6,"w":0.74,"h":0.5,"segments":8}'),
  ('spire','cone','mat.copper',0,'','{"y":4.5,"z":-0.6,"w":0.16,"h":0.55,"segments":6}'),
  ('vane','banner','mat.cloth',1,'','{"y":4.95,"z":-0.6,"h":0.7,"fw":0.42,"fh":0.3,"notch":0.08,"far":0}'),
  ('banners','banner','mat.cloth',1,'','{"x":-1.85,"z":0.35,"h":3,"fw":0.6,"fh":1.05,"repeat":2,"dx":3.7}'),
  ('house_sw','include','mat.lime_plaster',0,'','{"mesh":"mesh.house_early_modern","x":-2.2,"z":2.2}'),
  ('house_se','include','mat.ochre_plaster',0,'','{"mesh":"mesh.house_early_modern","x":2.2,"z":2.2}'),
  ('house_w','include','mat.lime_plaster',0,'','{"mesh":"mesh.house_early_modern","x":-2.5,"z":-1,"yaw":90}'),
  ('house_e','include','mat.lime_plaster',0,'','{"mesh":"mesh.house_early_modern","x":2.5,"z":-1,"yaw":-90}'),
  ('garden','box','mat.turf',0,'','{"z":-2.65,"w":2.4,"d":0.9,"h":0.04}'),
  ('trees','dome','mat.turf',0,'','{"x":-0.8,"z":-2.65,"w":0.55,"h":0.7,"segments":6,"repeat":3,"dx":0.8,"tint":"#6a8a50"}'),
  ('basin','cylinder','mat.ashlar',0,'','{"z":2.45,"w":0.95,"h":0.22,"segments":8,"far":0}'),
  ('water','cylinder','mat.slate',0,'','{"y":0.17,"z":2.45,"w":0.8,"h":0.04,"segments":8,"tint":"#7aa0c0","far":0}'),
  ('obelisk','box','mat.ashlar',0,'','{"y":0.2,"z":2.45,"w":0.16,"d":0.16,"h":0.9,"taper":0.5,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.city_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.city'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.city_gunpowder'), (SELECT id FROM def WHERE key='mat.ochre_plaster'), 100);
