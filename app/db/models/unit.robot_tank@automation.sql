-- unit.robot_tank@automation: Robot tank (mesh.robot_tank_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.robot_tank_automation','Robot tank');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.robot_tank_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('track_w','track','mat.graphite',0,'','{"x":-0.6,"l":2.3,"h":0.42,"t":0.32,"wheels":5,"wheel_colour":"#2c2e31"}'),
  ('track_e','track','mat.graphite',0,'','{"x":0.6,"l":2.3,"h":0.42,"t":0.32,"wheels":5,"wheel_colour":"#2c2e31"}'),
  ('hull','box','mat.white_composite',0,'','{"y":0.12,"z":-0.05,"w":0.9,"d":2,"h":0.3}'),
  ('glacis','box','mat.white_composite',0,'','{"y":0.14,"z":0.94,"w":0.9,"d":0.05,"h":0.36,"pitch":-70}'),
  ('deck','box','mat.white_composite',0,'','{"y":0.42,"w":1.5,"d":2.24,"h":0.05}'),
  ('skirts','box','mat.white_composite',1,'','{"x":-0.77,"y":0.16,"w":0.04,"d":2.1,"h":0.28,"repeat":2,"dx":1.54}'),
  ('nose_strip','box','mat.light_strip',0,'','{"y":0.4,"z":1.12,"w":0.9,"d":0.02,"h":0.025,"far":0}'),
  ('turret','box','mat.white_composite',0,'','{"y":0.47,"z":-0.15,"w":0.96,"d":1,"h":0.28,"taper":0.32,"traverse":25}'),
  ('turret_top','box','mat.white_composite',1,'turret','{"y":0.28,"w":0.62,"d":0.64,"h":0.02}'),
  ('gun','cylinder','mat.graphite',0,'turret','{"y":0.13,"z":0.4,"w":0.09,"h":1.5,"segments":6,"pitch":90,"recoil":0.12}'),
  ('muzzle','cylinder','mat.graphite',0,'gun','{"y":1.38,"w":0.12,"h":0.12,"segments":6,"far":0}'),
  ('mast','cylinder','mat.graphite',0,'turret','{"x":-0.24,"y":0.2,"z":-0.28,"w":0.06,"h":0.9,"segments":5}'),
  ('mast_head','box','mat.white_composite',0,'mast','{"y":0.9,"w":0.26,"d":0.18,"h":0.16}'),
  ('mast_eye','box','mat.light_strip',0,'mast_head','{"y":0.05,"z":0.09,"w":0.18,"d":0.02,"h":0.05,"far":0}'),
  ('mast_dome','dome','mat.white_composite',1,'mast_head','{"y":0.16,"w":0.14,"h":0.08,"segments":6,"far":0}'),
  ('spark','cone','mat.flash',0,'gun','{"y":1.5,"w":0.12,"h":0.12,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.55,"w":0.55,"h":0.55,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.robot_tank_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.robot_tank'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.robot_tank_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
