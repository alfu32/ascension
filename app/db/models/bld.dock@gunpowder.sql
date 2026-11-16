-- bld.dock@gunpowder: Early modern dock (mesh.dock_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.dock_gunpowder','Early modern dock');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.dock_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.04,"steps":1}'),
  ('warehouse','box','mat.brick',0,'','{"x":-1,"z":-0.95,"w":1.5,"d":1.6,"h":1.45,"door":0.4,"door_h":0.6,"windows":2,"window_y":0.2,"window_w":0.16,"window_h":0.3,"window_rows":2,"row_gap":0.66,"side_windows":2,"opening":"#2c3642"}'),
  ('warehouse_roof','hip_roof','mat.pantile',0,'','{"x":-1,"y":1.45,"z":-0.95,"w":1.5,"d":1.6,"pitch":42,"overhang":0.12,"t":0.08}'),
  ('warehouse_cap','hip_roof','mat.cloth',1,'','{"x":-1,"y":1.873,"z":-0.95,"w":0.8,"d":0.9,"pitch":42,"overhang":0,"t":0.03}'),
  ('hoist','box','mat.oak',0,'','{"x":-1,"y":1.32,"z":-0.05,"w":0.08,"d":0.5,"h":0.08,"far":0}'),
  ('hull','hull','mat.timber',0,'','{"x":-0.9,"y":0.18,"z":0.95,"yaw":90,"w":1.6,"d":0.62,"h":0.45,"sheer":0.14,"tint":"#7a6450"}'),
  ('hull_stripe','box','mat.cloth',1,'hull','{"y":0.38,"w":1,"d":0.66,"h":0.08}'),
  ('mast','cylinder','mat.timber',0,'hull','{"y":0.3,"w":0.06,"h":1.6,"segments":5}'),
  ('chocks','box','mat.oak',0,'','{"x":-0.9,"z":0.45,"w":0.5,"d":0.14,"h":0.2,"repeat":3,"dz":0.5,"far":0}'),
  ('quay','box','mat.ashlar',0,'','{"x":0.95,"w":1.2,"d":3.5,"h":0.42}'),
  ('quay_edge','box','mat.ashlar',0,'','{"x":1.5,"y":0.42,"w":0.12,"d":3.5,"h":0.06,"tint":"#e8e0cc","far":0}'),
  ('bollards','cylinder','mat.iron',0,'','{"x":1.38,"y":0.42,"z":-0.4,"w":0.1,"h":0.16,"segments":6,"repeat":3,"dz":0.75,"tint":"#4a4e56","far":0}'),
  ('crane_house','cylinder','mat.timber',0,'','{"x":0.95,"y":0.42,"z":-1.15,"w":0.95,"h":1,"segments":8}'),
  ('crane_roof','cone','mat.pantile',0,'','{"x":0.95,"y":1.42,"z":-1.15,"w":1.12,"h":0.55,"segments":8}'),
  ('crane_cap','cone','mat.cloth',1,'','{"x":0.95,"y":1.6,"z":-1.15,"w":0.75,"h":0.37,"segments":8}'),
  ('jib','box','mat.oak',0,'','{"x":1.2,"y":1.2,"z":-1.15,"w":0.12,"d":0.12,"h":1,"roll":52}'),
  ('brace','box','mat.oak',0,'','{"x":1.3,"y":0.6,"z":-1.15,"w":0.08,"d":0.08,"h":0.9,"roll":62,"far":0}'),
  ('rope','cylinder','mat.thatch',0,'','{"x":1.98,"y":0.95,"z":-1.15,"w":0.03,"h":0.85,"segments":4,"far":0}'),
  ('load','box','mat.timber',0,'','{"x":1.98,"y":0.62,"z":-1.15,"w":0.36,"d":0.36,"h":0.32}'),
  ('barrels','cylinder','mat.timber',0,'','{"x":0.75,"y":0.42,"z":0.9,"w":0.24,"h":0.34,"segments":7,"repeat":2,"dz":0.32,"far":0,"tint":"#c8a888"}'),
  ('steps','plinth','mat.ashlar',0,'','{"x":0.22,"z":0.2,"w":0.3,"d":0.6,"h":0.42,"steps":3,"inset":0.05}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.3,"y":0.42,"z":1.5,"h":2.5,"fw":0.7,"fh":0.9}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.dock_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.dock'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.dock_gunpowder'), (SELECT id FROM def WHERE key='mat.brick'), 100);
