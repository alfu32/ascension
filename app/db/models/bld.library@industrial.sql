-- bld.library@industrial: Industrial library (mesh.library_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.library_industrial','Industrial library');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.library_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','plinth','mat.ashlar',0,'','{"w":3.6,"d":3.4,"h":0.3,"steps":2,"inset":0.12}'),
  ('block','box','mat.red_brick',0,'','{"y":0.3,"z":-0.15,"w":2.8,"d":2.4,"h":1.6,"side_windows":2,"windows":2,"window_y":0.45,"window_w":0.18,"window_h":0.7,"opening":"#22282f"}'),
  ('quoins','box','mat.ashlar',0,'','{"x":-1.4,"y":0.3,"z":1.05,"w":0.1,"d":0.1,"h":1.6,"repeat":2,"dx":2.8,"far":0}'),
  ('frieze','box','mat.cloth',1,'','{"y":1.7,"z":-0.15,"w":2.84,"d":2.44,"h":0.22}'),
  ('cornice','box','mat.ashlar',0,'','{"y":1.92,"z":-0.15,"w":2.96,"d":2.56,"h":0.1}'),
  ('roof','flat_roof','mat.slate',0,'','{"y":2.02,"z":-0.15,"w":2.8,"d":2.4,"t":0.06,"overhang":0}'),
  ('parapet','wall','mat.cloth',1,'','{"y":2.08,"z":-0.15,"w":2.9,"d":2.5,"h":0.16,"t":0.12,"ring":1}'),
  ('drum','cylinder','mat.red_brick',0,'','{"y":2.08,"z":-0.15,"w":1.9,"h":0.5,"segments":12}'),
  ('drum_band','cylinder','mat.cloth',1,'','{"y":2.5,"z":-0.15,"w":1.96,"h":0.12,"segments":12}'),
  ('dome','dome','mat.glass',0,'','{"y":2.62,"z":-0.15,"w":2,"h":1.05,"segments":12}'),
  ('lantern','cylinder','mat.cast_iron',0,'','{"y":3.6,"z":-0.15,"w":0.4,"h":0.32,"segments":8}'),
  ('lantern_cap','dome','mat.cloth',1,'','{"y":3.92,"z":-0.15,"w":0.5,"h":0.3,"segments":8}'),
  ('finial','cone','mat.brass',0,'','{"y":4.18,"z":-0.15,"w":0.1,"h":0.34,"segments":6,"far":0}'),
  ('columns','column_row','mat.ashlar',0,'','{"y":0.3,"z":1.42,"count":4,"radius":0.09,"spacing":0.5,"h":1.42,"segments":8,"capital":1}'),
  ('entablature','box','mat.ashlar',0,'','{"y":1.72,"z":1.28,"w":1.9,"d":0.5,"h":0.16}'),
  ('pediment','box','mat.ashlar',0,'','{"y":1.88,"z":1.28,"yaw":90,"w":0.5,"d":1.9,"h":0.02,"gable":22}'),
  ('portico_roof','gable_roof','mat.cloth',1,'','{"y":1.9,"z":1.28,"yaw":90,"w":0.5,"d":1.9,"pitch":22,"overhang":0.05,"t":0.05}'),
  ('door','box','mat.oak',0,'','{"y":0.3,"z":1.06,"w":0.5,"d":0.04,"h":0.9,"tint":"#5a3d26","far":0}'),
  ('lamps','cylinder','mat.cast_iron',0,'','{"x":-1.1,"y":0.3,"z":1.5,"w":0.05,"h":0.9,"segments":4,"repeat":2,"dx":2.2,"far":0}'),
  ('lamp_glow','box','mat.glow',0,'','{"x":-1.1,"y":1.18,"z":1.5,"w":0.12,"d":0.12,"h":0.14,"repeat":2,"dx":2.2,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.library_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.library'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.library_industrial'), (SELECT id FROM def WHERE key='mat.red_brick'), 100);
