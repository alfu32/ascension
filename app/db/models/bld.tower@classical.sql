-- bld.tower@classical: Classical tower (mesh.tower_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.tower_classical','Classical tower');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.tower_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','plinth','mat.limestone',0,'','{"w":1.8,"d":1.8,"h":0.3,"steps":2,"inset":0.08}'),
  ('shaft','box','mat.limestone',0,'','{"y":0.3,"w":1.45,"d":1.45,"h":3.3,"door":0.42,"door_h":0.8,"side_windows":1,"window_w":0.12,"window_h":0.34,"window_y":1.6,"window_rows":2,"row_gap":1.1}'),
  ('corbel','box','mat.limestone',0,'','{"y":3.6,"w":1.65,"d":1.65,"h":0.2}'),
  ('merlons','crenellation','mat.limestone',0,'','{"y":3.8,"w":1.65,"d":1.65,"h":0.35,"t":0.15,"merlon":0.24,"ring":1}'),
  ('hanging','box','mat.cloth',1,'','{"y":1.95,"z":0.735,"w":0.5,"d":0.03,"h":1.45}'),
  ('roof_posts','column_row','mat.limestone',0,'','{"y":3.8,"z":-0.5,"count":2,"radius":0.07,"spacing":1.0,"h":0.85,"segments":6,"capital":0,"repeat":2,"dz":1.0}'),
  ('roof','hip_roof','mat.terracotta',0,'','{"y":4.65,"w":1.2,"d":1.2,"pitch":30,"overhang":0.18,"t":0.08}'),
  ('roof_cap','hip_roof','mat.cloth',1,'','{"y":4.8,"w":1,"d":1,"pitch":30,"overhang":0,"t":0.03}'),
  ('banner','banner','mat.cloth',1,'','{"y":4.95,"h":1.3,"fw":0.5,"fh":0.6}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.tower_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.tower'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.tower_classical'), (SELECT id FROM def WHERE key='mat.limestone'), 100);
