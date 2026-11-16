-- bld.university@industrial: Industrial university (mesh.university_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.university_industrial','Industrial university');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.university_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','plinth','mat.ashlar',0,'','{"w":3.7,"d":3.7,"h":0.12,"steps":1}'),
  ('hall','box','mat.red_brick',0,'','{"y":0.12,"z":-1.2,"w":3.5,"d":1.1,"h":1.8,"windows":6,"window_y":0.24,"window_w":0.16,"window_h":0.48,"window_rows":2,"row_gap":0.8,"side_windows":1,"opening":"#22282f"}'),
  ('hall_string','box','mat.ashlar',0,'','{"y":1,"z":-1.2,"w":3.54,"d":1.14,"h":0.05,"far":0}'),
  ('hall_roof','gable_roof','mat.slate',0,'','{"x":0,"y":1.92,"z":-1.2,"w":3.5,"d":1.1,"pitch":50,"overhang":0.08,"t":0.07}'),
  ('hall_roof_cap','gable_roof','mat.cloth',1,'','{"x":0,"y":2.448,"z":-1.2,"w":3.66,"d":0.396,"pitch":50,"overhang":0,"t":0.03}'),
  ('tower','box','mat.red_brick',0,'','{"y":0.12,"z":-0.5,"w":0.95,"d":0.75,"h":2.9,"door":0.42,"door_h":0.82,"windows":1,"window_y":1.5,"window_w":0.22,"window_h":0.6,"opening":"#22282f"}'),
  ('tower_band','box','mat.ashlar',0,'','{"y":2.9,"z":-0.5,"w":1,"d":0.8,"h":0.1}'),
  ('pinnacles','cone','mat.ashlar',0,'','{"x":-0.44,"y":3,"z":-0.15,"w":0.14,"h":0.6,"segments":4,"repeat":2,"dx":0.88}'),
  ('pinnacles_n','cone','mat.ashlar',0,'','{"x":-0.44,"y":3,"z":-0.85,"w":0.14,"h":0.6,"segments":4,"repeat":2,"dx":0.88,"far":0}'),
  ('spire','hip_roof','mat.slate',0,'','{"y":3,"z":-0.5,"w":0.85,"d":0.65,"pitch":72,"overhang":0,"t":0.06}'),
  ('spire_band','box','mat.cloth',1,'','{"y":3,"z":-0.5,"w":0.9,"d":0.7,"h":0.14}'),
  ('clock','shield','mat.canvas',0,'','{"y":2.45,"z":-0.11,"w":0.4,"t":0.03,"boss":0.04,"segments":10,"rim":"#2a2622","far":0}'),
  ('wing_w','box','mat.red_brick',0,'','{"x":-1.35,"y":0.12,"z":0.3,"yaw":90,"w":1.9,"d":0.85,"h":1.45,"windows":4,"window_y":0.24,"window_w":0.15,"window_h":0.36,"window_rows":2,"row_gap":0.62,"opening":"#22282f"}'),
  ('wing_e','box','mat.red_brick',0,'','{"x":1.35,"y":0.12,"z":0.3,"yaw":-90,"w":1.9,"d":0.85,"h":1.45,"windows":4,"window_y":0.24,"window_w":0.15,"window_h":0.36,"window_rows":2,"row_gap":0.62,"opening":"#22282f"}'),
  ('wing_roof_w','gable_roof','mat.slate',0,'','{"x":-1.35,"y":1.57,"z":0.3,"yaw":90,"w":1.9,"d":0.85,"pitch":45,"overhang":0.06,"t":0.06}'),
  ('wing_roof_w_cap','gable_roof','mat.cloth',1,'','{"x":-1.35,"y":1.91,"z":0.3,"yaw":90,"w":2.02,"d":0.34,"pitch":45,"overhang":0,"t":0.03}'),
  ('wing_roof_e','gable_roof','mat.slate',0,'','{"x":1.35,"y":1.57,"z":0.3,"yaw":90,"w":1.9,"d":0.85,"pitch":45,"overhang":0.06,"t":0.06}'),
  ('wing_roof_e_cap','gable_roof','mat.cloth',1,'','{"x":1.35,"y":1.91,"z":0.3,"yaw":90,"w":2.02,"d":0.34,"pitch":45,"overhang":0,"t":0.03}'),
  ('observatory','cylinder','mat.red_brick',0,'','{"x":1.35,"y":1.57,"z":1,"w":0.5,"h":0.5,"segments":8}'),
  ('observatory_dome','dome','mat.copper',0,'','{"x":1.35,"y":2.07,"z":1,"w":0.56,"h":0.32,"segments":8}'),
  ('railing','fence','mat.cast_iron',0,'','{"x":0,"y":0.12,"z":1.7,"w":3,"h":0.4,"spacing":0.12,"rails":1,"far":0}'),
  ('gate_piers','box','mat.ashlar',0,'','{"x":-0.5,"y":0.12,"z":1.7,"w":0.22,"d":0.22,"h":0.9,"repeat":2,"dx":1}'),
  ('lawn','box','mat.turf',0,'','{"y":0.12,"z":0.55,"w":1.6,"d":1.6,"h":0.02}'),
  ('banners','banner','mat.cloth',1,'','{"x":-0.95,"y":0.12,"z":1.45,"h":2.4,"fw":0.42,"fh":0.8,"repeat":2,"dx":1.9}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.university_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.university'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.university_industrial'), (SELECT id FROM def WHERE key='mat.red_brick'), 100);
