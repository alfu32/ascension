-- unit.hoplite@classical: Hoplite (mesh.hoplite_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.hoplite_classical','Hoplite');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.hoplite_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"attack":1,"hold_left":35,"hold_right":12,"build":1.05}'),
  ('cuirass','box','mat.bronze',0,'body','{"y":0.52,"w":0.34,"d":0.23,"h":0.24,"far":0}'),
  ('helmet','dome','mat.bronze',0,'head','{"y":-0.15,"w":0.25,"h":0.18,"segments":8}'),
  ('crest','box','mat.cloth',1,'head','{"y":-0.01,"z":-0.03,"w":0.04,"d":0.27,"h":0.11,"taper":0.15}'),
  ('shield','shield','mat.cloth',1,'left_hand','{"y":0.06,"z":0.08,"w":0.62,"t":0.05,"boss":0.1,"segments":10}'),
  ('spear','cylinder','mat.timber',0,'right_hand','{"y":-0.5,"w":0.035,"h":1.95,"segments":4}'),
  ('spear_tip','cone','mat.iron',0,'spear','{"y":1.95,"w":0.07,"h":0.16,"segments":4}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.hoplite_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.hoplite'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.hoplite_classical'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
