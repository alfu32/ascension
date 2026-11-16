-- bld.university@automation: Future university (mesh.university_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.university_automation','Future university');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.university_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','plinth','mat.paving',0,'','{"w":3.7,"d":3.7,"h":0.12,"steps":1}'),
  ('tower','cylinder','mat.composite_panel',0,'','{"y":0.12,"z":-1,"w":1.3,"h":3.5,"taper":0.12,"segments":12}'),
  ('tower_glass','cylinder','mat.smart_glass',0,'','{"y":1,"z":-1,"w":1.29,"h":0.3,"segments":12}'),
  ('tower_glass_2','cylinder','mat.smart_glass',0,'','{"y":2.2,"z":-1,"w":1.24,"h":0.3,"segments":12,"far":0}'),
  ('tower_crown','cylinder','mat.white_composite',1,'','{"y":3.62,"z":-1,"w":1.2,"h":0.32,"segments":12}'),
  ('observatory','cylinder','mat.white_composite',0,'','{"y":3.94,"z":-1,"w":0.8,"h":0.22,"segments":12}'),
  ('observatory_dome','dome','mat.white_composite',0,'','{"y":4.16,"z":-1,"w":0.82,"h":0.45,"segments":12}'),
  ('slot','box','mat.graphite',0,'','{"y":4.25,"z":-0.7,"w":0.12,"d":0.12,"h":0.28,"pitch":-40,"far":0}'),
  ('wing_w','box','mat.composite_panel',0,'','{"x":-1.3,"y":0.12,"z":-0.05,"yaw":90,"w":2.6,"d":0.95,"h":1.2}'),
  ('wing_e','box','mat.composite_panel',0,'','{"x":1.3,"y":0.12,"z":-0.05,"yaw":-90,"w":2.6,"d":0.95,"h":1.2}'),
  ('wing_ends_n','cylinder','mat.composite_panel',0,'','{"x":-1.3,"y":0.12,"z":-1.35,"w":0.95,"h":1.2,"segments":12,"repeat":2,"dx":2.6}'),
  ('wing_ends_s','cylinder','mat.composite_panel',0,'','{"x":-1.3,"y":0.12,"z":1.25,"w":0.95,"h":1.2,"segments":12,"repeat":2,"dx":2.6}'),
  ('wing_glass','box','mat.smart_glass',0,'','{"x":-1.3,"y":0.5,"z":-0.05,"w":0.97,"d":2.6,"h":0.3,"repeat":2,"dx":2.6}'),
  ('wing_bands','box','mat.white_composite',1,'','{"x":-1.3,"y":1.2,"z":-0.05,"w":0.99,"d":2.6,"h":0.2,"repeat":2,"dx":2.6}'),
  ('wing_band_ends','cylinder','mat.white_composite',1,'','{"x":-1.3,"y":1.2,"z":-1.35,"w":0.99,"h":0.2,"segments":12,"repeat":2,"dx":2.6,"far":0}'),
  ('wing_band_ends_s','cylinder','mat.white_composite',1,'','{"x":-1.3,"y":1.2,"z":1.25,"w":0.99,"h":0.2,"segments":12,"repeat":2,"dx":2.6,"far":0}'),
  ('wing_roofs','box','mat.white_composite',0,'','{"x":-1.3,"y":1.4,"z":-0.05,"w":0.95,"d":2.6,"h":0.02,"repeat":2,"dx":2.6,"far":0}'),
  ('atrium','dome','mat.smart_glass',0,'','{"y":0.12,"z":0.3,"w":1.4,"h":0.95,"segments":12}'),
  ('atrium_ring','cylinder','mat.light_strip',0,'','{"y":0.12,"z":0.3,"w":1.42,"h":0.03,"segments":12,"far":0}'),
  ('lawn','box','mat.turf',0,'','{"y":0.12,"z":1.4,"w":1.6,"d":0.75,"h":0.02}'),
  ('ring','torus','mat.white_composite',1,'','{"x":-0.45,"y":0.14,"z":1.45,"w":0.55,"t":0.08,"segments":12,"sides":4,"roll":90,"far":0}'),
  ('banners','banner','mat.cloth',1,'','{"x":-0.4,"y":0.12,"z":1.75,"h":2.2,"fw":0.42,"fh":0.7,"pole_colour":"#e8ecec","repeat":2,"dx":0.8}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.university_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.university'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.university_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
