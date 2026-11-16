-- bld.library@gunpowder: Early modern library (mesh.library_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.library_gunpowder','Early modern library');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.library_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','plinth','mat.ashlar',0,'','{"w":3.6,"d":3.4,"h":0.3,"steps":2,"inset":0.12}'),
  ('block','box','mat.lime_plaster',0,'','{"y":0.3,"z":-0.15,"w":2.8,"d":2.4,"h":1.5,"side_windows":2,"windows":2,"window_y":0.5,"window_w":0.16,"window_h":0.56,"opening":"#2c3642"}'),
  ('frieze','box','mat.cloth',1,'','{"y":1.62,"z":-0.15,"w":2.84,"d":2.44,"h":0.2}'),
  ('cornice','box','mat.ashlar',0,'','{"y":1.82,"z":-0.15,"w":2.96,"d":2.56,"h":0.1}'),
  ('roof','flat_roof','mat.slate',0,'','{"y":1.92,"z":-0.15,"w":2.8,"d":2.4,"t":0.06,"overhang":0}'),
  ('balustrade','wall','mat.cloth',1,'','{"y":1.98,"z":-0.15,"w":2.9,"d":2.5,"h":0.14,"t":0.12,"ring":1}'),
  ('drum','cylinder','mat.ashlar',0,'','{"y":1.98,"z":-0.15,"w":1.9,"h":0.55,"segments":12}'),
  ('drum_band','cylinder','mat.ashlar',0,'','{"y":2.5,"z":-0.15,"w":2,"h":0.07,"segments":12,"tint":"#ece4d2","far":0}'),
  ('dome','dome','mat.copper',0,'','{"y":2.57,"z":-0.15,"w":2,"h":1,"segments":12}'),
  ('lantern','cylinder','mat.lime_plaster',0,'','{"y":3.5,"z":-0.15,"w":0.42,"h":0.36,"segments":8}'),
  ('lantern_cap','dome','mat.cloth',1,'','{"y":3.86,"z":-0.15,"w":0.52,"h":0.3,"segments":8}'),
  ('finial','cone','mat.brass',0,'','{"y":4.12,"z":-0.15,"w":0.1,"h":0.36,"segments":6}'),
  ('columns','column_row','mat.ashlar',0,'','{"y":0.3,"z":1.42,"count":4,"radius":0.09,"spacing":0.5,"h":1.32,"segments":8,"capital":1}'),
  ('entablature','box','mat.ashlar',0,'','{"y":1.62,"z":1.28,"w":1.9,"d":0.5,"h":0.16}'),
  ('pediment','box','mat.ashlar',0,'','{"y":1.78,"z":1.28,"yaw":90,"w":0.5,"d":1.9,"h":0.02,"gable":22}'),
  ('portico_roof','gable_roof','mat.cloth',1,'','{"y":1.8,"z":1.28,"yaw":90,"w":0.5,"d":1.9,"pitch":22,"overhang":0.05,"t":0.05}'),
  ('tympanum','box','mat.cloth',1,'','{"y":1.82,"z":1.54,"yaw":90,"w":0.02,"d":1.4,"h":0.01,"gable":22,"far":0}'),
  ('door','box','mat.oak',0,'','{"y":0.3,"z":1.06,"w":0.5,"d":0.04,"h":0.9,"tint":"#5a3d26","far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.library_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.library'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.library_gunpowder'), (SELECT id FROM def WHERE key='mat.lime_plaster'), 100);
