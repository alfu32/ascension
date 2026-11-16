-- unit.anti_tank_rifleman@industrial: Anti-tank rifleman (mesh.anti_tank_rifleman_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.anti_tank_rifleman_industrial','Anti-tank rifleman');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.anti_tank_rifleman_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"attack":5,"hold_right":14,"build":1.05,"legs":"#4e4a3a"}'),
  ('helmet','dome','mat.olive_drab',0,'head','{"y":-0.1,"w":0.27,"h":0.14,"segments":6}'),
  ('helmet_brim','cylinder','mat.olive_drab',0,'head','{"y":-0.1,"w":0.34,"h":0.02,"segments":6,"far":0}'),
  ('stock','box','mat.timber',0,'right_hand','{"y":-0.28,"w":0.06,"d":0.1,"h":0.44}'),
  ('barrel','cylinder','mat.cast_iron',0,'stock','{"y":0.4,"w":0.05,"h":0.95,"segments":5}'),
  ('brake','cylinder','mat.cast_iron',0,'barrel','{"y":0.95,"w":0.08,"h":0.09,"segments":4}'),
  ('bipod','box','mat.steel',0,'barrel','{"y":0.72,"z":0.03,"w":0.02,"d":0.02,"h":0.3,"roll":8,"pitch":12,"far":0}'),
  ('bipod_2','box','mat.steel',0,'barrel','{"y":0.72,"z":0.03,"w":0.02,"d":0.02,"h":0.3,"roll":-8,"pitch":12,"far":0}'),
  ('spark','cone','mat.flash',0,'brake','{"y":0.1,"w":0.1,"h":0.1,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.36,"w":0.38,"h":0.36,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.anti_tank_rifleman_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.anti_tank_rifleman'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.anti_tank_rifleman_industrial'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
