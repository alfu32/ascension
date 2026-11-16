-- bld.farm@automation: Future farm (mesh.farm_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.farm_automation','Future farm');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.farm_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('soil','plinth','mat.soil',0,'','{"w":3.8,"d":3.8,"h":0.03,"steps":1}'),
  ('beds','box','mat.white_composite',0,'','{"y":0.03,"z":-0.05,"w":3.3,"d":0.34,"h":0.1,"repeat":4,"dz":0.5}'),
  ('crops','box','mat.greens',0,'','{"y":0.13,"z":-0.05,"w":3.2,"d":0.28,"h":0.12,"repeat":4,"dz":0.5}'),
  ('gantry_legs','box','mat.white_composite',0,'','{"x":-1.72,"y":0.03,"z":0.7,"w":0.08,"d":0.12,"h":0.8,"repeat":2,"dx":3.44}'),
  ('gantry','box','mat.white_composite',1,'','{"y":0.83,"z":0.7,"w":3.56,"d":0.2,"h":0.12}'),
  ('gantry_glow','box','mat.light_strip',0,'','{"y":0.8,"z":0.7,"w":3.3,"d":0.1,"h":0.03,"far":0}'),
  ('gantry_head','box','mat.graphite',0,'','{"x":0.4,"y":0.55,"z":0.7,"w":0.22,"d":0.24,"h":0.25,"far":0}'),
  ('barn','box','mat.composite_panel',0,'','{"x":-1,"y":0.03,"z":-1.2,"w":1.6,"d":1.1,"h":0.6,"door":0.5,"door_h":0.45,"door_colour":"#3a4650"}'),
  ('barn_vault','cylinder','mat.white_composite',0,'','{"x":-1.8,"y":0.63,"z":-1.2,"w":1.1,"h":1.6,"segments":14,"roll":90}'),
  ('barn_band','box','mat.white_composite',1,'','{"x":-1,"y":0.55,"z":-1.2,"w":1.64,"d":1.14,"h":0.12}'),
  ('barn_solar','box','mat.solar_panel',0,'','{"x":-1,"y":1.17,"z":-1.2,"w":1.4,"d":0.36,"h":0.03,"far":0}'),
  ('silo','include','mat.white_composite',0,'','{"mesh":"mesh.silo_future","x":1.4,"y":0.03,"z":-1.35,"scale":0.85}'),
  ('tank','cylinder','mat.white_composite',0,'','{"x":0.35,"y":0.03,"z":-1.3,"w":0.7,"h":0.5,"segments":12}'),
  ('tank_top','dome','mat.white_composite',0,'','{"x":0.35,"y":0.53,"z":-1.3,"w":0.7,"h":0.2,"segments":12,"far":0}'),
  ('tank_band','cylinder','mat.white_composite',1,'','{"x":0.35,"y":0.3,"z":-1.3,"w":0.72,"h":0.1,"segments":12}'),
  ('flag','banner','mat.cloth',1,'','{"x":-0.15,"y":0.03,"z":-0.55,"h":1.9,"fw":0.45,"fh":0.32,"pole_colour":"#e8ecec"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.farm_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.farm'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.farm_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
