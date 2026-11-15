-- bld.library@ancient: Early library (mesh.library_ancient)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.library_ancient','Early library');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.library_ancient';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','plinth','mat.fieldstone',0,'','{"w":3.6,"d":3.45,"h":0.42,"steps":2,"inset":0.16}'),
  ('block','box','mat.mud_brick',0,'','{"y":0.42,"w":2.8,"d":2.5,"h":1.65,"door":0.55,"door_h":1.0,"windows":2,"side_windows":1,"window_y":1.12,"window_w":0.15,"window_h":0.24}'),
  ('porch_posts','column_row','mat.timber',0,'','{"y":0.42,"z":1.52,"count":2,"radius":0.07,"spacing":1.0,"h":1.25,"segments":6,"capital":0}'),
  ('porch_roof','box','mat.timber',0,'','{"y":1.67,"z":1.4,"w":1.4,"d":0.5,"h":0.1}'),
  ('lower_roof','hip_roof','mat.thatch',0,'','{"y":2.07,"w":2.8,"d":2.5,"pitch":26,"overhang":0.3,"t":0.12}'),
  ('upper_block','box','mat.mud_brick',0,'','{"y":2.35,"w":1.5,"d":1.2,"h":0.7,"windows":3,"side_windows":2,"window_y":0.36,"window_w":0.12,"window_h":0.16}'),
  ('upper_roof','hip_roof','mat.thatch',1,'','{"y":3.05,"w":1.5,"d":1.2,"pitch":34,"overhang":0.22,"t":0.1}'),
  ('finial','cone','mat.bronze',0,'','{"y":3.38,"w":0.18,"h":0.38,"segments":6}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.library_ancient'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.library'), NULL, (SELECT id FROM def WHERE key='age.ancient'), (SELECT id FROM def WHERE key='mesh.library_ancient'), (SELECT id FROM def WHERE key='mat.mud_brick'), 100);
