-- bld.auto_mine@automation: Automated Mine (mesh.auto_mine_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.auto_mine_automation','Automated Mine');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.auto_mine_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.cinders',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('deck','box','mat.white_composite',0,'','{"x":-0.7,"y":0.05,"z":-0.6,"w":1.4,"d":1.4,"h":0.25}'),
  ('deck_glow','box','mat.light_strip',0,'','{"x":-0.7,"y":0.18,"z":-0.6,"w":1.42,"d":1.42,"h":0.03,"far":0}'),
  ('leg_1','box','mat.white_composite',0,'','{"x":-1.2,"y":0.3,"z":-1.1,"w":0.12,"d":0.12,"h":3.3,"roll":5,"pitch":5}'),
  ('leg_2','box','mat.white_composite',0,'','{"x":-0.2,"y":0.3,"z":-1.1,"w":0.12,"d":0.12,"h":3.3,"roll":-5,"pitch":5}'),
  ('leg_3','box','mat.white_composite',0,'','{"x":-1.2,"y":0.3,"z":-0.1,"w":0.12,"d":0.12,"h":3.3,"roll":5,"pitch":-5}'),
  ('leg_4','box','mat.white_composite',0,'','{"x":-0.2,"y":0.3,"z":-0.1,"w":0.12,"d":0.12,"h":3.3,"roll":-5,"pitch":-5}'),
  ('band','box','mat.white_composite',1,'','{"x":-0.7,"y":1.4,"z":-0.6,"w":0.86,"d":0.86,"h":0.12}'),
  ('band_2','box','mat.white_composite',1,'','{"x":-0.7,"y":2.5,"z":-0.6,"w":0.66,"d":0.66,"h":0.12}'),
  ('crown','box','mat.white_composite',1,'','{"x":-0.7,"y":3.55,"z":-0.6,"w":0.56,"d":0.56,"h":0.3}'),
  ('crown_light','box','mat.light_strip',0,'','{"x":-0.7,"y":3.85,"z":-0.6,"w":0.12,"d":0.12,"h":0.1,"far":0}'),
  ('drill','cylinder','mat.graphite',0,'','{"x":-0.7,"y":0.3,"z":-0.6,"w":0.16,"h":3.3,"segments":6}'),
  ('drill_head','cylinder','mat.plasma',0,'','{"x":-0.7,"y":0.3,"z":-0.6,"w":0.3,"h":0.05,"segments":8,"far":0}'),
  ('store','dome','mat.white_composite',0,'','{"x":0.95,"y":0.05,"z":-0.85,"w":1.5,"h":1.15,"segments":14}'),
  ('store_band','cylinder','mat.white_composite',1,'','{"x":0.95,"y":0.05,"z":-0.85,"w":1.52,"h":0.22,"segments":14}'),
  ('store_glow','cylinder','mat.light_strip',0,'','{"x":0.95,"y":0.27,"z":-0.85,"w":1.5,"h":0.03,"segments":14,"far":0}'),
  ('conveyor','box','mat.white_composite',0,'','{"x":0.1,"y":0.8,"z":-0.75,"w":1.2,"d":0.2,"h":0.12,"roll":-25}'),
  ('spoil','cone','mat.spoil',0,'','{"x":-1.05,"y":0.05,"z":1.05,"w":1.3,"h":0.8,"segments":8}'),
  ('hauler','box','mat.white_composite',0,'','{"x":0.55,"y":0.2,"z":0.95,"w":0.5,"d":0.95,"h":0.16}'),
  ('hauler_bed','box','mat.white_composite',1,'','{"x":0.55,"y":0.36,"z":0.95,"w":0.5,"d":0.8,"h":0.24,"taper":0.12}'),
  ('hauler_ore','cone','mat.ore',0,'','{"x":0.55,"y":0.6,"z":0.95,"w":0.4,"h":0.12,"segments":6,"far":0}'),
  ('hauler_glow','box','mat.light_strip',0,'','{"x":0.55,"y":0.24,"z":1.43,"w":0.4,"d":0.02,"h":0.04,"far":0}'),
  ('hauler_wheels','cylinder','mat.rubber',0,'','{"x":0.28,"y":0.17,"z":0.65,"w":0.32,"h":0.1,"segments":8,"roll":90,"repeat":2,"dx":0.44,"far":0}'),
  ('hauler_wheels_f','cylinder','mat.rubber',0,'','{"x":0.28,"y":0.17,"z":1.25,"w":0.32,"h":0.1,"segments":8,"roll":90,"repeat":2,"dx":0.44,"far":0}'),
  ('solar','box','mat.solar_panel',0,'','{"x":1.35,"y":0.3,"z":0.6,"w":0.7,"d":0.55,"h":0.03,"pitch":22,"repeat":2,"dz":0.62,"far":0}'),
  ('solar_posts','box','mat.graphite',0,'','{"x":1.35,"y":0.05,"z":0.6,"w":0.05,"d":0.05,"h":0.28,"repeat":2,"dz":0.62,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":-1.6,"y":0.05,"z":0.25,"h":2.4,"fw":0.5,"fh":0.75,"pole_colour":"#e8ecec"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.auto_mine_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.auto_mine'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.auto_mine_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
