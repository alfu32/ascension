-- bld.woodcutter_camp@classical: Classical woodcutter's camp (mesh.woodcutter_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.woodcutter_classical','Classical woodcutter''s camp');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.woodcutter_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('footings','box','mat.limestone',0,'','{"x":-1.5,"z":-0.35,"w":0.26,"d":0.26,"h":0.2,"repeat":2,"dx":1.8,"far":0}'),
  ('posts','box','mat.timber',0,'','{"x":-1.5,"y":0.2,"z":-0.35,"w":0.14,"d":0.14,"h":1.25,"repeat":2,"dx":1.8}'),
  ('back_wall','wall','mat.lime_plaster',0,'','{"x":-0.6,"z":-1.55,"w":2.0,"h":1.45,"t":0.16}'),
  ('beam','box','mat.timber',0,'','{"x":-0.6,"y":1.45,"z":-0.35,"w":2.0,"d":0.12,"h":0.12}'),
  ('roof','gable_roof','mat.terracotta',0,'','{"x":-0.6,"y":1.57,"z":-0.95,"w":1.85,"d":1.2,"pitch":24,"overhang":0.18,"t":0.08}'),
  ('ridge_cap','gable_roof','mat.cloth',1,'','{"x":-0.6,"y":1.747,"z":-0.95,"w":2.21,"d":0.8,"pitch":24,"overhang":0,"t":0.03}'),
  ('ridge','box','mat.cloth',1,'','{"x":-0.6,"y":1.922,"z":-0.95,"w":2.25,"d":0.16,"h":0.08}'),
  ('valance','box','mat.cloth',1,'','{"x":-0.6,"y":1.3,"z":-0.16,"w":2.25,"d":0.04,"h":0.18,"far":0}'),
  ('saw_legs','box','mat.timber',0,'','{"x":-1.05,"z":-1.1,"w":0.07,"d":0.07,"h":0.72,"pitch":26,"repeat":2,"dx":0.85,"far":0}'),
  ('saw_legs_back','box','mat.timber',0,'','{"x":-1.05,"z":-0.8,"w":0.07,"d":0.07,"h":0.72,"pitch":-26,"repeat":2,"dx":0.85,"far":0}'),
  ('saw_log','cylinder','mat.timber',0,'','{"x":-1.4,"y":0.62,"z":-0.95,"w":0.26,"h":1.45,"segments":6,"roll":90}'),
  ('logs','include',NULL,0,'','{"mesh":"mesh.log_pile","x":0.75,"z":0.7}'),
  ('beams_low','box','mat.sawn_timber',0,'','{"x":-1.45,"z":0.85,"yaw":90,"w":1.5,"d":0.2,"h":0.2,"repeat":4,"dx":0.24}'),
  ('beams_high','box','mat.sawn_timber',0,'','{"x":-1.33,"y":0.2,"z":0.85,"yaw":90,"w":1.4,"d":0.2,"h":0.2,"repeat":3,"dx":0.24,"tint":"#e6dccc"}'),
  ('house','include',NULL,0,'','{"mesh":"mesh.house_classical","x":1.2,"z":-1.2,"scale":0.72}'),
  ('block','cylinder','mat.timber',0,'','{"x":1.3,"z":-0.3,"w":0.45,"h":0.38,"segments":7,"tint":"#d6c4a6"}'),
  ('axe','box','mat.timber',0,'','{"x":1.3,"y":0.34,"z":-0.3,"w":0.05,"d":0.05,"h":0.55,"roll":35,"far":0}'),
  ('axe_head','box','mat.iron',0,'','{"x":1.3,"y":0.33,"z":-0.3,"w":0.2,"d":0.04,"h":0.12}'),
  ('banner','banner','mat.cloth',1,'','{"x":0.55,"z":-0.2,"h":2.6,"fw":0.5,"fh":0.8}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.woodcutter_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.woodcutter_camp'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.woodcutter_classical'), (SELECT id FROM def WHERE key='mat.timber'), 100);
