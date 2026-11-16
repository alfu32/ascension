-- unit.armoured_car@industrial: Armoured car (mesh.armoured_car_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.armoured_car_industrial','Armoured car');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.armoured_car_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('wheels_front','wheel','mat.rubber',0,'','{"x":-0.5,"z":0.62,"w":0.66,"t":0.15,"spokes":1,"segments":10,"repeat":2,"dx":1}'),
  ('wheels_back','wheel','mat.rubber',0,'','{"x":-0.5,"z":-0.6,"w":0.66,"t":0.15,"spokes":1,"segments":10,"repeat":2,"dx":1}'),
  ('chassis','box','mat.cast_iron',0,'','{"y":0.25,"w":0.8,"d":1.9,"h":0.12}'),
  ('bonnet','box','mat.olive_drab',1,'','{"y":0.36,"z":0.62,"w":0.7,"d":0.66,"h":0.36,"taper":0.1}'),
  ('radiator','box','mat.cast_iron',0,'','{"y":0.36,"z":0.96,"w":0.58,"d":0.05,"h":0.32,"far":0}'),
  ('hull','box','mat.olive_drab',0,'','{"y":0.36,"z":-0.33,"w":0.92,"d":1.25,"h":0.6,"windows":1,"window_y":0.34,"window_w":0.32,"window_h":0.08,"side_windows":2,"opening":"#22262a"}'),
  ('roof','box','mat.olive_drab',0,'','{"y":0.96,"z":-0.33,"w":0.86,"d":1.18,"h":0.04}'),
  ('guards','box','mat.olive_drab',0,'','{"x":-0.5,"y":0.66,"z":0.62,"w":0.2,"d":0.72,"h":0.04,"repeat":2,"dx":1,"far":0}'),
  ('guards_back','box','mat.olive_drab',0,'','{"x":-0.5,"y":0.66,"z":-0.6,"w":0.2,"d":0.72,"h":0.04,"repeat":2,"dx":1,"far":0}'),
  ('lamps','cylinder','mat.brass',0,'','{"x":-0.24,"y":0.66,"z":0.92,"w":0.1,"h":0.1,"segments":6,"pitch":90,"repeat":2,"dx":0.48,"far":0}'),
  ('turret','cylinder','mat.olive_drab',1,'','{"y":1,"z":-0.33,"w":0.62,"h":0.34,"segments":8,"taper":0.1,"traverse":35}'),
  ('turret_top','dome','mat.cast_iron',0,'turret','{"y":0.34,"w":0.56,"h":0.08,"segments":8,"far":0}'),
  ('mg','cylinder','mat.cast_iron',0,'turret','{"y":0.16,"z":0.26,"w":0.06,"h":0.5,"segments":5,"pitch":90}'),
  ('spark','cone','mat.flash',0,'mg','{"y":0.5,"w":0.08,"h":0.08,"segments":3,"flash":4,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.3,"w":0.3,"h":0.3,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.armoured_car_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.armoured_car'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.armoured_car_industrial'), (SELECT id FROM def WHERE key='mat.olive_drab'), 100);
