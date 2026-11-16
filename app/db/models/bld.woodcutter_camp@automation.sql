-- bld.woodcutter_camp@automation: Future woodcutter's camp (mesh.woodcutter_camp_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.woodcutter_camp_automation','Future woodcutter''s camp');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.woodcutter_camp_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('posts','cylinder','mat.white_composite',0,'','{"x":-1.55,"y":0.05,"z":-1.55,"w":0.1,"h":1.2,"segments":6,"repeat":3,"dx":0.95}'),
  ('posts_s','cylinder','mat.white_composite',0,'','{"x":-1.55,"y":0.05,"z":-0.35,"w":0.1,"h":1.2,"segments":6,"repeat":3,"dx":0.95,"far":0}'),
  ('back_wall','box','mat.composite_panel',0,'','{"x":-0.6,"y":0.05,"z":-1.58,"w":1.95,"d":0.04,"h":1.15}'),
  ('fascia','box','mat.white_composite',1,'','{"x":-0.6,"y":1.19,"z":-0.95,"w":2.16,"d":1.46,"h":0.24}'),
  ('fascia_glow','box','mat.light_strip',0,'','{"x":-0.6,"y":1.16,"z":-0.21,"w":2.1,"d":0.03,"h":0.03,"far":0}'),
  ('roof','flat_roof','mat.white_composite',1,'','{"x":-0.6,"y":1.43,"z":-0.95,"w":2.16,"d":1.46,"t":0.05,"overhang":0}'),
  ('solar','box','mat.solar_panel',0,'','{"x":-0.6,"y":1.5,"z":-1.35,"w":1.95,"d":0.62,"h":0.03,"pitch":14,"repeat":2,"dz":0.66,"far":0}'),
  ('bench','box','mat.graphite',0,'','{"x":-0.85,"y":0.05,"z":-0.95,"w":1.3,"d":0.5,"h":0.45}'),
  ('blade','shield','mat.steel',0,'','{"x":-0.7,"y":0.55,"z":-0.95,"yaw":90,"w":0.5,"t":0.02,"boss":0.03,"segments":10,"rim":"#7a7f88"}'),
  ('laser','box','mat.light_strip',0,'','{"x":-0.7,"y":0.52,"z":-0.95,"w":0.02,"d":0.95,"h":0.02,"far":0}'),
  ('bench_log','cylinder','mat.timber',0,'','{"x":-1.45,"y":0.62,"z":-0.85,"w":0.28,"h":0.6,"segments":6,"roll":90,"far":0}'),
  ('logs','include',NULL,0,'','{"mesh":"mesh.log_pile","x":0.8,"y":0.05,"z":0.75}'),
  ('logs_w','include',NULL,0,'','{"mesh":"mesh.log_pile","x":-1,"y":0.05,"z":0.9,"yaw":90,"scale":0.85}'),
  ('saw_legs','box','mat.white_composite',0,'','{"x":0.1,"y":0.05,"z":0,"w":0.06,"d":0.06,"h":0.72,"pitch":26,"repeat":2,"dx":0.8,"far":0}'),
  ('saw_legs_back','box','mat.white_composite',0,'','{"x":0.1,"y":0.05,"z":0.3,"w":0.06,"d":0.06,"h":0.72,"pitch":-26,"repeat":2,"dx":0.8,"far":0}'),
  ('saw_log','cylinder','mat.timber',0,'','{"x":-0.2,"y":0.67,"z":0.15,"w":0.26,"h":1.4,"segments":6,"roll":90}'),
  ('loader','box','mat.white_composite',1,'','{"x":1.15,"y":0.2,"z":-1.1,"w":0.5,"d":0.95,"h":0.25}'),
  ('loader_cab','dome','mat.smart_glass',0,'','{"x":1.15,"y":0.45,"z":-0.8,"w":0.42,"h":0.34,"segments":8}'),
  ('loader_glow','box','mat.light_strip',0,'','{"x":1.15,"y":0.17,"z":-1.1,"w":0.52,"d":0.97,"h":0.03,"far":0}'),
  ('loader_wheels','cylinder','mat.rubber',0,'','{"x":0.88,"y":0.18,"z":-1.4,"w":0.36,"h":0.1,"segments":8,"roll":90,"repeat":2,"dx":0.44,"far":0}'),
  ('loader_wheels_f','cylinder','mat.rubber',0,'','{"x":0.88,"y":0.18,"z":-0.78,"w":0.36,"h":0.1,"segments":8,"roll":90,"repeat":2,"dx":0.44,"far":0}'),
  ('crane_post','cylinder','mat.white_composite',0,'','{"x":1.15,"y":0.45,"z":-1.35,"w":0.14,"h":0.5,"segments":6,"far":0}'),
  ('crane_boom','box','mat.white_composite',0,'','{"x":1.15,"y":0.9,"z":-1.35,"w":0.08,"d":0.1,"h":1.1,"pitch":50}'),
  ('grapple','box','mat.graphite',0,'','{"x":1.15,"y":1.38,"z":-0.5,"w":0.16,"d":0.16,"h":0.2,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.65,"y":0.05,"z":-0.25,"h":2.6,"fw":0.55,"fh":1,"pole_colour":"#e8ecec"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.woodcutter_camp_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.woodcutter_camp'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.woodcutter_camp_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
