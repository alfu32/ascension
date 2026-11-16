-- bld.barracks@industrial: Industrial barracks (mesh.barracks_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.barracks_industrial','Industrial barracks');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.barracks_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.cinders',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('walls','wall','mat.red_brick',0,'','{"w":3.5,"d":3.5,"h":0.8,"t":0.2,"ring":1,"gap":1.2}'),
  ('coping','wall','mat.ashlar',0,'','{"y":0.8,"w":3.54,"d":3.54,"h":0.06,"t":0.26,"ring":1,"gap":1.2,"far":0}'),
  ('gate_piers','box','mat.ashlar',0,'','{"x":-0.7,"z":1.64,"w":0.3,"d":0.3,"h":1.4,"repeat":2,"dx":1.4}'),
  ('gate_caps','box','mat.ashlar',0,'','{"x":-0.7,"y":1.4,"z":1.64,"w":0.36,"d":0.36,"h":0.08,"repeat":2,"dx":1.4,"far":0}'),
  ('gate_arch','box','mat.cast_iron',0,'','{"y":1.28,"z":1.64,"w":1.2,"d":0.05,"h":0.1}'),
  ('gate_arms','shield','mat.cloth',1,'','{"y":1.5,"z":1.68,"w":0.3,"h":0.34,"t":0.03,"boss":0,"point":0.4,"rim":"#d6cbb0","far":0}'),
  ('block','box','mat.red_brick',0,'','{"y":0.05,"z":-0.95,"w":3,"d":1.2,"h":1.75,"door":0.42,"door_h":0.7,"windows":6,"window_y":0.2,"window_w":0.16,"window_h":0.36,"window_rows":2,"row_gap":0.8,"side_windows":1,"opening":"#22282f"}'),
  ('block_string','box','mat.ashlar',0,'','{"y":0.9,"z":-0.95,"w":3.04,"d":1.24,"h":0.05,"far":0}'),
  ('block_cornice','box','mat.ashlar',0,'','{"y":1.78,"z":-0.95,"w":3.08,"d":1.28,"h":0.07,"far":0}'),
  ('roof','hip_roof','mat.slate',0,'','{"x":0,"y":1.85,"z":-0.95,"w":3,"d":1.2,"pitch":36,"overhang":0.1,"t":0.08}'),
  ('roof_cap','hip_roof','mat.cloth',1,'','{"x":0,"y":2.115,"z":-0.95,"w":2.544,"d":0.744,"pitch":36,"overhang":0,"t":0.03}'),
  ('stacks','box','mat.red_brick',0,'','{"x":-1,"y":1.85,"z":-0.95,"w":0.16,"d":0.3,"h":0.6,"repeat":2,"dx":2,"far":0}'),
  ('flag','banner','mat.cloth',1,'','{"x":1.1,"z":0.5,"h":4,"fw":0.9,"fh":1.3}'),
  ('sentry_box','box','mat.cloth',1,'','{"x":-1.05,"z":1.25,"w":0.32,"d":0.32,"h":0.75,"door":0.18,"door_h":0.5,"door_colour":"#2a221c"}'),
  ('sentry_roof','hip_roof','mat.slate',0,'','{"x":-1.05,"y":0.75,"z":1.25,"w":0.32,"d":0.32,"pitch":45,"overhang":0.04,"t":0.03}'),
  ('rack','fence','mat.oak',0,'','{"x":-0.9,"z":0,"w":1,"h":0.5,"spacing":0.5,"rails":1,"far":0}'),
  ('rifles','cylinder','mat.cast_iron',0,'','{"x":-1.3,"z":-0.05,"w":0.03,"h":0.95,"segments":4,"repeat":5,"dx":0.2,"roll":-8,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.barracks_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.barracks'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.barracks_industrial'), (SELECT id FROM def WHERE key='mat.red_brick'), 100);
