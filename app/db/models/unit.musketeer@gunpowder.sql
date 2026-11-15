-- unit.musketeer@gunpowder: Musketeer (mesh.musketeer_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.musketeer_gunpowder','Musketeer');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.musketeer_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"attack":5,"hold_right":14,"legs":"#4a3a30"}'),
  ('brim','cylinder','mat.felt',0,'head','{"y":-0.08,"w":0.42,"h":0.025,"segments":8,"far":0}'),
  ('crown','cylinder','mat.felt',0,'head','{"y":-0.07,"w":0.21,"h":0.14,"taper":0.15,"segments":6}'),
  ('plume','box','mat.canvas',0,'head','{"x":0.1,"y":-0.06,"z":-0.02,"w":0.06,"d":0.22,"h":0.07,"roll":-55,"far":0}'),
  ('stock','box','mat.timber',0,'right_hand','{"y":-0.3,"w":0.05,"d":0.08,"h":0.44,"far":0}'),
  ('barrel','cylinder','mat.iron',0,'stock','{"y":0.36,"w":0.035,"h":0.62,"segments":5}'),
  ('spark','cone','mat.flash',0,'barrel','{"y":0.62,"w":0.08,"h":0.08,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.26,"w":0.24,"h":0.26,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.musketeer_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.musketeer'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.musketeer_gunpowder'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
