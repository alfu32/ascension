-- bld.temple@modern: Modern temple (mesh.temple_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.temple_modern','Modern temple');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.temple_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','plinth','mat.concrete',0,'','{"w":3.7,"d":3.7,"h":0.45,"steps":3,"inset":0.1}'),
  ('nave','box','mat.concrete',0,'','{"y":0.45,"z":-0.35,"yaw":90,"w":2.7,"d":2.1,"h":1.5,"gable":50,"windows":5,"window_w":0.14,"window_h":1.15,"window_y":0.2,"side_windows":0,"opening":"#2b3b49"}'),
  ('nave_roof','gable_roof','mat.copper',0,'','{"y":1.95,"z":-0.35,"yaw":90,"w":2.7,"d":2.1,"pitch":50,"overhang":0.12,"t":0.08}'),
  ('nave_roof_cap','gable_roof','mat.cloth',1,'','{"y":2.9684,"z":-0.35,"yaw":90,"w":2.94,"d":0.6,"pitch":50,"overhang":0,"t":0.03}'),
  ('rose','shield','mat.curtain_wall',0,'','{"y":2.25,"z":-1.71,"yaw":180,"w":0.6,"t":0.02,"boss":0,"segments":10,"rim":"#c8c4ba","far":0}'),
  ('columns','column_row','mat.concrete',0,'','{"y":0.45,"z":1.42,"count":4,"radius":0.07,"spacing":0.6,"h":1.75,"segments":6,"capital":0}'),
  ('slab','box','mat.concrete',0,'','{"y":2.2,"z":1.2,"w":2.2,"d":0.66,"h":0.14}'),
  ('pediment','box','mat.concrete',0,'','{"y":2.34,"z":1.2,"yaw":90,"w":0.66,"d":2.2,"h":0.02,"gable":26}'),
  ('portico_roof','gable_roof','mat.cloth',1,'','{"y":2.36,"z":1.2,"yaw":90,"w":0.66,"d":2.2,"pitch":26,"overhang":0.04,"t":0.05}'),
  ('portal','box','mat.curtain_wall',0,'','{"y":0.45,"z":1,"w":0.8,"d":0.04,"h":1.3,"far":0}'),
  ('campanile','box','mat.concrete',0,'','{"x":1.4,"y":0.45,"z":1.2,"w":0.36,"d":0.36,"h":3.4,"windows":1,"side_windows":1,"window_w":0.16,"window_h":0.5,"window_y":2.6,"opening":"#1e2328"}'),
  ('campanile_band','box','mat.cloth',1,'','{"x":1.4,"y":3.85,"z":1.2,"w":0.4,"d":0.4,"h":0.22}'),
  ('campanile_cap','box','mat.concrete',0,'','{"x":1.4,"y":4.07,"z":1.2,"w":0.44,"d":0.44,"h":0.06}'),
  ('finial','cone','mat.steel',0,'','{"x":1.4,"y":4.13,"z":1.2,"w":0.08,"h":0.6,"segments":4,"far":0}'),
  ('railings','fence','mat.steel',0,'','{"y":0,"z":1.78,"w":3.6,"h":0.3,"spacing":0.3,"rails":2,"gap":1,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.temple_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.temple'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.temple_modern'), (SELECT id FROM def WHERE key='mat.concrete'), 100);
