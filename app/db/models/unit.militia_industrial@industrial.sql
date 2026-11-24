-- unit.militia_industrial@industrial: Home guard (mesh.militia_industrial_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.militia_industrial_industrial','Home guard');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.militia_industrial_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"attack":5,"hold_right":12,"legs":"#4e4a3a"}'),
  ('cap','cylinder','mat.felt',0,'head','{"y":-0.06,"w":0.23,"h":0.08,"segments":6}'),
  ('peak','box','mat.felt',0,'head','{"y":-0.05,"z":0.11,"w":0.19,"d":0.09,"h":0.02,"pitch":-8,"far":0}'),
  ('scarf','box','mat.signal_red',0,'body','{"y":0.72,"w":0.3,"d":0.23,"h":0.05,"far":0}'),
  ('stock','box','mat.timber',0,'right_hand','{"y":-0.26,"w":0.05,"d":0.08,"h":0.4,"far":0}'),
  ('barrel','cylinder','mat.cast_iron',0,'stock','{"y":0.34,"w":0.03,"h":0.46,"segments":5}'),
  ('spark','cone','mat.flash',0,'barrel','{"y":0.46,"w":0.08,"h":0.08,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.26,"w":0.24,"h":0.26,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.militia_industrial_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.militia_industrial'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.militia_industrial_industrial'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
