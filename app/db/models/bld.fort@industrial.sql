-- bld.fort@industrial: Industrial fort (mesh.fort_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.fort_industrial','Industrial fort');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.fort_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":5.6,"d":5.6,"h":0.05,"steps":1}'),
  ('scarp','wall','mat.red_brick',0,'','{"w":5.5,"d":5.5,"h":0.55,"t":0.16,"ring":1,"gap":1.1}'),
  ('rampart','wall','mat.turf',0,'','{"w":4.9,"d":4.9,"h":0.85,"t":0.75,"ring":1,"gap":1.1}'),
  ('parapet','wall','mat.cloth',1,'','{"y":0.85,"w":5.1,"d":5.1,"h":0.1,"t":0.14,"ring":1,"gap":1.1,"far":0}'),
  ('cupola_nw','dome','mat.cast_iron',1,'','{"x":-2.05,"y":0.85,"z":-2.05,"w":0.95,"h":0.5,"segments":8}'),
  ('cupola_ne','dome','mat.cast_iron',1,'','{"x":2.05,"y":0.85,"z":-2.05,"w":0.95,"h":0.5,"segments":8}'),
  ('cupola_sw','dome','mat.cast_iron',1,'','{"x":-2.05,"y":0.85,"z":2.05,"w":0.95,"h":0.5,"segments":8}'),
  ('cupola_se','dome','mat.cast_iron',1,'','{"x":2.05,"y":0.85,"z":2.05,"w":0.95,"h":0.5,"segments":8}'),
  ('guns_s','cylinder','mat.cast_iron',0,'','{"x":-2.05,"y":1.05,"z":2.3,"w":0.11,"h":0.55,"segments":6,"pitch":86,"repeat":2,"dx":4.1,"far":0}'),
  ('guns_n','cylinder','mat.cast_iron',0,'','{"x":-2.05,"y":1.05,"z":-2.3,"w":0.11,"h":0.55,"segments":6,"pitch":-86,"repeat":2,"dx":4.1,"far":0}'),
  ('gate','box','mat.red_brick',0,'','{"z":2.3,"w":1.5,"d":0.7,"h":1.35,"door":0.6,"door_h":0.9,"door_colour":"#2a221c"}'),
  ('gate_parapet','crenellation','mat.red_brick',0,'','{"y":1.35,"z":2.3,"w":1.5,"d":0.7,"h":0.18,"t":0.12,"merlon":0.25,"ring":1,"far":0}'),
  ('gate_arms','shield','mat.cloth',1,'','{"y":1.05,"z":2.66,"w":0.36,"h":0.42,"t":0.03,"boss":0,"point":0.4,"rim":"#d6cbb0","far":0}'),
  ('casemates','box','mat.red_brick',0,'','{"z":-1.05,"w":3,"d":1,"h":1.1,"windows":5,"window_y":0.2,"window_w":0.26,"window_h":0.5,"opening":"#22282f"}'),
  ('casemate_earth','box','mat.turf',0,'','{"y":1.1,"z":-1.05,"w":3,"d":1,"h":0.3,"taper":0.2}'),
  ('barracks','box','mat.red_brick',0,'','{"x":-1.25,"z":0.75,"yaw":90,"w":1.6,"d":0.8,"h":0.95,"windows":3,"window_y":0.3,"window_w":0.14,"window_h":0.3,"opening":"#22282f"}'),
  ('barracks_roof','gable_roof','mat.slate',0,'','{"x":-1.25,"y":0.95,"z":0.75,"yaw":90,"w":1.6,"d":0.8,"pitch":30,"overhang":0.08,"t":0.06}'),
  ('barracks_roof_cap','gable_roof','mat.cloth',1,'','{"x":-1.25,"y":1.149,"z":0.75,"yaw":90,"w":1.76,"d":0.352,"pitch":30,"overhang":0,"t":0.03}'),
  ('magazine','dome','mat.turf',0,'','{"x":1.25,"z":0.7,"w":1.1,"h":0.55,"segments":8}'),
  ('magazine_door','box','mat.red_brick',0,'','{"x":1.25,"z":1.18,"w":0.36,"d":0.1,"h":0.4,"far":0}'),
  ('flagstaff','banner','mat.cloth',1,'','{"x":0.3,"z":0.2,"h":3.4,"fw":1,"fh":0.7}'),
  ('searchlight','cylinder','mat.cast_iron',0,'','{"x":0.6,"y":1.4,"z":-1.05,"w":0.3,"h":0.3,"segments":8,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.fort_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.fort'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.fort_industrial'), (SELECT id FROM def WHERE key='mat.red_brick'), 100);
