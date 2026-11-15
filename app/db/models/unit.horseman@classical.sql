-- unit.horseman@classical: Classical horseman (mesh.horseman_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.horseman_classical','Classical horseman');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.horseman_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('beast','quadruped','mat.hide',0,'','{"pace":0.62}'),
  ('blanket','box','mat.cloth',1,'beast.rider','{"y":-0.08,"z":-0.02,"w":0.36,"d":0.44,"h":0.1}'),
  ('rider','figure','mat.cloth',1,'beast.rider','{"y":-0.4,"seat":1,"attack":2,"hold_left":20}'),
  ('cap','dome','mat.leather',0,'rider.head','{"y":-0.11,"w":0.235,"h":0.12,"segments":8}'),
  ('javelin','cylinder','mat.timber',0,'rider.right_hand','{"y":-0.35,"w":0.03,"h":1.15,"segments":4}'),
  ('javelin_tip','cone','mat.iron',0,'javelin','{"y":1.15,"w":0.06,"h":0.13,"segments":4}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.horseman_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.horseman'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.horseman_classical'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
