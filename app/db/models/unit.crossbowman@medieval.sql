-- unit.crossbowman@medieval: Crossbowman (mesh.crossbowman_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.crossbowman_medieval','Crossbowman');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.crossbowman_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"attack":5,"hold_right":20}'),
  ('cap','dome','mat.iron',0,'head','{"y":-0.13,"w":0.25,"h":0.15,"segments":8}'),
  ('stock','box','mat.timber',0,'right_hand','{"y":-0.16,"w":0.05,"d":0.06,"h":0.62}'),
  ('prod','bow','mat.iron',0,'stock','{"y":0.42,"roll":90,"pitch":-90,"h":0.56,"bend":0.09,"w":0.03}'),
  ('pavise','shield','mat.cloth',1,'back','{"y":-0.12,"z":-0.05,"yaw":180,"w":0.5,"h":0.82,"t":0.04,"boss":0,"rim":"#5a3d26"}'),
  ('bolts','box','mat.leather',0,'body','{"x":0.2,"y":0.32,"z":0.02,"w":0.07,"d":0.12,"h":0.2}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.crossbowman_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.crossbowman'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.crossbowman_medieval'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
