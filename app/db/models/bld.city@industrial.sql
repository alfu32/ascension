-- bld.city@industrial: Industrial city (mesh.city_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.city_industrial','Industrial city');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.city_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.dressed_stone',0,'','{"w":6.4,"d":6.4,"h":0.05,"steps":1,"tint":"#b0aa9e"}'),
  ('hall','box','mat.red_brick',0,'','{"z":-0.75,"w":3.4,"d":1.9,"h":2.1,"windows":4,"window_y":0.24,"window_w":0.22,"window_h":0.5,"window_rows":2,"row_gap":0.95,"side_windows":1,"opening":"#22282f"}'),
  ('hall_cornice','box','mat.ashlar',0,'','{"y":2.08,"z":-0.75,"w":3.5,"d":2,"h":0.09,"far":0}'),
  ('hall_roof','hip_roof','mat.slate',0,'','{"x":0,"y":2.17,"z":-0.75,"w":3.4,"d":1.9,"pitch":48,"overhang":0.1,"t":0.09}'),
  ('hall_roof_cap','hip_roof','mat.cloth',1,'','{"x":0,"y":2.885,"z":-0.75,"w":2.355,"d":0.855,"pitch":48,"overhang":0,"t":0.03}'),
  ('tower','box','mat.red_brick',0,'','{"z":0.3,"w":1,"d":0.9,"h":3.6,"door":0.46,"door_h":0.86,"windows":1,"window_y":1.3,"window_w":0.2,"window_h":0.6,"opening":"#22282f"}'),
  ('belfry','box','mat.ashlar',0,'','{"y":3.6,"z":0.3,"w":1.06,"d":0.96,"h":0.85,"windows":1,"side_windows":0,"window_y":0.22,"window_w":0.26,"window_h":0.46,"opening":"#2a221c"}'),
  ('clock','shield','mat.canvas',0,'','{"y":3,"z":0.76,"w":0.56,"t":0.03,"boss":0.05,"segments":12,"rim":"#2a2622"}'),
  ('tower_cornice','box','mat.ashlar',0,'','{"y":4.45,"z":0.3,"w":1.16,"d":1.06,"h":0.08,"far":0}'),
  ('spire','hip_roof','mat.slate',0,'','{"y":4.53,"z":0.3,"w":1,"d":0.9,"pitch":74,"overhang":0.06,"t":0.08}'),
  ('spire_band','box','mat.cloth',1,'','{"y":4.53,"z":0.3,"w":1.12,"d":1.02,"h":0.12}'),
  ('finial','cone','mat.cast_iron',0,'','{"y":6.05,"z":0.3,"w":0.08,"h":0.5,"segments":4,"far":0}'),
  ('banners','banner','mat.cloth',1,'','{"x":-1.95,"z":0.4,"h":3.2,"fw":0.6,"fh":1.1,"repeat":2,"dx":3.9}'),
  ('house_sw','include','mat.red_brick',0,'','{"mesh":"mesh.house_industrial","x":-2.1,"z":2.25}'),
  ('house_se','include','mat.red_brick',0,'','{"mesh":"mesh.house_industrial","x":2.1,"z":2.25,"tint":"#d8c8c0"}'),
  ('house_w','include','mat.red_brick',0,'','{"mesh":"mesh.house_industrial","x":-2.55,"z":-0.8,"yaw":90}'),
  ('house_e','include','mat.red_brick',0,'','{"mesh":"mesh.house_industrial","x":2.55,"z":-0.8,"yaw":-90,"tint":"#d8c8c0"}'),
  ('rails_bed','box','mat.cinders',0,'','{"x":0,"y":0,"z":-2.75,"w":6.2,"d":0.5,"h":0.05}'),
  ('rails_sleepers','box','mat.oak',0,'rails_bed','{"x":-2.95,"y":0.05,"w":0.08,"d":0.44,"h":0.02,"repeat":10,"dx":0.6,"far":0}'),
  ('rails','box','mat.cast_iron',0,'rails_bed','{"y":0.07,"z":-0.11,"w":6.2,"d":0.04,"h":0.04,"repeat":2,"dz":0.22,"far":0}'),
  ('platform','box','mat.ashlar',0,'','{"z":-2.2,"w":3.6,"d":0.5,"h":0.18,"tint":"#c0b8a8","far":0}'),
  ('canopy_posts','column_row','mat.cast_iron',0,'','{"x":0,"y":0.18,"z":-2.2,"count":3,"radius":0.04,"spacing":1.5,"h":0.9,"segments":5,"capital":0,"far":0}'),
  ('canopy','flat_roof','mat.glass',0,'','{"y":1.08,"z":-2.2,"w":3.6,"d":0.7,"t":0.05,"overhang":0,"far":0}'),
  ('canopy_valance','box','mat.cloth',1,'','{"y":1,"z":-1.84,"w":3.6,"d":0.02,"h":0.1,"far":0}'),
  ('fountain','cylinder','mat.ashlar',0,'','{"z":1.75,"w":1,"h":0.22,"segments":6,"far":0}'),
  ('water','cylinder','mat.slate',0,'','{"y":0.17,"z":1.75,"w":0.86,"h":0.04,"segments":6,"tint":"#7aa0c0","far":0}'),
  ('fountain_iron','cylinder','mat.cast_iron',0,'','{"y":0.2,"z":1.75,"w":0.18,"h":0.7,"taper":0.5,"segments":6,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.city_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.city'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.city_industrial'), (SELECT id FROM def WHERE key='mat.red_brick'), 100);
