-- mesh.horse: Horse (shared sub-assembly)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.horse','Horse');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.horse';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','box','mat.hide',0,'','{"y":0.52,"w":0.34,"d":0.92,"h":0.36}'),
  ('legs_front','box','mat.hide',0,'','{"x":-0.11,"z":0.32,"w":0.09,"d":0.1,"h":0.55,"repeat":2,"dx":0.22,"far":0}'),
  ('legs_back','box','mat.hide',0,'','{"x":-0.11,"z":-0.32,"w":0.09,"d":0.1,"h":0.55,"repeat":2,"dx":0.22,"far":0}'),
  ('neck','box','mat.hide',0,'','{"y":0.7,"z":0.36,"w":0.16,"d":0.24,"h":0.5,"pitch":38}'),
  ('head','box','mat.hide',0,'neck','{"y":0.4,"z":0.08,"w":0.15,"d":0.4,"h":0.17,"pitch":14}'),
  ('mane','box','mat.timber',0,'neck','{"y":0.05,"z":-0.12,"w":0.05,"d":0.05,"h":0.5,"tint":"#5a4a40","far":0}'),
  ('tail','box','mat.timber',0,'','{"y":0.84,"z":-0.46,"w":0.07,"d":0.07,"h":0.45,"pitch":-160,"tint":"#5a4a40","far":0}'),
  ('blanket','box','mat.cloth',1,'','{"y":0.66,"z":-0.02,"w":0.38,"d":0.44,"h":0.24}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.horse'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
