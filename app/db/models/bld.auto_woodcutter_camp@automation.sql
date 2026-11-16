-- bld.auto_woodcutter_camp@automation: Automated Woodcutter Camp (mesh.auto_woodcutter_camp_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.auto_woodcutter_camp_automation','Automated Woodcutter Camp');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.auto_woodcutter_camp_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('hall','cylinder','mat.white_composite',0,'','{"x":-1.75,"y":0.68,"z":-1.05,"w":1.36,"h":2.5,"segments":14,"roll":90}'),
  ('hall_band','cylinder','mat.white_composite',1,'','{"x":-0.95,"y":0.68,"z":-1.05,"w":1.4,"h":0.36,"segments":14,"roll":90}'),
  ('hall_glass','cylinder','mat.smart_glass',0,'','{"x":-0.25,"y":0.68,"z":-1.05,"w":1.38,"h":0.45,"segments":14,"roll":90,"far":0}'),
  ('hall_glow','cylinder','mat.light_strip',0,'','{"x":-1.45,"y":0.68,"z":-1.05,"w":1.39,"h":0.04,"segments":14,"roll":90,"far":0}'),
  ('hall_door','box','mat.graphite',0,'','{"x":0.76,"y":0.05,"z":-1.05,"w":0.02,"d":0.6,"h":0.6,"far":0}'),
  ('conveyor','box','mat.graphite',0,'','{"x":1.2,"y":0.32,"z":-1.05,"w":0.9,"d":0.34,"h":0.06}'),
  ('conveyor_legs','box','mat.white_composite',0,'','{"x":1,"y":0.05,"z":-1.05,"w":0.06,"d":0.3,"h":0.27,"repeat":2,"dx":0.4,"far":0}'),
  ('conveyor_logs','cylinder','mat.timber',0,'','{"x":1,"y":0.45,"z":-1.2,"w":0.14,"h":0.4,"segments":5,"pitch":90,"repeat":2,"dx":0.35,"far":0}'),
  ('logs','include',NULL,0,'','{"mesh":"mesh.log_pile","x":0.95,"y":0.05,"z":0.5}'),
  ('logs_w','include',NULL,0,'','{"mesh":"mesh.log_pile","x":-0.5,"y":0.05,"z":0.9,"yaw":90,"scale":0.9}'),
  ('planks','box','mat.sawn_timber',0,'','{"x":-1.35,"y":0.05,"z":0.65,"w":0.7,"d":1,"h":0.3}'),
  ('planks_2','box','mat.sawn_timber',0,'','{"x":-1.35,"y":0.35,"z":0.65,"w":0.66,"d":1,"h":0.2,"tint":"#f0e6d8","far":0}'),
  ('gantry_legs','box','mat.white_composite',0,'','{"x":-1.65,"y":0.05,"z":0.25,"w":0.14,"d":0.14,"h":2.1,"repeat":2,"dx":3.3}'),
  ('gantry_legs_s','box','mat.white_composite',0,'','{"x":-1.65,"y":0.05,"z":1.45,"w":0.14,"d":0.14,"h":2.1,"repeat":2,"dx":3.3,"far":0}'),
  ('gantry_ends','box','mat.white_composite',0,'','{"x":-1.65,"y":2.15,"z":0.85,"w":0.16,"d":1.4,"h":0.14,"repeat":2,"dx":3.3}'),
  ('gantry','box','mat.white_composite',1,'','{"y":2.15,"z":0.85,"w":3.46,"d":0.24,"h":0.24}'),
  ('gantry_glow','box','mat.light_strip',0,'','{"y":2.12,"z":0.85,"w":3.2,"d":0.1,"h":0.03,"far":0}'),
  ('trolley','box','mat.graphite',0,'','{"x":0.95,"y":1.95,"z":0.85,"w":0.34,"d":0.34,"h":0.2}'),
  ('cable','cylinder','mat.steel',0,'','{"x":0.95,"y":1.2,"z":0.85,"w":0.04,"h":0.75,"segments":3,"far":0}'),
  ('claw','box','mat.graphite',0,'','{"x":0.95,"y":1,"z":0.85,"w":0.4,"d":0.22,"h":0.2}'),
  ('drone_pad','cylinder','mat.graphite',0,'','{"x":-0.1,"y":0.05,"z":-0.05,"w":0.6,"h":0.03,"segments":10,"far":0}'),
  ('drone','box','mat.white_composite',0,'','{"x":-0.1,"y":0.08,"z":-0.05,"w":0.3,"d":0.3,"h":0.1,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.65,"y":0.05,"z":-0.3,"h":2.7,"fw":0.55,"fh":1,"pole_colour":"#e8ecec"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.auto_woodcutter_camp_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.auto_woodcutter_camp'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.auto_woodcutter_camp_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
