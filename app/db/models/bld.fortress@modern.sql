-- bld.fortress@modern: Fortress (mesh.fortress_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.fortress_modern','Fortress');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.fortress_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":5.6,"d":5.6,"h":0.05,"steps":1}'),
  ('walls','wall','mat.concrete',0,'','{"y":0.05,"w":5,"d":5,"h":1,"t":0.3,"ring":1,"gap":1}'),
  ('wall_band','wall','mat.cloth',1,'','{"y":0.78,"w":5,"d":5,"h":0.14,"t":0.34,"ring":1,"gap":1}'),
  ('corners_n','cylinder','mat.concrete',0,'','{"x":-2.3,"y":0.05,"z":-2.3,"w":1.1,"h":1.35,"segments":6,"repeat":2,"dx":4.6}'),
  ('corners_s','cylinder','mat.concrete',0,'','{"x":-2.3,"y":0.05,"z":2.3,"w":1.1,"h":1.35,"segments":6,"repeat":2,"dx":4.6}'),
  ('slits_n','cylinder','mat.crude',0,'','{"x":-2.3,"y":0.95,"z":-2.3,"w":1.11,"h":0.07,"segments":6,"repeat":2,"dx":4.6,"far":0}'),
  ('slits_s','cylinder','mat.crude',0,'','{"x":-2.3,"y":0.95,"z":2.3,"w":1.11,"h":0.07,"segments":6,"repeat":2,"dx":4.6,"far":0}'),
  ('corner_roofs_n','cylinder','mat.concrete',0,'','{"x":-2.3,"y":1.4,"z":-2.3,"w":1.26,"h":0.14,"segments":6,"repeat":2,"dx":4.6,"far":0}'),
  ('corner_roofs_s','cylinder','mat.concrete',0,'','{"x":-2.3,"y":1.4,"z":2.3,"w":1.26,"h":0.14,"segments":6,"repeat":2,"dx":4.6,"far":0}'),
  ('cupolas_n','dome','mat.cast_iron',1,'','{"x":-2.3,"y":1.54,"z":-2.3,"w":0.62,"h":0.3,"segments":8,"repeat":2,"dx":4.6,"far":0}'),
  ('cupolas_s','dome','mat.cast_iron',1,'','{"x":-2.3,"y":1.54,"z":2.3,"w":0.62,"h":0.3,"segments":8,"repeat":2,"dx":4.6,"far":0}'),
  ('gun_se','cylinder','mat.cast_iron',0,'','{"x":2.3,"y":1.68,"z":2.3,"w":0.1,"h":0.75,"segments":6,"yaw":45,"pitch":86,"far":0}'),
  ('gun_ne','cylinder','mat.cast_iron',0,'','{"x":2.3,"y":1.68,"z":-2.3,"w":0.1,"h":0.75,"segments":6,"yaw":135,"pitch":86,"far":0}'),
  ('gun_nw','cylinder','mat.cast_iron',0,'','{"x":-2.3,"y":1.68,"z":-2.3,"w":0.1,"h":0.75,"segments":6,"yaw":225,"pitch":86,"far":0}'),
  ('gun_sw','cylinder','mat.cast_iron',0,'','{"x":-2.3,"y":1.68,"z":2.3,"w":0.1,"h":0.75,"segments":6,"yaw":-45,"pitch":86,"far":0}'),
  ('keep','box','mat.concrete',0,'','{"y":0.05,"z":-0.45,"w":2.6,"d":2,"h":1.7,"door":0.5,"door_h":0.8,"door_colour":"#3a3e44","windows":4,"window_w":0.32,"window_h":0.09,"window_y":1.25,"side_windows":3,"opening":"#15181b"}'),
  ('keep_parapet','crenellation','mat.concrete',0,'','{"y":1.75,"z":-0.45,"w":2.6,"d":2,"h":0.2,"t":0.1,"merlon":0.3,"ring":1,"far":0}'),
  ('keep_upper','box','mat.concrete',0,'','{"y":1.75,"z":-0.65,"w":1.8,"d":1.3,"h":0.85,"windows":3,"window_w":0.3,"window_h":0.09,"window_y":0.45,"side_windows":2,"opening":"#15181b"}'),
  ('keep_crown','box','mat.cloth',1,'','{"y":2.6,"z":-0.65,"w":1.86,"d":1.36,"h":0.2}'),
  ('keep_roof','flat_roof','mat.cloth',1,'','{"y":2.8,"z":-0.65,"w":1.86,"d":1.36,"t":0.06,"overhang":0.03}'),
  ('radar_mast','cylinder','mat.steel',0,'','{"x":0.4,"y":2.86,"z":-0.75,"w":0.1,"h":2,"segments":6}'),
  ('radar_head','box','mat.steel',0,'','{"x":0.4,"y":4.86,"z":-0.75,"w":0.16,"d":0.16,"h":0.1}'),
  ('radar','box','mat.white_metal',0,'','{"x":0.4,"y":4.96,"z":-0.75,"yaw":20,"w":1,"d":0.06,"h":0.32,"pitch":-12}'),
  ('radar_light','box','mat.signal_red',0,'','{"x":0.4,"y":4.86,"z":-0.62,"w":0.07,"d":0.07,"h":0.07,"far":0}'),
  ('aerial','cylinder','mat.steel',0,'','{"x":-0.6,"y":2.86,"z":-0.95,"w":0.04,"h":1.3,"segments":4,"far":0}'),
  ('pit_w','cylinder','mat.concrete',0,'','{"x":-1.35,"y":0.05,"z":1.35,"w":1,"h":0.6,"segments":10,"far":0}'),
  ('turret_w','box','mat.olive_drab',0,'','{"x":-1.35,"y":0.65,"z":1.35,"w":0.55,"d":0.6,"h":0.28,"taper":0.2}'),
  ('guns_w','cylinder','mat.cast_iron',0,'turret_w','{"x":-0.09,"y":0.14,"z":0.2,"w":0.07,"h":0.85,"segments":5,"pitch":72,"repeat":2,"dx":0.18,"far":0}'),
  ('pit_e','cylinder','mat.concrete',0,'','{"x":1.35,"y":0.05,"z":1.35,"w":1,"h":0.6,"segments":10,"far":0}'),
  ('turret_e','box','mat.olive_drab',0,'','{"x":1.35,"y":0.65,"z":1.35,"w":0.55,"d":0.6,"h":0.28,"taper":0.2}'),
  ('guns_e','cylinder','mat.cast_iron',0,'turret_e','{"x":-0.09,"y":0.14,"z":0.2,"w":0.07,"h":0.85,"segments":5,"pitch":72,"repeat":2,"dx":0.18,"far":0}'),
  ('gate_pylons','box','mat.concrete',0,'','{"x":-0.62,"y":0.05,"z":2.5,"w":0.3,"d":0.42,"h":1.3,"repeat":2,"dx":1.24}'),
  ('boom','box','mat.cloth',1,'','{"x":-0.05,"y":0.75,"z":2.55,"w":0.95,"d":0.05,"h":0.07}'),
  ('hut_w','cylinder','mat.cladding',0,'','{"x":-1.85,"y":0.05,"z":-1.6,"w":0.8,"h":1.4,"segments":10,"pitch":90,"bottom":1,"far":0}'),
  ('hut_e','cylinder','mat.cladding',0,'','{"x":1.85,"y":0.05,"z":-1.6,"w":0.8,"h":1.4,"segments":10,"pitch":90,"bottom":1,"far":0}'),
  ('flag','banner','mat.cloth',1,'','{"x":0,"y":0.05,"z":1,"h":3.4,"fw":0.9,"fh":0.6}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.fortress_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.fortress'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.fortress_modern'), (SELECT id FROM def WHERE key='mat.concrete'), 100);
