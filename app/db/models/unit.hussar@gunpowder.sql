-- unit.hussar@gunpowder: Hussar (mesh.hussar_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.hussar_gunpowder','Hussar');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.hussar_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('beast','quadruped','mat.hide',0,'','{"pace":0.55,"stride":34,"tint":"#8a6448"}'),
  ('saddle_cloth','box','mat.cloth',1,'beast.rider','{"y":-0.1,"z":-0.04,"w":0.38,"d":0.56,"h":0.14,"far":0}'),
  ('rider','figure','mat.cloth',1,'beast.rider','{"y":-0.4,"seat":1,"build":0.95}'),
  ('cap','cylinder','mat.fur',0,'rider.head','{"y":-0.12,"w":0.24,"h":0.24,"taper":0.08,"segments":8}'),
  ('cap_bag','box','mat.cloth',1,'rider.head','{"x":0.06,"y":0.08,"z":-0.04,"w":0.08,"d":0.08,"h":0.12,"roll":-40,"far":0}'),
  ('wings','bow','mat.canvas',0,'rider.back','{"x":-0.1,"y":0.42,"yaw":180,"h":1.05,"bend":0.2,"w":0.06,"string":"#e8e2d4","repeat":2,"dx":0.2}'),
  ('sabre','box','mat.steel',0,'rider.right_hand','{"y":-0.04,"w":0.02,"d":0.055,"h":0.6,"pitch":8,"far":0}'),
  ('hilt','box','mat.bronze',0,'rider.right_hand','{"y":-0.06,"w":0.1,"d":0.03,"h":0.03,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.hussar_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.hussar'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.hussar_gunpowder'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
