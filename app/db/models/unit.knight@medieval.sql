-- unit.knight@medieval: Knight (mesh.knight_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.knight_medieval','Knight');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.knight_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('beast','quadruped','mat.hide',0,'','{"pace":0.75,"stride":26,"tint":"#d8d2c8"}'),
  ('caparison','box','mat.cloth',1,'beast','{"y":0.36,"w":0.4,"d":1.08,"h":0.48,"taper":0.06}'),
  ('chanfron','box','mat.steel',0,'beast.head','{"y":-0.2,"z":-0.12,"w":0.13,"d":0.12,"h":0.24,"pitch":40,"far":0}'),
  ('rider','figure','mat.cloth',1,'beast.rider','{"y":-0.37,"seat":1,"attack":1,"build":1.1,"hold_left":25}'),
  ('armour','box','mat.steel',0,'rider','{"y":0.5,"w":0.37,"d":0.25,"h":0.27,"far":0}'),
  ('helm','cylinder','mat.steel',0,'rider.head','{"y":-0.17,"w":0.25,"h":0.25,"segments":8}'),
  ('visor','box','mat.felt',0,'rider.head','{"y":-0.07,"z":0.12,"w":0.17,"d":0.02,"h":0.025,"far":0}'),
  ('crest','box','mat.cloth',1,'rider.head','{"y":0.08,"w":0.04,"d":0.22,"h":0.12,"taper":0.3}'),
  ('lance','cylinder','mat.timber',0,'rider.right_hand','{"y":-0.7,"w":0.045,"h":2.4,"segments":4}'),
  ('lance_tip','cone','mat.steel',0,'lance','{"y":2.4,"w":0.07,"h":0.18,"segments":4}'),
  ('pennon','box','mat.cloth',1,'lance','{"y":1.95,"z":-0.14,"w":0.015,"d":0.26,"h":0.18}'),
  ('shield','shield','mat.cloth',1,'rider.left_hand','{"y":0.05,"z":0.07,"w":0.38,"h":0.48,"point":0.45,"t":0.04,"boss":0,"rim":"#c9ccd2"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.knight_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.knight'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.knight_medieval'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
