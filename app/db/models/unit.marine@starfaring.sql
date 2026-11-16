-- unit.marine@starfaring: Marine (mesh.marine_starfaring)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.marine_starfaring','Marine');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.marine_starfaring';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"attack":5,"hold_right":10,"build":1.1,"skin":"#c9ced2","hair":"#30363c","legs":"#59616b"}'),
  ('helmet','dome','mat.white_composite',0,'head','{"y":-0.1,"w":0.36,"h":0.22,"segments":10}'),
  ('shell','box','mat.white_composite',1,'head','{"y":-0.26,"w":0.32,"d":0.33,"h":0.17,"taper":-0.05}'),
  ('visor','box','mat.brass',0,'head','{"y":-0.15,"z":0.125,"w":0.26,"d":0.07,"h":0.13,"taper":0.1,"far":0}'),
  ('pack','box','mat.white_composite',0,'back','{"y":-0.3,"z":-0.07,"w":0.3,"d":0.14,"h":0.36,"far":0}'),
  ('carbine','box','mat.graphite',0,'right_hand','{"y":-0.2,"w":0.07,"d":0.11,"h":0.48,"far":0}'),
  ('muzzle','cylinder','mat.graphite',0,'carbine','{"y":0.48,"w":0.05,"h":0.12,"segments":4,"far":0}'),
  ('spark','cone','mat.flash',0,'muzzle','{"y":0.13,"w":0.12,"h":0.12,"segments":3,"flash":1,"far":0}'),
  ('bolt','cylinder','mat.light_strip',0,'spark','{"y":0.3,"w":0.05,"h":0.45,"segments":3,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.marine_starfaring'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.marine'), NULL, (SELECT id FROM def WHERE key='age.starfaring'), (SELECT id FROM def WHERE key='mesh.marine_starfaring'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
