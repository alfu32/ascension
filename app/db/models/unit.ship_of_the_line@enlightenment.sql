-- unit.ship_of_the_line@enlightenment: Ship of the Line (mesh.ship_of_the_line_enlightenment)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.ship_of_the_line_enlightenment','Ship of the Line');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.ship_of_the_line_enlightenment';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','hull','mat.timber',0,'','{"yaw":-90,"y":-0.12,"w":3.6,"d":1,"h":0.72,"sheer":0.18,"segments":10,"float":1,"heel":6,"tint":"#4a3e36"}'),
  ('strake','box','mat.cloth',1,'hull','{"y":0.5,"z":-0.48,"w":2.7,"d":0.05,"h":0.12,"repeat":2,"dz":0.96}'),
  ('ports','box','mat.felt',0,'hull','{"x":-1.12,"y":0.52,"z":-0.505,"w":0.1,"d":0.04,"h":0.08,"repeat":8,"dx":0.32,"far":0}'),
  ('ports_s','box','mat.felt',0,'hull','{"x":-1.12,"y":0.52,"z":0.505,"w":0.1,"d":0.04,"h":0.08,"repeat":8,"dx":0.32,"far":0}'),
  ('deck','box','mat.sawn_timber',0,'hull','{"y":0.62,"w":2.9,"d":0.84,"h":0.05}'),
  ('quarterdeck','box','mat.sawn_timber',0,'hull','{"x":-1.35,"y":0.62,"w":0.75,"d":0.84,"h":0.16,"side_windows":3,"window_y":0.04,"window_w":0.12,"window_h":0.08,"opening":"#e8c870"}'),
  ('guns','cylinder','mat.iron',0,'hull','{"x":-0.2,"y":0.53,"z":0.5,"w":0.07,"h":0.16,"segments":5,"pitch":90,"recoil":0.06,"repeat":2,"dx":0.64,"tint":"#3a3d42"}'),
  ('spark','cone','mat.brass',0,'guns','{"y":0.16,"w":0.1,"h":0.1,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.brass',0,'spark','{"y":0.55,"w":0.5,"h":0.55,"segments":6,"pitch":180,"far":0}'),
  ('bowsprit','cylinder','mat.timber',0,'hull','{"x":1.6,"y":0.7,"w":0.06,"h":1,"roll":-64,"segments":4}'),
  ('jib','sail','mat.canvas',0,'hull','{"x":1.75,"y":0.95,"roll":40,"w":0.7,"h":0.85,"billow":0.06,"lateen":1,"far":0}'),
  ('foremast','cylinder','mat.timber',0,'hull','{"x":0.95,"y":0.62,"w":0.07,"h":2.5,"segments":5}'),
  ('fore_course','sail','mat.cloth',1,'foremast','{"y":0.55,"yaw":90,"w":0.95,"h":0.72,"billow":0.14}'),
  ('fore_top','sail','mat.canvas',0,'foremast','{"y":1.36,"yaw":90,"w":0.75,"h":0.56,"billow":0.1}'),
  ('fore_gallant','sail','mat.canvas',0,'foremast','{"y":1.98,"yaw":90,"w":0.52,"h":0.36,"billow":0.06,"far":0}'),
  ('mainmast','cylinder','mat.timber',0,'hull','{"x":0.05,"y":0.62,"w":0.08,"h":2.85,"segments":5}'),
  ('main_course','sail','mat.cloth',1,'mainmast','{"y":0.6,"yaw":90,"w":1.1,"h":0.8,"billow":0.16}'),
  ('main_top','sail','mat.canvas',0,'mainmast','{"y":1.5,"yaw":90,"w":0.86,"h":0.62,"billow":0.1}'),
  ('main_gallant','sail','mat.canvas',0,'mainmast','{"y":2.2,"yaw":90,"w":0.6,"h":0.4,"billow":0.06,"far":0}'),
  ('pennant','box','mat.cloth',1,'mainmast','{"y":2.72,"z":-0.22,"w":0.015,"d":0.42,"h":0.08,"far":0}'),
  ('mizzen','cylinder','mat.timber',0,'hull','{"x":-0.95,"y":0.62,"w":0.06,"h":2.1,"segments":5}'),
  ('mizzen_top','sail','mat.canvas',0,'mizzen','{"y":1.05,"yaw":90,"w":0.62,"h":0.5,"billow":0.08}'),
  ('spanker','sail','mat.canvas',0,'mizzen','{"x":-0.4,"y":0.25,"w":0.8,"h":0.7,"billow":0.05,"lateen":1,"roll":-20,"far":0}'),
  ('ensign','box','mat.cloth',1,'hull','{"x":-1.82,"y":1,"w":0.4,"d":0.015,"h":0.28}'),
  ('ensign_staff','cylinder','mat.timber',0,'hull','{"x":-1.7,"y":0.62,"w":0.03,"h":0.68,"segments":4,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.ship_of_the_line_enlightenment'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.ship_of_the_line'), NULL, (SELECT id FROM def WHERE key='age.enlightenment'), (SELECT id FROM def WHERE key='mesh.ship_of_the_line_enlightenment'), (SELECT id FROM def WHERE key='mat.timber'), 100);
