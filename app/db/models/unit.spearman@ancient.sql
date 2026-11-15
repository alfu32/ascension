-- unit.spearman@ancient: Early spearman (mesh.spearman_ancient)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.spearman_ancient','Early spearman');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.spearman_ancient';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"attack":1,"hold_right":12,"hold_left":25}'),
  ('cap','dome','mat.leather',0,'head','{"y":-0.11,"w":0.235,"h":0.12,"segments":8}'),
  ('spear','cylinder','mat.timber',0,'right_hand','{"y":-0.45,"w":0.035,"h":1.75,"segments":4}'),
  ('spear_tip','cone','mat.bronze',0,'spear','{"y":1.75,"w":0.07,"h":0.16,"segments":4}'),
  ('shield','shield','mat.cloth',1,'left_hand','{"y":0.06,"z":0.07,"yaw":-15,"w":0.38,"t":0.03,"boss":0.08,"segments":8,"rim":"#5a3d26"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.spearman_ancient'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.spearman'), NULL, (SELECT id FROM def WHERE key='age.ancient'), (SELECT id FROM def WHERE key='mesh.spearman_ancient'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
