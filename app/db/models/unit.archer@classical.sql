-- unit.archer@classical: Classical archer (mesh.archer_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.archer_classical','Classical archer');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.archer_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"attack":3}'),
  ('cap','dome','mat.leather',0,'head','{"y":-0.12,"z":-0.01,"w":0.245,"h":0.15,"segments":8}'),
  ('cap_tip','cone','mat.leather',0,'head','{"y":0.0,"z":0.02,"w":0.09,"h":0.09,"pitch":55,"segments":4}'),
  ('bow','bow','mat.timber',0,'left_hand','{"z":0.03,"h":0.88,"bend":0.15,"w":0.035}'),
  ('quiver','cylinder','mat.leather',0,'back','{"x":0.06,"y":-0.3,"z":-0.03,"w":0.11,"h":0.46,"roll":-20,"segments":6}'),
  ('fletching','box','mat.cloth',0,'quiver','{"y":0.46,"w":0.08,"d":0.08,"h":0.08,"tint":"#f4f0e6"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.archer_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.archer'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.archer_classical'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
