-- unit.militia_starfaring@starfaring: Planetary guard (mesh.militia_starfaring_starfaring)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.militia_starfaring_starfaring','Planetary guard');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.militia_starfaring_starfaring';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"attack":5,"hold_right":10,"legs":"#59616b"}'),
  ('helmet','dome','mat.white_composite',0,'head','{"y":-0.1,"w":0.32,"h":0.18,"segments":8}'),
  ('visor','box','mat.light_strip',0,'head','{"y":-0.06,"z":0.12,"w":0.2,"d":0.02,"h":0.05,"far":0}'),
  ('scarf','box','mat.signal_red',0,'body','{"y":0.72,"w":0.3,"d":0.23,"h":0.05,"far":0}'),
  ('carbine','box','mat.graphite',0,'right_hand','{"y":-0.2,"w":0.06,"d":0.1,"h":0.44,"far":0}'),
  ('muzzle','cylinder','mat.graphite',0,'carbine','{"y":0.44,"w":0.045,"h":0.1,"segments":4,"far":0}'),
  ('spark','cone','mat.flash',0,'muzzle','{"y":0.11,"w":0.1,"h":0.1,"segments":3,"flash":1,"far":0}'),
  ('bolt','cylinder','mat.light_strip',0,'spark','{"y":0.3,"w":0.05,"h":0.4,"segments":3,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.militia_starfaring_starfaring'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.militia_starfaring'), NULL, (SELECT id FROM def WHERE key='age.starfaring'), (SELECT id FROM def WHERE key='mesh.militia_starfaring_starfaring'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
