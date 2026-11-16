-- bld.bastion@automation: Future bastion (mesh.bastion_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.bastion_automation','Future bastion');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.bastion_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('works','custom','mat.white_composite',0,'','{"file":"bld.bastion@automation/works.mesh"}'),
  ('gorge','box','mat.white_composite',0,'','{"z":-0.88,"yaw":180,"w":1,"d":0.06,"h":0.9,"door":0.3,"door_h":0.6,"door_colour":"#2c3a40"}'),
  ('casemate','box','mat.composite_panel',0,'','{"y":0.82,"z":0.1,"w":0.78,"d":0.66,"h":0.3,"taper":0.22,"windows":1,"window_w":0.4,"window_h":0.07,"window_y":0.12,"opening":"#2c5258"}'),
  ('casemate_band','box','mat.white_composite',1,'','{"y":0.82,"z":0.1,"w":0.82,"d":0.7,"h":0.1}'),
  ('casemate_glow','box','mat.light_strip',0,'','{"y":0.92,"z":0.1,"w":0.8,"d":0.68,"h":0.02,"far":0}'),
  ('turret','dome','mat.white_composite',0,'','{"y":1.12,"z":0.05,"w":0.46,"h":0.24,"segments":8}'),
  ('gun','cylinder','mat.graphite',0,'turret','{"y":0.1,"z":0.1,"w":0.08,"h":0.62,"segments":5,"pitch":86}'),
  ('flag','banner','mat.cloth',1,'','{"x":0.45,"y":0.82,"z":-0.55,"h":1.8,"fw":0.8,"fh":0.56,"pole_colour":"#e8ecec"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.bastion_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.bastion'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.bastion_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
