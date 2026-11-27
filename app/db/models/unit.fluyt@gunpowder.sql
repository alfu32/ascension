-- unit.fluyt@gunpowder: Fluyt (mesh.fluyt_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.fluyt_gunpowder','Fluyt');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.fluyt_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','hull','mat.timber',0,'','{"yaw":-90,"y":-0.12,"w":3.5,"d":1.15,"h":0.82,"sheer":0.3,"segments":10,"float":1,"heel":6,"tint":"#7a6250"}'),
  ('wale','box','mat.tar',0,'hull','{"y":0.55,"z":-0.55,"w":2.6,"d":0.04,"h":0.07,"repeat":2,"dz":1.1,"far":0}'),
  ('band','box','mat.cloth',1,'hull','{"y":0.68,"z":-0.53,"w":2.4,"d":0.03,"h":0.06,"repeat":2,"dz":1.06,"far":0}'),
  ('deck','box','mat.sawn_timber',0,'hull','{"y":0.7,"w":2.6,"d":0.72,"h":0.05}'),
  ('hatches','box','mat.oak',0,'hull','{"x":-0.4,"y":0.74,"w":0.32,"d":0.38,"h":0.08,"repeat":3,"dx":0.55,"far":0}'),
  ('forecastle','box','mat.sawn_timber',0,'hull','{"x":1.3,"y":0.72,"w":0.5,"d":0.66,"h":0.28,"taper":0.1}'),
  ('stern','box','mat.sawn_timber',0,'hull','{"x":-1.3,"y":0.72,"w":0.7,"d":0.62,"h":0.8,"taper":0.16,"windows":0,"side_windows":2,"window_y":0.5,"window_w":0.1,"window_h":0.12,"opening":"#2a2018"}'),
  ('stern_trim','box','mat.cloth',1,'hull','{"x":-1.3,"y":1.52,"w":0.5,"d":0.42,"h":0.07}'),
  ('lantern','cylinder','mat.brass',0,'hull','{"x":-1.55,"y":1.6,"w":0.08,"h":0.16,"segments":5,"far":0}'),
  ('bowsprit','cylinder','mat.timber',0,'hull','{"x":1.55,"y":0.95,"w":0.06,"h":0.9,"roll":64,"segments":4}'),
  ('mainmast','cylinder','mat.timber',0,'hull','{"x":0.1,"y":0.7,"w":0.08,"h":2.5,"segments":5}'),
  ('mainsail','sail','mat.cloth',1,'mainmast','{"y":0.65,"yaw":90,"w":1.2,"h":1.0,"billow":0.2}'),
  ('main_topsail','sail','mat.canvas',0,'mainmast','{"y":1.75,"yaw":90,"w":0.85,"h":0.65,"billow":0.14}'),
  ('foremast','cylinder','mat.timber',0,'hull','{"x":1.0,"y":0.72,"w":0.07,"h":2.1,"segments":5}'),
  ('foresail','sail','mat.canvas',0,'foremast','{"y":0.6,"yaw":90,"w":1.0,"h":0.85,"billow":0.18}'),
  ('fore_topsail','sail','mat.canvas',0,'foremast','{"y":1.5,"yaw":90,"w":0.7,"h":0.5,"billow":0.12,"far":0}'),
  ('mizzen','cylinder','mat.timber',0,'hull','{"x":-0.9,"y":0.72,"w":0.06,"h":1.7,"segments":5}'),
  ('mizzen_sail','sail','mat.canvas',0,'mizzen','{"y":0.55,"roll":28,"w":0.85,"h":0.8,"billow":0.1,"lateen":1}'),
  ('flag','box','mat.cloth',1,'mainmast','{"y":2.45,"z":-0.18,"w":0.015,"d":0.36,"h":0.2,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.fluyt_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.fluyt'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.fluyt_gunpowder'), (SELECT id FROM def WHERE key='mat.timber'), 100);
