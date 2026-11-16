-- unit.cargo_drone@automation: Cargo drone (mesh.cargo_drone_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.cargo_drone_automation','Cargo drone');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.cargo_drone_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','box','mat.white_composite',0,'','{"y":1.7,"w":0.6,"d":0.9,"h":0.2,"taper":0.15,"bank":6}'),
  ('top','box','mat.white_composite',1,'body','{"y":0.2,"w":0.42,"d":0.66,"h":0.08,"taper":0.3}'),
  ('arm_a','box','mat.graphite',0,'body','{"y":0.08,"w":1.7,"d":0.07,"h":0.05,"yaw":40}'),
  ('arm_b','box','mat.graphite',0,'body','{"y":0.08,"w":1.7,"d":0.07,"h":0.05,"yaw":-40}'),
  ('motors','cylinder','mat.graphite',0,'body','{"x":-0.65,"y":0.06,"z":-0.55,"w":0.12,"h":0.1,"segments":6,"repeat":2,"dx":1.3,"far":0}'),
  ('motors_2','cylinder','mat.graphite',0,'body','{"x":-0.65,"y":0.06,"z":0.55,"w":0.12,"h":0.1,"segments":6,"repeat":2,"dx":1.3,"far":0}'),
  ('rotor_1','propeller','mat.composite',0,'body','{"x":-0.65,"y":0.17,"z":-0.55,"w":0.72,"blades":2,"chord":0.1,"pitch":-90}'),
  ('rotor_2','propeller','mat.composite',0,'body','{"x":0.65,"y":0.17,"z":-0.55,"w":0.72,"blades":2,"chord":0.1,"pitch":-90}'),
  ('rotor_3','propeller','mat.composite',0,'body','{"x":-0.65,"y":0.17,"z":0.55,"w":0.72,"blades":2,"chord":0.1,"pitch":-90}'),
  ('rotor_4','propeller','mat.composite',0,'body','{"x":0.65,"y":0.17,"z":0.55,"w":0.72,"blades":2,"chord":0.1,"pitch":-90}'),
  ('guards','torus','mat.white_composite',0,'body','{"x":-0.65,"y":0.13,"z":-0.55,"w":0.82,"t":0.035,"segments":12,"sides":3,"repeat":2,"dx":1.3,"far":0}'),
  ('guards_2','torus','mat.white_composite',0,'body','{"x":-0.65,"y":0.13,"z":0.55,"w":0.82,"t":0.035,"segments":12,"sides":3,"repeat":2,"dx":1.3,"far":0}'),
  ('clamps','box','mat.graphite',0,'body','{"x":-0.3,"y":-0.1,"w":0.04,"d":0.6,"h":0.12,"repeat":2,"dx":0.6,"far":0}'),
  ('pod','box','mat.composite_panel',0,'body','{"y":-0.56,"w":0.6,"d":0.92,"h":0.48}'),
  ('pod_band','box','mat.white_composite',1,'pod','{"y":0.16,"w":0.62,"d":0.94,"h":0.12}'),
  ('nav','box','mat.light_strip',0,'body','{"y":0.04,"z":0.46,"w":0.3,"d":0.02,"h":0.03,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.cargo_drone_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.cargo_drone'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.cargo_drone_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
