-- bld.dock@automation: Future dock (mesh.dock_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.dock_automation','Future dock');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.dock_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.paving',0,'','{"w":3.6,"d":3.6,"h":0.04,"steps":1}'),
  ('basin','box','mat.white_metal',0,'','{"x":-0.85,"y":0.04,"z":0,"w":1.9,"d":3.5,"h":0.005,"tint":"#5f90b8"}'),
  ('quay','box','mat.composite_panel',0,'','{"x":0.95,"w":1.6,"d":3.5,"h":0.42}'),
  ('quay_edge','box','mat.light_strip',0,'','{"x":0.17,"y":0.39,"w":0.04,"d":3.5,"h":0.03,"far":0}'),
  ('ship','hull','mat.white_composite',0,'','{"x":-0.85,"y":0.12,"z":0.3,"yaw":90,"w":2.6,"d":0.75,"h":0.42,"sheer":0.06,"segments":6}'),
  ('deck','box','mat.graphite',0,'ship','{"y":0.38,"w":2.3,"d":0.62,"h":0.04}'),
  ('bridge','box','mat.white_composite',0,'ship','{"x":-0.9,"y":0.42,"w":0.42,"d":0.62,"h":0.4,"taper":0.15}'),
  ('bridge_glass','box','mat.smart_glass',0,'ship','{"x":-0.9,"y":0.66,"w":0.38,"d":0.58,"h":0.1,"far":0}'),
  ('fin','box','mat.white_composite',1,'ship','{"x":-1.08,"y":0.82,"w":0.3,"d":0.06,"h":0.26,"taper":0.4}'),
  ('containers','box','mat.white_composite',0,'ship','{"x":-0.4,"y":0.42,"w":0.45,"d":0.6,"h":0.2,"repeat":3,"dx":0.5,"tint":"#e88a5a"}'),
  ('containers_2','box','mat.white_composite',0,'ship','{"x":-0.15,"y":0.62,"w":0.45,"d":0.6,"h":0.2,"repeat":2,"dx":0.5,"tint":"#7ab0d8","far":0}'),
  ('warehouse','box','mat.composite_panel',0,'','{"x":0.95,"y":0.42,"z":-1.3,"w":1.5,"d":0.85,"h":0.6,"door":0.6,"door_h":0.5,"door_colour":"#c8ccd0"}'),
  ('warehouse_vault','cylinder','mat.white_composite',1,'','{"x":0.2,"y":1.02,"z":-1.3,"w":0.85,"h":1.5,"segments":12,"roll":90}'),
  ('warehouse_band','box','mat.white_composite',0,'','{"x":0.95,"y":0.92,"z":-1.3,"w":1.54,"d":0.89,"h":0.12}'),
  ('crane_legs','box','mat.white_composite',0,'','{"x":0.4,"y":0.42,"z":-0.55,"w":0.14,"d":0.14,"h":2.4,"repeat":2,"dx":1}'),
  ('crane_legs_s','box','mat.white_composite',0,'','{"x":0.4,"y":0.42,"z":0.05,"w":0.14,"d":0.14,"h":2.4,"repeat":2,"dx":1}'),
  ('crane_sills','box','mat.white_composite',0,'','{"x":0.4,"y":0.42,"z":-0.25,"w":0.18,"d":0.76,"h":0.12,"repeat":2,"dx":1,"far":0}'),
  ('boom','box','mat.white_composite',1,'','{"x":0.1,"y":2.82,"z":-0.25,"w":3.3,"d":0.26,"h":0.22}'),
  ('boom_glow','box','mat.light_strip',0,'','{"x":0.1,"y":2.79,"z":-0.25,"w":3.1,"d":0.1,"h":0.03,"far":0}'),
  ('machinery','box','mat.white_composite',0,'','{"x":1.2,"y":3.04,"z":-0.25,"w":0.55,"d":0.42,"h":0.3,"taper":0.15}'),
  ('trolley','box','mat.graphite',0,'','{"x":-0.85,"y":2.66,"z":-0.25,"w":0.3,"d":0.3,"h":0.16}'),
  ('cables','cylinder','mat.steel',0,'','{"x":-0.85,"y":1.86,"z":-0.25,"w":0.03,"h":0.8,"segments":3,"far":0}'),
  ('load','box','mat.white_composite',0,'','{"x":-0.85,"y":1.66,"z":-0.25,"w":0.6,"d":0.25,"h":0.2,"tint":"#f0c048","far":0}'),
  ('stack','box','mat.white_composite',0,'','{"x":0.75,"y":0.42,"z":0.95,"w":0.25,"d":0.6,"h":0.22,"repeat":3,"dx":0.3,"tint":"#e88a5a"}'),
  ('stack_2','box','mat.white_composite',0,'','{"x":0.9,"y":0.64,"z":0.95,"w":0.25,"d":0.6,"h":0.22,"repeat":2,"dx":0.3,"tint":"#7ad0a0","far":0}'),
  ('light_mast','cylinder','mat.white_composite',0,'','{"x":1.6,"y":0.42,"z":1.55,"w":0.07,"h":2.2,"segments":4,"far":0}'),
  ('light_head','box','mat.light_strip',0,'','{"x":1.6,"y":2.62,"z":1.55,"w":0.3,"d":0.12,"h":0.08,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.dock_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.dock'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.dock_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
