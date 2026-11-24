-- unit.militia_medieval@medieval: Levy (mesh.militia_medieval_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.militia_medieval_medieval','Levy');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.militia_medieval_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"attack":1,"hold_right":10}'),
  ('hood','dome','mat.felt',0,'head','{"y":-0.12,"w":0.25,"h":0.15,"segments":8}'),
  ('bill','cylinder','mat.timber',0,'right_hand','{"y":-0.5,"w":0.035,"h":1.9,"segments":4}'),
  ('bill_blade','box','mat.iron',0,'bill','{"y":1.9,"z":0.04,"w":0.02,"d":0.12,"h":0.24,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.militia_medieval_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.militia_medieval'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.militia_medieval_medieval'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
