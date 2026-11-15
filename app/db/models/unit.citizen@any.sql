-- unit.citizen@any: Citizen (mesh.citizen)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.citizen','Citizen');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.citizen';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"h":1.0}'),
  ('hoe','cylinder','mat.timber',0,'right_hand','{"y":-0.4,"w":0.035,"h":0.8,"segments":4}'),
  ('blade','box','mat.iron',0,'hoe','{"z":0.06,"w":0.06,"d":0.17,"h":0.08}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.citizen'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.citizen'), NULL, NULL, (SELECT id FROM def WHERE key='mesh.citizen'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
