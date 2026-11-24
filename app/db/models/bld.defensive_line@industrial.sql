-- bld.defensive_line@industrial: Trench line (mesh.defensive_line_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.defensive_line_industrial','Trench line');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.defensive_line_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('parapet','box','mat.soil',0,'','{"z":0.12,"w":1.0,"d":0.7,"h":0.25}'),
  ('parapet_2','box','mat.soil',0,'','{"y":0.25,"z":0.12,"w":1.0,"d":0.48,"h":0.17}'),
  ('revetment','box','mat.sawn_timber',0,'','{"z":-0.25,"w":1.0,"d":0.08,"h":0.4}'),
  ('sandbags','box','mat.sandbag',0,'','{"y":0.42,"z":0.1,"w":1.0,"d":0.34,"h":0.2}'),
  ('wire_posts','fence','mat.iron',0,'','{"z":0.46,"w":1.0,"h":0.55,"spacing":0.5,"rails":4}'),
  ('pennant','banner','mat.cloth',1,'','{"x":0.35,"y":0.6,"z":-0.1,"h":0.65,"fw":0.28,"fh":0.18}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.defensive_line_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.defensive_line'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.defensive_line_industrial'), (SELECT id FROM def WHERE key='mat.soil'), 100);
