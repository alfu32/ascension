-- bld.defensive_line@modern: Defensive line (mesh.defensive_line_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.defensive_line_modern','Defensive line');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.defensive_line_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('parapet','box','mat.soil',0,'','{"z":-0.12,"w":1.0,"d":0.55,"h":0.22}'),
  ('parapet_2','box','mat.soil',0,'','{"y":0.22,"z":-0.12,"w":1.0,"d":0.4,"h":0.16}'),
  ('sandbags','box','mat.sandbag',0,'','{"y":0.38,"z":-0.14,"w":1.0,"d":0.3,"h":0.2}'),
  ('wire','torus','mat.iron',0,'','{"y":0.2,"z":0.18,"roll":90,"w":0.4,"t":0.03,"segments":10,"sides":3,"repeat":3,"dx":0.33,"x":-0.33}'),
  ('traps','cone','mat.concrete',0,'','{"x":-0.25,"z":0.42,"w":0.3,"h":0.4,"segments":4,"repeat":2,"dx":0.5}'),
  ('pennant','banner','mat.cloth',1,'','{"x":0.4,"y":0.58,"z":-0.2,"h":0.6,"fw":0.28,"fh":0.18}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.defensive_line_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.defensive_line'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.defensive_line_modern'), (SELECT id FROM def WHERE key='mat.concrete'), 100);
