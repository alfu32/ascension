-- bld.vehicle_plant@industrial: Vehicle plant (mesh.vehicle_plant_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.vehicle_plant_industrial','Vehicle plant');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.vehicle_plant_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.cinders',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('hall','box','mat.red_brick',0,'','{"z":-0.7,"w":3.4,"d":2,"h":1.9,"windows":0,"side_windows":3,"window_y":0.9,"window_w":0.3,"window_h":0.6,"opening":"#22282f"}'),
  ('doors','box','mat.iron',0,'','{"x":-0.75,"z":0.31,"w":1.15,"d":0.04,"h":1.35,"repeat":2,"dx":1.5,"tint":"#a8acb0"}'),
  ('door_frames','box','mat.cloth',1,'','{"x":-0.75,"y":1.35,"z":0.33,"w":1.25,"d":0.04,"h":0.14,"repeat":2,"dx":1.5}'),
  ('pier','box','mat.red_brick',0,'','{"x":0,"z":0.3,"w":0.32,"d":0.08,"h":1.9,"far":0}'),
  ('roof','gable_roof','mat.corrugated',0,'','{"y":1.9,"z":-0.7,"yaw":90,"w":2,"d":3.4,"pitch":24,"overhang":0.12,"t":0.05}'),
  ('roof_cap','gable_roof','mat.cloth',1,'','{"y":2.467,"z":-0.7,"yaw":90,"w":2.24,"d":1.1,"pitch":24,"overhang":0,"t":0.03}'),
  ('skylight','box','mat.glass',0,'','{"y":2.62,"z":-0.7,"w":0.5,"d":2.1,"h":0.14,"far":0}'),
  ('front_gable','box','mat.red_brick',0,'','{"y":1.9,"z":0.3,"yaw":90,"w":0.06,"d":3.4,"h":0.02,"gable":24}'),
  ('crane_rails','box','mat.cast_iron',0,'','{"x":-1.55,"y":1.55,"z":1,"w":0.12,"d":1.6,"h":0.12,"repeat":2,"dx":3.1}'),
  ('crane_posts','box','mat.cast_iron',0,'','{"x":-1.55,"z":1.65,"w":0.12,"d":0.12,"h":1.55,"repeat":2,"dx":3.1}'),
  ('crane_bridge','box','mat.cloth',1,'','{"y":1.67,"z":1.05,"w":3.2,"d":0.2,"h":0.18}'),
  ('crane_trolley','box','mat.cast_iron',0,'','{"x":-0.3,"y":1.5,"z":1.05,"w":0.3,"d":0.3,"h":0.17,"far":0}'),
  ('crane_hook','cylinder','mat.steel',0,'','{"x":-0.3,"y":0.85,"z":1.05,"w":0.03,"h":0.65,"segments":3,"far":0}'),
  ('chassis','box','mat.olive_drab',0,'','{"x":-0.3,"y":0.25,"z":1.05,"w":0.6,"d":1.2,"h":0.14}'),
  ('chassis_wheels','cylinder','mat.rubber',0,'','{"x":-0.62,"y":0.2,"z":0.6,"w":0.4,"h":0.1,"segments":8,"roll":90,"repeat":2,"dx":0.54,"far":0}'),
  ('chassis_wheels_b','cylinder','mat.rubber',0,'','{"x":-0.62,"y":0.2,"z":1.5,"w":0.4,"h":0.1,"segments":8,"roll":90,"repeat":2,"dx":0.54,"far":0}'),
  ('engine','box','mat.cast_iron',0,'','{"x":-0.3,"y":0.8,"z":1.05,"w":0.36,"d":0.4,"h":0.3}'),
  ('chimney','cylinder','mat.red_brick',0,'','{"x":1.45,"y":0,"z":-1.45,"w":0.42,"h":3.4,"taper":0.3,"segments":8}'),
  ('chimney_band','cylinder','mat.cloth',1,'','{"x":1.45,"y":2.584,"z":-1.45,"w":0.353,"h":0.374,"segments":8}'),
  ('chimney_cap','cylinder','mat.cast_iron',0,'','{"x":1.45,"y":3.4,"z":-1.45,"w":0.336,"h":0.12,"segments":8,"far":0}'),
  ('drums','cylinder','mat.rust',0,'','{"x":0.9,"z":1.3,"w":0.22,"h":0.32,"segments":6,"repeat":2,"dx":0.26,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.vehicle_plant_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.vehicle_plant'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.vehicle_plant_industrial'), (SELECT id FROM def WHERE key='mat.red_brick'), 100);
