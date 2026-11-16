-- bld.woodcutter_camp@gunpowder: Early modern woodcutter's camp (mesh.woodcutter_camp_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.woodcutter_camp_gunpowder','Early modern woodcutter''s camp');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.woodcutter_camp_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('back_wall','wall','mat.brick',0,'','{"x":-0.6,"z":-1.55,"w":2,"h":1.3,"t":0.18}'),
  ('posts','box','mat.sawn_timber',0,'','{"x":-1.55,"z":-0.35,"w":0.13,"d":0.13,"h":1.35,"repeat":2,"dx":1.9}'),
  ('beam','box','mat.sawn_timber',0,'','{"x":-0.6,"y":1.33,"z":-0.35,"w":2.05,"d":0.12,"h":0.12}'),
  ('roof','gable_roof','mat.pantile',0,'','{"x":-0.6,"y":1.4,"z":-0.95,"w":1.9,"d":1.2,"pitch":40,"overhang":0.18,"t":0.08}'),
  ('roof_cap','gable_roof','mat.cloth',1,'','{"x":-0.6,"y":1.756,"z":-0.95,"w":2.3,"d":0.6,"pitch":40,"overhang":0,"t":0.03}'),
  ('saw_legs','box','mat.oak',0,'','{"x":-1.05,"z":-1.1,"w":0.07,"d":0.07,"h":0.72,"pitch":26,"repeat":2,"dx":0.85,"far":0}'),
  ('saw_legs_back','box','mat.oak',0,'','{"x":-1.05,"z":-0.8,"w":0.07,"d":0.07,"h":0.72,"pitch":-26,"repeat":2,"dx":0.85,"far":0}'),
  ('saw_log','cylinder','mat.timber',0,'','{"x":-1.4,"y":0.62,"z":-0.95,"w":0.26,"h":1.45,"segments":6,"roll":90}'),
  ('saw','box','mat.iron',0,'','{"x":-0.85,"y":0.78,"z":-0.95,"w":0.03,"d":0.7,"h":0.12,"far":0}'),
  ('logs','include',NULL,0,'','{"mesh":"mesh.log_pile","x":0.75,"z":0.7}'),
  ('logs_w','include',NULL,0,'','{"mesh":"mesh.log_pile","x":-1.05,"z":0.85,"yaw":90,"scale":0.85}'),
  ('house','include','mat.lime_plaster',0,'','{"mesh":"mesh.house_early_modern","x":1.2,"z":-1.15,"scale":0.75}'),
  ('block','cylinder','mat.timber',0,'','{"x":0.5,"z":-0.25,"w":0.45,"h":0.38,"segments":7,"tint":"#d6c4a6"}'),
  ('axe','box','mat.timber',0,'','{"x":0.5,"y":0.34,"z":-0.25,"w":0.05,"d":0.05,"h":0.55,"roll":35,"far":0}'),
  ('axe_head','box','mat.iron',0,'','{"x":0.5,"y":0.33,"z":-0.25,"w":0.2,"d":0.04,"h":0.12}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.55,"z":-0.3,"h":2.6,"fw":0.5,"fh":0.8}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.woodcutter_camp_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.woodcutter_camp'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.woodcutter_camp_gunpowder'), (SELECT id FROM def WHERE key='mat.timber'), 100);
