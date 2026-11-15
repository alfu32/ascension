-- bld.woodcutter_camp@ancient: Early woodcutter's camp (mesh.woodcutter_ancient)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.woodcutter_ancient','Early woodcutter''s camp');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.woodcutter_ancient';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('posts_back','cylinder','mat.timber',0,'','{"x":-1.5,"z":-1.55,"w":0.14,"h":1.3,"segments":6,"repeat":2,"dx":1.9}'),
  ('posts_front','cylinder','mat.timber',0,'','{"x":-1.5,"z":-0.35,"w":0.14,"h":1.3,"segments":6,"repeat":2,"dx":1.9}'),
  ('beams','box','mat.timber',0,'','{"x":-0.55,"y":1.22,"z":-1.55,"w":2.15,"d":0.12,"h":0.12,"repeat":2,"dz":1.2}'),
  ('roof','gable_roof','mat.thatch',0,'','{"x":-0.55,"y":1.34,"z":-0.95,"w":2.0,"d":1.2,"pitch":38,"overhang":0.22,"t":0.1}'),
  ('ridge_cap','gable_roof','mat.cloth',1,'','{"x":-0.55,"y":1.662,"z":-0.95,"w":2.44,"d":0.7,"pitch":38,"overhang":0,"t":0.03}'),
  ('ridge','box','mat.cloth',1,'','{"x":-0.55,"y":1.896,"z":-0.95,"w":2.44,"d":0.2,"h":0.1}'),
  ('valance','box','mat.cloth',1,'','{"x":-0.55,"y":0.98,"z":-0.12,"w":2.44,"d":0.04,"h":0.2,"far":0}'),
  ('saw_legs','box','mat.timber',0,'','{"x":-1.0,"z":-1.1,"w":0.07,"d":0.07,"h":0.72,"pitch":26,"repeat":2,"dx":0.85,"far":0}'),
  ('saw_legs_back','box','mat.timber',0,'','{"x":-1.0,"z":-0.8,"w":0.07,"d":0.07,"h":0.72,"pitch":-26,"repeat":2,"dx":0.85,"far":0}'),
  ('saw_log','cylinder','mat.timber',0,'','{"x":-1.35,"y":0.62,"z":-0.95,"w":0.26,"h":1.45,"segments":6,"roll":90}'),
  ('logs_e','include',NULL,0,'','{"mesh":"mesh.log_pile","x":0.75,"z":0.7}'),
  ('logs_w','include',NULL,0,'','{"mesh":"mesh.log_pile","x":-1.05,"z":0.85,"yaw":90,"scale":0.85}'),
  ('block','cylinder','mat.timber',0,'','{"x":1.2,"z":-1.0,"w":0.45,"h":0.38,"segments":7,"tint":"#d6c4a6"}'),
  ('axe','box','mat.timber',0,'','{"x":1.2,"y":0.34,"z":-1.0,"w":0.05,"d":0.05,"h":0.55,"roll":35,"far":0}'),
  ('axe_head','box','mat.iron',0,'','{"x":1.2,"y":0.33,"z":-1.0,"w":0.2,"d":0.04,"h":0.12}'),
  ('stumps','cylinder','mat.timber',0,'','{"x":1.35,"z":-0.2,"w":0.4,"h":0.22,"segments":7,"tint":"#c8b395","far":0}'),
  ('stump_n','cylinder','mat.timber',0,'','{"x":1.45,"z":-1.55,"w":0.36,"h":0.2,"segments":7,"tint":"#c8b395","far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.5,"z":-0.35,"h":2.4,"fw":0.5,"fh":0.75}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.woodcutter_ancient'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.woodcutter_camp'), NULL, (SELECT id FROM def WHERE key='age.ancient'), (SELECT id FROM def WHERE key='mesh.woodcutter_ancient'), (SELECT id FROM def WHERE key='mat.timber'), 100);
