-- unit.gun_drone@information: Gun drone (mesh.gun_drone_information)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.gun_drone_information','Gun drone');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.gun_drone_information';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('track_w','track','mat.cast_iron',0,'','{"x":-0.27,"l":1,"h":0.3,"t":0.16,"wheels":3,"wheel_colour":"#3e4145"}'),
  ('track_e','track','mat.cast_iron',0,'','{"x":0.27,"l":1,"h":0.3,"t":0.16,"wheels":3,"wheel_colour":"#3e4145"}'),
  ('body','box','mat.nato_green',0,'','{"y":0.1,"w":0.4,"d":0.86,"h":0.24,"taper":0.1}'),
  ('nose','box','mat.nato_green',0,'','{"y":0.12,"z":0.45,"w":0.4,"d":0.04,"h":0.24,"pitch":-45}'),
  ('turret','box','mat.nato_green',1,'','{"y":0.34,"z":-0.05,"w":0.32,"d":0.36,"h":0.16,"taper":0.15,"traverse":30}'),
  ('gun','cylinder','mat.composite',0,'turret','{"y":0.08,"z":0.15,"w":0.045,"h":0.5,"segments":5,"pitch":90}'),
  ('ammo','box','mat.composite',0,'turret','{"x":0.2,"y":0.03,"w":0.08,"d":0.18,"h":0.1,"far":0}'),
  ('sensor','dome','mat.composite',0,'turret','{"x":-0.08,"y":0.16,"z":0.06,"w":0.13,"h":0.1,"segments":6}'),
  ('eye','box','mat.glass',0,'sensor','{"z":0.06,"y":0.03,"w":0.06,"d":0.01,"h":0.04,"tint":"#7fa0c0","far":0}'),
  ('aerials','cylinder','mat.composite',0,'','{"x":-0.14,"y":0.34,"z":-0.38,"w":0.012,"h":0.5,"segments":3,"repeat":2,"dx":0.28,"far":0}'),
  ('stripe','box','mat.cloth',1,'','{"y":0.34,"z":-0.32,"w":0.3,"d":0.12,"h":0.01,"far":0}'),
  ('spark','cone','mat.flash',0,'gun','{"y":0.5,"w":0.08,"h":0.08,"segments":3,"flash":4,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.26,"w":0.26,"h":0.26,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.gun_drone_information'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.gun_drone'), NULL, (SELECT id FROM def WHERE key='age.information'), (SELECT id FROM def WHERE key='mesh.gun_drone_information'), (SELECT id FROM def WHERE key='mat.nato_green'), 100);
