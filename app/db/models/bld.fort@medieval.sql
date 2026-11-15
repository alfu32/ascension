-- bld.fort@medieval: Medieval fort (mesh.fort_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.fort_medieval','Medieval fort');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.fort_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":5.6,"d":5.6,"h":0.05,"steps":1}'),
  ('curtain','wall','mat.rough_stone',0,'','{"w":4.8,"d":4.8,"h":1.6,"t":0.32,"ring":1,"gap":1.2}'),
  ('curtain_merlons','crenellation','mat.rough_stone',0,'','{"y":1.6,"w":4.8,"d":4.8,"h":0.3,"t":0.32,"merlon":0.44,"ring":1,"gap":1.2,"far":0}'),
  ('tower_nw','include',NULL,0,'','{"mesh":"mesh.round_tower","x":-2.38,"z":-2.38,"scale":0.92}'),
  ('tower_ne','include',NULL,0,'','{"mesh":"mesh.round_tower","x":2.38,"z":-2.38,"scale":0.92}'),
  ('tower_sw','include',NULL,0,'','{"mesh":"mesh.round_tower","x":-2.38,"z":2.38,"scale":0.92}'),
  ('tower_se','include',NULL,0,'','{"mesh":"mesh.round_tower","x":2.38,"z":2.38,"scale":0.92}'),
  ('gate','arch','mat.dressed_stone',0,'','{"z":2.25,"w":1.3,"h":1.95,"t":0.4,"span":0.8,"spring":0.95}'),
  ('gate_top','crenellation','mat.dressed_stone',0,'','{"y":1.95,"z":2.25,"w":1.3,"h":0.28,"t":0.4,"merlon":0.26,"far":0}'),
  ('gate_towers','include',NULL,0,'','{"mesh":"mesh.round_tower","x":-0.95,"z":2.35,"scale":0.82}'),
  ('gate_towers_e','include',NULL,0,'','{"mesh":"mesh.round_tower","x":0.95,"z":2.35,"scale":0.82}'),
  ('portcullis','box','mat.iron',0,'','{"y":0.75,"z":2.25,"w":0.78,"d":0.04,"h":0.95,"tint":"#5a5e66","far":0}'),
  ('keep','box','mat.dressed_stone',0,'','{"z":-0.5,"w":2.3,"d":2.3,"h":3.6,"door":0.5,"door_h":0.9,"windows":2,"side_windows":2,"window_y":1.5,"window_w":0.1,"window_h":0.42,"window_rows":2,"row_gap":1}'),
  ('keep_corbel','box','mat.dressed_stone',0,'','{"y":3.6,"z":-0.5,"w":2.5,"d":2.5,"h":0.22,"tint":"#d8d2c8","far":0}'),
  ('keep_merlons','crenellation','mat.dressed_stone',0,'','{"y":3.82,"z":-0.5,"w":2.5,"d":2.5,"h":0.32,"t":0.18,"merlon":0.34,"ring":1,"far":0}'),
  ('keep_roof','hip_roof','mat.slate',0,'','{"y":3.82,"z":-0.5,"w":1.6,"d":1.6,"pitch":50,"overhang":0,"t":0.06}'),
  ('turrets','cylinder','mat.dressed_stone',0,'','{"x":-1.2,"y":3.3,"z":-1.7,"w":0.5,"h":1,"segments":6,"repeat":2,"dx":2.4,"far":0}'),
  ('turrets_s','cylinder','mat.dressed_stone',0,'','{"x":-1.2,"y":3.3,"z":0.7,"w":0.5,"h":1,"segments":6,"repeat":2,"dx":2.4,"far":0}'),
  ('turret_roofs','cone','mat.cloth',1,'','{"x":-1.2,"y":4.3,"z":-1.7,"w":0.62,"h":0.6,"segments":6,"repeat":2,"dx":2.4}'),
  ('turret_roofs_s','cone','mat.cloth',1,'','{"x":-1.2,"y":4.3,"z":0.7,"w":0.62,"h":0.6,"segments":6,"repeat":2,"dx":2.4}'),
  ('keep_banner','banner','mat.cloth',1,'','{"y":4.95,"z":-0.5,"h":1.8,"fw":1,"fh":0.95}'),
  ('hanging','box','mat.cloth',1,'','{"y":2.1,"z":0.66,"w":0.7,"d":0.03,"h":1.25}'),
  ('banners','banner','mat.cloth',1,'','{"x":-1.6,"z":1.45,"h":2.6,"fw":0.55,"fh":0.85,"repeat":2,"dx":3.2}'),
  ('hall','box','mat.rough_stone',0,'','{"x":-1.6,"z":-0.4,"yaw":90,"w":2.4,"d":0.8,"h":1.1,"gable":45}'),
  ('hall_roof','gable_roof','mat.slate',0,'','{"x":-1.6,"y":1.1,"z":-0.4,"yaw":90,"w":2.4,"d":0.8,"pitch":45,"overhang":0.08,"t":0.08}'),
  ('hall_roof_cap','gable_roof','mat.cloth',1,'','{"x":-1.6,"y":1.433,"z":-0.4,"yaw":90,"w":2.6,"d":0.36,"pitch":45,"overhang":0,"t":0.03}'),
  ('well','cylinder','mat.rough_stone',0,'','{"x":1.4,"z":1,"w":0.5,"h":0.3,"segments":8,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.fort_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.fort'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.fort_medieval'), (SELECT id FROM def WHERE key='mat.dressed_stone'), 100);
