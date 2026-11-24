-- unit.militia_ancient@ancient: Levy (mesh.militia_ancient_ancient)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.militia_ancient_ancient','Levy');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.militia_ancient_ancient';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"attack":1,"hold_right":12}'),
  ('cap','dome','mat.leather',0,'head','{"y":-0.1,"w":0.23,"h":0.1,"segments":8}'),
  ('spear','cylinder','mat.timber',0,'right_hand','{"y":-0.45,"w":0.035,"h":1.6,"segments":4}'),
  ('spear_tip','cone','mat.bronze',0,'spear','{"y":1.6,"w":0.07,"h":0.16,"segments":4}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.militia_ancient_ancient'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.militia_ancient'), NULL, (SELECT id FROM def WHERE key='age.ancient'), (SELECT id FROM def WHERE key='mesh.militia_ancient_ancient'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
