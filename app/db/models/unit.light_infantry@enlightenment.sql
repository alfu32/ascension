-- unit.light_infantry@enlightenment: Light infantry (mesh.light_infantry_enlightenment)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.light_infantry_enlightenment','Light infantry');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.light_infantry_enlightenment';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"attack":5,"hold_right":10,"build":0.95,"stride":32,"legs":"#4a4c40"}'),
  ('belt','box','mat.felt',0,'body','{"x":-0.1,"y":0.49,"z":0.115,"w":0.045,"d":0.02,"h":0.33,"roll":36,"far":0}'),
  ('pack','box','mat.leather',0,'back','{"y":-0.14,"z":-0.03,"w":0.24,"d":0.1,"h":0.22}'),
  ('cap','cylinder','mat.felt',0,'head','{"y":-0.08,"w":0.22,"h":0.15,"taper":0.05,"segments":6}'),
  ('peak','box','mat.felt',0,'head','{"y":-0.07,"z":0.11,"w":0.18,"d":0.08,"h":0.02,"pitch":-12,"far":0}'),
  ('tuft','dome','mat.cloth',0,'head','{"y":0.07,"z":0.04,"w":0.1,"h":0.09,"segments":6,"tint":"#3f6a32"}'),
  ('stock','box','mat.timber',0,'right_hand','{"y":-0.24,"w":0.05,"d":0.08,"h":0.36,"far":0,"tint":"#6a4a30"}'),
  ('barrel','cylinder','mat.iron',0,'stock','{"y":0.28,"w":0.032,"h":0.44,"segments":5}'),
  ('spark','cone','mat.flash',0,'barrel','{"y":0.44,"w":0.08,"h":0.08,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.26,"w":0.24,"h":0.26,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.light_infantry_enlightenment'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.light_infantry'), NULL, (SELECT id FROM def WHERE key='age.enlightenment'), (SELECT id FROM def WHERE key='mesh.light_infantry_enlightenment'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
