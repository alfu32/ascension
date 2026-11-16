-- bld.fortress@automation: Future fortress (mesh.fortress_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.fortress_automation','Future fortress');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.fortress_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.paving',0,'','{"w":5.6,"d":5.6,"h":0.05,"steps":1}'),
  ('walls','wall','mat.composite_panel',0,'','{"y":0.05,"w":5,"d":5,"h":1,"t":0.3,"ring":1,"gap":1}'),
  ('wall_band','wall','mat.white_composite',1,'','{"y":0.78,"w":5,"d":5,"h":0.14,"t":0.34,"ring":1,"gap":1,"far":0}'),
  ('wall_glow','wall','mat.light_strip',0,'','{"y":0.6,"w":5,"d":5,"h":0.03,"t":0.32,"ring":1,"gap":1,"far":0}'),
  ('corners_n','cylinder','mat.white_composite',0,'','{"x":-2.3,"y":0.05,"z":-2.3,"w":1.1,"h":1.35,"segments":8,"repeat":2,"dx":4.6}'),
  ('corners_s','cylinder','mat.white_composite',0,'','{"x":-2.3,"y":0.05,"z":2.3,"w":1.1,"h":1.35,"segments":8,"repeat":2,"dx":4.6}'),
  ('slits_n','cylinder','mat.graphite',0,'','{"x":-2.3,"y":0.95,"z":-2.3,"w":1.12,"h":0.07,"segments":10,"repeat":2,"dx":4.6,"far":0}'),
  ('slits_s','cylinder','mat.graphite',0,'','{"x":-2.3,"y":0.95,"z":2.3,"w":1.12,"h":0.07,"segments":10,"repeat":2,"dx":4.6,"far":0}'),
  ('cupolas_n','dome','mat.white_composite',1,'','{"x":-2.3,"y":1.4,"z":-2.3,"w":1.1,"h":0.4,"segments":8,"repeat":2,"dx":4.6}'),
  ('cupolas_s','dome','mat.white_composite',1,'','{"x":-2.3,"y":1.4,"z":2.3,"w":1.1,"h":0.4,"segments":8,"repeat":2,"dx":4.6}'),
  ('gun_se','cylinder','mat.graphite',0,'','{"x":2.3,"y":1.62,"z":2.3,"yaw":45,"w":0.1,"h":0.8,"segments":6,"pitch":84,"far":0}'),
  ('gun_ne','cylinder','mat.graphite',0,'','{"x":2.3,"y":1.62,"z":-2.3,"yaw":135,"w":0.1,"h":0.8,"segments":6,"pitch":84,"far":0}'),
  ('gun_nw','cylinder','mat.graphite',0,'','{"x":-2.3,"y":1.62,"z":-2.3,"yaw":225,"w":0.1,"h":0.8,"segments":6,"pitch":84,"far":0}'),
  ('gun_sw','cylinder','mat.graphite',0,'','{"x":-2.3,"y":1.62,"z":2.3,"yaw":-45,"w":0.1,"h":0.8,"segments":6,"pitch":84,"far":0}'),
  ('keep','box','mat.composite_panel',0,'','{"y":0.05,"z":-0.45,"w":2.6,"d":2,"h":1.6,"door":0.5,"door_h":0.8,"door_colour":"#2c3a40"}'),
  ('keep_glass','box','mat.smart_glass',0,'','{"y":1,"z":-0.45,"w":2.62,"d":2.02,"h":0.22,"far":0}'),
  ('keep_roof','flat_roof','mat.white_composite',0,'','{"y":1.65,"z":-0.45,"w":2.6,"d":2,"t":0.05,"overhang":0.03}'),
  ('keep_upper','cylinder','mat.white_composite',0,'','{"y":1.7,"z":-0.65,"w":1.6,"h":0.75,"segments":14}'),
  ('keep_crown','cylinder','mat.white_composite',1,'','{"y":2.45,"z":-0.65,"w":1.64,"h":0.2,"segments":14,"far":0}'),
  ('keep_dome','dome','mat.smart_glass',0,'','{"y":2.65,"z":-0.65,"w":1.6,"h":0.55,"segments":14}'),
  ('radar_mast','cylinder','mat.white_composite',0,'','{"x":0.95,"y":1.7,"z":-1.1,"w":0.1,"h":1.9,"segments":6,"far":0}'),
  ('radar','dome','mat.white_composite',0,'','{"x":0.95,"y":3.6,"z":-1.1,"yaw":20,"w":0.95,"h":0.1,"segments":10,"pitch":45}'),
  ('radar_back','dome','mat.white_composite',0,'','{"x":0.95,"y":3.6,"z":-1.1,"yaw":20,"w":0.95,"h":0.25,"segments":10,"pitch":-135,"far":0}'),
  ('radar_light','box','mat.light_strip',0,'','{"x":0.95,"y":3.62,"z":-1.1,"w":0.07,"d":0.07,"h":0.07,"far":0}'),
  ('pit_w','cylinder','mat.white_composite',0,'','{"x":-1.35,"y":0.05,"z":1.35,"w":1,"h":0.55,"segments":10,"far":0}'),
  ('turret_w','dome','mat.white_composite',0,'','{"x":-1.35,"y":0.6,"z":1.35,"w":0.7,"h":0.36,"segments":10,"far":0}'),
  ('guns_w','cylinder','mat.graphite',0,'turret_w','{"x":-0.09,"y":0.18,"z":0.15,"w":0.07,"h":0.85,"segments":5,"pitch":72,"repeat":2,"dx":0.18,"far":0}'),
  ('pit_e','cylinder','mat.white_composite',0,'','{"x":1.35,"y":0.05,"z":1.35,"w":1,"h":0.55,"segments":10,"far":0}'),
  ('turret_e','dome','mat.white_composite',0,'','{"x":1.35,"y":0.6,"z":1.35,"w":0.7,"h":0.36,"segments":10,"far":0}'),
  ('guns_e','cylinder','mat.graphite',0,'turret_e','{"x":-0.09,"y":0.18,"z":0.15,"w":0.07,"h":0.85,"segments":5,"pitch":72,"repeat":2,"dx":0.18,"far":0}'),
  ('gate_pylons','box','mat.white_composite',0,'','{"x":-0.62,"y":0.05,"z":2.5,"w":0.3,"d":0.42,"h":1.3,"repeat":2,"dx":1.24}'),
  ('gate_field','box','mat.light_strip',0,'','{"x":0,"y":0.7,"z":2.55,"w":0.95,"d":0.04,"h":0.06}'),
  ('hab_w','dome','mat.white_composite',0,'','{"x":-1.85,"y":0.05,"z":-1.6,"w":0.85,"h":0.55,"segments":10,"far":0}'),
  ('hab_e','dome','mat.white_composite',0,'','{"x":1.85,"y":0.05,"z":-1.6,"w":0.85,"h":0.55,"segments":10,"far":0}'),
  ('flag','banner','mat.cloth',1,'','{"x":0,"y":0.05,"z":1,"h":3.4,"fw":0.9,"fh":0.6,"pole_colour":"#e8ecec"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.fortress_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.fortress'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.fortress_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
