-- bld.library@medieval: Medieval library (mesh.library_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.library_medieval','Medieval library');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.library_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','plinth','mat.rough_stone',0,'','{"w":3.6,"d":3.4,"h":0.32,"steps":2,"inset":0.14}'),
  ('block','box','mat.dressed_stone',0,'','{"y":0.32,"w":2.7,"d":2.4,"h":1.85,"door":0.5,"door_h":1,"side_windows":2,"window_y":0.95,"window_w":0.13,"window_h":0.48}'),
  ('buttresses','box','mat.dressed_stone',0,'','{"x":-1.4,"y":0.32,"z":-0.6,"w":0.24,"d":0.26,"h":1.55,"taper":0.4,"repeat":3,"dz":0.6,"tint":"#d4cec4"}'),
  ('buttresses_e','box','mat.dressed_stone',0,'','{"x":1.4,"y":0.32,"z":-0.6,"w":0.24,"d":0.26,"h":1.55,"taper":0.4,"repeat":3,"dz":0.6,"tint":"#d4cec4"}'),
  ('portal','arch','mat.dressed_stone',0,'','{"y":0.32,"z":1.24,"w":0.95,"h":1.35,"t":0.14,"span":0.55,"spring":0.75,"tint":"#d4cec4"}'),
  ('frieze','box','mat.cloth',1,'','{"y":1.95,"w":2.72,"d":2.42,"h":0.22}'),
  ('hanging','box','mat.cloth',1,'','{"y":1.42,"z":1.32,"w":0.7,"d":0.02,"h":0.5}'),
  ('lower_roof','hip_roof','mat.slate',0,'','{"y":2.17,"w":2.7,"d":2.4,"pitch":45,"overhang":0.18,"t":0.1}'),
  ('lantern','box','mat.dressed_stone',0,'','{"y":2.85,"w":1.1,"d":1.1,"h":0.8,"windows":1,"side_windows":1,"window_y":0.3,"window_w":0.16,"window_h":0.32}'),
  ('lantern_roof','hip_roof','mat.cloth',1,'','{"y":3.65,"w":1.1,"d":1.1,"pitch":64,"overhang":0.14,"t":0.06}'),
  ('finial','cone','mat.bronze',0,'','{"y":4.93,"w":0.14,"h":0.4,"segments":6}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.library_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.library'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.library_medieval'), (SELECT id FROM def WHERE key='mat.dressed_stone'), 100);
