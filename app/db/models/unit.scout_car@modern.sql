-- unit.scout_car@modern: Scout car (mesh.scout_car_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.scout_car_modern','Scout car');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.scout_car_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('wheels_front','wheel','mat.rubber',0,'','{"x":-0.42,"z":0.56,"w":0.46,"t":0.15,"spokes":1,"segments":10,"repeat":2,"dx":0.84}'),
  ('wheels_back','wheel','mat.rubber',0,'','{"x":-0.42,"z":-0.52,"w":0.46,"t":0.15,"spokes":1,"segments":10,"repeat":2,"dx":0.84}'),
  ('chassis','box','mat.cast_iron',0,'','{"y":0.2,"w":0.66,"d":1.6,"h":0.1}'),
  ('tub','box','mat.olive_drab',0,'','{"y":0.28,"z":-0.2,"w":0.8,"d":1.1,"h":0.3}'),
  ('stripe','box','mat.cloth',1,'','{"y":0.44,"z":-0.2,"w":0.81,"d":1,"h":0.08,"far":0}'),
  ('gunner','box','mat.cloth',1,'','{"y":0.5,"z":-0.62,"w":0.26,"d":0.18,"h":0.4}'),
  ('gunner_helmet','dome','mat.olive_drab',0,'','{"y":0.9,"z":-0.62,"w":0.24,"h":0.15,"segments":8}'),
  ('gunner_face','box','mat.leather',0,'','{"y":0.86,"z":-0.62,"w":0.15,"d":0.15,"h":0.06,"tint":"#e0b090","far":0}'),
  ('bonnet','box','mat.olive_drab',1,'','{"y":0.28,"z":0.58,"w":0.62,"d":0.52,"h":0.26}'),
  ('grille','box','mat.cast_iron',0,'','{"y":0.3,"z":0.85,"w":0.52,"d":0.03,"h":0.2,"far":0}'),
  ('fenders','box','mat.olive_drab',0,'','{"x":-0.42,"y":0.52,"z":0.56,"w":0.2,"d":0.56,"h":0.03,"repeat":2,"dx":0.84}'),
  ('windscreen','box','mat.glass',0,'','{"y":0.55,"z":0.32,"w":0.72,"d":0.03,"h":0.24,"pitch":-12}'),
  ('seat_backs','box','mat.khaki',0,'','{"x":-0.18,"y":0.5,"z":-0.02,"w":0.24,"d":0.06,"h":0.2,"repeat":2,"dx":0.36,"far":0}'),
  ('driver','box','mat.cloth',1,'','{"x":0.18,"y":0.52,"z":0.1,"w":0.24,"d":0.16,"h":0.26}'),
  ('driver_helmet','dome','mat.olive_drab',0,'','{"x":0.18,"y":0.82,"z":0.1,"w":0.22,"h":0.14,"segments":7}'),
  ('driver_face','box','mat.leather',0,'','{"x":0.18,"y":0.78,"z":0.1,"w":0.14,"d":0.14,"h":0.06,"tint":"#e0b090","far":0}'),
  ('post','cylinder','mat.cast_iron',0,'','{"y":0.5,"z":-0.4,"w":0.06,"h":0.4,"segments":5}'),
  ('gun','cylinder','mat.cast_iron',0,'','{"y":0.92,"z":-0.5,"w":0.05,"h":0.75,"segments":5,"pitch":90}'),
  ('gun_box','box','mat.cast_iron',0,'','{"y":0.88,"z":-0.42,"w":0.1,"d":0.22,"h":0.1,"far":0}'),
  ('spare','cylinder','mat.rubber',0,'','{"y":0.36,"z":-0.76,"w":0.44,"h":0.12,"segments":10,"pitch":90,"far":0}'),
  ('jerrycans','box','mat.olive_drab',0,'','{"x":0.3,"y":0.58,"z":-0.62,"w":0.12,"d":0.14,"h":0.2,"far":0}'),
  ('aerial','cylinder','mat.steel',0,'','{"x":-0.34,"y":0.58,"z":-0.68,"w":0.015,"h":0.9,"segments":3,"roll":-8,"far":0}'),
  ('pennant','box','mat.cloth',1,'aerial','{"y":0.75,"z":-0.08,"w":0.01,"d":0.16,"h":0.1,"far":0}'),
  ('spark','cone','mat.flash',0,'gun','{"y":0.75,"w":0.08,"h":0.08,"segments":3,"flash":4,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.28,"w":0.28,"h":0.28,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.scout_car_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.scout_car'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.scout_car_modern'), (SELECT id FROM def WHERE key='mat.olive_drab'), 100);
