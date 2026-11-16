-- unit.medic@industrial: Medic (mesh.medic_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.medic_industrial','Medic');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.medic_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"work":2,"legs":"#4e4a3a"}'),
  ('cap','dome','mat.khaki',0,'head','{"y":-0.08,"w":0.25,"h":0.13,"segments":6}'),
  ('cap_band','cylinder','mat.canvas',0,'head','{"y":-0.09,"w":0.255,"h":0.04,"segments":7,"far":0}'),
  ('armband','box','mat.canvas',0,'left_hand','{"y":0.17,"w":0.1,"d":0.11,"h":0.07}'),
  ('strap','box','mat.leather',0,'body','{"x":0.02,"y":0.48,"z":0.115,"w":0.04,"d":0.02,"h":0.36,"roll":-36,"far":0}'),
  ('satchel','box','mat.khaki',0,'body','{"x":0.09,"y":0.3,"z":0.13,"w":0.17,"d":0.07,"h":0.15}'),
  ('stretcher','cylinder','mat.canvas',0,'back','{"x":-0.31,"y":0.12,"z":-0.04,"w":0.14,"h":0.62,"segments":6,"roll":90}'),
  ('poles','cylinder','mat.timber',0,'back','{"x":-0.4,"y":0.19,"z":-0.04,"w":0.035,"h":0.8,"segments":4,"roll":90,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.medic_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.medic'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.medic_industrial'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
