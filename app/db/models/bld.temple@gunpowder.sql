-- bld.temple@gunpowder: Early modern temple (mesh.temple_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.temple_gunpowder','Early modern temple');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.temple_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','plinth','mat.ashlar',0,'','{"w":3.7,"d":3.7,"h":0.45,"steps":3,"inset":0.1}'),
  ('nave','box','mat.lime_plaster',0,'','{"y":0.45,"z":-0.35,"yaw":90,"w":2.7,"d":2.1,"h":1.9,"gable":30,"windows":3,"side_windows":1,"window_y":0.55,"window_w":0.2,"window_h":0.75,"opening":"#2c3642"}'),
  ('pilasters','box','mat.ashlar',0,'','{"x":-1.07,"y":0.45,"z":-1.6,"w":0.12,"d":0.1,"h":1.9,"repeat":4,"dz":0.84,"far":0}'),
  ('pilasters_e','box','mat.ashlar',0,'','{"x":1.07,"y":0.45,"z":-1.6,"w":0.12,"d":0.1,"h":1.9,"repeat":4,"dz":0.84,"far":0}'),
  ('nave_roof','gable_roof','mat.slate',0,'','{"y":2.35,"z":-0.35,"yaw":90,"w":2.7,"d":2.1,"pitch":30,"overhang":0.1,"t":0.08}'),
  ('nave_roof_cap','gable_roof','mat.cloth',1,'','{"y":2.818,"z":-0.35,"yaw":90,"w":2.9,"d":0.8,"pitch":30,"overhang":0,"t":0.03}'),
  ('tower','box','mat.ashlar',0,'','{"y":2.3,"z":0.45,"w":0.8,"d":0.8,"h":1.05,"windows":1,"side_windows":1,"window_y":0.5,"window_w":0.16,"window_h":0.36,"opening":"#2c3642"}'),
  ('tower_cornice','box','mat.ashlar',0,'','{"y":3.35,"z":0.45,"w":0.9,"d":0.9,"h":0.08,"far":0}'),
  ('belfry','box','mat.lime_plaster',0,'','{"y":3.43,"z":0.45,"w":0.62,"d":0.62,"h":0.62,"windows":1,"side_windows":1,"window_y":0.16,"window_w":0.18,"window_h":0.36,"opening":"#2a221c"}'),
  ('clock','shield','mat.canvas',0,'','{"y":3.08,"z":0.86,"w":0.3,"t":0.02,"boss":0.04,"segments":8,"rim":"#3a3530","far":0}'),
  ('lantern','cylinder','mat.ashlar',0,'','{"y":4.05,"z":0.45,"w":0.5,"h":0.42,"segments":8}'),
  ('spire','cone','mat.slate',0,'','{"y":4.47,"z":0.45,"w":0.56,"h":1.1,"segments":8}'),
  ('spire_tip','cone','mat.cloth',1,'','{"y":5,"z":0.45,"w":0.3,"h":0.6,"segments":8}'),
  ('finial','cone','mat.brass',0,'','{"y":5.55,"z":0.45,"w":0.06,"h":0.3,"segments":4,"far":0}'),
  ('columns','column_row','mat.ashlar',0,'','{"y":0.45,"z":1.42,"count":4,"radius":0.11,"spacing":0.56,"h":1.7,"segments":8,"capital":1}'),
  ('entablature','box','mat.ashlar',0,'','{"y":2.15,"z":1.2,"w":2.2,"d":0.62,"h":0.2}'),
  ('pediment','box','mat.ashlar',0,'','{"y":2.35,"z":1.2,"yaw":90,"w":0.62,"d":2.2,"h":0.02,"gable":26}'),
  ('portico_roof','gable_roof','mat.cloth',1,'','{"y":2.37,"z":1.2,"yaw":90,"w":0.62,"d":2.2,"pitch":26,"overhang":0.05,"t":0.05}'),
  ('tympanum','box','mat.cloth',1,'','{"y":2.39,"z":1.52,"yaw":90,"w":0.02,"d":1.7,"h":0.01,"gable":26,"far":0}'),
  ('portal','box','mat.oak',0,'','{"y":0.45,"z":1.02,"w":0.6,"d":0.04,"h":1.05,"tint":"#5a3d26","far":0}'),
  ('curtain','box','mat.cloth',1,'','{"y":1.1,"z":1.05,"w":0.5,"d":0.02,"h":0.4,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.temple_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.temple'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.temple_gunpowder'), (SELECT id FROM def WHERE key='mat.lime_plaster'), 100);
