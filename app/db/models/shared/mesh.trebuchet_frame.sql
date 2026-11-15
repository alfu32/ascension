-- mesh.trebuchet_frame: Trebuchet frame (shared sub-assembly)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.trebuchet_frame','Trebuchet frame');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.trebuchet_frame';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('beams','box','mat.oak',0,'','{"x":-0.4,"y":0,"w":0.16,"d":1.8,"h":0.16,"repeat":2,"dx":0.8}'),
  ('cross','box','mat.oak',0,'','{"y":0,"z":-0.75,"w":0.96,"d":0.14,"h":0.14,"repeat":2,"dz":1.5,"far":0}'),
  ('legs_f','box','mat.oak',0,'','{"x":-0.4,"y":0.16,"z":-0.55,"w":0.1,"d":0.1,"h":1.45,"pitch":22,"repeat":2,"dx":0.8}'),
  ('legs_b','box','mat.oak',0,'','{"x":-0.4,"y":0.16,"z":0.55,"w":0.1,"d":0.1,"h":1.45,"pitch":-22,"repeat":2,"dx":0.8,"far":0}'),
  ('axle','box','mat.oak',0,'','{"y":1.47,"w":0.96,"d":0.1,"h":0.1,"far":0}'),
  ('arm','box','mat.sawn_timber',0,'','{"y":0.16,"z":0.75,"w":0.12,"d":0.12,"h":2.2,"pitch":-30}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.trebuchet_frame'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
