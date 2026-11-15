-- bld.farm@ancient: Early farm (mesh.farm_ancient)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.farm_ancient','Early farm');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.farm_ancient';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('soil','plinth','mat.soil',0,'','{"w":3.8,"d":3.8,"h":0.03,"steps":1}'),
  ('barn','box','mat.timber',0,'','{"x":-1.2,"z":-1.25,"w":1.1,"d":0.9,"h":0.8,"gable":40,"door":0.42,"door_h":0.6,"door_colour":"#3a2a1c"}'),
  ('barn_roof','gable_roof','mat.thatch',0,'','{"x":-1.2,"y":0.8,"z":-1.25,"w":1.1,"d":0.9,"pitch":40,"overhang":0.12,"t":0.08}'),
  ('ridge_cap','gable_roof','mat.cloth',1,'','{"x":-1.2,"y":1.156,"z":-1.25,"w":1.34,"d":0.3,"pitch":40,"overhang":0,"t":0.03}'),
  ('barn_ridge','box','mat.cloth',1,'','{"x":-1.2,"y":1.254,"z":-1.25,"w":1.35,"d":0.16,"h":0.08}'),
  ('flag','banner','mat.cloth',1,'','{"x":-0.55,"z":-0.75,"h":1.7,"fw":0.36,"fh":0.5}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.farm_ancient'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.farm'), NULL, (SELECT id FROM def WHERE key='age.ancient'), (SELECT id FROM def WHERE key='mesh.farm_ancient'), (SELECT id FROM def WHERE key='mat.mud_plaster'), 100);
