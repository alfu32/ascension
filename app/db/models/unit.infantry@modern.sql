-- unit.infantry@modern: Infantry (mesh.infantry_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.infantry_modern','Infantry');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.infantry_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"attack":5,"hold_right":12,"legs":"#4d4c3a"}'),
  ('helmet','dome','mat.olive_drab',0,'head','{"y":-0.13,"w":0.32,"h":0.17,"segments":6}'),
  ('braces','box','mat.khaki',0,'body','{"x":-0.08,"y":0.5,"z":0.105,"w":0.045,"d":0.02,"h":0.27,"repeat":2,"dx":0.16,"far":0}'),
  ('pouches','box','mat.khaki',0,'body','{"y":0.4,"z":0.12,"w":0.3,"d":0.05,"h":0.1,"far":0}'),
  ('pack','box','mat.khaki',0,'back','{"y":-0.13,"z":-0.03,"w":0.2,"d":0.09,"h":0.16}'),
  ('receiver','box','mat.timber',0,'right_hand','{"y":-0.22,"w":0.05,"d":0.075,"h":0.44,"tint":"#8a8a8a"}'),
  ('magazine','box','mat.cast_iron',0,'receiver','{"y":0.3,"z":0.06,"w":0.035,"d":0.05,"h":0.13,"pitch":150,"far":0}'),
  ('barrel','cylinder','mat.cast_iron',0,'receiver','{"y":0.44,"w":0.03,"h":0.22,"segments":4}'),
  ('spark','cone','mat.flash',0,'barrel','{"y":0.22,"w":0.08,"h":0.08,"segments":3,"flash":3,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.24,"w":0.22,"h":0.24,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.infantry_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.infantry'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.infantry_modern'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
