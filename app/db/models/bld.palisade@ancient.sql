-- bld.palisade@ancient: Palisade (mesh.palisade_ancient)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.palisade_ancient','Palisade');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.palisade_ancient';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('bed','box','mat.packed_earth',0,'','{"w":0.96,"d":0.86,"h":0.02}'),
  ('bank','box','mat.packed_earth',0,'','{"w":0.98,"d":0.62,"h":0.12}'),
  ('stakes','palisade','mat.timber',0,'','{"y":0.1,"w":0.98,"h":1.45,"t":0.2,"spacing":0.2}'),
  ('rail','box','mat.timber',0,'','{"y":0.55,"z":-0.13,"w":0.98,"d":0.06,"h":0.08}'),
  ('prop','box','mat.timber',0,'','{"y":0.1,"z":-0.32,"roll":0,"pitch":-35,"w":0.08,"d":0.08,"h":0.8,"far":0}'),
  ('team_front','box','mat.cloth',1,'','{"y":0.5,"z":0.115,"w":0.98,"d":0.02,"h":0.4}'),
  ('team_back','box','mat.cloth',1,'','{"y":0.5,"z":-0.115,"w":0.98,"d":0.02,"h":0.4}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.palisade_ancient'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.palisade'), NULL, (SELECT id FROM def WHERE key='age.ancient'), (SELECT id FROM def WHERE key='mesh.palisade_ancient'), (SELECT id FROM def WHERE key='mat.timber'), 100);
