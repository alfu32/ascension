-- bld.city@automation: Future city (mesh.city_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.city_automation','Future city');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.city_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.paving',0,'','{"w":6.4,"d":6.4,"h":0.05,"steps":1}'),
  ('podium','cylinder','mat.composite_panel',0,'','{"y":0.05,"z":-0.5,"w":3.4,"h":0.55,"segments":16}'),
  ('podium_glow','cylinder','mat.light_strip',0,'','{"y":0.58,"z":-0.5,"w":3.42,"h":0.03,"segments":16,"far":0}'),
  ('podium_glass','cylinder','mat.smart_glass',0,'','{"y":0.6,"z":-0.5,"w":3.3,"h":0.46,"segments":16}'),
  ('podium_band','cylinder','mat.white_composite',1,'','{"y":1.06,"z":-0.5,"w":3.44,"h":0.1,"segments":16}'),
  ('roof_garden','cylinder','mat.greens',0,'','{"y":1.16,"z":-0.5,"w":2.7,"h":0.03,"segments":16,"far":0}'),
  ('tower','cylinder','mat.smart_glass',0,'','{"y":1.2,"z":-0.5,"w":1.6,"h":5.4,"taper":0.42,"segments":12}'),
  ('ring_1','cylinder','mat.white_composite',0,'','{"y":2.3,"z":-0.5,"w":1.5631,"h":0.1,"segments":12,"far":0}'),
  ('ring_2','cylinder','mat.white_composite',0,'','{"y":3.4,"z":-0.5,"w":1.4262,"h":0.1,"segments":12,"far":0}'),
  ('ring_3','cylinder','mat.white_composite',0,'','{"y":4.5,"z":-0.5,"w":1.2893,"h":0.1,"segments":12,"far":0}'),
  ('ring_4','cylinder','mat.white_composite',0,'','{"y":5.6,"z":-0.5,"w":1.1524,"h":0.1,"segments":12}'),
  ('halo','torus','mat.white_composite',0,'','{"y":3.86,"z":-0.5,"w":2.6,"t":0.16,"segments":16,"sides":6}'),
  ('halo_spokes','box','mat.white_composite',0,'','{"y":3.9,"z":-0.5,"w":2.4,"d":0.07,"h":0.07,"far":0}'),
  ('halo_spokes_2','box','mat.white_composite',0,'','{"y":3.9,"z":-0.5,"w":0.07,"d":2.4,"h":0.07,"far":0}'),
  ('crown','cylinder','mat.white_composite',1,'','{"y":6.6,"z":-0.5,"w":0.98,"h":0.24,"taper":0.1,"segments":12}'),
  ('cupola','dome','mat.smart_glass',0,'','{"y":6.84,"z":-0.5,"w":0.9,"h":0.5,"segments":12}'),
  ('spire','cylinder','mat.white_composite',0,'','{"y":7.3,"z":-0.5,"w":0.1,"h":1.4,"taper":0.7,"segments":5}'),
  ('spire_light','box','mat.light_strip',0,'','{"y":8.7,"z":-0.5,"w":0.09,"d":0.09,"h":0.1,"far":0}'),
  ('house_sw','include','mat.white_composite',0,'','{"mesh":"mesh.house_future","x":-2.2,"y":0.05,"z":2.3}'),
  ('house_se','include','mat.white_composite',0,'','{"mesh":"mesh.house_future","x":2.2,"y":0.05,"z":2.3,"tint":"#e8ece8"}'),
  ('house_w','include','mat.white_composite',0,'','{"mesh":"mesh.house_future","x":-2.55,"y":0.05,"z":-0.5,"yaw":90,"tint":"#e8ece8"}'),
  ('house_e','include','mat.white_composite',0,'','{"mesh":"mesh.house_future","x":2.55,"y":0.05,"z":-0.5,"yaw":-90}'),
  ('solar','box','mat.solar_panel',0,'','{"y":0.32,"z":-2.75,"w":4.4,"d":0.6,"h":0.03,"pitch":22,"far":0}'),
  ('solar_legs','box','mat.graphite',0,'','{"x":-2,"y":0.05,"z":-2.75,"w":0.05,"d":0.05,"h":0.3,"repeat":4,"dx":1.33,"far":0}'),
  ('flags','banner','mat.cloth',1,'','{"x":-0.6,"y":0.05,"z":1.75,"h":2.4,"fw":0.5,"fh":0.34,"pole_colour":"#e8ecec","repeat":2,"dx":1.2,"far":0}'),
  ('walk_glow','box','mat.light_strip',0,'','{"y":0.05,"z":2,"w":0.08,"d":1,"h":0.01,"repeat":2,"dx":0.6,"x":-0.3,"far":0}'),
  ('trunks','cylinder','mat.white_composite',0,'','{"x":-1,"y":0.05,"z":2.6,"w":0.06,"h":0.45,"segments":4,"repeat":2,"dx":2,"far":0}'),
  ('trees','dome','mat.turf',0,'','{"x":-1,"y":0.45,"z":2.6,"w":0.6,"h":0.65,"segments":6,"repeat":2,"dx":2,"tint":"#7aa060","far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.city_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.city'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.city_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
