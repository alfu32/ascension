-- unit.priest@classical: Priest (mesh.priest_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.priest_classical','Priest');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.priest_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',0,'','{"robe":1,"work":1,"hold_right":15,"tint":"#fffaf0"}'),
  ('mantle','box','mat.cloth',1,'body','{"y":0.5,"w":0.38,"d":0.26,"h":0.27,"taper":0.3}'),
  ('headdress','cylinder','mat.bronze',0,'head','{"y":-0.07,"w":0.2,"h":0.22,"segments":8}'),
  ('staff','cylinder','mat.timber',0,'right_hand','{"y":-0.45,"w":0.04,"h":1.32,"segments":5}'),
  ('orb','dome','mat.bronze',0,'staff','{"y":1.4,"w":0.17,"h":0.09,"segments":8}'),
  ('orb_under','dome','mat.bronze',0,'staff','{"y":1.4,"w":0.17,"h":0.09,"segments":8,"pitch":180}'),
  ('ribbons','box','mat.cloth',1,'staff','{"y":1.06,"z":0.03,"w":0.05,"d":0.02,"h":0.26}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.priest_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.priest'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.priest_classical'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
