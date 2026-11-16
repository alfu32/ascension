-- bld.tower@gunpowder: Early modern tower (mesh.tower_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.tower_gunpowder','Early modern tower');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.tower_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','box','mat.ashlar',0,'','{"w":1.75,"d":1.75,"h":0.6,"taper":0.08}'),
  ('shaft','box','mat.brick',0,'','{"y":0.6,"w":1.5,"d":1.5,"h":2.9,"door":0.4,"door_h":0.72,"side_windows":1,"windows":1,"window_w":0.12,"window_h":0.36,"window_y":1,"window_rows":2,"row_gap":0.95,"opening":"#2c3642"}'),
  ('quoins','box','mat.ashlar',0,'','{"x":-0.73,"y":0.6,"z":-0.73,"w":0.1,"d":0.1,"h":2.9,"repeat":2,"dx":1.46,"far":0}'),
  ('quoins_s','box','mat.ashlar',0,'','{"x":-0.73,"y":0.6,"z":0.73,"w":0.1,"d":0.1,"h":2.9,"repeat":2,"dx":1.46,"far":0}'),
  ('band','box','mat.cloth',1,'','{"y":3.22,"w":1.56,"d":1.56,"h":0.3}'),
  ('corbel','box','mat.ashlar',0,'','{"y":3.52,"w":1.8,"d":1.8,"h":0.12}'),
  ('merlons','crenellation','mat.ashlar',0,'','{"y":3.64,"w":1.8,"d":1.8,"h":0.3,"t":0.16,"merlon":0.38,"ring":1}'),
  ('lookout','cylinder','mat.cloth',1,'','{"y":3.64,"w":0.7,"h":0.6,"segments":8}'),
  ('lookout_dome','dome','mat.cloth',1,'','{"y":4.24,"w":0.82,"h":0.46,"segments":8}'),
  ('lookout_finial','cone','mat.copper',0,'','{"y":4.66,"w":0.12,"h":0.3,"segments":6,"far":0}'),
  ('hanging','box','mat.cloth',1,'','{"y":1.6,"z":0.77,"w":0.66,"d":0.03,"h":1.5,"repeat":2,"dz":-1.54}'),
  ('gun','cylinder','mat.iron',0,'','{"z":0.45,"y":3.78,"w":0.12,"h":0.6,"segments":6,"pitch":86,"tint":"#5a5e66","far":0}'),
  ('banner','banner','mat.cloth',1,'','{"y":4.62,"h":1.2,"fw":0.8,"fh":0.66}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.tower_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.tower'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.tower_gunpowder'), (SELECT id FROM def WHERE key='mat.brick'), 100);
