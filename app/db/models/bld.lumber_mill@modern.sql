-- bld.lumber_mill@modern: Modern lumber mill (mesh.lumber_mill_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.lumber_mill_modern','Modern lumber mill');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.lumber_mill_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('mill','box','mat.cladding',0,'','{"x":-0.6,"y":0.05,"z":-0.95,"w":2.3,"d":1.4,"h":1.4,"gable":12,"door":0.6,"door_h":0.85,"door_x":0.6,"door_colour":"#c8ccd0","windows":1,"window_w":1,"window_h":0.2,"window_y":1,"side_windows":1,"opening":"#2b3b49"}'),
  ('mill_roof','gable_roof','mat.white_metal',0,'','{"x":-0.6,"y":1.45,"z":-0.95,"w":2.3,"d":1.4,"pitch":12,"overhang":0.08,"t":0.05}'),
  ('mill_roof_cap','gable_roof','mat.cloth',1,'','{"x":-0.6,"y":1.586,"z":-0.95,"w":2.5,"d":0.6,"pitch":12,"overhang":0,"t":0.03}'),
  ('cyclone_legs','box','mat.steel',0,'','{"x":1,"y":0.05,"z":-1.45,"w":0.06,"d":0.06,"h":1.55,"repeat":2,"dx":0.4}'),
  ('cyclone_legs_s','box','mat.steel',0,'','{"x":1,"y":0.05,"z":-1.05,"w":0.06,"d":0.06,"h":1.55,"repeat":2,"dx":0.4,"far":0}'),
  ('cyclone_cone','cone','mat.white_metal',0,'','{"x":1.2,"y":2,"z":-1.25,"w":0.62,"h":0.55,"segments":10,"pitch":180}'),
  ('cyclone','cylinder','mat.white_metal',0,'','{"x":1.2,"y":2,"z":-1.25,"w":0.62,"h":0.5,"segments":10}'),
  ('cyclone_band','cylinder','mat.cloth',1,'','{"x":1.2,"y":2.2,"z":-1.25,"w":0.64,"h":0.16,"segments":10}'),
  ('cyclone_outlet','cylinder','mat.steel',0,'','{"x":1.2,"y":2.5,"z":-1.25,"w":0.16,"h":0.3,"segments":6}'),
  ('duct_up','box','mat.white_metal',0,'','{"x":0.35,"y":1.45,"z":-1.25,"w":0.14,"d":0.14,"h":0.75}'),
  ('duct','box','mat.white_metal',0,'','{"x":0.65,"y":2.06,"z":-1.25,"w":0.6,"d":0.14,"h":0.14}'),
  ('stack','cylinder','mat.steel',0,'','{"x":1.5,"y":0.05,"z":-0.35,"w":0.2,"h":3.3,"segments":6}'),
  ('stack_band','cylinder','mat.cloth',1,'','{"x":1.5,"y":2.7,"z":-0.35,"w":0.22,"h":0.3,"segments":6}'),
  ('conveyor','box','mat.rubber',0,'','{"x":0.25,"y":0.3,"z":-0.05,"w":0.3,"d":0.9,"h":0.06,"pitch":-18}'),
  ('conveyor_legs','box','mat.steel',0,'','{"x":0.13,"y":0.05,"z":0.25,"w":0.04,"d":0.04,"h":0.3,"repeat":2,"dx":0.24,"far":0}'),
  ('logs','include',NULL,0,'','{"mesh":"mesh.log_pile","x":-0.9,"y":0.05,"z":0.85,"scale":0.85}'),
  ('planks_1','box','mat.sawn_timber',0,'','{"x":1.05,"y":0.05,"z":0.85,"w":1.1,"d":0.95,"h":0.13}'),
  ('planks_2','box','mat.sawn_timber',0,'','{"x":1.05,"y":0.22,"z":0.85,"w":1.07,"d":0.95,"h":0.13,"tint":"#f0e6d8","yaw":2}'),
  ('planks_3','box','mat.sawn_timber',0,'','{"x":1.05,"y":0.39,"z":0.85,"w":1.04,"d":0.95,"h":0.13,"yaw":-1.5}'),
  ('stickers','box','mat.oak',0,'','{"x":0.65,"y":0.18,"z":0.85,"w":0.06,"d":0.95,"h":0.04,"repeat":3,"dx":0.4,"far":0}'),
  ('forklift','box','mat.white_metal',0,'','{"x":0.3,"y":0.12,"z":1.25,"w":0.26,"d":0.42,"h":0.24,"tint":"#e0a828","far":0}'),
  ('forklift_mast','box','mat.steel',0,'','{"x":0.3,"y":0.1,"z":1.48,"w":0.22,"d":0.04,"h":0.6,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":-1.65,"y":0.05,"z":1.6,"h":2.4,"fw":0.5,"fh":0.75}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.lumber_mill_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.lumber_mill'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.lumber_mill_modern'), (SELECT id FROM def WHERE key='mat.cladding'), 100);
