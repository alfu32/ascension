-- bld.wall_gate@medieval: Gatehouse (mesh.wall_gate_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.wall_gate_medieval','Gatehouse');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.wall_gate_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('bed','box','mat.rough_stone',0,'','{"w":0.96,"d":0.86,"h":0.02}'),
  ('arch','arch','mat.rough_stone',0,'','{"w":0.98,"h":2.6,"t":0.94,"span":0.6,"spring":1.3,"segments":8}'),
  ('portcullis','fence','mat.iron',0,'','{"y":0.55,"z":0.3,"w":0.6,"h":0.95,"spacing":0.15,"rails":4,"far":0}'),
  ('walk','box','mat.cloth',1,'','{"y":2.6,"w":0.96,"d":0.96,"h":0.06}'),
  ('merlons','crenellation','mat.rough_stone',0,'','{"y":2.66,"w":0.98,"d":0.94,"h":0.42,"t":0.16,"merlon":0.22,"ring":1}'),
  ('banner','banner','mat.cloth',1,'','{"x":0.11,"y":2.66,"z":-0.32,"h":1.0,"fw":0.38,"fh":0.28}'),
  ('hanging','box','mat.cloth',1,'','{"y":1.7,"z":0.505,"w":0.4,"d":0.03,"h":0.6}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.wall_gate_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.wall_gate'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.wall_gate_medieval'), (SELECT id FROM def WHERE key='mat.rough_stone'), 100);
