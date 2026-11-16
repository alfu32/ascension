-- bld.stable@industrial: Industrial stable (mesh.stable_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.stable_industrial','Industrial stable');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.stable_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('barn','box','mat.red_brick',0,'','{"z":-1,"w":3.4,"d":1.2,"h":1.1,"gable":38,"door":0.5,"door_h":0.75,"windows":6,"window_w":0.2,"window_h":0.2,"window_y":0.65,"opening":"#2a221c"}'),
  ('quoins','box','mat.ashlar',0,'','{"x":-1.7,"z":-0.4,"w":0.09,"d":0.09,"h":1.1,"repeat":2,"dx":3.4,"far":0}'),
  ('roof','gable_roof','mat.slate',0,'','{"x":0,"y":1.1,"z":-1,"w":3.4,"d":1.2,"pitch":38,"overhang":0.12,"t":0.08}'),
  ('roof_cap','gable_roof','mat.cloth',1,'','{"x":0,"y":1.473,"z":-1,"w":3.64,"d":0.504,"pitch":38,"overhang":0,"t":0.03}'),
  ('vents','box','mat.sawn_timber',0,'','{"x":-0.8,"y":1.5,"z":-1,"w":0.32,"d":0.32,"h":0.26,"repeat":2,"dx":1.6,"far":0}'),
  ('vent_roofs','hip_roof','mat.slate',0,'','{"x":-0.8,"y":1.76,"z":-1,"w":0.32,"d":0.32,"pitch":40,"overhang":0.06,"t":0.03,"repeat":2,"dx":1.6}'),
  ('paddock','fence','mat.cast_iron',0,'','{"z":0.75,"w":3.5,"d":1.9,"h":0.6,"spacing":0.85,"rails":3,"ring":1,"gap":0.8,"far":0}'),
  ('horse_w','include',NULL,0,'','{"mesh":"mesh.horse","x":-0.8,"z":0.75,"yaw":70,"scale":0.9}'),
  ('horse_e','include',NULL,0,'','{"mesh":"mesh.horse","x":0.7,"z":0.95,"yaw":-120,"scale":0.9}'),
  ('trough','box','mat.cast_iron',0,'','{"x":1,"z":0.15,"w":0.9,"d":0.28,"h":0.3,"far":0}'),
  ('hay','box','mat.thatch',0,'','{"x":-1.35,"z":0.25,"w":0.55,"d":0.4,"h":0.35}'),
  ('lamp','cylinder','mat.cast_iron',0,'','{"x":-1.55,"z":-0.25,"w":0.05,"h":1.2,"segments":4,"far":0}'),
  ('lamp_glow','box','mat.glow',0,'','{"x":-1.55,"y":1.2,"z":-0.25,"w":0.12,"d":0.12,"h":0.14,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":0.55,"z":1.7,"h":2.2,"fw":0.45,"fh":0.65}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.stable_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.stable'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.stable_industrial'), (SELECT id FROM def WHERE key='mat.red_brick'), 100);
