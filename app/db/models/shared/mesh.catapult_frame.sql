-- mesh.catapult_frame: Catapult frame (shared sub-assembly)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.catapult_frame','Catapult frame');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.catapult_frame';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('frame','box','mat.timber',0,'','{"x":-0.35,"y":0.22,"w":0.16,"d":1.5,"h":0.16,"repeat":2,"dx":0.7}'),
  ('frame_cross','box','mat.timber',0,'','{"y":0.22,"z":-0.55,"w":0.86,"d":0.14,"h":0.14,"repeat":2,"dz":1.1,"far":0}'),
  ('wheels_w','cylinder','mat.timber',0,'','{"x":-0.55,"y":0.27,"z":-0.45,"w":0.54,"h":0.08,"segments":10,"roll":90,"repeat":2,"dz":0.9}'),
  ('wheel_e','cylinder','mat.timber',0,'','{"x":0.47,"y":0.27,"z":0.45,"w":0.54,"h":0.08,"segments":10,"roll":90}'),
  ('uprights_f','box','mat.timber',0,'','{"x":-0.35,"y":0.38,"z":-0.45,"w":0.1,"d":0.1,"h":1.1,"pitch":25,"repeat":2,"dx":0.7}'),
  ('uprights_b','box','mat.timber',0,'','{"x":-0.35,"y":0.38,"z":0.45,"w":0.1,"d":0.1,"h":1.1,"pitch":-25,"repeat":2,"dx":0.7,"far":0}'),
  ('crossbar','box','mat.timber',0,'','{"y":1.33,"w":0.86,"d":0.1,"h":0.1,"far":0}'),
  ('arm','box','mat.timber',0,'','{"y":0.4,"z":0.5,"w":0.12,"d":0.12,"h":1.75,"pitch":-30}'),
  ('cup','box','mat.timber',0,'arm','{"y":1.65,"w":0.28,"d":0.28,"h":0.12,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.catapult_frame'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
