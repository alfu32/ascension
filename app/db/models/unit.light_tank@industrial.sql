-- unit.light_tank@industrial: Light tank (mesh.light_tank_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.light_tank_industrial','Light tank');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.light_tank_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('track_w','track','mat.cast_iron',0,'','{"x":-0.5,"l":1.9,"h":0.5,"t":0.3,"wheels":4,"wheel_colour":"#5a5a40"}'),
  ('track_e','track','mat.cast_iron',0,'','{"x":0.5,"l":1.9,"h":0.5,"t":0.3,"wheels":4,"wheel_colour":"#5a5a40"}'),
  ('hull','box','mat.olive_drab',0,'','{"y":0.14,"z":-0.15,"w":0.72,"d":1.5,"h":0.52}'),
  ('glacis','box','mat.olive_drab',0,'','{"y":0.14,"z":0.92,"w":0.72,"d":0.05,"h":0.5,"pitch":-40}'),
  ('fenders','box','mat.olive_drab',0,'','{"y":0.5,"w":1.34,"d":1.96,"h":0.05}'),
  ('skirts','box','mat.olive_drab',0,'','{"x":-0.67,"y":0.3,"w":0.03,"d":1.5,"h":0.2,"repeat":2,"dx":1.34,"far":0}'),
  ('engine_deck','box','mat.cast_iron',0,'','{"y":0.66,"z":-0.62,"w":0.6,"d":0.5,"h":0.04,"far":0}'),
  ('exhaust','cylinder','mat.rust',0,'','{"x":0.3,"y":0.55,"z":-0.92,"w":0.08,"h":0.4,"segments":5,"roll":90,"far":0}'),
  ('turret','box','mat.olive_drab',1,'','{"y":0.66,"z":0.05,"w":0.64,"d":0.66,"h":0.36,"taper":0.16,"traverse":25}'),
  ('cupola','cylinder','mat.olive_drab',0,'turret','{"z":-0.08,"y":0.36,"w":0.26,"h":0.12,"segments":7}'),
  ('hatch','dome','mat.cast_iron',0,'cupola','{"y":0.12,"w":0.24,"h":0.05,"segments":7,"far":0}'),
  ('mantlet','box','mat.olive_drab',0,'turret','{"y":0.08,"z":0.31,"w":0.24,"d":0.1,"h":0.18}'),
  ('gun','cylinder','mat.cast_iron',0,'mantlet','{"y":0.09,"z":0,"w":0.08,"h":0.62,"segments":6,"pitch":90,"recoil":0.1}'),
  ('spark','cone','mat.flash',0,'gun','{"y":0.62,"w":0.1,"h":0.1,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.45,"w":0.44,"h":0.45,"segments":6,"pitch":180,"far":0}'),
  ('aerial','cylinder','mat.steel',0,'turret','{"x":0.22,"y":0.36,"z":-0.2,"w":0.015,"h":0.6,"segments":3,"far":0}'),
  ('pennant','box','mat.cloth',1,'aerial','{"y":0.48,"z":-0.08,"w":0.01,"d":0.16,"h":0.1,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.light_tank_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.light_tank'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.light_tank_industrial'), (SELECT id FROM def WHERE key='mat.olive_drab'), 100);
