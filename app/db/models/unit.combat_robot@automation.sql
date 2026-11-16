-- unit.combat_robot@automation: Combat robot (mesh.combat_robot_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.combat_robot_automation','Combat robot');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.combat_robot_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.white_composite',1,'','{"h":1.08,"build":1.1,"attack":6,"skin":"#3e4145","hair":"#2c2e31","legs":"#4d545c","stride":30}'),
  ('chest','box','mat.white_composite',0,'body','{"y":0.6,"z":0.015,"w":0.34,"d":0.24,"h":0.2,"taper":-0.12}'),
  ('helmet','box','mat.graphite',0,'head','{"y":-0.24,"w":0.2,"d":0.25,"h":0.22,"taper":0.3}'),
  ('visor','box','mat.signal_red',0,'head','{"y":-0.15,"z":0.105,"w":0.16,"d":0.02,"h":0.035}'),
  ('pads','box','mat.white_composite',0,'body','{"x":-0.25,"y":0.76,"w":0.16,"d":0.22,"h":0.1,"taper":0.3,"repeat":2,"dx":0.5}'),
  ('gun_arm','box','mat.graphite',0,'right_hand','{"y":-0.06,"w":0.11,"d":0.13,"h":0.28}'),
  ('barrel','cylinder','mat.graphite',0,'gun_arm','{"y":0.28,"w":0.05,"h":0.12,"segments":5}'),
  ('spark','cone','mat.flash',0,'barrel','{"y":0.13,"w":0.08,"h":0.08,"segments":3,"flash":3,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.22,"w":0.2,"h":0.22,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.combat_robot_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.combat_robot'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.combat_robot_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
