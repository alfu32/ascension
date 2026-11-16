-- bld.barracks@automation: Future barracks (mesh.barracks_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.barracks_automation','Future barracks');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.barracks_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.paving',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('walls','wall','mat.composite_panel',0,'','{"w":3.5,"d":3.5,"h":0.75,"t":0.18,"ring":1,"gap":1.2}'),
  ('wall_glow','wall','mat.light_strip',0,'','{"y":0.6,"w":3.5,"d":3.5,"h":0.03,"t":0.2,"ring":1,"gap":1.2,"far":0}'),
  ('gate_pylons','cylinder','mat.white_composite',0,'','{"x":-0.7,"z":1.67,"w":0.32,"h":1.15,"segments":8,"repeat":2,"dx":1.4}'),
  ('gate_caps','cylinder','mat.white_composite',1,'','{"x":-0.7,"y":1.15,"z":1.67,"w":0.36,"h":0.14,"segments":8,"repeat":2,"dx":1.4}'),
  ('gate_field','box','mat.light_strip',0,'','{"y":0.6,"z":1.67,"w":1.1,"d":0.04,"h":0.06}'),
  ('block','box','mat.composite_panel',0,'','{"y":0.05,"z":-0.95,"w":2.2,"d":1.2,"h":1.3,"door":0.42,"door_h":0.6,"door_colour":"#2c3a40"}'),
  ('block_ends','cylinder','mat.composite_panel',0,'','{"x":-1.1,"y":0.05,"z":-0.95,"w":1.2,"h":1.3,"segments":12,"repeat":2,"dx":2.2}'),
  ('block_glass','box','mat.smart_glass',0,'','{"y":0.6,"z":-0.95,"w":2.22,"d":1.22,"h":0.28}'),
  ('block_glass_ends','cylinder','mat.smart_glass',0,'','{"x":-1.1,"y":0.6,"z":-0.95,"w":1.22,"h":0.28,"segments":12,"repeat":2,"dx":2.2,"far":0}'),
  ('block_band','box','mat.white_composite',1,'','{"y":1.15,"z":-0.95,"w":2.24,"d":1.24,"h":0.2}'),
  ('block_band_ends','cylinder','mat.white_composite',1,'','{"x":-1.1,"y":1.15,"z":-0.95,"w":1.24,"h":0.2,"segments":12,"repeat":2,"dx":2.2}'),
  ('block_roof','box','mat.white_composite',0,'','{"y":1.35,"z":-0.95,"w":2.2,"d":1.2,"h":0.03}'),
  ('block_roof_ends','cylinder','mat.white_composite',1,'','{"x":-1.1,"y":1.35,"z":-0.95,"w":1.2,"h":0.03,"segments":12,"repeat":2,"dx":2.2}'),
  ('antenna','cylinder','mat.white_composite',0,'','{"x":1.2,"y":1.35,"z":-1.2,"w":0.05,"h":1.4,"segments":4}'),
  ('antenna_light','box','mat.light_strip',0,'','{"x":1.2,"y":2.75,"z":-1.2,"w":0.08,"d":0.08,"h":0.08,"far":0}'),
  ('annex','dome','mat.white_composite',0,'','{"x":1.1,"y":0.05,"z":0.5,"w":1,"h":0.7,"segments":10}'),
  ('annex_door','box','mat.graphite',0,'','{"x":0.62,"y":0.05,"z":0.5,"w":0.06,"d":0.3,"h":0.4,"far":0}'),
  ('flag','banner','mat.cloth',1,'','{"x":-0.55,"y":0.05,"z":0.35,"h":4,"fw":0.9,"fh":0.6,"pole_colour":"#e8ecec"}'),
  ('hover','box','mat.white_composite',0,'','{"x":0.1,"y":0.2,"z":0.6,"w":0.34,"d":0.62,"h":0.14,"far":0}'),
  ('hover_cab','dome','mat.smart_glass',0,'','{"x":0.1,"y":0.34,"z":0.55,"w":0.26,"h":0.14,"segments":6,"far":0}'),
  ('hover_glow','box','mat.light_strip',0,'','{"x":0.1,"y":0.1,"z":0.6,"w":0.3,"d":0.56,"h":0.1,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.barracks_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.barracks'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.barracks_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
