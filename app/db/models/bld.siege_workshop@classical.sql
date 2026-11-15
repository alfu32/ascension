-- bld.siege_workshop@classical: Classical siege workshop (mesh.siege_workshop_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.siege_workshop_classical','Classical siege workshop');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.siege_workshop_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('posts_back','box','mat.timber',0,'','{"x":-1.55,"z":-1.6,"w":0.16,"d":0.16,"h":2.3,"repeat":3,"dx":1.55}'),
  ('posts_front','box','mat.timber',0,'','{"x":-1.55,"z":-0.55,"w":0.16,"d":0.16,"h":2.3,"repeat":3,"dx":1.55}'),
  ('ties','box','mat.timber',0,'','{"y":2.16,"z":-1.6,"w":3.3,"d":0.14,"h":0.14,"repeat":2,"dz":1.05}'),
  ('cross_ties','box','mat.timber',0,'','{"x":-1.55,"y":2.16,"z":-1.075,"yaw":90,"w":1.2,"d":0.14,"h":0.14,"repeat":3,"dx":1.55,"far":0}'),
  ('braces_r','box','mat.timber',0,'','{"x":-1.55,"y":1.6,"z":-0.55,"w":0.08,"d":0.08,"h":0.75,"roll":45,"repeat":2,"dx":1.55,"far":0}'),
  ('braces_l','box','mat.timber',0,'','{"x":1.55,"y":1.6,"z":-0.55,"w":0.08,"d":0.08,"h":0.75,"roll":-45,"repeat":2,"dx":-1.55,"far":0}'),
  ('gable_ends','box','mat.timber',0,'','{"x":-1.55,"y":2.28,"z":-1.075,"w":0.08,"d":1.05,"h":0.02,"gable":28,"repeat":2,"dx":3.1}'),
  ('roof','gable_roof','mat.terracotta',0,'','{"y":2.3,"z":-1.075,"w":3.1,"d":1.05,"pitch":28,"overhang":0.18,"t":0.08}'),
  ('ridge_cap','gable_roof','mat.cloth',1,'','{"y":2.55,"z":-1.075,"w":3.46,"d":0.45,"pitch":28,"overhang":0,"t":0.03}'),
  ('ridge','box','mat.cloth',1,'','{"y":2.661,"z":-1.075,"w":3.4,"d":0.16,"h":0.08}'),
  ('hanging','box','mat.cloth',1,'','{"x":0.78,"y":1.0,"z":-1.6,"w":1.2,"d":0.03,"h":1.1}'),
  ('bench','box','mat.timber',0,'','{"x":-0.8,"z":-1.2,"w":1.2,"d":0.5,"h":0.55}'),
  ('beams','box','mat.sawn_timber',0,'','{"x":0.8,"z":-1.42,"w":1.3,"d":0.18,"h":0.18,"repeat":3,"dz":0.22}'),
  ('beams_top','box','mat.sawn_timber',0,'','{"x":0.8,"y":0.18,"z":-1.31,"w":1.25,"d":0.18,"h":0.18,"repeat":2,"dz":0.22,"tint":"#e6dccc","far":0}'),
  ('catapult','include',NULL,0,'','{"mesh":"mesh.catapult_frame","x":-0.2,"z":0.55,"scale":1.2}'),
  ('spare_wheel','cylinder','mat.timber',0,'','{"x":1.15,"z":0.75,"w":0.6,"h":0.08,"segments":10,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.55,"z":1.55,"h":2.8,"fw":0.55,"fh":0.9}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.siege_workshop_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.siege_workshop'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.siege_workshop_classical'), (SELECT id FROM def WHERE key='mat.timber'), 100);
