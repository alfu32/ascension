-- bld.woodcutter_camp@industrial: Industrial woodcutter camp (mesh.woodcutter_camp_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.woodcutter_camp_industrial','Industrial woodcutter camp');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.woodcutter_camp_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('posts','box','mat.cast_iron',0,'','{"x":-1.55,"z":-1.55,"w":0.08,"d":0.08,"h":1.4,"repeat":3,"dx":0.95}'),
  ('posts_s','box','mat.cast_iron',0,'','{"x":-1.55,"z":-0.35,"w":0.08,"d":0.08,"h":1.4,"repeat":3,"dx":0.95}'),
  ('back_wall','box','mat.iron',0,'','{"x":-0.6,"y":0.1,"z":-1.58,"w":1.95,"d":0.04,"h":1.25,"tint":"#b0b4b8"}'),
  ('roof','gable_roof','mat.corrugated',0,'','{"x":-0.6,"y":1.4,"z":-0.95,"w":2,"d":1.3,"pitch":26,"overhang":0.16,"t":0.04}'),
  ('roof_cap','gable_roof','mat.cloth',1,'','{"x":-0.6,"y":1.603,"z":-0.95,"w":2.32,"d":0.65,"pitch":26,"overhang":0,"t":0.03}'),
  ('bench','box','mat.oak',0,'','{"x":-0.85,"z":-0.95,"w":1.3,"d":0.5,"h":0.5}'),
  ('blade','shield','mat.steel',0,'','{"x":-0.7,"y":0.62,"z":-0.95,"yaw":90,"w":0.5,"t":0.02,"boss":0.03,"segments":10,"rim":"#7a7f88"}'),
  ('bench_log','cylinder','mat.timber',0,'','{"x":-1.45,"y":0.5,"z":-0.85,"w":0.28,"h":0.6,"segments":6,"roll":90,"far":0}'),
  ('boiler','cylinder','mat.cast_iron',0,'','{"x":0.25,"y":0.3,"z":-1.25,"w":0.5,"h":0.85,"segments":8,"roll":90}'),
  ('boiler_band','cylinder','mat.cloth',1,'','{"x":0.5,"y":0.3,"z":-1.25,"w":0.52,"h":0.12,"segments":8,"roll":90,"far":0}'),
  ('boiler_wheels','cylinder','mat.oak',0,'','{"x":0.3,"z":-1.55,"w":0.5,"h":0.06,"segments":8,"pitch":90,"repeat":2,"dx":0.65,"far":0}'),
  ('stack','cylinder','mat.cast_iron',0,'','{"x":1,"y":0.5,"z":-1.25,"w":0.12,"h":0.85,"segments":6}'),
  ('belt','box','mat.leather',0,'','{"x":-0.3,"y":0.58,"z":-1.12,"w":0.7,"d":0.02,"h":0.05,"roll":-8,"far":0}'),
  ('logs','include',NULL,0,'','{"mesh":"mesh.log_pile","x":0.8,"z":0.75}'),
  ('logs_w','include',NULL,0,'','{"mesh":"mesh.log_pile","x":-1,"z":0.9,"yaw":90,"scale":0.85}'),
  ('saw_legs','box','mat.oak',0,'','{"x":0.1,"z":0,"w":0.07,"d":0.07,"h":0.72,"pitch":26,"repeat":2,"dx":0.8,"far":0}'),
  ('saw_legs_back','box','mat.oak',0,'','{"x":0.1,"z":0.3,"w":0.07,"d":0.07,"h":0.72,"pitch":-26,"repeat":2,"dx":0.8,"far":0}'),
  ('saw_log','cylinder','mat.timber',0,'','{"x":-0.2,"y":0.62,"z":0.15,"w":0.26,"h":1.4,"segments":6,"roll":90}'),
  ('hut','box','mat.red_brick',0,'','{"x":1.25,"z":-1.2,"w":0.85,"d":0.8,"h":0.85,"door":0.24,"door_h":0.5,"opening":"#22282f"}'),
  ('hut_roof','gable_roof','mat.slate',0,'','{"x":1.25,"y":0.85,"z":-1.2,"w":0.85,"d":0.8,"pitch":34,"overhang":0.06,"t":0.05}'),
  ('hut_stack','box','mat.red_brick',0,'','{"x":1.55,"y":0.85,"z":-1.2,"w":0.14,"d":0.2,"h":0.5,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.6,"z":0,"h":2.6,"fw":0.5,"fh":0.8}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.woodcutter_camp_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.woodcutter_camp'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.woodcutter_camp_industrial'), (SELECT id FROM def WHERE key='mat.timber'), 100);
