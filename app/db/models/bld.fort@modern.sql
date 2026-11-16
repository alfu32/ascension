-- bld.fort@modern: Modern fort (mesh.fort_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.fort_modern','Modern fort');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.fort_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":5.6,"d":5.6,"h":0.05,"steps":1}'),
  ('wire','fence','mat.steel',0,'','{"w":5.55,"d":5.55,"h":0.32,"spacing":0.5,"rails":3,"ring":1,"gap":1.1,"far":0}'),
  ('scarp','wall','mat.concrete',0,'','{"w":5.3,"d":5.3,"h":0.5,"t":0.14,"ring":1,"gap":1.1}'),
  ('rampart','wall','mat.turf',0,'','{"w":4.8,"d":4.8,"h":0.8,"t":0.75,"ring":1,"gap":1.1}'),
  ('sandbags','wall','mat.sandbag',0,'','{"y":0.8,"w":5.06,"d":5.06,"h":0.14,"t":0.22,"ring":1,"gap":1.1,"far":0}'),
  ('parapet','wall','mat.cloth',1,'','{"y":0.5,"w":5.32,"d":5.32,"h":0.1,"t":0.16,"ring":1,"gap":1.1,"far":0}'),
  ('pillboxes_n','cylinder','mat.concrete',0,'','{"x":-2.1,"y":0.55,"z":-2.1,"w":0.95,"h":0.6,"segments":6,"repeat":2,"dx":4.2}'),
  ('pillboxes_s','cylinder','mat.concrete',0,'','{"x":-2.1,"y":0.55,"z":2.1,"w":0.95,"h":0.6,"segments":6,"repeat":2,"dx":4.2}'),
  ('pill_slits_n','cylinder','mat.crude',0,'','{"x":-2.1,"y":0.92,"z":-2.1,"w":0.96,"h":0.06,"segments":6,"repeat":2,"dx":4.2,"far":0}'),
  ('pill_slits_s','cylinder','mat.crude',0,'','{"x":-2.1,"y":0.92,"z":2.1,"w":0.96,"h":0.06,"segments":6,"repeat":2,"dx":4.2,"far":0}'),
  ('pill_roofs_n','cylinder','mat.cloth',1,'','{"x":-2.1,"y":1.15,"z":-2.1,"w":1.1,"h":0.12,"segments":6,"repeat":2,"dx":4.2}'),
  ('pill_roofs_s','cylinder','mat.cloth',1,'','{"x":-2.1,"y":1.15,"z":2.1,"w":1.1,"h":0.12,"segments":6,"repeat":2,"dx":4.2}'),
  ('gate','box','mat.concrete',0,'','{"z":2.3,"w":1.5,"d":0.7,"h":1.15,"door":0.75,"door_h":0.85,"door_colour":"#3a3e44"}'),
  ('gate_boom','box','mat.cloth',1,'','{"x":0.1,"y":0.62,"z":2.72,"w":0.9,"d":0.05,"h":0.06}'),
  ('command','box','mat.concrete',0,'','{"y":0.05,"z":-1.05,"w":2.4,"d":1.1,"h":0.85,"door":0.4,"door_h":0.6,"door_colour":"#3a3e44","windows":4,"window_w":0.3,"window_h":0.08,"window_y":0.55,"opening":"#15181b"}'),
  ('command_earth','box','mat.turf',0,'','{"y":0.9,"z":-1.05,"w":2.4,"d":1.1,"h":0.25,"taper":0.25}'),
  ('antenna','cylinder','mat.steel',0,'','{"x":0.8,"y":1.15,"z":-1.1,"w":0.05,"h":1.6,"segments":4,"far":0}'),
  ('antenna_light','box','mat.signal_red',0,'','{"x":0.8,"y":2.75,"z":-1.1,"w":0.07,"d":0.07,"h":0.07,"far":0}'),
  ('searchlight','cylinder','mat.white_metal',0,'','{"x":-0.7,"y":1.15,"z":-1,"w":0.28,"h":0.26,"segments":8,"far":0}'),
  ('hut_w','cylinder','mat.cladding',0,'','{"x":-1.3,"y":0.05,"z":-0.2,"w":0.8,"h":1.45,"segments":10,"pitch":90,"bottom":1,"tint":"#a8b098","far":0}'),
  ('hut_e','cylinder','mat.cladding',0,'','{"x":1.3,"y":0.05,"z":-0.2,"w":0.8,"h":1.45,"segments":10,"pitch":90,"bottom":1,"tint":"#a8b098","far":0}'),
  ('aa_pit','cylinder','mat.sandbag',0,'','{"x":0,"y":0.05,"z":0.95,"w":0.9,"h":0.22,"segments":8,"far":0}'),
  ('aa_mount','box','mat.olive_drab',0,'','{"x":0,"y":0.1,"z":0.95,"w":0.3,"d":0.3,"h":0.3}'),
  ('aa_guns','cylinder','mat.cast_iron',0,'aa_mount','{"x":-0.06,"y":0.24,"w":0.05,"h":0.7,"segments":4,"pitch":35,"repeat":2,"dx":0.12}'),
  ('flag','banner','mat.cloth',1,'','{"x":-0.45,"y":0.05,"z":0.55,"h":3.2,"fw":0.9,"fh":0.6}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.fort_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.fort'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.fort_modern'), (SELECT id FROM def WHERE key='mat.concrete'), 100);
