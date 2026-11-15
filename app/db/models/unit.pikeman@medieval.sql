-- unit.pikeman@medieval: Pikeman (mesh.pikeman_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.pikeman_medieval','Pikeman');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.pikeman_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"attack":1,"hold_right":10,"hold_left":28,"build":1.05}'),
  ('coat','box','mat.cloth',1,'body','{"y":0.26,"w":0.4,"d":0.28,"h":0.32,"taper":0.12,"tint":"#e6e0d6"}'),
  ('helmet','dome','mat.iron',0,'head','{"y":-0.14,"w":0.25,"h":0.15,"segments":8}'),
  ('brim','cylinder','mat.iron',0,'head','{"y":-0.13,"w":0.42,"h":0.025,"segments":8}'),
  ('pike','cylinder','mat.timber',0,'right_hand','{"y":-0.5,"w":0.035,"h":3,"segments":4}'),
  ('pike_tip','cone','mat.iron',0,'pike','{"y":3,"w":0.07,"h":0.2,"segments":4}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.pikeman_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.pikeman'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.pikeman_medieval'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
