-- unit.trireme@classical: Trireme (mesh.trireme_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.trireme_classical','Trireme');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.trireme_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','hull','mat.timber',0,'','{"yaw":-90,"y":-0.08,"w":3.4,"d":0.72,"h":0.34,"sheer":0.3,"segments":10,"float":1,"tint":"#8c7a6c"}'),
  ('oars','oar_bank','mat.timber',0,'hull','{"y":0.28,"w":2.2,"count":6,"beam":0.74,"length":0.8,"drop":28}'),
  ('stripes','box','mat.cloth',1,'hull','{"y":0.24,"z":-0.37,"w":2.5,"d":0.05,"h":0.09,"repeat":2,"dz":0.74}'),
  ('deck','box','mat.sawn_timber',0,'hull','{"y":0.3,"w":2.6,"d":0.34,"h":0.05}'),
  ('ram','box','mat.timber',0,'hull','{"x":1.0,"y":0.07,"w":0.1,"d":0.1,"h":0.75,"roll":90}'),
  ('ram_tip','cone','mat.bronze',0,'hull','{"x":1.72,"y":0.07,"w":0.16,"h":0.3,"roll":90,"segments":4}'),
  ('sternpost','box','mat.timber',0,'hull','{"x":-1.66,"y":0.5,"w":0.07,"d":0.07,"h":0.42,"roll":-35}'),
  ('rudder_s','box','mat.timber',0,'hull','{"x":-1.3,"y":0.36,"z":0.34,"w":0.04,"d":0.04,"h":0.7,"pitch":140,"far":0}'),
  ('rudder_p','box','mat.timber',0,'hull','{"x":-1.3,"y":0.36,"z":-0.34,"w":0.04,"d":0.04,"h":0.7,"pitch":-140,"far":0}'),
  ('mast','cylinder','mat.timber',0,'hull','{"x":0.2,"y":0.25,"w":0.07,"h":1.85,"segments":5}'),
  ('sail','sail','mat.cloth',1,'mast','{"y":0.8,"w":1.35,"h":1.0,"billow":0.16,"yaw":90}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.trireme_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.trireme'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.trireme_classical'), (SELECT id FROM def WHERE key='mat.timber'), 100);
