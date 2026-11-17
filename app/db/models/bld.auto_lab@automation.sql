-- bld.auto_lab@automation: Auto Lab (mesh.auto_lab_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.auto_lab_automation','Auto Lab');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.auto_lab_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','plinth','mat.paving',0,'','{"w":3.7,"d":3.7,"h":0.1,"steps":1}'),
  ('hall','box','mat.white_composite',0,'','{"x":-0.45,"y":0.1,"z":-0.95,"w":2.5,"d":1.6,"h":1.4,"windows":4,"window_w":0.34,"window_h":0.5,"window_y":0.55,"side_windows":2,"opening":"#3a5a68"}'),
  ('hall_band','box','mat.white_composite',1,'','{"x":-0.45,"y":1.5,"z":-0.95,"w":2.54,"d":1.64,"h":0.14}'),
  ('hall_roof','flat_roof','mat.composite_panel',0,'','{"x":-0.45,"y":1.64,"z":-0.95,"w":2.5,"d":1.6,"t":0.08,"parapet":0.08}'),
  ('skylights','box','mat.smart_glass',0,'','{"x":-1.3,"y":1.7,"z":-0.95,"w":0.42,"d":1.2,"h":0.2,"taper":0.5,"repeat":3,"dx":0.85,"far":0}'),
  ('glow','box','mat.light_strip',0,'','{"x":-0.45,"y":1.1,"z":-0.14,"w":2.52,"d":0.02,"h":0.04,"far":0}'),
  ('vents','cylinder','mat.white_metal',0,'','{"x":-1.4,"y":1.7,"z":-1.55,"w":0.18,"h":0.75,"segments":6,"repeat":3,"dx":0.45,"far":0}'),
  ('vent_caps','cone','mat.graphite',0,'','{"x":-1.4,"y":2.45,"z":-1.55,"w":0.26,"h":0.12,"segments":6,"repeat":3,"dx":0.45,"far":0}'),
  ('ring','torus','mat.white_composite',1,'','{"x":0.2,"y":0.1,"z":0.8,"w":2.0,"t":0.26,"segments":24,"sides":6}'),
  ('ring_glow','torus','mat.light_strip',0,'','{"x":0.2,"y":0.34,"z":0.8,"w":2.0,"t":0.06,"segments":24,"sides":4,"far":0}'),
  ('ring_magnets','box','mat.white_composite',0,'','{"x":-0.8,"y":0.1,"z":0.8,"w":0.24,"d":0.4,"h":0.38,"repeat":2,"dx":2.0}'),
  ('ring_magnets_2','box','mat.white_composite',0,'','{"x":0.2,"y":0.1,"z":1.8,"w":0.4,"d":0.24,"h":0.38,"far":0}'),
  ('detector','cylinder','mat.graphite',0,'','{"x":0.2,"y":0.1,"z":0.8,"w":0.55,"h":0.45,"segments":8}'),
  ('detector_light','cylinder','mat.light_strip',0,'','{"x":0.2,"y":0.55,"z":0.8,"w":0.3,"h":0.05,"segments":8,"far":0}'),
  ('greenhouse','dome','mat.smart_glass',0,'','{"x":1.3,"y":0.1,"z":-1.1,"w":1.0,"h":0.85,"segments":12}'),
  ('greenhouse_rim','cylinder','mat.white_composite',0,'','{"x":1.3,"y":0.1,"z":-1.1,"w":1.03,"h":0.12,"segments":12,"far":0}'),
  ('greens','dome','mat.greens',0,'','{"x":1.3,"y":0.22,"z":-1.1,"w":0.7,"h":0.3,"segments":8,"far":0}'),
  ('mast','cylinder','mat.white_composite',0,'','{"x":1.55,"y":0.1,"z":0.3,"w":0.1,"h":1.9,"segments":5}'),
  ('mast_light','box','mat.signal_red',0,'','{"x":1.55,"y":2.0,"z":0.3,"w":0.08,"d":0.08,"h":0.08,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":-1.55,"y":0.1,"z":0.6,"h":2.2,"fw":0.45,"fh":0.62,"pole_colour":"#e8ecec"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.auto_lab_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.auto_lab'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.auto_lab_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
