-- unit.scholar@classical: Scholar (mesh.scholar_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.scholar_classical','Scholar');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.scholar_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',0,'','{"robe":1,"hold_right":38,"hold_left":38,"stride":24,"pace":1.15,"hair":"#d6d0c4"}'),
  ('beard','box','mat.cloth',0,'head','{"y":-0.225,"z":0.08,"w":0.1,"d":0.04,"h":0.08,"tint":"#e2ddd2"}'),
  ('stole','box','mat.cloth',1,'body','{"x":-0.075,"y":0.3,"z":0.118,"w":0.08,"d":0.02,"h":0.46,"repeat":2,"dx":0.15,"far":0}'),
  ('hem','box','mat.cloth',1,'body','{"w":0.43,"d":0.29,"h":0.12}'),
  ('cap','dome','mat.cloth',1,'head','{"y":-0.09,"z":-0.01,"w":0.225,"h":0.1,"segments":8}'),
  ('scroll','cylinder','mat.cloth',0,'right_hand','{"y":0.02,"z":0.06,"w":0.06,"h":0.42,"roll":90,"segments":6,"tint":"#f6eccf"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.scholar_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.scholar'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.scholar_classical'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
