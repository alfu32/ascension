-- unit.strategic_bomber@information: Strategic bomber (mesh.strategic_bomber_information)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.strategic_bomber_information','Strategic bomber');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.strategic_bomber_information';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('fuselage','box','mat.white_metal',0,'','{"y":4.2,"z":-0.1,"w":0.48,"d":3.4,"h":0.46,"taper":0.2,"bank":8}'),
  ('nose','cone','mat.white_metal',0,'fuselage','{"y":0.21,"z":1.7,"w":0.4,"h":0.6,"segments":8,"pitch":90}'),
  ('cockpit','box','mat.glass',0,'fuselage','{"y":0.42,"z":1.25,"w":0.3,"d":0.4,"h":0.12,"taper":0.4,"tint":"#8098b0"}'),
  ('wing_e','box','mat.white_metal',0,'fuselage','{"y":0.2,"z":0.1,"w":0.08,"d":1.3,"h":2.2,"taper":0.66,"roll":90,"yaw":35}'),
  ('wing_w','box','mat.white_metal',0,'fuselage','{"y":0.2,"z":0.1,"w":0.08,"d":1.3,"h":2.2,"taper":0.66,"roll":-90,"yaw":-35}'),
  ('tip_e','box','mat.cloth',1,'wing_e','{"y":1.6,"w":0.07,"d":0.66,"h":0.45,"taper":0.2}'),
  ('tip_w','box','mat.cloth',1,'wing_w','{"y":1.6,"w":0.07,"d":0.66,"h":0.45,"taper":0.2}'),
  ('pods_in','cylinder','mat.cast_iron',0,'fuselage','{"x":-0.75,"y":0.02,"z":-0.51,"w":0.2,"h":0.62,"segments":6,"pitch":90,"repeat":2,"dx":1.5}'),
  ('pods_out','cylinder','mat.cast_iron',0,'fuselage','{"x":-1.35,"y":0.08,"z":-0.91,"w":0.2,"h":0.62,"segments":6,"pitch":90,"repeat":2,"dx":2.7}'),
  ('exhaust_in','wheel','mat.glow',0,'fuselage','{"x":-0.75,"y":-0.06,"z":-0.53,"yaw":90,"w":0.16,"t":0.03,"spokes":3,"segments":6,"repeat":2,"dx":1.5}'),
  ('exhaust_out','wheel','mat.glow',0,'fuselage','{"x":-1.35,"y":0,"z":-0.93,"yaw":90,"w":0.16,"t":0.03,"spokes":3,"segments":6,"repeat":2,"dx":2.7}'),
  ('band','box','mat.cloth',1,'fuselage','{"y":-0.01,"z":-0.75,"w":0.52,"d":0.32,"h":0.5,"taper":0.2}'),
  ('stab_e','box','mat.white_metal',0,'fuselage','{"y":0.25,"z":-1.45,"w":0.05,"d":0.55,"h":0.85,"taper":0.6,"roll":90,"yaw":35}'),
  ('stab_w','box','mat.white_metal',0,'fuselage','{"y":0.25,"z":-1.45,"w":0.05,"d":0.55,"h":0.85,"taper":0.6,"roll":-90,"yaw":-35}'),
  ('fin','box','mat.cloth',1,'fuselage','{"y":0.3,"z":-1.35,"w":0.05,"d":0.75,"h":0.8,"taper":0.55,"pitch":-28}'),
  ('bay','box','mat.cast_iron',0,'fuselage','{"y":-0.02,"z":0.2,"w":0.3,"d":1,"h":0.02,"far":0}'),
  ('bomb_drop','box','mat.flash',0,'fuselage','{"y":-0.02,"z":0.2,"w":0.02,"d":0.02,"h":0.02,"flash":1,"far":0}'),
  ('bombs','cylinder','mat.cast_iron',0,'bomb_drop','{"y":-0.5,"z":-0.35,"w":0.12,"h":0.3,"segments":5,"pitch":90,"repeat":3,"dz":0.3,"far":0}'),
  ('bombs_low','cylinder','mat.cast_iron',0,'bomb_drop','{"y":-1.3,"z":-0.2,"w":0.12,"h":0.3,"segments":5,"pitch":90,"repeat":2,"dz":0.36,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.strategic_bomber_information'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.strategic_bomber'), NULL, (SELECT id FROM def WHERE key='age.information'), (SELECT id FROM def WHERE key='mesh.strategic_bomber_information'), (SELECT id FROM def WHERE key='mat.white_metal'), 100);
