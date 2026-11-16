-- unit.worker_drone@automation: Worker drone (mesh.worker_drone_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.worker_drone_automation','Worker drone');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.worker_drone_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.white_composite',1,'','{"h":0.95,"build":1.2,"work":0,"skin":"#6a7078","hair":"#30363c","legs":"#4d545c","stride":22,"pace":0.8}'),
  ('head_box','box','mat.white_composite',0,'head','{"y":-0.21,"w":0.25,"d":0.23,"h":0.21,"taper":0.1}'),
  ('visor','box','mat.light_strip',0,'head','{"y":-0.13,"z":0.112,"w":0.19,"d":0.02,"h":0.045}'),
  ('antenna','cylinder','mat.graphite',0,'head','{"x":0.08,"w":0.015,"h":0.2,"segments":3,"far":0}'),
  ('pack','box','mat.white_composite',0,'back','{"y":-0.2,"z":-0.04,"w":0.3,"d":0.13,"h":0.3}'),
  ('pack_strip','box','mat.light_strip',0,'pack','{"y":0.12,"z":-0.07,"w":0.2,"d":0.01,"h":0.03,"far":0}'),
  ('lance','cylinder','mat.graphite',0,'right_hand','{"y":-0.3,"w":0.05,"h":0.6,"segments":4}'),
  ('tool_head','box','mat.white_composite',0,'lance','{"y":0.56,"z":0.03,"w":0.1,"d":0.14,"h":0.09}'),
  ('bit','cone','mat.graphite',0,'tool_head','{"y":0.045,"z":0.07,"w":0.06,"h":0.14,"segments":4,"pitch":90,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.worker_drone_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.worker_drone'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.worker_drone_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
