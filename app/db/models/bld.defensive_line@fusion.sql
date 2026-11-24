-- bld.defensive_line@fusion: Field barrier (mesh.defensive_line_fusion)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.defensive_line_fusion','Field barrier');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.defensive_line_fusion';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('bed','box','mat.white_composite',0,'','{"w":0.96,"d":0.86,"h":0.02}'),
  ('rail','box','mat.white_composite',0,'','{"w":0.98,"d":0.42,"h":0.22}'),
  ('pylon','box','mat.white_metal',0,'','{"x":0.42,"y":0.22,"w":0.14,"d":0.2,"h":1.5,"taper":0.3}'),
  ('pylon_head','box','mat.cloth',1,'','{"x":0.4,"y":1.72,"w":0.18,"d":0.24,"h":0.1}'),
  ('field_1','box','mat.light_strip',0,'','{"x":-0.07,"y":0.75,"w":0.84,"d":0.03,"h":0.05}'),
  ('field_2','box','mat.light_strip',0,'','{"x":-0.07,"y":1.05,"w":0.84,"d":0.03,"h":0.05}'),
  ('field_3','box','mat.light_strip',0,'','{"x":-0.07,"y":1.35,"w":0.84,"d":0.03,"h":0.05,"far":0}'),
  ('field_edge','box','mat.light_strip',0,'','{"x":-0.07,"y":1.55,"w":0.84,"d":0.05,"h":0.04}'),
  ('emitter','box','mat.light_strip',0,'','{"y":0.22,"w":0.9,"d":0.08,"h":0.05}'),
  ('team_front','box','mat.cloth',1,'','{"y":0.03,"z":0.225,"w":0.98,"d":0.02,"h":0.17}'),
  ('team_back','box','mat.cloth',1,'','{"y":0.03,"z":-0.225,"w":0.98,"d":0.02,"h":0.17}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.defensive_line_fusion'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.defensive_line'), NULL, (SELECT id FROM def WHERE key='age.fusion'), (SELECT id FROM def WHERE key='mesh.defensive_line_fusion'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
