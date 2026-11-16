-- unit.jet_fighter@information: Jet fighter (mesh.jet_fighter_information)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.jet_fighter_information','Jet fighter');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.jet_fighter_information';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('fuselage','box','mat.haze_grey',0,'','{"y":3.8,"z":-0.15,"w":0.3,"d":2.2,"h":0.28,"taper":0.3,"bank":24}'),
  ('radome','cone','mat.haze_grey',0,'fuselage','{"y":0.13,"z":1.08,"w":0.24,"h":0.55,"segments":7,"pitch":90,"tint":"#c0c4c8"}'),
  ('canopy','box','mat.glass',0,'fuselage','{"y":0.24,"z":0.55,"w":0.18,"d":0.5,"h":0.13,"taper":0.5,"tint":"#8098b0"}'),
  ('intakes','box','mat.cloth',1,'fuselage','{"x":-0.22,"y":0.02,"z":0.2,"w":0.14,"d":0.6,"h":0.2,"repeat":2,"dx":0.44}'),
  ('wing_e','box','mat.haze_grey',0,'fuselage','{"y":0.05,"z":-0.1,"w":0.05,"d":1,"h":1.15,"taper":0.72,"roll":90,"yaw":38}'),
  ('wing_w','box','mat.haze_grey',0,'fuselage','{"y":0.05,"z":-0.1,"w":0.05,"d":1,"h":1.15,"taper":0.72,"roll":-90,"yaw":-38}'),
  ('tip_e','box','mat.cloth',1,'wing_e','{"y":0.82,"w":0.04,"d":0.42,"h":0.28,"taper":0.3}'),
  ('tip_w','box','mat.cloth',1,'wing_w','{"y":0.82,"w":0.04,"d":0.42,"h":0.28,"taper":0.3}'),
  ('stab_e','box','mat.haze_grey',0,'fuselage','{"y":0.06,"z":-0.95,"w":0.04,"d":0.45,"h":0.55,"taper":0.6,"roll":90,"yaw":38}'),
  ('stab_w','box','mat.haze_grey',0,'fuselage','{"y":0.06,"z":-0.95,"w":0.04,"d":0.45,"h":0.55,"taper":0.6,"roll":-90,"yaw":-38}'),
  ('fin_e','box','mat.cloth',1,'fuselage','{"x":0.14,"y":0.22,"z":-0.8,"w":0.03,"d":0.5,"h":0.45,"taper":0.55,"roll":14,"pitch":-18}'),
  ('fin_w','box','mat.cloth',1,'fuselage','{"x":-0.14,"y":0.22,"z":-0.8,"w":0.03,"d":0.5,"h":0.45,"taper":0.55,"roll":-14,"pitch":-18}'),
  ('nozzles','cylinder','mat.cast_iron',0,'fuselage','{"x":-0.08,"y":0.12,"z":-1.22,"w":0.14,"h":0.16,"segments":6,"pitch":-90,"repeat":2,"dx":0.16,"far":0}'),
  ('burner','wheel','mat.glow',0,'fuselage','{"x":-0.08,"y":0.06,"z":-1.4,"yaw":90,"w":0.12,"t":0.03,"spokes":3,"segments":6,"repeat":2,"dx":0.16}'),
  ('missiles','cylinder','mat.white_metal',0,'fuselage','{"x":-0.75,"y":-0.02,"z":-0.5,"w":0.06,"h":0.6,"segments":5,"pitch":90,"repeat":2,"dx":1.5,"far":0}'),
  ('spark','cone','mat.flash',0,'missiles','{"y":0.6,"w":0.12,"h":0.12,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.32,"w":0.36,"h":0.32,"segments":6,"pitch":180,"far":0}'),
  ('missile','cylinder','mat.white_metal',0,'spark','{"y":1,"w":0.07,"h":0.3,"segments":5,"far":0}'),
  ('trail','cone','mat.canvas',0,'missile','{"y":-0.88,"w":0.2,"h":0.9,"segments":5,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.jet_fighter_information'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.jet_fighter'), NULL, (SELECT id FROM def WHERE key='age.information'), (SELECT id FROM def WHERE key='mesh.jet_fighter_information'), (SELECT id FROM def WHERE key='mat.haze_grey'), 100);
