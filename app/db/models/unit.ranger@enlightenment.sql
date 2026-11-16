-- unit.ranger@enlightenment: Ranger (mesh.ranger_enlightenment)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.ranger_enlightenment','Ranger');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.ranger_enlightenment';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"attack":5,"hold_right":10,"stride":32,"legs":"#7a6448"}'),
  ('coat','box','mat.cloth',1,'body','{"y":0.18,"w":0.37,"d":0.26,"h":0.34}'),
  ('fringe','box','mat.canvas',0,'body','{"y":0.12,"w":0.39,"d":0.28,"h":0.07,"tint":"#e8d4ae","far":0}'),
  ('brim','cylinder','mat.leather',0,'head','{"y":-0.07,"w":0.5,"h":0.025,"segments":8}'),
  ('crown','cylinder','mat.leather',0,'head','{"y":-0.06,"w":0.22,"h":0.12,"taper":0.1,"segments":6,"tint":"#d8c8b0","far":0}'),
  ('stock','box','mat.timber',0,'right_hand','{"y":-0.3,"w":0.05,"d":0.08,"h":0.42,"far":0,"tint":"#5a3a24"}'),
  ('barrel','cylinder','mat.iron',0,'stock','{"y":0.34,"w":0.028,"h":0.66,"segments":5}'),
  ('spark','cone','mat.flash',0,'barrel','{"y":0.66,"w":0.08,"h":0.08,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.26,"w":0.24,"h":0.26,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.ranger_enlightenment'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.ranger'), NULL, (SELECT id FROM def WHERE key='age.enlightenment'), (SELECT id FROM def WHERE key='mesh.ranger_enlightenment'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
