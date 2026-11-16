-- bld.tower@industrial: Industrial tower (mesh.tower_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.tower_industrial','Industrial tower');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.tower_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','box','mat.ashlar',0,'','{"w":1.75,"d":1.75,"h":0.55,"taper":0.08}'),
  ('shaft','box','mat.red_brick',0,'','{"y":0.55,"w":1.45,"d":1.45,"h":3,"door":0.4,"door_h":0.72,"side_windows":1,"windows":1,"window_w":0.12,"window_h":0.4,"window_y":1,"window_rows":2,"row_gap":1,"opening":"#22282f"}'),
  ('bands','box','mat.ashlar',0,'','{"y":1.5,"w":1.5,"d":1.5,"h":0.06,"far":0}'),
  ('band','box','mat.cloth',1,'','{"y":3.1,"w":1.5,"d":1.5,"h":0.38}'),
  ('hanging','box','mat.cloth',1,'','{"y":1.65,"z":0.74,"w":0.6,"d":0.03,"h":1.3,"repeat":2,"dz":-1.48}'),
  ('corbel','box','mat.ashlar',0,'','{"y":3.55,"w":1.75,"d":1.75,"h":0.12}'),
  ('parapet','box','mat.red_brick',0,'','{"y":3.67,"w":1.75,"d":1.75,"h":0.12}'),
  ('merlons','crenellation','mat.red_brick',0,'','{"y":3.79,"w":1.75,"d":1.75,"h":0.32,"t":0.16,"merlon":0.36,"ring":1}'),
  ('turret','cylinder','mat.red_brick',0,'','{"x":-0.72,"y":1.8,"z":0.72,"w":0.42,"h":2.3,"segments":8}'),
  ('turret_roof','cone','mat.cloth',1,'','{"x":-0.72,"y":4.1,"z":0.72,"w":0.5,"h":0.6,"segments":8}'),
  ('lamp_house','cylinder','mat.cast_iron',0,'','{"y":3.79,"w":0.62,"h":0.42,"segments":8}'),
  ('lamp','cylinder','mat.glow',0,'','{"y":4,"z":0.24,"w":0.32,"h":0.12,"segments":8,"pitch":90}'),
  ('lamp_roof','dome','mat.cloth',1,'','{"y":4.21,"w":0.7,"h":0.28,"segments":8}'),
  ('mast','banner','mat.cloth',1,'','{"x":0.55,"y":3.79,"z":-0.55,"h":1.5,"fw":0.7,"fh":0.5}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.tower_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.tower'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.tower_industrial'), (SELECT id FROM def WHERE key='mat.red_brick'), 100);
