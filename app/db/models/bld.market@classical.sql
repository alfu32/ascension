-- bld.market@classical: Classical market (mesh.market_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.market_classical','Classical market');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.market_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.limestone',0,'','{"w":3.7,"d":3.7,"h":0.08,"steps":1}'),
  ('stoa_back','box','mat.lime_plaster',0,'','{"y":0.08,"z":-1.58,"w":3.4,"d":0.24,"h":1.74,"windows":0}'),
  ('stoa_ends','box','mat.lime_plaster',0,'','{"x":-1.55,"y":0.08,"z":-1.1,"w":0.3,"d":1.2,"h":1.74,"gable":20,"repeat":2,"dx":3.1}'),
  ('stoa_columns','column_row','mat.limestone',0,'','{"y":0.08,"z":-0.58,"count":5,"radius":0.1,"spacing":0.66,"h":1.56,"segments":8}'),
  ('stoa_beam','box','mat.limestone',0,'','{"y":1.64,"z":-0.58,"w":3.4,"d":0.24,"h":0.18}'),
  ('stoa_frieze','box','mat.cloth',1,'','{"y":1.68,"z":-0.58,"w":3.42,"d":0.26,"h":0.11}'),
  ('stoa_roof','gable_roof','mat.terracotta',0,'','{"y":1.82,"z":-1.1,"w":3.4,"d":1.2,"pitch":20,"overhang":0.1,"t":0.08}'),
  ('ridge_cap','gable_roof','mat.cloth',1,'','{"y":2.033,"z":-1.1,"w":3.6,"d":0.5,"pitch":20,"overhang":0,"t":0.03}'),
  ('stall_w','include',NULL,0,'','{"mesh":"mesh.stall","x":-1.3,"y":0.08,"z":0.55,"yaw":90,"scale":0.85}'),
  ('stall_e','include',NULL,0,'','{"mesh":"mesh.stall","x":1.3,"y":0.08,"z":0.55,"yaw":-90,"scale":0.85}'),
  ('stall_s','include',NULL,0,'','{"mesh":"mesh.stall","y":0.08,"z":1.3,"yaw":180,"scale":0.85}'),
  ('basin','cylinder','mat.limestone',0,'','{"y":0.08,"z":0.25,"w":0.75,"h":0.28,"segments":10}'),
  ('spout','cylinder','mat.bronze',0,'','{"y":0.36,"z":0.25,"w":0.12,"h":0.5,"segments":6,"taper":0.4}'),
  ('amphorae','cylinder','mat.terracotta',0,'','{"x":-0.55,"y":0.08,"z":-0.25,"w":0.22,"h":0.42,"segments":6,"taper":0.35,"repeat":3,"dx":0.26}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.market_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.market'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.market_classical'), (SELECT id FROM def WHERE key='mat.limestone'), 100);
