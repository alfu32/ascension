-- unit.rocket_artillery@information: Rocket artillery (mesh.rocket_artillery_information)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.rocket_artillery_information','Rocket artillery');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.rocket_artillery_information';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('wheels_front','wheel','mat.rubber',0,'','{"x":-0.52,"z":0.9,"w":0.62,"t":0.18,"spokes":1,"segments":10,"repeat":2,"dx":1.04}'),
  ('wheels_mid','wheel','mat.rubber',0,'','{"x":-0.52,"z":-0.35,"w":0.62,"t":0.18,"spokes":1,"segments":10,"repeat":2,"dx":1.04}'),
  ('wheels_back','wheel','mat.rubber',0,'','{"x":-0.52,"z":-0.98,"w":0.62,"t":0.18,"spokes":1,"segments":10,"repeat":2,"dx":1.04}'),
  ('chassis','box','mat.cast_iron',0,'','{"y":0.24,"w":0.74,"d":2.6,"h":0.14,"far":0}'),
  ('cab','box','mat.nato_green',0,'','{"y":0.36,"z":0.95,"w":1.08,"d":0.62,"h":0.74,"windows":2,"window_y":0.48,"window_w":0.4,"window_h":0.17,"side_windows":1,"opening":"#22262a"}'),
  ('cab_roof','box','mat.cloth',1,'','{"y":1.1,"z":0.95,"w":1.1,"d":0.64,"h":0.04}'),
  ('guards','box','mat.nato_green',0,'','{"x":-0.52,"y":0.66,"z":-0.66,"w":0.24,"d":0.95,"h":0.04,"repeat":2,"dx":1.04,"far":0}'),
  ('bed','box','mat.nato_green',0,'','{"y":0.38,"z":-0.5,"w":1,"d":1.5,"h":0.2}'),
  ('turntable','box','mat.nato_green',0,'','{"y":0.58,"z":-0.7,"w":0.8,"d":0.5,"h":0.14,"traverse":20}'),
  ('cradle','box','mat.nato_green',0,'turntable','{"y":0.14,"z":-0.05,"w":0.3,"d":0.3,"h":0.3,"far":0}'),
  ('pod','box','mat.nato_green',0,'turntable','{"y":0.36,"z":0.15,"w":0.98,"d":1.4,"h":0.5,"pitch":-25,"windows":3,"window_rows":2,"window_w":0.16,"window_h":0.12,"window_y":0.1,"row_gap":0.2,"opening":"#1c1e20"}'),
  ('pod_band','box','mat.cloth',1,'pod','{"y":-0.01,"z":-0.3,"w":1,"d":0.28,"h":0.52}'),
  ('jacks','box','mat.cast_iron',0,'','{"x":-0.58,"y":0.04,"z":-1.2,"w":0.06,"d":0.06,"h":0.4,"repeat":2,"dx":1.16,"far":0}'),
  ('spark','cone','mat.flash',0,'pod','{"y":0.25,"z":0.7,"pitch":90,"w":0.144,"h":0.144,"segments":3,"flash":4,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.384,"w":0.432,"h":0.384,"segments":6,"pitch":180,"far":0}'),
  ('missile','cylinder','mat.white_metal',0,'spark','{"y":1.2,"w":0.084,"h":0.36,"segments":5,"far":0}'),
  ('trail','cone','mat.canvas',0,'missile','{"y":-1.08,"w":0.24,"h":1.1,"segments":5,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.rocket_artillery_information'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.rocket_artillery'), NULL, (SELECT id FROM def WHERE key='age.information'), (SELECT id FROM def WHERE key='mesh.rocket_artillery_information'), (SELECT id FROM def WHERE key='mat.nato_green'), 100);
