-- bld.dock@industrial: Industrial dock (mesh.dock_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.dock_industrial','Industrial dock');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.dock_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.cinders',0,'','{"w":3.6,"d":3.6,"h":0.04,"steps":1}'),
  ('warehouse','box','mat.red_brick',0,'','{"x":-1,"z":-0.95,"w":1.55,"d":1.6,"h":1.55,"door":0.45,"door_h":0.7,"windows":2,"window_y":0.2,"window_w":0.16,"window_h":0.3,"window_rows":2,"row_gap":0.7,"side_windows":2,"opening":"#22282f"}'),
  ('warehouse_roof','gable_roof','mat.slate',0,'','{"x":-1,"y":1.55,"z":-0.95,"w":1.55,"d":1.6,"pitch":30,"overhang":0.1,"t":0.07}'),
  ('warehouse_roof_cap','gable_roof','mat.cloth',1,'','{"x":-1,"y":1.862,"z":-0.95,"w":1.75,"d":0.8,"pitch":30,"overhang":0,"t":0.03}'),
  ('hoist','box','mat.cast_iron',0,'','{"x":-1,"y":1.42,"z":-0.05,"w":0.08,"d":0.5,"h":0.08,"far":0}'),
  ('steamer','hull','mat.iron',0,'','{"x":-0.9,"y":0.12,"z":0.95,"yaw":0,"w":1.6,"d":0.56,"h":0.4,"sheer":0.1,"segments":6,"tint":"#6a6e74"}'),
  ('steamer_house','box','mat.lime_plaster',0,'steamer','{"x":-0.15,"y":0.36,"w":0.5,"d":0.32,"h":0.22,"side_windows":2,"window_y":0.08,"window_w":0.06,"window_h":0.06,"opening":"#22282f"}'),
  ('steamer_funnel','cylinder','mat.cast_iron',0,'steamer','{"x":0.2,"y":0.4,"w":0.16,"h":0.7,"segments":6}'),
  ('steamer_band','cylinder','mat.cloth',1,'steamer_funnel','{"y":0.48,"w":0.17,"h":0.12,"segments":6}'),
  ('quay','box','mat.ashlar',0,'','{"x":0.95,"w":1.2,"d":3.5,"h":0.42,"tint":"#bdb4a0"}'),
  ('quay_edge','box','mat.ashlar',0,'','{"x":1.5,"y":0.42,"w":0.12,"d":3.5,"h":0.06,"tint":"#e8e0cc","far":0}'),
  ('bollards','cylinder','mat.cast_iron',0,'','{"x":1.38,"y":0.42,"z":-0.4,"w":0.1,"h":0.16,"segments":6,"repeat":3,"dz":0.75,"far":0}'),
  ('rails_bed','box','mat.cinders',0,'','{"x":0.75,"y":0.42,"z":0.5,"yaw":90,"w":2.4,"d":0.5,"h":0.05}'),
  ('rails_sleepers','box','mat.oak',0,'rails_bed','{"x":-1.05,"y":0.05,"w":0.08,"d":0.44,"h":0.02,"repeat":4,"dx":0.6,"far":0}'),
  ('rails','box','mat.cast_iron',0,'rails_bed','{"y":0.07,"z":-0.11,"w":2.4,"d":0.04,"h":0.04,"repeat":2,"dz":0.22,"far":0}'),
  ('crane_base','box','mat.cast_iron',0,'','{"x":0.95,"y":0.42,"z":-1.15,"w":0.6,"d":0.6,"h":0.3}'),
  ('crane_cab','box','mat.cloth',1,'','{"x":0.95,"y":0.72,"z":-1.15,"w":0.55,"d":0.5,"h":0.5,"windows":1,"window_y":0.2,"window_w":0.2,"window_h":0.16,"opening":"#22282f"}'),
  ('crane_roof','gable_roof','mat.corrugated',0,'','{"x":0.95,"y":1.22,"z":-1.15,"w":0.55,"d":0.5,"pitch":20,"overhang":0.04,"t":0.03}'),
  ('crane_stack','cylinder','mat.cast_iron',0,'','{"x":0.78,"y":1.22,"z":-1.3,"w":0.08,"h":0.35,"segments":5,"far":0}'),
  ('jib','box','mat.cast_iron',0,'','{"x":1.12,"y":0.85,"z":-1.15,"w":0.1,"d":0.14,"h":1.25,"roll":48}'),
  ('rope','cylinder','mat.steel',0,'','{"x":2.04,"y":0.75,"z":-1.15,"w":0.02,"h":0.93,"segments":3,"far":0}'),
  ('load','box','mat.timber',0,'','{"x":2.04,"y":0.45,"z":-1.15,"w":0.32,"d":0.32,"h":0.3}'),
  ('barrels','cylinder','mat.timber',0,'','{"x":0.6,"y":0.42,"z":1.45,"w":0.24,"h":0.34,"segments":7,"repeat":2,"dx":0.3,"far":0,"tint":"#c8a888"}'),
  ('steps','plinth','mat.ashlar',0,'','{"x":0.22,"z":-0.2,"w":0.3,"d":0.6,"h":0.42,"steps":3,"inset":0.05}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.35,"y":0.42,"z":1.55,"h":2.5,"fw":0.7,"fh":0.9}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.dock_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.dock'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.dock_industrial'), (SELECT id FROM def WHERE key='mat.red_brick'), 100);
