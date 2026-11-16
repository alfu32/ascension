-- unit.lancer@enlightenment: Lancer (mesh.lancer_enlightenment)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.lancer_enlightenment','Lancer');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.lancer_enlightenment';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('beast','quadruped','mat.hide',0,'','{"pace":0.6,"stride":32,"tint":"#7a6050"}'),
  ('saddle_cloth','box','mat.canvas',0,'beast.rider','{"y":-0.1,"z":-0.04,"w":0.38,"d":0.6,"h":0.14,"tint":"#d8d0c0"}'),
  ('rider','figure','mat.cloth',1,'beast.rider','{"y":-0.4,"seat":1,"attack":1,"legs":"#2a2a34"}'),
  ('plastron','box','mat.canvas',0,'rider','{"y":0.5,"z":0.03,"w":0.24,"d":0.2,"h":0.25,"far":0}'),
  ('cap','cylinder','mat.felt',0,'rider.head','{"y":-0.08,"w":0.2,"h":0.12,"segments":8,"far":0}'),
  ('cap_neck','cylinder','mat.felt',0,'rider.head','{"y":0.04,"w":0.12,"h":0.06,"segments":6,"far":0}'),
  ('cap_top','box','mat.cloth',1,'rider.head','{"y":0.1,"yaw":45,"w":0.24,"d":0.24,"h":0.07,"far":0}'),
  ('plume','box','mat.canvas',0,'rider.head','{"x":0.07,"y":0.1,"z":0.06,"w":0.04,"d":0.04,"h":0.2,"far":0}'),
  ('lance','cylinder','mat.timber',0,'rider.right_hand','{"y":-0.75,"w":0.04,"h":2.75,"segments":4}'),
  ('lance_tip','cone','mat.steel',0,'lance','{"y":2.75,"w":0.06,"h":0.18,"segments":4,"far":0}'),
  ('pennant','box','mat.cloth',1,'lance','{"y":2.42,"z":-0.17,"w":0.015,"d":0.34,"h":0.11}'),
  ('pennant_low','box','mat.canvas',0,'lance','{"y":2.31,"z":-0.17,"w":0.015,"d":0.34,"h":0.11,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.lancer_enlightenment'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.lancer'), NULL, (SELECT id FROM def WHERE key='age.enlightenment'), (SELECT id FROM def WHERE key='mesh.lancer_enlightenment'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
