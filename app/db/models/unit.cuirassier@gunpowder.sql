-- unit.cuirassier@gunpowder: Cuirassier (mesh.cuirassier_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.cuirassier_gunpowder','Cuirassier');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.cuirassier_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('beast','quadruped','mat.hide',0,'','{"pace":0.72,"stride":26,"tint":"#5a463c"}'),
  ('saddle_cloth','box','mat.cloth',1,'beast.rider','{"y":-0.1,"z":-0.04,"w":0.4,"d":0.6,"h":0.16}'),
  ('holsters','box','mat.leather',0,'beast.rider','{"z":0.26,"y":-0.06,"w":0.42,"d":0.1,"h":0.14,"far":0}'),
  ('rider','figure','mat.cloth',1,'beast.rider','{"y":-0.38,"seat":1,"attack":6,"hold_right":10,"build":1.1}'),
  ('cuirass','box','mat.steel',0,'rider','{"y":0.46,"w":0.38,"d":0.26,"h":0.32,"taper":0.08}'),
  ('helmet','dome','mat.steel',0,'rider.head','{"y":-0.14,"w":0.25,"h":0.17,"segments":8}'),
  ('tail','box','mat.steel',0,'rider.head','{"y":-0.2,"z":-0.1,"w":0.18,"d":0.06,"h":0.12,"pitch":-30,"far":0}'),
  ('visor','box','mat.steel',0,'rider.head','{"y":-0.13,"z":0.11,"w":0.04,"d":0.03,"h":0.12,"far":0}'),
  ('grip','box','mat.timber',0,'rider.right_hand','{"y":-0.06,"w":0.04,"d":0.05,"h":0.12}'),
  ('pistol','cylinder','mat.iron',0,'grip','{"y":0.06,"w":0.035,"h":0.32,"segments":5}'),
  ('spark','cone','mat.flash',0,'pistol','{"y":0.32,"w":0.08,"h":0.08,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.28,"w":0.28,"h":0.28,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.cuirassier_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.cuirassier'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.cuirassier_gunpowder'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
