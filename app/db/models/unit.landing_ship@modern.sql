-- unit.landing_ship@modern: Landing ship (mesh.landing_ship_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.landing_ship_modern','Landing ship');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.landing_ship_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','hull','mat.haze_grey',0,'','{"yaw":-90,"y":-0.12,"w":4.2,"d":1.05,"h":0.5,"sheer":0.04,"segments":12,"float":1}'),
  ('bow_doors','box','mat.haze_grey',0,'hull','{"x":1.7,"y":0.0,"w":0.5,"d":0.8,"h":0.6,"taper":0.3}'),
  ('ramp','box','mat.steel',0,'hull','{"x":1.95,"y":0.22,"w":0.06,"d":0.5,"h":0.4,"roll":18,"far":0}'),
  ('bulwark','box','mat.haze_grey',0,'hull','{"x":0.3,"y":0.42,"z":-0.49,"w":2.6,"d":0.04,"h":0.2,"repeat":2,"dz":0.98}'),
  ('band','box','mat.cloth',1,'hull','{"x":0.1,"y":0.22,"z":-0.51,"w":3.2,"d":0.03,"h":0.18,"repeat":2,"dz":1.02}'),
  ('tank_deck','box','mat.tarmac',0,'hull','{"x":0.35,"y":0.42,"w":2.6,"d":0.9,"h":0.03}'),
  ('vehicles','box','mat.olive_drab',0,'hull','{"x":-0.3,"y":0.45,"z":-0.18,"w":0.42,"d":0.24,"h":0.16,"repeat":3,"dx":0.55,"far":0}'),
  ('vehicles_port','box','mat.olive_drab',0,'hull','{"x":-0.3,"y":0.45,"z":0.18,"w":0.42,"d":0.24,"h":0.16,"repeat":3,"dx":0.55,"far":0}'),
  ('deckhouse','box','mat.haze_grey',0,'hull','{"x":-1.35,"y":0.42,"w":0.9,"d":0.86,"h":0.42,"windows":0,"side_windows":4,"window_y":0.26,"window_w":0.07,"window_h":0.06,"opening":"#22262a"}'),
  ('house_band','box','mat.cloth',1,'hull','{"x":-1.35,"y":0.5,"w":0.92,"d":0.88,"h":0.1}'),
  ('bridge','box','mat.haze_grey',0,'hull','{"x":-1.25,"y":0.84,"w":0.5,"d":0.6,"h":0.28,"windows":3,"window_y":0.14,"window_w":0.08,"window_h":0.07,"opening":"#22262a"}'),
  ('bridge_roof','box','mat.cloth',1,'hull','{"x":-1.25,"y":1.12,"w":0.52,"d":0.62,"h":0.04,"far":0}'),
  ('mast','cylinder','mat.haze_grey',0,'hull','{"x":-1.1,"y":1.12,"w":0.05,"h":0.9,"segments":5}'),
  ('yard','box','mat.haze_grey',0,'mast','{"y":0.7,"w":0.04,"d":0.5,"h":0.04,"far":0}'),
  ('stacks','cylinder','mat.haze_grey',0,'hull','{"x":-1.6,"y":0.84,"z":-0.22,"w":0.12,"h":0.32,"segments":6,"repeat":2,"dz":0.44,"far":0}'),
  ('gun_tubs','cylinder','mat.haze_grey',0,'hull','{"x":1.2,"y":0.42,"z":-0.32,"w":0.2,"h":0.12,"segments":8,"repeat":2,"dz":0.64,"far":0}'),
  ('ensign','box','mat.cloth',1,'hull','{"x":-2.05,"y":0.8,"w":0.32,"d":0.015,"h":0.22,"far":0}'),
  ('ensign_staff','cylinder','mat.haze_grey',0,'hull','{"x":-1.95,"y":0.42,"w":0.03,"h":0.6,"segments":4,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.landing_ship_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.landing_ship'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.landing_ship_modern'), (SELECT id FROM def WHERE key='mat.haze_grey'), 100);
