-- unit.gunship_helicopter@information: Gunship helicopter (mesh.gunship_helicopter_information)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.gunship_helicopter_information','Gunship helicopter');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.gunship_helicopter_information';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('fuselage','box','mat.haze_grey',0,'','{"y":1.25,"z":0,"w":0.4,"d":1.55,"h":0.46,"taper":0.25,"bank":7}'),
  ('canopy','box','mat.glass',0,'fuselage','{"y":0.18,"z":0.5,"w":0.3,"d":0.6,"h":0.24,"taper":0.5,"tint":"#8098b0"}'),
  ('sensor_ball','dome','mat.graphite',0,'fuselage','{"y":-0.2,"z":0.68,"w":0.2,"h":0.14,"segments":7,"roll":180,"far":0}'),
  ('chin_gun','cylinder','mat.cast_iron',0,'fuselage','{"x":0.12,"y":-0.22,"z":0.5,"w":0.05,"h":0.36,"segments":4,"pitch":90,"far":0}'),
  ('spark','cone','mat.flash',0,'chin_gun','{"y":0.36,"w":0.08,"h":0.08,"segments":3,"flash":4,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.26,"w":0.24,"h":0.26,"segments":5,"pitch":180,"far":0}'),
  ('engine_cowl','box','mat.cloth',1,'fuselage','{"y":0.3,"z":-0.28,"w":0.32,"d":0.6,"h":0.18,"taper":0.3}'),
  ('exhaust','cylinder','mat.cast_iron',0,'fuselage','{"x":-0.11,"y":0.32,"z":-0.62,"w":0.09,"h":0.12,"segments":5,"pitch":-90,"repeat":2,"dx":0.22,"far":0}'),
  ('tail_boom','box','mat.cloth',1,'fuselage','{"y":0.1,"z":-1.2,"w":0.14,"d":1.25,"h":0.15,"taper":0.5}'),
  ('tailplane','box','mat.haze_grey',0,'fuselage','{"y":0.14,"z":-1.5,"w":0.7,"d":0.18,"h":0.02}'),
  ('fin','box','mat.cloth',1,'fuselage','{"y":0.24,"z":-1.72,"w":0.03,"d":0.34,"h":0.5,"taper":0.45}'),
  ('tail_rotor','propeller','mat.graphite',0,'fuselage','{"x":0.08,"y":0.46,"z":-1.74,"w":0.48,"blades":4,"chord":0.04,"yaw":90,"far":0}'),
  ('mast','cylinder','mat.cast_iron',0,'fuselage','{"y":0.44,"z":-0.05,"w":0.1,"h":0.22,"segments":6}'),
  ('main_rotor','propeller','mat.graphite',0,'fuselage','{"y":0.68,"z":-0.05,"w":2.9,"blades":4,"chord":0.09,"pitch":-90}'),
  ('stub_wings','box','mat.cloth',1,'fuselage','{"y":-0.04,"z":0.02,"w":1.9,"d":0.3,"h":0.04,"taper":0.2}'),
  ('wing_tips','box','mat.cloth',1,'fuselage','{"x":-0.98,"y":-0.04,"z":0.02,"w":0.1,"d":0.3,"h":0.05,"repeat":2,"dx":1.96}'),
  ('missile_racks','box','mat.graphite',0,'fuselage','{"x":-0.78,"y":-0.16,"z":0.12,"w":0.12,"d":0.5,"h":0.12,"repeat":2,"dx":1.56,"far":0}'),
  ('missiles','cylinder','mat.white_metal',0,'fuselage','{"far":0,"x":-0.82,"y":-0.2,"z":0.34,"w":0.07,"h":0.5,"segments":5,"pitch":90,"repeat":2,"dx":1.64}'),
  ('missiles_inner','cylinder','mat.white_metal',0,'fuselage','{"x":-0.7,"y":-0.2,"z":0.34,"w":0.07,"h":0.5,"segments":5,"pitch":90,"repeat":2,"dx":1.4,"far":0}'),
  ('missile_spark','cone','mat.flash',0,'missiles','{"y":0.5,"w":0.1,"h":0.1,"segments":3,"flash":2,"far":0}'),
  ('missile_flash','cone','mat.flash',0,'missile_spark','{"y":0.3,"w":0.3,"h":0.3,"segments":6,"pitch":180,"far":0}'),
  ('skids','box','mat.graphite',0,'fuselage','{"x":-0.26,"y":-0.44,"z":0.05,"w":0.035,"d":1.2,"h":0.035,"repeat":2,"dx":0.52}'),
  ('skid_struts','box','mat.graphite',0,'fuselage','{"x":-0.26,"y":-0.33,"z":0.35,"w":0.03,"d":0.03,"h":0.24,"repeat":2,"dx":0.52,"far":0}'),
  ('skid_struts_back','box','mat.graphite',0,'fuselage','{"x":-0.26,"y":-0.33,"z":-0.25,"w":0.03,"d":0.03,"h":0.24,"repeat":2,"dx":0.52,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.gunship_helicopter_information'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.gunship_helicopter'), NULL, (SELECT id FROM def WHERE key='age.information'), (SELECT id FROM def WHERE key='mesh.gunship_helicopter_information'), (SELECT id FROM def WHERE key='mat.haze_grey'), 100);
