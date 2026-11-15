-- mesh.stall: Market stall (shared sub-assembly)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.stall','Market stall');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.stall';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('table','box','mat.timber',0,'','{"z":0.15,"w":1.3,"d":0.55,"h":0.5}'),
  ('jars','cylinder','mat.mud_plaster',0,'','{"x":-0.45,"y":0.5,"z":0.15,"w":0.16,"h":0.24,"segments":6,"taper":0.3,"repeat":3,"dx":0.17}'),
  ('basket','box','mat.thatch',0,'','{"x":0.2,"y":0.5,"z":0.15,"w":0.32,"d":0.3,"h":0.1,"far":0}'),
  ('greens','dome','mat.cloth',0,'','{"x":0.2,"y":0.6,"z":0.15,"w":0.28,"h":0.1,"segments":6,"tint":"#8fc04a"}'),
  ('fruit','dome','mat.cloth',0,'','{"x":0.48,"y":0.5,"z":0.15,"w":0.26,"h":0.13,"segments":6,"tint":"#f2a03a"}'),
  ('awning','awning','mat.cloth',0,'','{"w":1.6,"d":1.1,"h":1.3,"rise":0.35,"posts":0}'),
  ('stripes','awning','mat.cloth',1,'','{"x":-0.53,"y":0.012,"w":0.18,"d":1.1,"h":1.3,"rise":0.35,"posts":0,"repeat":3,"dx":0.53}'),
  ('posts_front','cylinder','mat.timber',0,'','{"x":-0.75,"z":0.5,"w":0.07,"h":1.3,"segments":4,"repeat":2,"dx":1.5,"far":0}'),
  ('posts_back','cylinder','mat.timber',0,'','{"x":-0.75,"z":-0.5,"w":0.07,"h":1.65,"segments":4,"repeat":2,"dx":1.5,"far":0}'),
  ('crate','box','mat.timber',0,'','{"x":0.6,"z":-0.3,"w":0.3,"d":0.3,"h":0.3,"far":0}'),
  ('sacks','dome','mat.cloth',0,'','{"x":-0.55,"z":-0.25,"w":0.32,"h":0.36,"segments":6,"tint":"#c9b48a","far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.stall'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
