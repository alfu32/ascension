-- unit.tank@modern: Tank (mesh.tank_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.tank_modern','Tank');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.tank_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('track_w','track','mat.cast_iron',0,'','{"x":-0.58,"l":2.3,"h":0.56,"t":0.34,"wheels":5,"wheel_colour":"#5a5a40"}'),
  ('track_e','track','mat.cast_iron',0,'','{"x":0.58,"l":2.3,"h":0.56,"t":0.34,"wheels":5,"wheel_colour":"#5a5a40"}'),
  ('hull','box','mat.olive_drab',0,'','{"y":0.16,"z":-0.12,"w":0.84,"d":1.84,"h":0.5}'),
  ('glacis','box','mat.olive_drab',0,'','{"y":0.2,"z":0.8,"w":0.84,"d":0.05,"h":0.6,"pitch":-55}'),
  ('fenders','box','mat.olive_drab',0,'','{"y":0.62,"z":-0.02,"w":1.56,"d":2.26,"h":0.05}'),
  ('upper_hull','box','mat.olive_drab',0,'','{"y":0.67,"z":-0.1,"w":1,"d":1.8,"h":0.1,"taper":0.06}'),
  ('engine_deck','box','mat.cast_iron',0,'','{"y":0.77,"z":-0.78,"w":0.7,"d":0.5,"h":0.03,"far":0}'),
  ('stowage','box','mat.khaki',0,'','{"x":-0.64,"y":0.67,"z":-0.55,"w":0.26,"d":0.6,"h":0.12,"far":0}'),
  ('turret','cylinder','mat.olive_drab',1,'','{"y":0.77,"z":0,"w":0.98,"h":0.32,"taper":0.22,"segments":10,"traverse":25}'),
  ('turret_top','dome','mat.olive_drab',1,'turret','{"y":0.32,"w":0.76,"h":0.1,"segments":10,"far":0}'),
  ('cupola','cylinder','mat.olive_drab',0,'turret','{"x":0.2,"z":-0.15,"y":0.35,"w":0.24,"h":0.12,"segments":7}'),
  ('hatch','dome','mat.cast_iron',0,'cupola','{"y":0.12,"w":0.22,"h":0.05,"segments":7,"far":0}'),
  ('mantlet','box','mat.olive_drab',0,'turret','{"y":0.06,"z":0.44,"w":0.34,"d":0.14,"h":0.2}'),
  ('gun','cylinder','mat.cast_iron',0,'mantlet','{"y":0.1,"z":0,"w":0.09,"h":1.3,"segments":6,"pitch":90,"recoil":0.14}'),
  ('brake','cylinder','mat.cast_iron',0,'gun','{"y":1.22,"w":0.15,"h":0.14,"segments":6,"far":0}'),
  ('spark','cone','mat.flash',0,'gun','{"y":1.36,"w":0.12,"h":0.12,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.55,"w":0.55,"h":0.55,"segments":6,"pitch":180,"far":0}'),
  ('aerial','cylinder','mat.steel',0,'turret','{"x":-0.3,"y":0.3,"z":-0.25,"w":0.015,"h":0.7,"segments":3,"far":0}'),
  ('pennant','box','mat.cloth',1,'aerial','{"y":0.56,"z":-0.08,"w":0.01,"d":0.16,"h":0.1,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.tank_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.tank'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.tank_modern'), (SELECT id FROM def WHERE key='mat.olive_drab'), 100);
