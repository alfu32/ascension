-- bld.fort@gunpowder: Early modern fort (mesh.fort_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.fort_gunpowder','Early modern fort');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.fort_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":5.6,"d":5.6,"h":0.05,"steps":1}'),
  ('curtain','wall','mat.ashlar',0,'','{"w":4.6,"d":4.6,"h":1.2,"t":0.45,"ring":1,"gap":1}'),
  ('parapet','wall','mat.ashlar',0,'','{"y":1.2,"w":4.66,"d":4.66,"h":0.2,"t":0.3,"ring":1,"gap":1,"tint":"#e0d8c4","far":0}'),
  ('rondel_nw','cylinder','mat.ashlar',0,'','{"x":-2.2,"z":-2.2,"w":1.4,"h":1.25,"segments":12,"taper":0.06}'),
  ('rondel_nw_cordon','cylinder','mat.cloth',1,'','{"x":-2.2,"y":1.15,"z":-2.2,"w":1.42,"h":0.14,"segments":12}'),
  ('rondel_nw_parapet','cylinder','mat.ashlar',0,'','{"x":-2.2,"y":1.29,"z":-2.2,"w":1.36,"h":0.18,"segments":12,"tint":"#e0d8c4","far":0}'),
  ('rondel_nw_floor','cylinder','mat.packed_earth',0,'','{"x":-2.2,"y":1.47,"z":-2.2,"w":1.08,"h":0.01,"segments":12,"far":0}'),
  ('rondel_ne','cylinder','mat.ashlar',0,'','{"x":2.2,"z":-2.2,"w":1.4,"h":1.25,"segments":12,"taper":0.06}'),
  ('rondel_ne_cordon','cylinder','mat.cloth',1,'','{"x":2.2,"y":1.15,"z":-2.2,"w":1.42,"h":0.14,"segments":12}'),
  ('rondel_ne_parapet','cylinder','mat.ashlar',0,'','{"x":2.2,"y":1.29,"z":-2.2,"w":1.36,"h":0.18,"segments":12,"tint":"#e0d8c4","far":0}'),
  ('rondel_ne_floor','cylinder','mat.packed_earth',0,'','{"x":2.2,"y":1.47,"z":-2.2,"w":1.08,"h":0.01,"segments":12,"far":0}'),
  ('rondel_sw','cylinder','mat.ashlar',0,'','{"x":-2.2,"z":2.2,"w":1.4,"h":1.25,"segments":12,"taper":0.06}'),
  ('rondel_sw_cordon','cylinder','mat.cloth',1,'','{"x":-2.2,"y":1.15,"z":2.2,"w":1.42,"h":0.14,"segments":12}'),
  ('rondel_sw_parapet','cylinder','mat.ashlar',0,'','{"x":-2.2,"y":1.29,"z":2.2,"w":1.36,"h":0.18,"segments":12,"tint":"#e0d8c4","far":0}'),
  ('rondel_sw_floor','cylinder','mat.packed_earth',0,'','{"x":-2.2,"y":1.47,"z":2.2,"w":1.08,"h":0.01,"segments":12,"far":0}'),
  ('rondel_se','cylinder','mat.ashlar',0,'','{"x":2.2,"z":2.2,"w":1.4,"h":1.25,"segments":12,"taper":0.06}'),
  ('rondel_se_cordon','cylinder','mat.cloth',1,'','{"x":2.2,"y":1.15,"z":2.2,"w":1.42,"h":0.14,"segments":12}'),
  ('rondel_se_parapet','cylinder','mat.ashlar',0,'','{"x":2.2,"y":1.29,"z":2.2,"w":1.36,"h":0.18,"segments":12,"tint":"#e0d8c4","far":0}'),
  ('rondel_se_floor','cylinder','mat.packed_earth',0,'','{"x":2.2,"y":1.47,"z":2.2,"w":1.08,"h":0.01,"segments":12,"far":0}'),
  ('guns','cylinder','mat.iron',0,'','{"x":-2.2,"y":1.36,"z":2.55,"w":0.11,"h":0.45,"segments":6,"pitch":86,"repeat":2,"dx":4.4,"tint":"#4a4e56","far":0}'),
  ('gate','box','mat.ashlar',0,'','{"z":2.15,"w":1.4,"d":0.6,"h":1.6,"door":0.6,"door_h":0.95,"door_colour":"#2a221c"}'),
  ('gate_pediment','box','mat.ashlar',0,'','{"y":1.6,"z":2.15,"yaw":90,"w":0.6,"d":1.4,"h":0.02,"gable":26,"far":0}'),
  ('gate_arms','shield','mat.cloth',1,'','{"y":1.2,"z":2.47,"w":0.36,"h":0.42,"t":0.03,"boss":0,"point":0.4,"rim":"#d6cbb0","far":0}'),
  ('keep','box','mat.lime_plaster',0,'','{"z":-0.6,"w":2,"d":1.8,"h":2.4,"door":0.45,"door_h":0.8,"windows":3,"window_y":0.3,"window_w":0.16,"window_h":0.36,"window_rows":2,"row_gap":0.95,"side_windows":2,"opening":"#2c3642"}'),
  ('keep_quoins','box','mat.ashlar',0,'','{"x":-1,"z":0.3,"w":0.1,"d":0.1,"h":2.4,"repeat":2,"dx":2,"far":0}'),
  ('keep_cornice','box','mat.ashlar',0,'','{"y":2.4,"z":-0.6,"w":2.1,"d":1.9,"h":0.08,"far":0}'),
  ('keep_roof','hip_roof','mat.cloth',1,'','{"y":2.48,"z":-0.6,"w":2,"d":1.8,"pitch":42,"overhang":0.14,"t":0.08}'),
  ('keep_banner','banner','mat.cloth',1,'','{"y":3.3,"z":-0.6,"h":1.8,"fw":1,"fh":0.8}'),
  ('barracks','box','mat.brick',0,'','{"x":-1.55,"z":0.2,"yaw":90,"w":2.4,"d":0.8,"h":0.9}'),
  ('barracks_roof','gable_roof','mat.pantile',0,'','{"x":-1.55,"y":0.9,"z":0.2,"yaw":90,"w":2.4,"d":0.8,"pitch":38,"overhang":0.08,"t":0.06}'),
  ('magazine','box','mat.ashlar',0,'','{"x":1.5,"z":0.5,"w":0.9,"d":0.9,"h":0.8}'),
  ('magazine_roof','hip_roof','mat.slate',0,'','{"x":1.5,"y":0.8,"z":0.5,"w":0.9,"d":0.9,"pitch":40,"overhang":0.06,"t":0.06,"far":0}'),
  ('banners','banner','mat.cloth',1,'','{"x":-2.2,"y":1.47,"z":-2.2,"h":1.5,"fw":0.5,"fh":0.4,"repeat":2,"dx":4.4}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.fort_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.fort'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.fort_gunpowder'), (SELECT id FROM def WHERE key='mat.ashlar'), 100);
