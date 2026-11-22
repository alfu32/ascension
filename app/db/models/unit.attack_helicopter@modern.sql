-- unit.attack_helicopter@modern: Attack helicopter (mesh.attack_helicopter_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.attack_helicopter_modern','Attack helicopter');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.attack_helicopter_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('fuselage','box','mat.olive_drab',0,'','{"y":1.2,"z":0,"w":0.34,"d":1.3,"h":0.42,"taper":0.3,"bank":8}'),
  ('canopy','box','mat.glass',0,'fuselage','{"y":0.16,"z":0.42,"w":0.26,"d":0.52,"h":0.22,"taper":0.45,"tint":"#8098b0"}'),
  ('nose_sensor','dome','mat.graphite',0,'fuselage','{"y":-0.12,"z":0.62,"w":0.16,"h":0.1,"segments":6,"far":0}'),
  ('chin_gun','cylinder','mat.cast_iron',0,'fuselage','{"y":-0.22,"z":0.5,"w":0.045,"h":0.32,"segments":4,"pitch":90,"far":0}'),
  ('spark','cone','mat.flash',0,'chin_gun','{"y":0.32,"w":0.08,"h":0.08,"segments":3,"flash":4,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.26,"w":0.24,"h":0.26,"segments":5,"pitch":180,"far":0}'),
  ('engine_cowl','box','mat.cloth',1,'fuselage','{"y":0.28,"z":-0.22,"w":0.26,"d":0.5,"h":0.16,"taper":0.3}'),
  ('exhaust','cylinder','mat.cast_iron',0,'fuselage','{"x":-0.1,"y":0.3,"z":-0.5,"w":0.07,"h":0.1,"segments":5,"pitch":-90,"repeat":2,"dx":0.2,"far":0}'),
  ('tail_boom','box','mat.cloth',1,'fuselage','{"y":0.1,"z":-1.0,"w":0.12,"d":1.1,"h":0.13,"taper":0.5}'),
  ('tailplane','box','mat.olive_drab',0,'fuselage','{"y":0.12,"z":-1.3,"w":0.55,"d":0.14,"h":0.02}'),
  ('fin','box','mat.cloth',1,'fuselage','{"y":0.22,"z":-1.5,"w":0.03,"d":0.3,"h":0.42,"taper":0.45}'),
  ('tail_rotor','propeller','mat.graphite',0,'fuselage','{"x":0.06,"y":0.4,"z":-1.52,"w":0.42,"blades":2,"chord":0.05,"yaw":90}'),
  ('mast','cylinder','mat.cast_iron',0,'fuselage','{"y":0.4,"z":-0.05,"w":0.08,"h":0.2,"segments":5}'),
  ('main_rotor','propeller','mat.graphite',0,'fuselage','{"y":0.62,"z":-0.05,"w":2.5,"blades":2,"chord":0.1,"pitch":-90}'),
  ('stub_wings','box','mat.cloth',1,'fuselage','{"y":-0.02,"z":0.0,"w":1.2,"d":0.22,"h":0.03}'),
  ('wing_tips','box','mat.cloth',1,'fuselage','{"x":-0.62,"y":-0.02,"z":0.0,"w":0.1,"d":0.24,"h":0.04,"repeat":2,"dx":1.24}'),
  ('rocket_pods','cylinder','mat.cast_iron',0,'fuselage','{"x":-0.44,"y":-0.1,"z":0.2,"w":0.12,"h":0.5,"segments":6,"pitch":90,"repeat":2,"dx":0.88,"far":0}'),
  ('skids','box','mat.cast_iron',0,'fuselage','{"x":-0.22,"y":-0.4,"z":0.05,"w":0.03,"d":1.0,"h":0.03,"repeat":2,"dx":0.44}'),
  ('skid_struts','box','mat.cast_iron',0,'fuselage','{"x":-0.22,"y":-0.3,"z":0.3,"w":0.03,"d":0.03,"h":0.22,"repeat":2,"dx":0.44,"far":0}'),
  ('skid_struts_back','box','mat.cast_iron',0,'fuselage','{"x":-0.22,"y":-0.3,"z":-0.22,"w":0.03,"d":0.03,"h":0.22,"repeat":2,"dx":0.44,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.attack_helicopter_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.attack_helicopter'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.attack_helicopter_modern'), (SELECT id FROM def WHERE key='mat.olive_drab'), 100);
