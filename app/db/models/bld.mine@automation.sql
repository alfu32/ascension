-- bld.mine@automation: Future mine (mesh.mine_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.mine_automation','Future mine');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.mine_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.cinders',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('collar','cylinder','mat.white_composite',0,'','{"x":-0.6,"y":0.05,"z":-0.4,"w":1.2,"h":0.3,"segments":14}'),
  ('collar_glow','cylinder','mat.light_strip',0,'','{"x":-0.6,"y":0.22,"z":-0.4,"w":1.22,"h":0.03,"segments":14,"far":0}'),
  ('pit','cylinder','mat.crude',0,'','{"x":-0.6,"y":0.35,"z":-0.4,"w":0.86,"h":0.005,"segments":14}'),
  ('legs','box','mat.white_composite',0,'','{"x":-1.05,"y":0.05,"z":-0.4,"w":0.16,"d":0.18,"h":3.3,"roll":6}'),
  ('legs_e','box','mat.white_composite',0,'','{"x":-0.15,"y":0.05,"z":-0.4,"w":0.16,"d":0.18,"h":3.3,"roll":-6}'),
  ('stays','box','mat.white_composite',0,'','{"x":-0.75,"y":0.05,"z":-1.6,"w":0.12,"d":0.12,"h":3.3,"pitch":20,"roll":4,"far":0}'),
  ('stays_e','box','mat.white_composite',0,'','{"x":-0.45,"y":0.05,"z":-1.6,"w":0.12,"d":0.12,"h":3.3,"pitch":20,"roll":-4,"far":0}'),
  ('braces','box','mat.white_composite',0,'','{"x":-0.6,"y":1.05,"z":-0.4,"w":0.86,"d":0.08,"h":0.08,"far":0}'),
  ('braces_2','box','mat.white_composite',0,'','{"x":-0.6,"y":2.1,"z":-0.4,"w":0.76,"d":0.08,"h":0.08,"far":0}'),
  ('head','box','mat.white_composite',1,'','{"x":-0.6,"y":3.3,"z":-0.45,"w":1,"d":0.6,"h":0.2}'),
  ('head_glow','box','mat.light_strip',0,'','{"x":-0.6,"y":3.27,"z":-0.45,"w":1.02,"d":0.62,"h":0.03,"far":0}'),
  ('sheave','shield','mat.graphite',0,'','{"x":-0.6,"y":3.6,"z":-0.3,"yaw":90,"w":0.56,"t":0.05,"boss":0.06,"segments":12,"rim":"#c8ced2"}'),
  ('sheave_2','shield','mat.graphite',0,'','{"x":-0.6,"y":3.6,"z":-0.6,"yaw":90,"w":0.56,"t":0.05,"boss":0.06,"segments":12,"rim":"#c8ced2","far":0}'),
  ('ropes','box','mat.steel',0,'','{"x":-0.6,"y":0.95,"z":0.75,"w":0.03,"d":0.03,"h":2.95,"pitch":-25,"far":0}'),
  ('cage','box','mat.smart_glass',0,'','{"x":-0.6,"y":0.4,"z":-0.4,"w":0.4,"d":0.4,"h":0.5}'),
  ('winding_house','box','mat.composite_panel',0,'','{"x":-0.6,"y":0.05,"z":1.2,"w":1.3,"d":0.9,"h":0.8,"door":0.3,"door_h":0.5,"door_colour":"#3a4650"}'),
  ('winding_glass','box','mat.smart_glass',0,'','{"x":-0.6,"y":0.45,"z":1.2,"w":1.32,"d":0.92,"h":0.22}'),
  ('winding_fascia','box','mat.white_composite',1,'','{"x":-0.6,"y":0.85,"z":1.2,"w":1.36,"d":0.96,"h":0.16}'),
  ('winding_dome','dome','mat.white_composite',0,'','{"x":-0.6,"y":1.01,"z":1.2,"w":0.8,"h":0.3,"segments":10,"far":0}'),
  ('bin_legs','box','mat.white_composite',0,'','{"x":1,"y":0.05,"z":-1.45,"w":0.07,"d":0.07,"h":1.15,"repeat":2,"dx":0.45}'),
  ('bin_legs_s','box','mat.white_composite',0,'','{"x":1,"y":0.05,"z":-1,"w":0.07,"d":0.07,"h":1.15,"repeat":2,"dx":0.45,"far":0}'),
  ('bin_cone','cone','mat.white_composite',0,'','{"x":1.22,"y":1.2,"z":-1.22,"w":0.75,"h":0.4,"segments":12,"pitch":180}'),
  ('bin','cylinder','mat.white_composite',0,'','{"x":1.22,"y":1.2,"z":-1.22,"w":0.75,"h":0.8,"segments":12}'),
  ('bin_band','cylinder','mat.white_composite',1,'','{"x":1.22,"y":1.65,"z":-1.22,"w":0.77,"h":0.22,"segments":12}'),
  ('bin_cap','dome','mat.white_composite',0,'','{"x":1.22,"y":2,"z":-1.22,"w":0.75,"h":0.22,"segments":12,"far":0}'),
  ('conveyor','box','mat.white_composite',0,'','{"x":0.475,"y":1.55,"z":-0.8,"yaw":28.9,"roll":-22.9,"w":1.8,"d":0.22,"h":0.2}'),
  ('spoil','cone','mat.spoil',0,'','{"x":1.05,"y":0.05,"z":0.55,"w":1.5,"h":0.95,"segments":8}'),
  ('spoil_back','cone','mat.spoil',0,'','{"x":1.35,"y":0.05,"z":1.35,"w":0.9,"h":0.55,"segments":7}'),
  ('hauler','box','mat.white_composite',0,'','{"x":0.3,"y":0.2,"z":1.2,"w":0.44,"d":0.9,"h":0.16}'),
  ('hauler_bed','box','mat.white_composite',0,'','{"x":0.3,"y":0.36,"z":1.15,"w":0.44,"d":0.7,"h":0.2,"taper":0.1,"far":0}'),
  ('hauler_glow','box','mat.light_strip',0,'','{"x":0.3,"y":0.24,"z":1.66,"w":0.36,"d":0.02,"h":0.04,"far":0}'),
  ('hauler_wheels','cylinder','mat.rubber',0,'','{"x":0.06,"y":0.17,"z":0.9,"w":0.32,"h":0.1,"segments":8,"roll":90,"repeat":2,"dx":0.38,"far":0}'),
  ('hauler_wheels_f','cylinder','mat.rubber',0,'','{"x":0.06,"y":0.17,"z":1.45,"w":0.32,"h":0.1,"segments":8,"roll":90,"repeat":2,"dx":0.38,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.65,"y":0.05,"z":-0.35,"h":2.4,"fw":0.5,"fh":0.75,"pole_colour":"#e8ecec"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.mine_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.mine'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.mine_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
