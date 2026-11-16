-- unit.missile_launcher@information: Missile launcher (mesh.missile_launcher_information)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.missile_launcher_information','Missile launcher');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.missile_launcher_information';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('track_w','track','mat.cast_iron',0,'','{"x":-0.5,"l":2.1,"h":0.46,"t":0.3,"wheels":5,"wheel_colour":"#4a503c"}'),
  ('track_e','track','mat.cast_iron',0,'','{"x":0.5,"l":2.1,"h":0.46,"t":0.3,"wheels":5,"wheel_colour":"#4a503c"}'),
  ('hull','box','mat.nato_green',0,'','{"y":0.14,"z":-0.05,"w":0.76,"d":1.9,"h":0.48}'),
  ('glacis','box','mat.nato_green',0,'','{"y":0.16,"z":0.92,"w":0.76,"d":0.05,"h":0.5,"pitch":-60}'),
  ('fenders','box','mat.nato_green',0,'','{"y":0.58,"w":1.36,"d":2.14,"h":0.05}'),
  ('cab','box','mat.nato_green',0,'','{"y":0.62,"z":0.62,"w":0.7,"d":0.5,"h":0.24,"taper":0.1,"windows":1,"window_y":0.12,"window_w":0.4,"window_h":0.07,"opening":"#22262a"}'),
  ('turntable','cylinder','mat.nato_green',0,'','{"y":0.62,"z":-0.3,"w":0.7,"h":0.12,"segments":10,"traverse":40}'),
  ('pylons','box','mat.nato_green',0,'turntable','{"x":-0.3,"y":0.12,"w":0.08,"d":0.36,"h":0.42,"repeat":2,"dx":0.6}'),
  ('pod_w','box','mat.nato_green',1,'turntable','{"x":-0.5,"y":0.42,"z":-0.05,"w":0.32,"d":0.95,"h":0.32,"pitch":-28,"windows":2,"window_rows":2,"window_w":0.09,"window_h":0.09,"window_y":0.07,"row_gap":0.14,"opening":"#1c1e20"}'),
  ('pod_e','box','mat.nato_green',1,'turntable','{"x":0.5,"y":0.42,"z":-0.05,"w":0.32,"d":0.95,"h":0.32,"pitch":-28,"windows":2,"window_rows":2,"window_w":0.09,"window_h":0.09,"window_y":0.07,"row_gap":0.14,"opening":"#1c1e20"}'),
  ('mast','cylinder','mat.nato_green',0,'turntable','{"z":-0.1,"y":0.12,"w":0.08,"h":0.55,"segments":5}'),
  ('radar','propeller','mat.composite',0,'mast','{"y":0.6,"w":0.8,"blades":2,"chord":0.14,"pitch":-90}'),
  ('spark','cone','mat.flash',0,'pod_e','{"y":0.16,"z":0.48,"pitch":90,"w":0.12,"h":0.12,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.32,"w":0.36,"h":0.32,"segments":6,"pitch":180,"far":0}'),
  ('missile','cylinder','mat.white_metal',0,'spark','{"y":1,"w":0.07,"h":0.3,"segments":5,"far":0}'),
  ('trail','cone','mat.canvas',0,'missile','{"y":-0.88,"w":0.2,"h":0.9,"segments":5,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.missile_launcher_information'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.missile_launcher'), NULL, (SELECT id FROM def WHERE key='age.information'), (SELECT id FROM def WHERE key='mesh.missile_launcher_information'), (SELECT id FROM def WHERE key='mat.nato_green'), 100);
