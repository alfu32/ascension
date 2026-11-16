-- unit.assault_infantry@information: Assault infantry (mesh.assault_infantry_information)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.assault_infantry_information','Assault infantry');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.assault_infantry_information';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"attack":5,"hold_right":10,"build":1.05,"legs":"#5f5c4b"}'),
  ('helmet','dome','mat.khaki',0,'head','{"y":-0.11,"w":0.28,"h":0.15,"segments":6}'),
  ('helmet_goggles','box','mat.glass',0,'head','{"y":-0.03,"z":0.11,"w":0.2,"d":0.06,"h":0.055,"tint":"#506a88","far":0}'),
  ('helmet_ears','cylinder','mat.composite',0,'head','{"y":-0.12,"x":-0.12,"w":0.065,"h":0.24,"segments":4,"roll":90,"far":0}'),
  ('plate','box','mat.khaki',0,'body','{"y":0.5,"z":0,"w":0.33,"d":0.25,"h":0.25,"taper":-0.05}'),
  ('receiver','box','mat.composite',0,'right_hand','{"y":-0.2,"w":0.05,"d":0.08,"h":0.42}'),
  ('magazine','box','mat.composite',0,'receiver','{"y":0.26,"z":0.06,"w":0.035,"d":0.05,"h":0.12,"pitch":160,"far":0}'),
  ('optic','cylinder','mat.composite',0,'receiver','{"y":0.22,"z":-0.065,"w":0.05,"h":0.14,"segments":4,"far":0}'),
  ('barrel','cylinder','mat.composite',0,'receiver','{"y":0.42,"w":0.035,"h":0.22,"segments":4}'),
  ('spark','cone','mat.flash',0,'barrel','{"y":0.24,"w":0.08,"h":0.08,"segments":3,"flash":3,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.22,"w":0.2,"h":0.22,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.assault_infantry_information'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.assault_infantry'), NULL, (SELECT id FROM def WHERE key='age.information'), (SELECT id FROM def WHERE key='mesh.assault_infantry_information'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
