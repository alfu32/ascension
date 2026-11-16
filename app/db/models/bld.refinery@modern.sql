-- bld.refinery@modern: Modern refinery (mesh.refinery_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.refinery_modern','Modern refinery');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.refinery_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.concrete',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1,"tint":"#a8a49c"}'),
  ('column_a','cylinder','mat.white_metal',0,'','{"x":-1.2,"y":0.05,"z":-1.2,"w":0.44,"h":3.6,"segments":10}'),
  ('column_a_band','cylinder','mat.cloth',1,'','{"x":-1.2,"y":2.7,"z":-1.2,"w":0.46,"h":0.4,"segments":10}'),
  ('column_a_top','dome','mat.white_metal',0,'','{"x":-1.2,"y":3.65,"z":-1.2,"w":0.44,"h":0.16,"segments":10}'),
  ('column_a_decks','cylinder','mat.cast_iron',0,'','{"x":-1.2,"y":1.3,"z":-1.2,"w":0.62,"h":0.04,"segments":10,"far":0}'),
  ('column_a_decks_2','cylinder','mat.cast_iron',0,'','{"x":-1.2,"y":2.4,"z":-1.2,"w":0.62,"h":0.04,"segments":10,"far":0}'),
  ('column_b','cylinder','mat.white_metal',0,'','{"x":-0.55,"y":0.05,"z":-1.35,"w":0.34,"h":2.9,"segments":10}'),
  ('column_b_band','cylinder','mat.cloth',1,'','{"x":-0.55,"y":2.1,"z":-1.35,"w":0.36,"h":0.35,"segments":10}'),
  ('column_b_top','dome','mat.white_metal',0,'','{"x":-0.55,"y":2.95,"z":-1.35,"w":0.34,"h":0.13,"segments":10,"far":0}'),
  ('column_c','cylinder','mat.white_metal',0,'','{"x":-1.25,"y":0.05,"z":-0.45,"w":0.3,"h":2.3,"segments":8,"far":0}'),
  ('column_c_top','dome','mat.white_metal',0,'','{"x":-1.25,"y":2.35,"z":-0.45,"w":0.3,"h":0.12,"segments":8,"far":0}'),
  ('sphere','dome','mat.white_metal',0,'','{"x":0.55,"y":0.8,"z":-1.2,"w":0.75,"h":0.375,"segments":10}'),
  ('sphere_under','dome','mat.white_metal',0,'','{"x":0.55,"y":0.8,"z":-1.2,"w":0.75,"h":0.375,"segments":10,"pitch":180,"far":0}'),
  ('sphere_2','dome','mat.white_metal',0,'','{"x":1.35,"y":0.8,"z":-1.2,"w":0.75,"h":0.375,"segments":10}'),
  ('sphere_2_under','dome','mat.white_metal',0,'','{"x":1.35,"y":0.8,"z":-1.2,"w":0.75,"h":0.375,"segments":10,"pitch":180,"far":0}'),
  ('sphere_legs','cylinder','mat.steel',0,'','{"x":0.3,"y":0.05,"z":-1.2,"w":0.05,"h":0.75,"segments":4,"repeat":2,"dx":0.5}'),
  ('sphere_2_legs','cylinder','mat.steel',0,'','{"x":1.1,"y":0.05,"z":-1.2,"w":0.05,"h":0.75,"segments":4,"repeat":2,"dx":0.5,"far":0}'),
  ('tank','cylinder','mat.white_metal',0,'','{"x":1.1,"y":0.05,"z":0.3,"w":1.05,"h":0.6,"segments":12}'),
  ('tank_band','cylinder','mat.cloth',1,'','{"x":1.1,"y":0.42,"z":0.3,"w":1.07,"h":0.18,"segments":12}'),
  ('tank_roof','cylinder','mat.cloth',1,'','{"x":1.1,"y":0.62,"z":0.3,"w":0.98,"h":0.02,"segments":12,"far":0}'),
  ('tank_2','cylinder','mat.white_metal',0,'','{"x":0.2,"y":0.05,"z":0.55,"w":0.8,"h":0.5,"segments":10}'),
  ('tank_2_band','cylinder','mat.cloth',1,'','{"x":0.2,"y":0.35,"z":0.55,"w":0.82,"h":0.14,"segments":10}'),
  ('tank_2_roof','cone','mat.cloth',1,'','{"x":0.2,"y":0.55,"z":0.55,"w":0.82,"h":0.1,"segments":10,"far":0}'),
  ('pipe_rack','box','mat.steel',0,'','{"x":0,"y":1.05,"z":-0.55,"w":3.2,"d":0.3,"h":0.06}'),
  ('pipes','box','mat.white_metal',0,'','{"x":0,"y":1.11,"z":-0.65,"w":3.2,"d":0.06,"h":0.06,"repeat":3,"dz":0.1,"far":0}'),
  ('rack_posts','box','mat.steel',0,'','{"x":-1.5,"y":0.05,"z":-0.55,"w":0.06,"d":0.24,"h":1,"repeat":4,"dx":1,"far":0}'),
  ('heater','box','mat.precast',0,'','{"x":-0.45,"y":0.05,"z":0.35,"w":0.8,"d":0.5,"h":0.9}'),
  ('heater_stack','cylinder','mat.steel',0,'','{"x":-0.45,"y":0.95,"z":0.35,"w":0.16,"h":1.4,"segments":6,"far":0}'),
  ('flare','cylinder','mat.steel',0,'','{"x":-1.5,"y":0.05,"z":1.45,"w":0.14,"h":3.4,"segments":6}'),
  ('flare_stays','box','mat.steel',0,'','{"x":-1.5,"y":0.05,"z":1.45,"w":0.02,"d":0.02,"h":2.6,"roll":14,"far":0}'),
  ('flare_tip','cone','mat.glow',0,'','{"x":-1.5,"y":3.45,"z":1.45,"w":0.28,"h":0.55,"segments":6}'),
  ('control','box','mat.concrete',0,'','{"x":-0.6,"y":0.05,"z":1.3,"w":0.9,"d":0.55,"h":0.55,"windows":1,"window_w":0.7,"window_h":0.18,"window_y":0.25,"opening":"#2b3b49"}'),
  ('control_fascia','box','mat.cloth',1,'','{"x":-0.6,"y":0.6,"z":1.3,"w":0.94,"d":0.59,"h":0.1}'),
  ('control_roof','flat_roof','mat.cloth',1,'','{"x":-0.6,"y":0.7,"z":1.3,"w":0.94,"d":0.59,"t":0.03,"overhang":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.6,"y":0.05,"z":1.6,"h":2.4,"fw":0.5,"fh":0.75}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.refinery_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.refinery'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.refinery_modern'), (SELECT id FROM def WHERE key='mat.white_metal'), 100);
