-- bld.city@classical: Classical city (mesh.city_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.city_classical','Classical city');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.city_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.limestone',0,'','{"w":6.4,"d":6.4,"h":0.05,"steps":1}'),
  ('hall_base','plinth','mat.limestone',0,'','{"z":-0.5,"w":3.7,"d":2.9,"h":0.5,"steps":3,"inset":0.12}'),
  ('cella','box','mat.lime_plaster',0,'','{"y":0.5,"z":-0.65,"w":2.4,"d":1.6,"h":1.7,"door":0.62,"door_h":1.15}'),
  ('front_columns','column_row','mat.limestone',0,'','{"y":0.5,"z":0.62,"count":6,"radius":0.11,"spacing":0.6,"h":1.75,"segments":8}'),
  ('back_columns','column_row','mat.limestone',0,'','{"y":0.5,"z":-1.62,"count":6,"radius":0.11,"spacing":0.6,"h":1.75,"segments":8,"far":0}'),
  ('entablature','box','mat.limestone',0,'','{"y":2.25,"z":-0.5,"w":3.45,"d":2.6,"h":0.22}'),
  ('pediment','box','mat.limestone',0,'','{"y":2.45,"z":-0.5,"w":3.4,"d":2.55,"h":0.02,"gable":20}'),
  ('roof','gable_roof','mat.terracotta',0,'','{"y":2.47,"z":-0.5,"w":3.4,"d":2.55,"pitch":20,"overhang":0.14,"t":0.1}'),
  ('ridge','box','mat.cloth',1,'','{"y":2.96,"z":-0.5,"w":3.7,"d":0.22,"h":0.1}'),
  ('banner','banner','mat.cloth',1,'','{"x":-1.4,"z":1.55,"h":3.3,"fw":0.6,"fh":1.15,"repeat":2,"dx":2.8}'),
  ('house_sw','include',NULL,0,'','{"mesh":"mesh.house_classical","x":-2.2,"z":2.2}'),
  ('house_se','include',NULL,0,'','{"mesh":"mesh.house_classical","x":2.2,"z":2.2}'),
  ('house_w','include',NULL,0,'','{"mesh":"mesh.house_classical","x":-2.5,"z":-1.0,"yaw":90}'),
  ('house_e','include',NULL,0,'','{"mesh":"mesh.house_classical","x":2.5,"z":-1.0,"yaw":-90}'),
  ('house_n','include',NULL,0,'','{"mesh":"mesh.house_classical","z":-2.65,"yaw":180,"scale":0.9}'),
  ('fountain','cylinder','mat.limestone',0,'','{"z":2.5,"w":0.7,"h":0.25,"segments":8}'),
  ('statue','box','mat.bronze',0,'','{"z":2.5,"y":0.25,"w":0.16,"d":0.16,"h":0.55,"taper":0.3}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.city_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.city'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.city_classical'), (SELECT id FROM def WHERE key='mat.limestone'), 100);
