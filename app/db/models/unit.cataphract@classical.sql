-- unit.cataphract@classical: Cataphract (mesh.cataphract_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.cataphract_classical','Cataphract');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.cataphract_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('beast','quadruped','mat.hide',0,'','{"pace":0.78,"stride":24,"tint":"#7a6a60"}'),
  ('barding','box','mat.iron',0,'beast','{"y":0.47,"w":0.37,"d":1.0,"h":0.36,"taper":0.08}'),
  ('trim','box','mat.cloth',1,'beast','{"y":0.43,"w":0.39,"d":1.02,"h":0.1,"far":0}'),
  ('rider','figure','mat.cloth',1,'beast.rider','{"y":-0.37,"seat":1,"attack":1,"build":1.1,"hold_left":20}'),
  ('armour','box','mat.iron',0,'rider','{"y":0.5,"w":0.37,"d":0.25,"h":0.27,"far":0}'),
  ('helmet','dome','mat.iron',0,'rider.head','{"y":-0.15,"w":0.25,"h":0.18,"segments":8}'),
  ('spike','cone','mat.iron',0,'rider.head','{"y":0.02,"w":0.05,"h":0.12,"segments":4}'),
  ('lance','cylinder','mat.timber',0,'rider.right_hand','{"y":-0.6,"w":0.042,"h":2.1,"segments":4}'),
  ('lance_tip','cone','mat.iron',0,'lance','{"y":2.1,"w":0.07,"h":0.18,"segments":4}'),
  ('pennon','box','mat.cloth',1,'lance','{"y":1.72,"z":-0.12,"w":0.015,"d":0.22,"h":0.16,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.cataphract_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.cataphract'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.cataphract_classical'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
