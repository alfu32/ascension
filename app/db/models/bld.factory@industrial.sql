-- bld.factory@industrial: Factory (mesh.factory_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.factory_industrial','Factory');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.factory_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.cinders',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('hall','box','mat.red_brick',0,'','{"x":-0.15,"z":-0.45,"w":3,"d":2.4,"h":1.45,"door":0.7,"door_h":0.95,"windows":5,"window_y":0.3,"window_w":0.26,"window_h":0.6,"side_windows":3,"opening":"#22282f","door_colour":"#2a2a2c"}'),
  ('roof_slopes','box','mat.slate',0,'','{"x":-0.15,"y":1.45,"z":-1.65,"w":2.92,"d":0.05,"h":0.75,"pitch":53.13,"repeat":4,"dz":0.6}'),
  ('roof_glass','box','mat.glass',0,'','{"x":-0.15,"y":1.45,"z":-1.07,"w":2.92,"d":0.04,"h":0.45,"repeat":4,"dz":0.6}'),
  ('parapets','box','mat.cloth',1,'','{"x":-1.6,"y":1.45,"z":-0.45,"w":0.12,"d":2.4,"h":0.5,"repeat":2,"dx":2.9}'),
  ('parapet_caps','box','mat.cloth',1,'','{"x":-1.6,"y":1.95,"z":-0.45,"w":0.16,"d":2.44,"h":0.08,"repeat":2,"dx":2.9}'),
  ('sign','box','mat.cloth',1,'','{"x":-0.15,"y":1.12,"z":0.77,"w":2.6,"d":0.04,"h":0.3}'),
  ('clock','shield','mat.canvas',0,'','{"x":-0.15,"y":1.62,"z":0.8,"w":0.36,"t":0.03,"boss":0.04,"segments":10,"rim":"#2a2622","far":0}'),
  ('clock_gable','box','mat.cloth',1,'','{"x":-0.15,"y":1.45,"z":0.75,"yaw":90,"w":0.06,"d":0.8,"h":0.02,"gable":40}'),
  ('chimney','cylinder','mat.red_brick',0,'','{"x":1.45,"y":0,"z":-1.3,"w":0.55,"h":4.6,"taper":0.3,"segments":8}'),
  ('chimney_band','cylinder','mat.cloth',1,'','{"x":1.45,"y":3.496,"z":-1.3,"w":0.462,"h":0.506,"segments":8}'),
  ('chimney_cap','cylinder','mat.cast_iron',0,'','{"x":1.45,"y":4.6,"z":-1.3,"w":0.44,"h":0.12,"segments":8,"far":0}'),
  ('chimney_2','cylinder','mat.red_brick',0,'','{"x":1.45,"y":0,"z":-0.35,"w":0.46,"h":3.8,"taper":0.3,"segments":8}'),
  ('chimney_2_band','cylinder','mat.cloth',1,'','{"x":1.45,"y":2.888,"z":-0.35,"w":0.386,"h":0.418,"segments":8}'),
  ('chimney_2_cap','cylinder','mat.cast_iron',0,'','{"x":1.45,"y":3.8,"z":-0.35,"w":0.368,"h":0.12,"segments":8,"far":0}'),
  ('boiler_house','box','mat.red_brick',0,'','{"x":1.45,"z":-0.8,"w":0.5,"d":1.3,"h":0.9}'),
  ('boiler_roof','gable_roof','mat.cloth',1,'','{"x":1.45,"y":0.9,"z":-0.8,"yaw":90,"w":1.3,"d":0.5,"pitch":30,"overhang":0.04,"t":0.04}'),
  ('siding_bed','box','mat.cinders',0,'','{"x":0,"y":0,"z":1.35,"w":3.4,"d":0.5,"h":0.05}'),
  ('siding_sleepers','box','mat.oak',0,'siding_bed','{"x":-1.55,"y":0.05,"w":0.08,"d":0.44,"h":0.02,"repeat":5,"dx":0.6,"far":0}'),
  ('siding','box','mat.cast_iron',0,'siding_bed','{"y":0.07,"z":-0.11,"w":3.4,"d":0.04,"h":0.04,"repeat":2,"dz":0.22,"far":0}'),
  ('wagon','box','mat.timber',0,'','{"x":0.6,"y":0.18,"z":1.35,"w":1,"d":0.5,"h":0.38,"tint":"#8a6a50"}'),
  ('wagon_wheels','cylinder','mat.cast_iron',0,'','{"x":0.25,"y":0.15,"z":1.12,"w":0.26,"h":0.04,"segments":6,"pitch":90,"repeat":2,"dx":0.7,"far":0}'),
  ('crates','box','mat.timber',0,'','{"x":-1.2,"z":1.25,"w":0.35,"d":0.35,"h":0.35,"repeat":2,"dx":0.42,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.factory_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.factory'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.factory_industrial'), (SELECT id FROM def WHERE key='mat.red_brick'), 100);
