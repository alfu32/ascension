-- unit.arquebusier@gunpowder: Arquebusier (mesh.arquebusier_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.arquebusier_gunpowder','Arquebusier');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.arquebusier_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"attack":5,"hold_right":10,"build":0.95,"legs":"#6a5444"}'),
  ('breastplate','box','mat.steel',0,'body','{"y":0.5,"w":0.32,"d":0.22,"h":0.25,"taper":0.1,"far":0}'),
  ('morion','dome','mat.steel',0,'head','{"y":-0.12,"w":0.25,"h":0.17,"segments":7}'),
  ('comb','box','mat.steel',0,'head','{"y":-0.02,"w":0.025,"d":0.22,"h":0.12,"taper":0.2}'),
  ('brim','box','mat.steel',0,'head','{"y":-0.115,"w":0.24,"d":0.44,"h":0.02,"far":0}'),
  ('stock','box','mat.timber',0,'right_hand','{"y":-0.25,"w":0.045,"d":0.07,"h":0.36,"far":0}'),
  ('barrel','cylinder','mat.iron',0,'stock','{"y":0.3,"w":0.03,"h":0.62,"segments":5}'),
  ('spark','cone','mat.flash',0,'barrel','{"y":0.62,"w":0.08,"h":0.08,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.24,"w":0.22,"h":0.24,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.arquebusier_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.arquebusier'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.arquebusier_gunpowder'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
