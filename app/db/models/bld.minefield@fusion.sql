-- bld.minefield@fusion: Field mines (mesh.minefield_fusion)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.minefield_fusion','Field mines');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.minefield_fusion';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('mines','cylinder','mat.white_metal',0,'','{"x":-0.28,"z":-0.22,"w":0.22,"h":0.06,"segments":8,"repeat":2,"dx":0.5,"dz":0.1}'),
  ('mines_back','cylinder','mat.white_metal',0,'','{"x":-0.05,"z":0.25,"w":0.22,"h":0.06,"segments":8,"repeat":2,"dx":0.36,"dz":-0.06}'),
  ('fuzes','cylinder','mat.glow',0,'','{"x":-0.28,"y":0.06,"z":-0.22,"w":0.06,"h":0.03,"segments":6,"repeat":2,"dx":0.5,"dz":0.1,"far":0}'),
  ('stake','box','mat.white_metal',0,'','{"x":0.42,"z":0.42,"w":0.04,"d":0.04,"h":0.5}'),
  ('sign','box','mat.cloth',1,'','{"x":0.42,"y":0.36,"z":0.445,"w":0.2,"d":0.015,"h":0.14}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.minefield_fusion'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.minefield'), NULL, (SELECT id FROM def WHERE key='age.fusion'), (SELECT id FROM def WHERE key='mesh.minefield_fusion'), (SELECT id FROM def WHERE key='mat.white_metal'), 100);
