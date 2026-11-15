-- unit.light_cavalry@medieval: Light cavalryman (mesh.light_cavalry_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.light_cavalry_medieval','Light cavalryman');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.light_cavalry_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('beast','quadruped','mat.hide',0,'','{"pace":0.55,"stride":34,"tint":"#c49a72"}'),
  ('saddle_cloth','box','mat.cloth',1,'beast.rider','{"y":-0.09,"z":-0.02,"w":0.36,"d":0.5,"h":0.12}'),
  ('rider','figure','mat.cloth',1,'beast.rider','{"y":-0.4,"seat":1,"attack":3,"build":0.95}'),
  ('cap','cone','mat.felt',0,'rider.head','{"y":-0.1,"w":0.23,"h":0.24,"segments":6}'),
  ('cap_rim','cylinder','mat.fur',0,'rider.head','{"y":-0.12,"w":0.25,"h":0.05,"segments":8}'),
  ('bow','bow','mat.timber',0,'rider.left_hand','{"z":0.03,"h":0.68,"bend":0.14,"w":0.035}'),
  ('quiver','cylinder','mat.leather',0,'rider.back','{"x":0.06,"y":-0.28,"z":-0.03,"w":0.1,"h":0.42,"roll":-20,"segments":6,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.light_cavalry_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.light_cavalry'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.light_cavalry_medieval'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
