-- unit.carrack@gunpowder: Carrack (mesh.carrack_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.carrack_gunpowder','Carrack');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.carrack_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','hull','mat.timber',0,'','{"yaw":-90,"y":-0.12,"w":3.3,"d":1.1,"h":0.8,"sheer":0.35,"segments":10,"float":1,"heel":8,"tint":"#806a58"}'),
  ('wale','box','mat.cloth',1,'hull','{"y":0.72,"z":-0.52,"w":2,"d":0.05,"h":0.08,"repeat":2,"dz":1.04}'),
  ('deck','box','mat.sawn_timber',0,'hull','{"y":0.7,"w":2.4,"d":0.8,"h":0.05}'),
  ('forecastle','box','mat.sawn_timber',0,'hull','{"x":1.05,"y":0.72,"w":0.75,"d":0.82,"h":0.48,"taper":0.08}'),
  ('sterncastle','box','mat.sawn_timber',0,'hull','{"x":-1.12,"y":0.72,"w":0.85,"d":0.8,"h":0.82,"taper":0.06}'),
  ('stern_trim','box','mat.cloth',1,'hull','{"x":-1.12,"y":1.47,"w":0.78,"d":0.76,"h":0.08}'),
  ('fore_trim','box','mat.cloth',1,'hull','{"x":1.05,"y":1.13,"w":0.66,"d":0.75,"h":0.07}'),
  ('bowsprit','cylinder','mat.timber',0,'hull','{"x":1.35,"y":1.05,"w":0.06,"h":0.9,"roll":62,"segments":4}'),
  ('mainmast','cylinder','mat.timber',0,'hull','{"x":0.05,"y":0.7,"w":0.08,"h":2.35,"segments":5}'),
  ('mainsail','sail','mat.cloth',1,'mainmast','{"y":0.75,"yaw":90,"w":1.25,"h":1.15,"billow":0.2}'),
  ('topsail','sail','mat.canvas',0,'mainmast','{"y":1.95,"yaw":90,"w":0.75,"h":0.4,"billow":0.1,"far":0}'),
  ('foremast','cylinder','mat.timber',0,'hull','{"x":0.95,"y":1.2,"w":0.07,"h":1.45,"segments":5}'),
  ('foresail','sail','mat.cloth',1,'foremast','{"y":0.5,"yaw":90,"w":0.95,"h":0.9,"billow":0.16}'),
  ('mizzen','cylinder','mat.timber',0,'hull','{"x":-0.95,"y":1.54,"w":0.06,"h":1.2,"segments":5}'),
  ('mizzen_sail','sail','mat.canvas',0,'mizzen','{"y":0.15,"roll":28,"w":0.9,"h":0.85,"billow":0.1,"lateen":1}'),
  ('flag','box','mat.cloth',1,'mainmast','{"y":2.3,"z":-0.18,"w":0.015,"d":0.34,"h":0.2,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.carrack_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.carrack'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.carrack_gunpowder'), (SELECT id FROM def WHERE key='mat.timber'), 100);
