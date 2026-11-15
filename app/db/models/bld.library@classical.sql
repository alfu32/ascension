-- bld.library@classical: Classical library (mesh.library_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.library_classical','Classical library');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.library_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','plinth','mat.limestone',0,'','{"w":3.5,"d":3.5,"h":0.45,"steps":3,"inset":0.14}'),
  ('block','box','mat.limestone',0,'','{"y":0.45,"z":-0.2,"w":2.7,"d":2.2,"h":1.75,"door":0.5,"door_h":1.05,"side_windows":2,"window_y":1.1,"window_w":0.14,"window_h":0.34}'),
  ('porch_columns','column_row','mat.limestone',0,'','{"y":0.45,"z":1.22,"count":4,"radius":0.1,"spacing":0.55,"h":1.6,"segments":8}'),
  ('porch_beam','box','mat.limestone',0,'','{"y":2.05,"z":1.07,"w":1.95,"d":0.5,"h":0.15}'),
  ('pediment','box','mat.cloth',1,'','{"y":2.2,"z":1.07,"yaw":90,"w":0.5,"d":1.95,"h":0.02,"gable":22}'),
  ('porch_roof','gable_roof','mat.terracotta',0,'','{"y":2.22,"z":1.07,"yaw":90,"w":0.5,"d":1.95,"pitch":22,"overhang":0.06,"t":0.07}'),
  ('top','flat_roof','mat.limestone',0,'','{"y":2.2,"z":-0.2,"w":2.7,"d":2.2,"t":0.12,"overhang":0.06}'),
  ('drum','cylinder','mat.limestone',0,'','{"y":2.32,"z":-0.2,"w":1.5,"h":0.45,"segments":12}'),
  ('drum_band','cylinder','mat.cloth',1,'','{"y":2.6,"z":-0.2,"w":1.53,"h":0.17,"segments":12}'),
  ('dome','dome','mat.cloth',1,'','{"y":2.77,"z":-0.2,"w":1.5,"h":0.8,"segments":12}'),
  ('finial','cone','mat.bronze',0,'','{"y":3.55,"z":-0.2,"w":0.16,"h":0.35,"segments":6}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.library_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.library'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.library_classical'), (SELECT id FROM def WHERE key='mat.limestone'), 100);
