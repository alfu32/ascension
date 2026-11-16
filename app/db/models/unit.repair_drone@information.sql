-- unit.repair_drone@information: Repair drone (mesh.repair_drone_information)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.repair_drone_information','Repair drone');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.repair_drone_information';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','box','mat.white_metal',0,'','{"y":1.55,"w":0.36,"d":0.52,"h":0.16,"taper":0.1,"bank":6}'),
  ('cover','box','mat.cloth',1,'body','{"y":0.16,"w":0.26,"d":0.36,"h":0.06,"taper":0.3}'),
  ('arm_a','box','mat.composite',0,'body','{"y":0.06,"w":1.1,"d":0.05,"h":0.04,"yaw":45}'),
  ('arm_b','box','mat.composite',0,'body','{"y":0.06,"w":1.1,"d":0.05,"h":0.04,"yaw":-45}'),
  ('motors','cylinder','mat.composite',0,'body','{"x":-0.39,"y":0.06,"z":-0.39,"w":0.09,"h":0.1,"segments":5,"repeat":2,"dx":0.78}'),
  ('motors_2','cylinder','mat.composite',0,'body','{"x":-0.39,"y":0.06,"z":0.39,"w":0.09,"h":0.1,"segments":5,"repeat":2,"dx":0.78}'),
  ('rotor_1','propeller','mat.composite',0,'body','{"x":-0.39,"y":0.17,"z":-0.39,"w":0.46,"blades":2,"chord":0.06,"pitch":-90}'),
  ('rotor_2','propeller','mat.composite',0,'body','{"x":0.39,"y":0.17,"z":-0.39,"w":0.46,"blades":2,"chord":0.06,"pitch":-90}'),
  ('rotor_3','propeller','mat.composite',0,'body','{"x":-0.39,"y":0.17,"z":0.39,"w":0.46,"blades":2,"chord":0.06,"pitch":-90}'),
  ('rotor_4','propeller','mat.composite',0,'body','{"x":0.39,"y":0.17,"z":0.39,"w":0.46,"blades":2,"chord":0.06,"pitch":-90}'),
  ('beacon','dome','mat.signal_red',0,'body','{"y":0.22,"w":0.07,"h":0.06,"segments":5,"far":0}'),
  ('shoulder','box','mat.composite',0,'body','{"z":0.18,"y":-0.06,"w":0.08,"d":0.08,"h":0.08}'),
  ('upper_arm','box','mat.white_metal',0,'shoulder','{"y":0.02,"w":0.05,"d":0.05,"h":0.3,"pitch":120}'),
  ('forearm','box','mat.white_metal',0,'shoulder','{"y":-0.13,"z":0.26,"w":0.045,"d":0.045,"h":0.26,"pitch":160}'),
  ('claw','box','mat.composite',0,'shoulder','{"x":-0.03,"y":-0.47,"z":0.35,"w":0.02,"d":0.03,"h":0.11,"repeat":2,"dx":0.06,"far":0}'),
  ('toolbox','box','mat.white_metal',0,'body','{"z":-0.12,"y":-0.22,"w":0.28,"d":0.2,"h":0.16,"tint":"#e09a3a"}'),
  ('toolbox_straps','box','mat.composite',0,'body','{"z":-0.12,"y":-0.1,"w":0.02,"d":0.02,"h":0.12,"x":-0.1,"repeat":2,"dx":0.2,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.repair_drone_information'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.repair_drone'), NULL, (SELECT id FROM def WHERE key='age.information'), (SELECT id FROM def WHERE key='mesh.repair_drone_information'), (SELECT id FROM def WHERE key='mat.white_metal'), 100);
