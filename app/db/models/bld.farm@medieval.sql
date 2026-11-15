-- bld.farm@medieval: Medieval farm (mesh.farm_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.farm_medieval','Medieval farm');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.farm_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('soil','plinth','mat.soil',0,'','{"w":3.8,"d":3.8,"h":0.03,"steps":1}'),
  ('footing','box','mat.rough_stone',0,'','{"x":-1.2,"z":-1.25,"w":1.16,"d":0.96,"h":0.3}'),
  ('barn','box','mat.daub',0,'','{"x":-1.2,"y":0.3,"z":-1.25,"w":1.12,"d":0.92,"h":0.52,"gable":48,"door":0.4,"door_h":0.45,"door_colour":"#3a2a1c"}'),
  ('posts','box','mat.oak',0,'','{"x":-1.75,"y":0.3,"z":-1.25,"w":0.06,"d":0.95,"h":0.52,"repeat":3,"dx":0.55,"far":0}'),
  ('roof','gable_roof','mat.shingle',0,'','{"x":-1.2,"y":0.82,"z":-1.25,"w":1.12,"d":0.92,"pitch":48,"overhang":0.12,"t":0.08}'),
  ('roof_cap','gable_roof','mat.cloth',1,'','{"x":-1.2,"y":1.217,"z":-1.25,"w":1.4,"d":0.42,"pitch":48,"overhang":0,"t":0.03}'),
  ('rick','cone','mat.thatch',0,'','{"x":-0.35,"z":-1.45,"w":0.62,"h":0.75,"segments":7}'),
  ('flag','banner','mat.cloth',1,'','{"x":-0.55,"z":-0.7,"h":1.8,"fw":0.38,"fh":0.52}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.farm_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.farm'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.farm_medieval'), (SELECT id FROM def WHERE key='mat.daub'), 100);
