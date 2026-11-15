-- bld.tower@medieval: Medieval tower (mesh.tower_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.tower_medieval','Medieval tower');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.tower_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','box','mat.rough_stone',0,'','{"w":1.75,"d":1.75,"h":0.6,"taper":0.08}'),
  ('shaft','box','mat.rough_stone',0,'','{"y":0.6,"w":1.55,"d":1.55,"h":3,"door":0.4,"door_h":0.75,"side_windows":1,"window_w":0.08,"window_h":0.34,"window_y":1,"window_rows":2,"row_gap":0.95}'),
  ('corbel','box','mat.dressed_stone',0,'','{"y":3.5,"w":1.78,"d":1.78,"h":0.1}'),
  ('band','box','mat.cloth',1,'','{"y":3.6,"w":1.8,"d":1.8,"h":0.24}'),
  ('merlons','crenellation','mat.dressed_stone',0,'','{"y":3.84,"w":1.78,"d":1.78,"h":0.32,"t":0.15,"merlon":0.26,"ring":1}'),
  ('hanging','box','mat.cloth',1,'','{"y":1.9,"z":0.79,"w":0.6,"d":0.03,"h":1.55,"repeat":2,"dz":-1.58}'),
  ('roof','hip_roof','mat.slate',0,'','{"y":3.84,"w":1.2,"d":1.2,"pitch":62,"overhang":0.04,"t":0.06}'),
  ('roof_tip','hip_roof','mat.cloth',1,'','{"y":4.27,"w":0.76,"d":0.76,"pitch":62,"overhang":0,"t":0.03}'),
  ('banner','banner','mat.cloth',1,'','{"y":5.05,"h":1.1,"fw":0.55,"fh":0.55}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.tower_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.tower'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.tower_medieval'), (SELECT id FROM def WHERE key='mat.rough_stone'), 100);
