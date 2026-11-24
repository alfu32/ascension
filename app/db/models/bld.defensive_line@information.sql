-- bld.defensive_line@information: Barrier line (mesh.defensive_line_information)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.defensive_line_information','Barrier line');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.defensive_line_information';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('bed','box','mat.precast',0,'','{"w":0.96,"d":0.86,"h":0.02}'),
  ('barrier','box','mat.precast',0,'','{"z":-0.1,"w":0.98,"d":0.5,"h":0.25}'),
  ('barrier_2','box','mat.precast',0,'','{"y":0.25,"z":-0.1,"w":0.98,"d":0.26,"h":0.45}'),
  ('stripe','box','mat.cloth',1,'','{"y":0.62,"z":-0.1,"w":0.98,"d":0.2,"h":0.08}'),
  ('fence','fence','mat.steel',0,'','{"z":0.3,"w":0.98,"h":1.1,"spacing":0.5,"rails":5}'),
  ('wire','torus','mat.steel',0,'','{"y":1.1,"z":0.3,"roll":90,"w":0.28,"t":0.025,"segments":10,"sides":3,"repeat":3,"dx":0.33,"x":-0.33,"far":0}'),
  ('traps','cone','mat.concrete',0,'','{"x":-0.25,"z":0.48,"w":0.26,"h":0.32,"segments":4,"repeat":2,"dx":0.5,"far":0}'),
  ('team_front','box','mat.cloth',1,'','{"y":0.03,"z":0.165,"w":0.98,"d":0.02,"h":0.2}'),
  ('team_back','box','mat.cloth',1,'','{"y":0.03,"z":-0.365,"w":0.98,"d":0.02,"h":0.2}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.defensive_line_information'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.defensive_line'), NULL, (SELECT id FROM def WHERE key='age.information'), (SELECT id FROM def WHERE key='mesh.defensive_line_information'), (SELECT id FROM def WHERE key='mat.precast'), 100);
