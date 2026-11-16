-- bld.refinery@automation: Future refinery (mesh.refinery_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.refinery_automation','Future refinery');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.refinery_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.paving',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('column_a','cylinder','mat.white_composite',0,'','{"x":-1.2,"y":0.05,"z":-1.2,"w":0.46,"h":3.6,"segments":10}'),
  ('column_a_band','cylinder','mat.white_composite',1,'','{"x":-1.2,"y":2.7,"z":-1.2,"w":0.48,"h":0.4,"segments":10}'),
  ('column_a_top','dome','mat.white_composite',0,'','{"x":-1.2,"y":3.65,"z":-1.2,"w":0.46,"h":0.18,"segments":10}'),
  ('column_a_glow','cylinder','mat.light_strip',0,'','{"x":-1.2,"y":1.3,"z":-1.2,"w":0.48,"h":0.04,"segments":10,"far":0}'),
  ('column_a_glow_2','cylinder','mat.light_strip',0,'','{"x":-1.2,"y":2,"z":-1.2,"w":0.48,"h":0.04,"segments":10,"far":0}'),
  ('column_b','cylinder','mat.white_composite',0,'','{"x":-0.55,"y":0.05,"z":-1.35,"w":0.36,"h":2.9,"segments":10}'),
  ('column_b_band','cylinder','mat.white_composite',1,'','{"x":-0.55,"y":2.1,"z":-1.35,"w":0.38,"h":0.35,"segments":10}'),
  ('column_b_top','dome','mat.white_composite',0,'','{"x":-0.55,"y":2.95,"z":-1.35,"w":0.36,"h":0.14,"segments":10,"far":0}'),
  ('column_c','cylinder','mat.white_composite',0,'','{"x":-1.25,"y":0.05,"z":-0.45,"w":0.3,"h":2.3,"segments":8,"far":0}'),
  ('column_c_top','dome','mat.white_composite',0,'','{"x":-1.25,"y":2.35,"z":-0.45,"w":0.3,"h":0.12,"segments":8,"far":0}'),
  ('sphere','dome','mat.white_composite',0,'','{"x":0.55,"y":0.8,"z":-1.2,"w":0.75,"h":0.375,"segments":10}'),
  ('sphere_under','dome','mat.white_composite',0,'','{"x":0.55,"y":0.8,"z":-1.2,"w":0.75,"h":0.375,"segments":10,"pitch":180,"far":0}'),
  ('sphere_2','dome','mat.white_composite',0,'','{"x":1.35,"y":0.8,"z":-1.2,"w":0.75,"h":0.375,"segments":10}'),
  ('sphere_2_under','dome','mat.white_composite',0,'','{"x":1.35,"y":0.8,"z":-1.2,"w":0.75,"h":0.375,"segments":10,"pitch":180,"far":0}'),
  ('sphere_belts','cylinder','mat.light_strip',0,'','{"x":0.55,"y":0.79,"z":-1.2,"w":0.77,"h":0.03,"segments":10,"repeat":2,"dx":0.8,"far":0}'),
  ('sphere_legs','cylinder','mat.graphite',0,'','{"x":0.3,"y":0.05,"z":-1.2,"w":0.05,"h":0.75,"segments":4,"repeat":2,"dx":0.5}'),
  ('sphere_2_legs','cylinder','mat.graphite',0,'','{"x":1.1,"y":0.05,"z":-1.2,"w":0.05,"h":0.75,"segments":4,"repeat":2,"dx":0.5,"far":0}'),
  ('tank','cylinder','mat.white_composite',0,'','{"x":1.1,"y":0.05,"z":0.3,"w":1.05,"h":0.6,"segments":12}'),
  ('tank_band','cylinder','mat.white_composite',1,'','{"x":1.1,"y":0.42,"z":0.3,"w":1.07,"h":0.18,"segments":12}'),
  ('tank_roof','dome','mat.white_composite',1,'','{"x":1.1,"y":0.65,"z":0.3,"w":1.05,"h":0.2,"segments":12,"far":0}'),
  ('tank_2','cylinder','mat.white_composite',0,'','{"x":0.2,"y":0.05,"z":0.55,"w":0.8,"h":0.5,"segments":10}'),
  ('tank_2_band','cylinder','mat.white_composite',1,'','{"x":0.2,"y":0.35,"z":0.55,"w":0.82,"h":0.14,"segments":10}'),
  ('tank_2_roof','dome','mat.white_composite',1,'','{"x":0.2,"y":0.55,"z":0.55,"w":0.8,"h":0.16,"segments":10,"far":0}'),
  ('pipe_rack','box','mat.white_composite',0,'','{"x":0,"y":1.05,"z":-0.55,"w":3.2,"d":0.3,"h":0.06}'),
  ('pipes','box','mat.graphite',0,'','{"x":0,"y":1.11,"z":-0.65,"w":3.2,"d":0.06,"h":0.06,"repeat":3,"dz":0.1,"far":0}'),
  ('rack_posts','box','mat.white_composite',0,'','{"x":-1.5,"y":0.05,"z":-0.55,"w":0.06,"d":0.24,"h":1,"repeat":4,"dx":1,"far":0}'),
  ('heater','box','mat.composite_panel',0,'','{"x":-0.45,"y":0.05,"z":0.35,"w":0.8,"d":0.5,"h":0.9}'),
  ('heater_stack','cylinder','mat.white_composite',0,'','{"x":-0.45,"y":0.95,"z":0.35,"w":0.16,"h":1.4,"segments":6,"far":0}'),
  ('flare','cylinder','mat.white_composite',0,'','{"x":-1.5,"y":0.05,"z":1.45,"w":0.14,"h":3.4,"segments":6}'),
  ('flare_band','cylinder','mat.white_composite',1,'','{"x":-1.5,"y":2.8,"z":1.45,"w":0.16,"h":0.3,"segments":6}'),
  ('flare_tip','cone','mat.glow',0,'','{"x":-1.5,"y":3.45,"z":1.45,"w":0.28,"h":0.55,"segments":6}'),
  ('control','dome','mat.smart_glass',0,'','{"x":-0.6,"y":0.05,"z":1.3,"w":0.9,"h":0.6,"segments":10}'),
  ('control_ring','cylinder','mat.white_composite',1,'','{"x":-0.6,"y":0.05,"z":1.3,"w":0.94,"h":0.12,"segments":10}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.6,"y":0.05,"z":1.6,"h":2.4,"fw":0.5,"fh":0.75,"pole_colour":"#e8ecec"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.refinery_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.refinery'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.refinery_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
