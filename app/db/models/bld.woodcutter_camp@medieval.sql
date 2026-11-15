-- bld.woodcutter_camp@medieval: Medieval woodcutter (mesh.woodcutter_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.woodcutter_medieval','Medieval woodcutter');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.woodcutter_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('back_wall','wall','mat.rough_stone',0,'','{"x":-0.6,"z":-1.55,"w":2,"h":0.7,"t":0.18}'),
  ('back_frame','wall','mat.daub',0,'','{"x":-0.6,"y":0.7,"z":-1.55,"w":2,"h":0.65,"t":0.14}'),
  ('posts','box','mat.oak',0,'','{"x":-1.55,"z":-0.35,"w":0.14,"d":0.14,"h":1.35,"repeat":2,"dx":1.9}'),
  ('braces','box','mat.oak',0,'','{"x":-1.5,"y":0.95,"z":-0.35,"w":0.07,"d":0.07,"h":0.5,"roll":45,"far":0}'),
  ('braces_e','box','mat.oak',0,'','{"x":0.3,"y":0.95,"z":-0.35,"w":0.07,"d":0.07,"h":0.5,"roll":-45,"far":0}'),
  ('beam','box','mat.oak',0,'','{"x":-0.6,"y":1.33,"z":-0.35,"w":2.05,"d":0.12,"h":0.12}'),
  ('roof','gable_roof','mat.shingle',0,'','{"x":-0.6,"y":1.4,"z":-0.95,"w":1.9,"d":1.2,"pitch":46,"overhang":0.18,"t":0.08}'),
  ('roof_cap','gable_roof','mat.cloth',1,'','{"x":-0.6,"y":1.826,"z":-0.95,"w":2.3,"d":0.6,"pitch":46,"overhang":0,"t":0.03}'),
  ('saw_legs','box','mat.oak',0,'','{"x":-1.05,"z":-1.1,"w":0.07,"d":0.07,"h":0.72,"pitch":26,"repeat":2,"dx":0.85,"far":0}'),
  ('saw_legs_back','box','mat.oak',0,'','{"x":-1.05,"z":-0.8,"w":0.07,"d":0.07,"h":0.72,"pitch":-26,"repeat":2,"dx":0.85,"far":0}'),
  ('saw_log','cylinder','mat.timber',0,'','{"x":-1.4,"y":0.62,"z":-0.95,"w":0.26,"h":1.45,"segments":6,"roll":90}'),
  ('logs','include',NULL,0,'','{"mesh":"mesh.log_pile","x":0.75,"z":0.7}'),
  ('logs_w','include',NULL,0,'','{"mesh":"mesh.log_pile","x":-1.05,"z":0.85,"yaw":90,"scale":0.85}'),
  ('house','include',NULL,0,'','{"mesh":"mesh.house_medieval","x":1.2,"z":-1.15,"scale":0.75}'),
  ('block','cylinder','mat.timber',0,'','{"x":0.5,"z":-0.25,"w":0.45,"h":0.38,"segments":7,"tint":"#d6c4a6"}'),
  ('axe','box','mat.timber',0,'','{"x":0.5,"y":0.34,"z":-0.25,"w":0.05,"d":0.05,"h":0.55,"roll":35,"far":0}'),
  ('axe_head','box','mat.iron',0,'','{"x":0.5,"y":0.33,"z":-0.25,"w":0.2,"d":0.04,"h":0.12}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.55,"z":-0.3,"h":2.6,"fw":0.5,"fh":0.8}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.woodcutter_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.woodcutter_camp'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.woodcutter_medieval'), (SELECT id FROM def WHERE key='mat.oak'), 100);
