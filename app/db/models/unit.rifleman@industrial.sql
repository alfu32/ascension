-- unit.rifleman@industrial: Rifleman (mesh.rifleman_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.rifleman_industrial','Rifleman');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.rifleman_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"attack":5,"hold_right":12,"legs":"#4e4a3a"}'),
  ('coat','box','mat.cloth',1,'body','{"y":0.16,"w":0.36,"d":0.25,"h":0.32,"taper":0.12}'),
  ('belt','box','mat.leather',0,'body','{"y":0.46,"w":0.33,"d":0.225,"h":0.05,"far":0}'),
  ('pack','box','mat.khaki',0,'back','{"y":-0.14,"z":-0.03,"w":0.22,"d":0.1,"h":0.2}'),
  ('cap','cylinder','mat.felt',0,'head','{"y":-0.06,"w":0.23,"h":0.1,"segments":6}'),
  ('peak','box','mat.leather',0,'head','{"y":-0.05,"z":0.11,"w":0.19,"d":0.09,"h":0.02,"pitch":-8,"far":0}'),
  ('stock','box','mat.timber',0,'right_hand','{"y":-0.26,"w":0.05,"d":0.08,"h":0.4,"far":0}'),
  ('barrel','cylinder','mat.cast_iron',0,'stock','{"y":0.34,"w":0.03,"h":0.46,"segments":5}'),
  ('spark','cone','mat.flash',0,'barrel','{"y":0.46,"w":0.08,"h":0.08,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.26,"w":0.24,"h":0.26,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.rifleman_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.rifleman'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.rifleman_industrial'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
