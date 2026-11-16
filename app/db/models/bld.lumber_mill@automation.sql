-- bld.lumber_mill@automation: Future lumber mill (mesh.lumber_mill_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.lumber_mill_automation','Future lumber mill');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.lumber_mill_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('mill','box','mat.composite_panel',0,'','{"x":-0.6,"y":0.05,"z":-0.95,"w":2.3,"d":1.4,"h":0.95,"door":0.6,"door_h":0.7,"door_x":0.6,"door_colour":"#c8ccd0"}'),
  ('mill_vault','cylinder','mat.white_composite',0,'','{"x":-1.75,"y":1,"z":-0.95,"w":1.4,"h":2.3,"segments":14,"roll":90}'),
  ('mill_band','cylinder','mat.white_composite',1,'','{"x":-1,"y":1,"z":-0.95,"w":1.43,"h":0.4,"segments":14,"roll":90}'),
  ('mill_stripe','box','mat.white_composite',1,'','{"x":-0.6,"y":1.66,"z":-0.95,"w":2.32,"d":0.5,"h":0.06}'),
  ('mill_glow','box','mat.light_strip',0,'','{"x":-0.6,"y":0.95,"z":-0.24,"w":2.32,"d":0.03,"h":0.03,"far":0}'),
  ('cyclone_legs','box','mat.white_composite',0,'','{"x":1,"y":0.05,"z":-1.45,"w":0.06,"d":0.06,"h":1.55,"repeat":2,"dx":0.4}'),
  ('cyclone_legs_s','box','mat.white_composite',0,'','{"x":1,"y":0.05,"z":-1.05,"w":0.06,"d":0.06,"h":1.55,"repeat":2,"dx":0.4,"far":0}'),
  ('cyclone_cone','cone','mat.white_composite',0,'','{"x":1.2,"y":2,"z":-1.25,"w":0.62,"h":0.55,"segments":10,"pitch":180}'),
  ('cyclone','cylinder','mat.white_composite',0,'','{"x":1.2,"y":2,"z":-1.25,"w":0.62,"h":0.5,"segments":10}'),
  ('cyclone_band','cylinder','mat.white_composite',1,'','{"x":1.2,"y":2.2,"z":-1.25,"w":0.64,"h":0.16,"segments":10}'),
  ('cyclone_cap','dome','mat.white_composite',0,'','{"x":1.2,"y":2.5,"z":-1.25,"w":0.62,"h":0.2,"segments":10,"far":0}'),
  ('duct_up','box','mat.white_composite',0,'','{"x":0.35,"y":1.3,"z":-1.25,"w":0.14,"d":0.14,"h":0.9}'),
  ('duct','box','mat.white_composite',0,'','{"x":0.65,"y":2.06,"z":-1.25,"w":0.6,"d":0.14,"h":0.14}'),
  ('stack','cylinder','mat.white_composite',0,'','{"x":1.5,"y":0.05,"z":-0.35,"w":0.22,"h":3,"segments":6}'),
  ('stack_band','cylinder','mat.white_composite',1,'','{"x":1.5,"y":2.4,"z":-0.35,"w":0.24,"h":0.3,"segments":6}'),
  ('stack_light','cylinder','mat.light_strip',0,'','{"x":1.5,"y":3.05,"z":-0.35,"w":0.24,"h":0.04,"segments":6,"far":0}'),
  ('conveyor','box','mat.graphite',0,'','{"x":0.25,"y":0.3,"z":-0.05,"w":0.3,"d":0.9,"h":0.06,"pitch":-18}'),
  ('conveyor_legs','box','mat.white_composite',0,'','{"x":0.13,"y":0.05,"z":0.25,"w":0.04,"d":0.04,"h":0.3,"repeat":2,"dx":0.24,"far":0}'),
  ('logs','include',NULL,0,'','{"mesh":"mesh.log_pile","x":-0.9,"y":0.05,"z":0.85,"scale":0.85}'),
  ('planks_1','box','mat.sawn_timber',0,'','{"x":1.05,"y":0.05,"z":0.85,"w":1.1,"d":0.95,"h":0.13}'),
  ('planks_2','box','mat.sawn_timber',0,'','{"x":1.05,"y":0.22,"z":0.85,"w":1.07,"d":0.95,"h":0.13,"tint":"#f0e6d8","yaw":2}'),
  ('planks_3','box','mat.sawn_timber',0,'','{"x":1.05,"y":0.39,"z":0.85,"w":1.04,"d":0.95,"h":0.13,"yaw":-1.5}'),
  ('stickers','box','mat.graphite',0,'','{"x":0.65,"y":0.18,"z":0.85,"w":0.06,"d":0.95,"h":0.04,"repeat":3,"dx":0.4,"far":0}'),
  ('forklift','box','mat.white_composite',1,'','{"x":0.3,"y":0.12,"z":1.25,"w":0.26,"d":0.42,"h":0.24}'),
  ('forklift_glow','box','mat.light_strip',0,'','{"x":0.3,"y":0.1,"z":1.25,"w":0.28,"d":0.44,"h":0.03,"far":0}'),
  ('forklift_mast','box','mat.graphite',0,'','{"x":0.3,"y":0.1,"z":1.48,"w":0.22,"d":0.04,"h":0.6,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":-1.65,"y":0.05,"z":1.6,"h":2.4,"fw":0.5,"fh":0.75,"pole_colour":"#e8ecec"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.lumber_mill_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.lumber_mill'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.lumber_mill_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
