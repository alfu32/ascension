-- bld.auto_university@automation: Automated University (mesh.auto_university_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.auto_university_automation','Automated University');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.auto_university_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','plinth','mat.paving',0,'','{"w":3.7,"d":3.7,"h":0.12,"steps":1}'),
  ('drum','cylinder','mat.composite_panel',0,'','{"x":0.35,"y":0.12,"z":0.4,"w":2.6,"h":0.6,"segments":16}'),
  ('drum_glow','cylinder','mat.light_strip',0,'','{"x":0.35,"y":0.5,"z":0.4,"w":2.62,"h":0.03,"segments":16,"far":0}'),
  ('drum_band','cylinder','mat.white_composite',1,'','{"x":0.35,"y":0.6,"z":0.4,"w":2.64,"h":0.3,"segments":16}'),
  ('dome','dome','mat.smart_glass',0,'','{"x":0.35,"y":0.9,"z":0.4,"w":2.6,"h":1.35,"segments":16}'),
  ('dome_cap','cylinder','mat.white_composite',1,'','{"x":0.35,"y":2.2,"z":0.4,"w":0.6,"h":0.12,"segments":10}'),
  ('core','cylinder','mat.graphite',0,'','{"x":-1.2,"y":0.12,"z":-1.2,"w":0.95,"h":3.4,"segments":12}'),
  ('core_ring','cylinder','mat.light_strip',0,'','{"x":-1.2,"y":0.9,"z":-1.2,"w":0.97,"h":0.05,"segments":12}'),
  ('core_ring_2','cylinder','mat.light_strip',0,'','{"x":-1.2,"y":1.7,"z":-1.2,"w":0.97,"h":0.05,"segments":12,"far":0}'),
  ('core_ring_3','cylinder','mat.light_strip',0,'','{"x":-1.2,"y":2.5,"z":-1.2,"w":0.97,"h":0.05,"segments":12,"far":0}'),
  ('core_crown','cylinder','mat.white_composite',1,'','{"x":-1.2,"y":3.3,"z":-1.2,"w":1.05,"h":0.28,"segments":12}'),
  ('core_cap','dome','mat.white_composite',0,'','{"x":-1.2,"y":3.58,"z":-1.2,"w":1,"h":0.3,"segments":12}'),
  ('bridge','box','mat.smart_glass',0,'','{"x":-0.6,"y":1.2,"z":-0.6,"yaw":-45,"w":0.3,"d":1,"h":0.22,"far":0}'),
  ('observatory','cylinder','mat.white_composite',0,'','{"x":1.3,"y":0.12,"z":-1.3,"w":0.9,"h":1.5,"segments":12}'),
  ('observatory_band','cylinder','mat.white_composite',1,'','{"x":1.3,"y":1.4,"z":-1.3,"w":0.92,"h":0.18,"segments":12}'),
  ('observatory_dome','dome','mat.white_composite',1,'','{"x":1.3,"y":1.62,"z":-1.3,"w":0.92,"h":0.48,"segments":12}'),
  ('slot','box','mat.graphite',0,'','{"x":1.3,"y":1.7,"z":-1,"w":0.12,"d":0.12,"h":0.3,"pitch":-40,"far":0}'),
  ('dish_post','cylinder','mat.white_composite',0,'','{"x":-1.35,"y":0.12,"z":1.3,"w":0.1,"h":0.8,"segments":6}'),
  ('dish','dome','mat.white_composite',0,'','{"x":-1.35,"y":0.92,"z":1.3,"yaw":30,"w":0.9,"h":0.1,"segments":10,"pitch":45}'),
  ('dish_back','dome','mat.white_composite',0,'','{"x":-1.35,"y":0.92,"z":1.3,"yaw":30,"w":0.9,"h":0.22,"segments":10,"pitch":-135,"far":0}'),
  ('dish_feed','box','mat.graphite',0,'','{"x":-1.35,"y":0.95,"z":1.3,"w":0.04,"d":0.04,"h":0.4,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.6,"y":0.12,"z":1.6,"h":2.4,"fw":0.5,"fh":0.7,"pole_colour":"#e8ecec"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.auto_university_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.auto_university'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.auto_university_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
