-- unit.ship_of_the_line@enlightenment: Ship of the Line (mesh.ship_of_the_line_enlightenment)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.ship_of_the_line_enlightenment','Ship of the Line');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.ship_of_the_line_enlightenment';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','hull','mat.timber',0,'','{"yaw":-90,"y":-0.12,"w":4.2,"d":1.12,"h":0.9,"sheer":0.2,"segments":10,"float":1,"heel":5,"tint":"#4a3e36"}'),
  ('band_upper','box','mat.cloth',1,'hull','{"y":0.64,"z":-0.515,"w":2.5,"d":0.05,"h":0.13,"repeat":2,"dz":1.03}'),
  ('band_lower','box','mat.cloth',1,'hull','{"y":0.4,"z":-0.475,"w":2.1,"d":0.05,"h":0.13,"repeat":2,"dz":0.95,"far":0}'),
  ('ports_port','box','mat.felt',0,'hull','{"x":-0.96,"y":0.665,"z":-0.545,"w":0.1,"d":0.02,"h":0.08,"repeat":7,"dx":0.32,"far":0}'),
  ('ports_stbd','box','mat.felt',0,'hull','{"x":-0.96,"y":0.665,"z":0.545,"w":0.1,"d":0.02,"h":0.08,"repeat":7,"dx":0.32,"far":0}'),
  ('lids','box','mat.felt',0,'hull','{"y":0.43,"z":-0.505,"w":1.9,"d":0.04,"h":0.07,"repeat":2,"dz":1.01,"far":0}'),
  ('deck','box','mat.sawn_timber',0,'hull','{"y":0.86,"w":3.3,"d":0.96,"h":0.05}'),
  ('quarterdeck','box','mat.sawn_timber',0,'hull','{"x":-1.4,"y":0.86,"w":0.95,"d":0.98,"h":0.2}'),
  ('stern_gallery','box','mat.timber',0,'hull','{"x":-1.98,"y":0.62,"w":0.12,"d":0.9,"h":0.3,"tint":"#6a5444","far":0}'),
  ('stern_windows','box','mat.glow',0,'hull','{"x":-2.045,"y":0.72,"w":0.02,"d":0.7,"h":0.08,"far":0}'),
  ('forecastle','box','mat.sawn_timber',0,'hull','{"x":1.35,"y":0.86,"w":0.8,"d":0.9,"h":0.12,"far":0}'),
  ('guns','cylinder','mat.iron',0,'hull','{"x":-0.2,"y":0.67,"z":0.53,"w":0.07,"h":0.16,"segments":5,"pitch":90,"recoil":0.06,"repeat":3,"dx":0.64,"tint":"#3a3d42"}'),
  ('spark','cone','mat.flash',0,'guns','{"y":0.16,"w":0.1,"h":0.1,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.55,"w":0.5,"h":0.55,"segments":6,"pitch":180,"far":0}'),
  ('bowsprit','cylinder','mat.timber',0,'hull','{"x":1.75,"y":0.95,"w":0.07,"h":1.15,"segments":4,"roll":-64}'),
  ('jib','sail','mat.canvas',0,'hull','{"x":2.05,"y":1.2,"roll":40,"w":0.8,"h":1,"billow":0.06,"lateen":1,"far":0}'),
  ('foremast','cylinder','mat.timber',0,'hull','{"x":1.05,"y":0.86,"w":0.08,"h":2.8,"segments":5}'),
  ('fore_course','sail','mat.cloth',1,'foremast','{"y":0.55,"yaw":90,"w":1,"h":0.72,"billow":0.15}'),
  ('fore_topsail','sail','mat.canvas',0,'foremast','{"y":1.26,"yaw":90,"w":0.82,"h":0.6,"billow":0.1}'),
  ('fore_gallant','sail','mat.canvas',0,'foremast','{"y":2.016,"yaw":90,"w":0.58,"h":0.4,"billow":0.06,"far":0}'),
  ('mainmast','cylinder','mat.timber',0,'hull','{"x":0.1,"y":0.86,"w":0.08,"h":3.2,"segments":5}'),
  ('main_course','sail','mat.cloth',1,'mainmast','{"y":0.55,"yaw":90,"w":1.15,"h":0.828,"billow":0.15}'),
  ('main_topsail','sail','mat.canvas',0,'mainmast','{"y":1.44,"yaw":90,"w":0.943,"h":0.69,"billow":0.1}'),
  ('main_gallant','sail','mat.canvas',0,'mainmast','{"y":2.304,"yaw":90,"w":0.667,"h":0.46,"billow":0.06,"far":0}'),
  ('mizzenmast','cylinder','mat.timber',0,'hull','{"x":-1,"y":0.86,"w":0.08,"h":2.4,"segments":5}'),
  ('mizzen_topsail','sail','mat.canvas',0,'mizzenmast','{"y":1.08,"yaw":90,"w":0.656,"h":0.48,"billow":0.1}'),
  ('mizzen_gallant','sail','mat.canvas',0,'mizzenmast','{"y":1.728,"yaw":90,"w":0.464,"h":0.32,"billow":0.06,"far":0}'),
  ('spanker','sail','mat.canvas',0,'mizzenmast','{"x":-0.45,"y":0.3,"w":0.9,"h":0.8,"billow":0.05,"lateen":1,"roll":-20,"far":0}'),
  ('pennant','box','mat.cloth',1,'mainmast','{"y":3.12,"z":-0.24,"w":0.015,"d":0.5,"h":0.08,"far":0}'),
  ('ensign','box','mat.cloth',1,'hull','{"x":-2.15,"y":1.25,"w":0.46,"d":0.015,"h":0.32}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.ship_of_the_line_enlightenment'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.ship_of_the_line'), NULL, (SELECT id FROM def WHERE key='age.enlightenment'), (SELECT id FROM def WHERE key='mesh.ship_of_the_line_enlightenment'), (SELECT id FROM def WHERE key='mat.timber'), 100);
