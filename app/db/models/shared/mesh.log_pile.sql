-- mesh.log_pile: Log pile (shared sub-assembly)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.log_pile','Log pile');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.log_pile';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('logs_low','cylinder','mat.timber',0,'','{"x":-0.8,"y":0.15,"z":-0.45,"w":0.3,"h":1.6,"segments":6,"roll":90,"repeat":4,"dz":0.3}'),
  ('logs_mid','cylinder','mat.timber',0,'','{"x":-0.72,"y":0.41,"z":-0.3,"w":0.3,"h":1.5,"segments":6,"roll":90,"repeat":3,"dz":0.3,"far":0,"tint":"#e0d2c0"}'),
  ('logs_top','cylinder','mat.timber',0,'','{"x":-0.84,"y":0.67,"z":-0.15,"w":0.3,"h":1.62,"segments":6,"roll":90,"repeat":2,"dz":0.3,"far":0}'),
  ('stakes','cylinder','mat.timber',0,'','{"x":-0.5,"z":-0.66,"w":0.07,"h":0.8,"segments":4,"repeat":2,"dx":1.0,"far":0}'),
  ('stakes_front','cylinder','mat.timber',0,'','{"x":-0.5,"z":0.66,"w":0.07,"h":0.8,"segments":4,"repeat":2,"dx":1.0,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.log_pile'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
