-- bld.stable@medieval: Medieval stable (mesh.stable_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.stable_medieval','Medieval stable');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.stable_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('footing','box','mat.rough_stone',0,'','{"z":-1,"w":3.5,"d":1.3,"h":0.4}'),
  ('barn','box','mat.daub',0,'','{"y":0.4,"z":-1,"w":3.4,"d":1.2,"h":0.75,"gable":46,"door":0.55,"door_h":0.6,"windows":5,"window_w":0.26,"window_h":0.3,"window_y":0.25,"opening":"#2a221c"}'),
  ('frame','box','mat.oak',0,'','{"x":-1.7,"y":0.4,"z":-1,"w":0.06,"d":1.23,"h":0.75,"repeat":6,"dx":0.68,"far":0}'),
  ('roof','gable_roof','mat.shingle',0,'','{"y":1.15,"z":-1,"w":3.4,"d":1.2,"pitch":46,"overhang":0.12,"t":0.08}'),
  ('roof_cap','gable_roof','mat.cloth',1,'','{"y":1.628,"z":-1,"w":3.68,"d":0.5,"pitch":46,"overhang":0,"t":0.03}'),
  ('paddock','fence','mat.oak',0,'','{"z":0.75,"w":3.5,"d":1.9,"h":0.6,"spacing":0.85,"rails":2,"ring":1,"gap":0.8,"far":0}'),
  ('horse_w','include',NULL,0,'','{"mesh":"mesh.horse","x":-0.8,"z":0.75,"yaw":70,"scale":0.9}'),
  ('horse_e','include',NULL,0,'','{"mesh":"mesh.horse","x":0.7,"z":0.95,"yaw":-120,"scale":0.9}'),
  ('trough','box','mat.timber',0,'','{"x":1,"z":0.15,"w":0.9,"d":0.28,"h":0.3,"far":0}'),
  ('hay','box','mat.thatch',0,'','{"x":-1.35,"z":0.25,"w":0.55,"d":0.4,"h":0.35}'),
  ('banner','banner','mat.cloth',1,'','{"x":0.55,"z":1.7,"h":2.2,"fw":0.45,"fh":0.65}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.stable_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.stable'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.stable_medieval'), (SELECT id FROM def WHERE key='mat.daub'), 100);
