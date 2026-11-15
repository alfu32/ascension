-- unit.merchant@ancient: Early merchant (mesh.merchant_ancient)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.merchant_ancient','Early merchant');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.merchant_ancient';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"build":1.08,"pace":1.15,"stride":22,"hold_left":10}'),
  ('pack','box','mat.thatch',0,'back','{"y":-0.2,"z":-0.09,"w":0.32,"d":0.22,"h":0.42}'),
  ('bedroll','cylinder','mat.cloth',1,'pack','{"x":-0.2,"y":0.42,"w":0.13,"h":0.4,"roll":90,"segments":6,"far":0}'),
  ('stick','cylinder','mat.timber',0,'left_hand','{"y":-0.45,"w":0.035,"h":1.15,"segments":4}'),
  ('brim','cylinder','mat.thatch',0,'head','{"y":-0.07,"w":0.36,"h":0.02,"segments":10,"far":0}'),
  ('crown','dome','mat.thatch',0,'head','{"y":-0.06,"w":0.2,"h":0.1,"segments":8}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.merchant_ancient'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.merchant'), NULL, (SELECT id FROM def WHERE key='age.ancient'), (SELECT id FROM def WHERE key='mesh.merchant_ancient'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
