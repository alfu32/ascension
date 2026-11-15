-- bld.temple@classical: Classical temple (mesh.temple_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.temple_classical','Classical temple');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.temple_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','plinth','mat.limestone',0,'','{"w":3.7,"d":3.7,"h":0.85,"steps":5,"inset":0.1}'),
  ('cella','box','mat.lime_plaster',0,'','{"y":0.85,"z":-0.15,"w":1.7,"d":1.9,"h":2.4,"door":0.6,"door_h":1.35}'),
  ('curtain','box','mat.cloth',1,'','{"y":0.85,"z":0.815,"w":0.55,"d":0.02,"h":1.27}'),
  ('front_columns','column_row','mat.limestone',0,'','{"y":0.85,"z":1.3,"count":6,"radius":0.11,"spacing":0.52,"h":2.4,"segments":8}'),
  ('back_columns','column_row','mat.limestone',0,'','{"y":0.85,"z":-1.3,"count":6,"radius":0.11,"spacing":0.52,"h":2.4,"segments":8,"far":0}'),
  ('side_columns','column_row','mat.limestone',0,'','{"x":-1.3,"y":0.85,"yaw":90,"count":4,"radius":0.11,"spacing":0.52,"h":2.4,"segments":8,"repeat":2,"dx":2.6}'),
  ('entablature','box','mat.limestone',0,'','{"y":3.25,"w":2.95,"d":2.95,"h":0.24}'),
  ('frieze','box','mat.cloth',1,'','{"y":3.3,"w":2.97,"d":2.97,"h":0.12}'),
  ('pediment','box','mat.cloth',1,'','{"y":3.49,"yaw":90,"w":2.95,"d":2.95,"h":0.02,"gable":20}'),
  ('roof','gable_roof','mat.terracotta',0,'','{"y":3.51,"yaw":90,"w":2.95,"d":2.95,"pitch":20,"overhang":0.1,"t":0.1}'),
  ('ridge_cap','gable_roof','mat.cloth',1,'','{"y":4.008,"yaw":90,"w":3.15,"d":0.8,"pitch":20,"overhang":0,"t":0.03}'),
  ('ridge','box','mat.cloth',1,'','{"y":4.156,"yaw":90,"w":3.2,"d":0.16,"h":0.08}'),
  ('acroteria','cone','mat.bronze',0,'','{"y":4.06,"z":-1.52,"w":0.2,"h":0.38,"segments":6,"repeat":2,"dz":3.04}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.temple_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.temple'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.temple_classical'), (SELECT id FROM def WHERE key='mat.limestone'), 100);
