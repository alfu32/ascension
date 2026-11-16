-- bld.university@modern: Modern university (mesh.university_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.university_modern','Modern university');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.university_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','plinth','mat.concrete',0,'','{"w":3.7,"d":3.7,"h":0.12,"steps":1}'),
  ('tower','box','mat.precast',0,'','{"y":0.12,"z":-1,"w":1.3,"d":1.1,"h":3.6,"windows":1,"window_w":1,"window_h":0.24,"window_y":0.5,"window_rows":6,"row_gap":0.52,"side_windows":1,"door":0.4,"door_h":0.42,"opening":"#2b3b49"}'),
  ('tower_crown','box','mat.cloth',1,'','{"y":3.58,"z":-1,"w":1.38,"d":1.18,"h":0.4}'),
  ('tower_roof','flat_roof','mat.concrete',0,'','{"y":3.98,"z":-1,"w":1.38,"d":1.18,"t":0.05,"overhang":0}'),
  ('observatory','cylinder','mat.concrete',0,'','{"y":4.03,"z":-1,"w":0.7,"h":0.24,"segments":10}'),
  ('observatory_dome','dome','mat.white_metal',0,'','{"y":4.27,"z":-1,"w":0.72,"h":0.4,"segments":10}'),
  ('slot','box','mat.cast_iron',0,'','{"y":4.35,"z":-0.72,"w":0.1,"d":0.12,"h":0.26,"pitch":-40,"far":0}'),
  ('wing_w','box','mat.precast',0,'','{"x":-1.3,"y":0.12,"z":-0.05,"yaw":90,"w":2.6,"d":0.95,"h":1.3,"windows":1,"window_w":2.2,"window_h":0.24,"window_y":0.3,"window_rows":2,"row_gap":0.55,"opening":"#2b3b49"}'),
  ('wing_e','box','mat.precast',0,'','{"x":1.3,"y":0.12,"z":-0.05,"yaw":-90,"w":2.6,"d":0.95,"h":1.3,"windows":1,"window_w":2.2,"window_h":0.24,"window_y":0.3,"window_rows":2,"row_gap":0.55,"opening":"#2b3b49"}'),
  ('wing_fascias','box','mat.cloth',1,'','{"x":-1.3,"y":1.26,"z":-0.05,"w":1.01,"d":2.66,"h":0.28,"repeat":2,"dx":2.6}'),
  ('wing_roofs','flat_roof','mat.tar',0,'','{"x":-1.3,"y":1.54,"z":-0.05,"w":1.01,"d":2.66,"t":0.04,"overhang":0,"repeat":2,"dx":2.6}'),
  ('hall','box','mat.concrete',0,'','{"y":0.12,"z":0.15,"w":1.5,"d":0.9,"h":0.9,"taper":0.12,"windows":3,"window_w":0.2,"window_h":0.4,"window_y":0.2,"opening":"#2b3b49"}'),
  ('hall_roof','flat_roof','mat.cloth',1,'','{"y":1.02,"z":0.15,"w":1.36,"d":0.78,"t":0.06,"overhang":0.04}'),
  ('lawn','box','mat.turf',0,'','{"y":0.12,"z":1.25,"w":2.4,"d":1,"h":0.02}'),
  ('path','box','mat.concrete',0,'','{"y":0.12,"z":1.25,"w":0.4,"d":1,"h":0.03,"far":0}'),
  ('sculpture','box','mat.steel',0,'','{"x":-0.7,"y":0.14,"z":1.2,"w":0.12,"d":0.12,"h":0.7,"roll":12,"far":0}'),
  ('banners','banner','mat.cloth',1,'','{"x":-0.95,"y":0.12,"z":1.7,"h":2.2,"fw":0.42,"fh":0.75,"repeat":2,"dx":1.9}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.university_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.university'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.university_modern'), (SELECT id FROM def WHERE key='mat.precast'), 100);
