-- unit.drone_fighter@automation: Drone fighter (mesh.drone_fighter_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.drone_fighter_automation','Drone fighter');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.drone_fighter_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('fuselage','box','mat.graphite',0,'','{"y":3.6,"z":-0.1,"w":0.26,"d":1.6,"h":0.22,"taper":0.35,"bank":26}'),
  ('nose','cone','mat.graphite',0,'fuselage','{"y":0.1,"z":0.8,"w":0.2,"h":0.45,"segments":6,"pitch":90}'),
  ('sensor','box','mat.white_composite',0,'fuselage','{"y":0.2,"z":0.32,"w":0.14,"d":0.42,"h":0.06,"taper":0.4}'),
  ('intake','box','mat.white_composite',0,'fuselage','{"y":0.18,"z":-0.15,"w":0.2,"d":0.42,"h":0.1,"taper":0.3}'),
  ('wing_e','box','mat.graphite',0,'fuselage','{"y":0.05,"z":-0.2,"w":0.04,"d":0.9,"h":0.75,"taper":0.8,"roll":90,"yaw":45}'),
  ('wing_w','box','mat.graphite',0,'fuselage','{"y":0.05,"z":-0.2,"w":0.04,"d":0.9,"h":0.75,"taper":0.8,"roll":-90,"yaw":-45}'),
  ('tip_e','box','mat.white_composite',1,'wing_e','{"y":0.42,"w":0.035,"d":0.55,"h":0.33,"taper":0.6}'),
  ('tip_w','box','mat.white_composite',1,'wing_w','{"y":0.42,"w":0.035,"d":0.55,"h":0.33,"taper":0.6}'),
  ('fin_e','box','mat.graphite',0,'fuselage','{"x":0.08,"y":0.18,"z":-0.62,"w":0.03,"d":0.4,"h":0.34,"taper":0.55,"roll":40,"pitch":-20}'),
  ('fin_w','box','mat.graphite',0,'fuselage','{"x":-0.08,"y":0.18,"z":-0.62,"w":0.03,"d":0.4,"h":0.34,"taper":0.55,"roll":-40,"pitch":-20}'),
  ('nozzle','cylinder','mat.graphite',0,'fuselage','{"y":0.1,"z":-0.8,"w":0.16,"h":0.12,"segments":6,"pitch":-90,"far":0}'),
  ('burner','wheel','mat.glow',0,'fuselage','{"y":0.04,"z":-0.94,"yaw":90,"w":0.13,"t":0.03,"spokes":3,"segments":6}'),
  ('gun','cylinder','mat.graphite',0,'fuselage','{"x":0.11,"y":0.06,"z":0.45,"w":0.03,"h":0.3,"segments":4,"pitch":90,"far":0}'),
  ('spark','cone','mat.flash',0,'gun','{"y":0.3,"w":0.08,"h":0.08,"segments":3,"flash":3,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.26,"w":0.24,"h":0.26,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.drone_fighter_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.drone_fighter'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.drone_fighter_automation'), (SELECT id FROM def WHERE key='mat.graphite'), 100);
