-- bld.barracks@gunpowder: Early modern barracks (mesh.barracks_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.barracks_gunpowder','Early modern barracks');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.barracks_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('walls','wall','mat.brick',0,'','{"w":3.5,"d":3.5,"h":0.85,"t":0.2,"ring":1,"gap":1.2}'),
  ('coping','wall','mat.ashlar',0,'','{"y":0.85,"w":3.54,"d":3.54,"h":0.07,"t":0.26,"ring":1,"gap":1.2,"far":0}'),
  ('gate_piers','box','mat.ashlar',0,'','{"x":-0.7,"z":1.64,"w":0.3,"d":0.3,"h":1.3,"repeat":2,"dx":1.4}'),
  ('gate_balls','dome','mat.ashlar',0,'','{"x":-0.7,"y":1.3,"z":1.64,"w":0.22,"h":0.16,"segments":6,"repeat":2,"dx":1.4,"far":0}'),
  ('gate_lintel','box','mat.ashlar',0,'','{"y":1.3,"z":1.64,"w":1.7,"d":0.28,"h":0.16}'),
  ('gate_pediment','box','mat.ashlar',0,'','{"y":1.46,"z":1.64,"yaw":90,"w":0.28,"d":1.7,"h":0.02,"gable":24}'),
  ('gate_arms','shield','mat.cloth',1,'','{"y":1.6,"z":1.79,"w":0.26,"h":0.3,"t":0.03,"boss":0,"point":0.4,"rim":"#d6cbb0","far":0}'),
  ('block','box','mat.brick',0,'','{"y":0.05,"z":-1,"w":3,"d":1.1,"h":1.55,"door":0.4,"door_h":0.68,"windows":5,"window_y":0.18,"window_w":0.16,"window_h":0.32,"window_rows":2,"row_gap":0.72,"side_windows":1,"opening":"#2c3642"}'),
  ('block_quoins','box','mat.ashlar',0,'','{"x":-1.5,"y":0.05,"z":-0.45,"w":0.09,"d":0.09,"h":1.55,"repeat":2,"dx":3,"far":0}'),
  ('block_cornice','box','mat.ashlar',0,'','{"y":1.58,"z":-1,"w":3.08,"d":1.18,"h":0.07,"far":0}'),
  ('roof','hip_roof','mat.pantile',0,'','{"y":1.65,"z":-1,"w":3,"d":1.1,"pitch":40,"overhang":0.12,"t":0.08}'),
  ('roof_cap','hip_roof','mat.cloth',1,'','{"y":1.956,"z":-1,"w":2.52,"d":0.62,"pitch":40,"overhang":0,"t":0.03}'),
  ('flag','banner','mat.cloth',1,'','{"x":1.15,"z":0.45,"h":3.9,"fw":0.85,"fh":1.3}'),
  ('sentry_box','box','mat.cloth',1,'','{"x":-1.05,"z":1.25,"w":0.32,"d":0.32,"h":0.72,"door":0.18,"door_h":0.5,"door_colour":"#2a221c"}'),
  ('sentry_roof','hip_roof','mat.pantile',0,'','{"x":-1.05,"y":0.72,"z":1.25,"w":0.32,"d":0.32,"pitch":45,"overhang":0.04,"t":0.03}'),
  ('rack','fence','mat.oak',0,'','{"x":-0.9,"z":-0.05,"w":1,"h":0.5,"spacing":0.5,"rails":1,"far":0}'),
  ('muskets','cylinder','mat.iron',0,'','{"x":-1.3,"z":-0.1,"w":0.03,"h":1,"segments":4,"repeat":5,"dx":0.2,"roll":-8,"far":0}'),
  ('balls','dome','mat.iron',0,'','{"x":0.3,"z":0.2,"w":0.4,"h":0.22,"segments":6,"tint":"#606368","far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.barracks_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.barracks'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.barracks_gunpowder'), (SELECT id FROM def WHERE key='mat.brick'), 100);
