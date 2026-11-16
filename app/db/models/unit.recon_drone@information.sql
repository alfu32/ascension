-- unit.recon_drone@information: Recon drone (mesh.recon_drone_information)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.recon_drone_information','Recon drone');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.recon_drone_information';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','box','mat.composite',0,'','{"y":1.15,"w":0.22,"d":0.34,"h":0.1,"taper":0.1,"bank":8}'),
  ('cover','box','mat.cloth',1,'body','{"y":0.1,"w":0.18,"d":0.28,"h":0.05,"taper":0.3}'),
  ('arm_a','box','mat.composite',0,'body','{"y":0.04,"w":0.84,"d":0.04,"h":0.03,"yaw":45}'),
  ('arm_b','box','mat.composite',0,'body','{"y":0.04,"w":0.84,"d":0.04,"h":0.03,"yaw":-45}'),
  ('motors','cylinder','mat.cloth',1,'body','{"x":-0.3,"y":0.06,"z":-0.3,"w":0.07,"h":0.06,"segments":5,"repeat":2,"dx":0.6}'),
  ('motors_2','cylinder','mat.cloth',1,'body','{"x":-0.3,"y":0.06,"z":0.3,"w":0.07,"h":0.06,"segments":5,"repeat":2,"dx":0.6}'),
  ('rotor_1','propeller','mat.composite',0,'body','{"x":-0.3,"y":0.13,"z":-0.3,"w":0.36,"blades":2,"chord":0.05,"pitch":-90}'),
  ('rotor_2','propeller','mat.composite',0,'body','{"x":0.3,"y":0.13,"z":-0.3,"w":0.36,"blades":2,"chord":0.05,"pitch":-90}'),
  ('rotor_3','propeller','mat.composite',0,'body','{"x":-0.3,"y":0.13,"z":0.3,"w":0.36,"blades":2,"chord":0.05,"pitch":-90}'),
  ('rotor_4','propeller','mat.composite',0,'body','{"x":0.3,"y":0.13,"z":0.3,"w":0.36,"blades":2,"chord":0.05,"pitch":-90}'),
  ('camera','dome','mat.white_metal',0,'body','{"z":0.08,"w":0.12,"h":0.1,"segments":6,"pitch":180}'),
  ('lens','box','mat.glass',0,'body','{"y":-0.06,"z":0.14,"w":0.05,"d":0.01,"h":0.04,"tint":"#304050","far":0}'),
  ('gun','cylinder','mat.composite',0,'body','{"x":0.07,"y":-0.04,"z":0.02,"w":0.03,"h":0.22,"segments":4,"pitch":90,"far":0}'),
  ('spark','cone','mat.flash',0,'gun','{"y":0.22,"w":0.06,"h":0.06,"segments":3,"flash":2,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.16,"w":0.14,"h":0.16,"segments":5,"pitch":180,"far":0}'),
  ('skids','box','mat.composite',0,'body','{"x":-0.08,"y":-0.12,"w":0.02,"d":0.26,"h":0.02,"repeat":2,"dx":0.16,"far":0}'),
  ('skid_legs','box','mat.composite',0,'body','{"x":-0.08,"y":-0.12,"w":0.02,"d":0.02,"h":0.12,"repeat":2,"dx":0.16,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.recon_drone_information'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.recon_drone'), NULL, (SELECT id FROM def WHERE key='age.information'), (SELECT id FROM def WHERE key='mesh.recon_drone_information'), (SELECT id FROM def WHERE key='mat.composite'), 100);
