-- unit.hover_gunship@automation: Hover gunship (mesh.hover_gunship_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.hover_gunship_automation','Hover gunship');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.hover_gunship_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','box','mat.graphite',0,'','{"y":1.0,"w":0.5,"d":0.95,"h":0.2,"taper":0.3,"bank":6}'),
  ('cover','box','mat.cloth',1,'body','{"y":0.17,"w":0.34,"d":0.55,"h":0.08,"taper":0.35}'),
  ('sensor','dome','mat.white_composite',0,'body','{"y":-0.1,"z":0.4,"w":0.2,"h":0.14,"segments":7,"roll":180}'),
  ('arm_a','box','mat.composite',0,'body','{"y":0.04,"w":1.55,"d":0.07,"h":0.05,"yaw":45}'),
  ('arm_b','box','mat.composite',0,'body','{"y":0.04,"w":1.55,"d":0.07,"h":0.05,"yaw":-45}'),
  ('duct_front','torus','mat.white_composite',0,'body','{"x":-0.6,"y":-0.02,"z":0.6,"w":0.62,"t":0.09,"segments":12,"sides":4,"repeat":2,"dx":1.2}'),
  ('duct_rear','torus','mat.white_composite',0,'body','{"x":-0.6,"y":-0.02,"z":-0.6,"w":0.62,"t":0.09,"segments":12,"sides":4,"repeat":2,"dx":1.2}'),
  ('fan_1','propeller','mat.composite',0,'body','{"x":-0.6,"y":0.07,"z":0.6,"w":0.46,"blades":3,"chord":0.08,"pitch":-90}'),
  ('fan_2','propeller','mat.composite',0,'body','{"x":0.6,"y":0.07,"z":0.6,"w":0.46,"blades":3,"chord":0.08,"pitch":-90}'),
  ('fan_3','propeller','mat.composite',0,'body','{"x":-0.6,"y":0.07,"z":-0.6,"w":0.46,"blades":3,"chord":0.08,"pitch":-90}'),
  ('fan_4','propeller','mat.composite',0,'body','{"x":0.6,"y":0.07,"z":-0.6,"w":0.46,"blades":3,"chord":0.08,"pitch":-90}'),
  ('motors','cylinder','mat.composite',0,'body','{"x":-0.6,"y":0.02,"z":-0.6,"w":0.12,"h":0.1,"segments":5,"repeat":2,"dx":1.2,"far":0}'),
  ('motors_front','cylinder','mat.composite',0,'body','{"x":-0.6,"y":0.02,"z":0.6,"w":0.12,"h":0.1,"segments":5,"repeat":2,"dx":1.2,"far":0}'),
  ('pods','cylinder','mat.white_composite',0,'body','{"x":-0.22,"y":-0.1,"z":0.15,"w":0.1,"h":0.42,"segments":5,"pitch":90,"repeat":2,"dx":0.44}'),
  ('spark','cone','mat.flash',0,'pods','{"y":0.42,"w":0.1,"h":0.1,"segments":3,"flash":2,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.28,"w":0.3,"h":0.3,"segments":6,"pitch":180,"far":0}'),
  ('beacon','dome','mat.glow',0,'body','{"y":0.22,"z":-0.3,"w":0.07,"h":0.05,"segments":5,"far":0}'),
  ('legs','box','mat.composite',0,'body','{"x":-0.3,"y":-0.2,"z":-0.2,"w":0.03,"d":0.03,"h":0.22,"repeat":2,"dx":0.6,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.hover_gunship_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.hover_gunship'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.hover_gunship_automation'), (SELECT id FROM def WHERE key='mat.graphite'), 100);
