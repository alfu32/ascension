-- bld.lumber_mill@industrial: Industrial lumber mill (mesh.lumber_mill_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.lumber_mill_industrial','Industrial lumber mill');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.lumber_mill_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('mill','box','mat.red_brick',0,'','{"x":-0.75,"z":-0.95,"w":1.9,"d":1.4,"h":1.5,"door":0.5,"door_h":0.7,"door_x":-0.4,"windows":3,"window_y":0.2,"window_w":0.16,"window_h":0.32,"window_rows":2,"row_gap":0.68,"opening":"#22282f"}'),
  ('mill_roof','gable_roof','mat.slate',0,'','{"x":-0.75,"y":1.5,"z":-0.95,"w":1.9,"d":1.4,"pitch":34,"overhang":0.12,"t":0.07}'),
  ('mill_roof_cap','gable_roof','mat.cloth',1,'','{"x":-0.75,"y":1.792,"z":-0.95,"w":2.14,"d":0.784,"pitch":34,"overhang":0,"t":0.03}'),
  ('vent','box','mat.corrugated',0,'','{"x":-0.75,"y":1.9,"z":-0.95,"w":0.9,"d":0.24,"h":0.22,"far":0}'),
  ('chimney','cylinder','mat.red_brick',0,'','{"x":0.75,"y":0,"z":-1.25,"w":0.5,"h":3.8,"taper":0.3,"segments":8}'),
  ('chimney_band','cylinder','mat.cloth',1,'','{"x":0.75,"y":2.888,"z":-1.25,"w":0.42,"h":0.418,"segments":8}'),
  ('chimney_cap','cylinder','mat.cast_iron',0,'','{"x":0.75,"y":3.8,"z":-1.25,"w":0.4,"h":0.12,"segments":8,"far":0}'),
  ('engine_house','box','mat.red_brick',0,'','{"x":0.75,"z":-0.6,"w":0.7,"d":0.6,"h":0.9}'),
  ('engine_roof','gable_roof','mat.slate',0,'','{"x":0.75,"y":0.9,"z":-0.6,"w":0.7,"d":0.6,"pitch":30,"overhang":0.05,"t":0.05}'),
  ('saw_shed','box','mat.cast_iron',0,'','{"x":-0.75,"z":0.45,"w":0.08,"d":0.08,"h":1.2,"repeat":2,"dx":1.1}'),
  ('saw_shed_roof','gable_roof','mat.corrugated',0,'','{"x":-0.2,"y":1.2,"z":0.45,"w":1.3,"d":0.7,"pitch":22,"overhang":0.08,"t":0.04}'),
  ('saw_frame','box','mat.cast_iron',0,'','{"x":-0.2,"z":0.45,"w":0.12,"d":0.3,"h":1.05}'),
  ('carriage_rails_bed','box','mat.cinders',0,'','{"x":-0.2,"y":0,"z":0.55,"yaw":90,"w":2,"d":0.5,"h":0.05}'),
  ('carriage_rails_sleepers','box','mat.oak',0,'carriage_rails_bed','{"x":-0.85,"y":0.05,"w":0.08,"d":0.44,"h":0.02,"repeat":3,"dx":0.6,"far":0}'),
  ('carriage_rails','box','mat.cast_iron',0,'carriage_rails_bed','{"y":0.07,"z":-0.11,"w":2,"d":0.04,"h":0.04,"repeat":2,"dz":0.22,"far":0}'),
  ('log','cylinder','mat.timber',0,'','{"x":-0.2,"y":0.38,"z":-0.25,"w":0.34,"h":1.6,"segments":7,"pitch":90}'),
  ('carriage','box','mat.cast_iron',0,'','{"x":-0.2,"y":0.1,"z":0.55,"w":0.4,"d":1.5,"h":0.12,"far":0}'),
  ('planks_1','box','mat.sawn_timber',0,'','{"x":1.05,"z":0.85,"w":1.1,"d":0.95,"h":0.13}'),
  ('planks_2','box','mat.sawn_timber',0,'','{"x":1.05,"y":0.17,"z":0.85,"w":1.07,"d":0.95,"h":0.13,"tint":"#f0e6d8","yaw":2}'),
  ('planks_3','box','mat.sawn_timber',0,'','{"x":1.05,"y":0.34,"z":0.85,"w":1.04,"d":0.95,"h":0.13,"yaw":-1.5}'),
  ('stickers','box','mat.oak',0,'','{"x":0.65,"y":0.13,"z":0.85,"w":0.06,"d":0.95,"h":0.04,"repeat":3,"dx":0.4,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":-1.6,"z":1.55,"h":2.4,"fw":0.5,"fh":0.75}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.lumber_mill_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.lumber_mill'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.lumber_mill_industrial'), (SELECT id FROM def WHERE key='mat.red_brick'), 100);
