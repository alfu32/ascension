-- unit.scout@ancient: Early scout (mesh.scout_ancient)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.scout_ancient','Early scout');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.scout_ancient';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.leather',0,'','{"pace":0.62,"stride":38,"build":0.92}'),
  ('cloak','box','mat.cloth',1,'back','{"y":-0.44,"z":-0.07,"w":0.46,"d":0.03,"h":0.58,"taper":0.38,"pitch":12}'),
  ('hood','dome','mat.cloth',1,'head','{"y":-0.12,"z":-0.02,"w":0.245,"h":0.14,"segments":8}'),
  ('staff','cylinder','mat.timber',0,'right_hand','{"y":-0.32,"w":0.035,"h":0.9,"segments":4}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.scout_ancient'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.scout'), NULL, (SELECT id FROM def WHERE key='age.ancient'), (SELECT id FROM def WHERE key='mesh.scout_ancient'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
