-- bld.wall_gate@automation: Sensor gate (mesh.wall_gate_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.wall_gate_automation','Sensor gate');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.wall_gate_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('bed','box','mat.white_composite',0,'','{"w":0.96,"d":0.86,"h":0.02}'),
  ('pillar_l','box','mat.white_composite',0,'','{"x":-0.41,"w":0.16,"d":0.3,"h":1.7,"taper":0.15}'),
  ('pillar_r','box','mat.white_composite',0,'','{"x":0.41,"w":0.16,"d":0.3,"h":1.7,"taper":0.15}'),
  ('lintel','box','mat.white_composite',0,'','{"y":1.6,"w":0.98,"d":0.3,"h":0.12}'),
  ('panel','box','mat.cloth',1,'','{"y":1.72,"w":0.6,"d":0.2,"h":0.03}'),
  ('scan_l','box','mat.light_strip',0,'','{"x":-0.335,"y":0.2,"w":0.02,"d":0.12,"h":1.2}'),
  ('scan_r','box','mat.light_strip',0,'','{"x":0.335,"y":0.2,"w":0.02,"d":0.12,"h":1.2}'),
  ('lamp','dome','mat.light_strip',0,'','{"y":1.72,"w":0.16,"h":0.1,"segments":6}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.wall_gate_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.wall_gate'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.wall_gate_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
